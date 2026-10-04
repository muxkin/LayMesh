// VS Code supplies this extension host. All language analysis runs in Rust.
const vscode=require('vscode');
const {spawn}=require('child_process');
const path=require('path');
const {activatePreview}=require('./preview-host.cjs');
let process_,seq=0,buffer=Buffer.alloc(0),pending=new Map(),output,diagnostics,ready,closed=false;
function send(method,params,id){const data=Buffer.from(JSON.stringify({jsonrpc:'2.0',method,params,...(id===undefined?{}:{id})}));process_.stdin.write(`Content-Length: ${data.length}\r\n\r\n`);process_.stdin.write(data);}
function request(method,params){return new Promise((resolve,reject)=>{const id=++seq;pending.set(id,{resolve,reject});send(method,params,id);});}
function position(p){return new vscode.Position(p.line,p.character)}
function range(r){return new vscode.Range(position(r.start),position(r.end))}
function wireRange(r){return {start:{line:r.start.line,character:r.start.character},end:{line:r.end.line,character:r.end.character}};}
function markdown(value){const m=new vscode.MarkdownString(typeof value==='string'?value:value?.value||'');m.isTrusted=false;m.supportHtml=false;return m;}
function receive(data){buffer=Buffer.concat([buffer,data]);while(true){const header=buffer.indexOf('\r\n\r\n');if(header<0)return;const match=/Content-Length:\s*(\d+)/i.exec(buffer.subarray(0,header).toString());if(!match){buffer=Buffer.alloc(0);return;}const size=Number(match[1]);if(buffer.length<header+4+size)return;const msg=JSON.parse(buffer.subarray(header+4,header+4+size).toString());buffer=buffer.subarray(header+4+size);if(msg.id!==undefined){const p=pending.get(msg.id);if(p){pending.delete(msg.id);msg.error?p.reject(new Error(msg.error.message)):p.resolve(msg.result);}}else if(msg.method==='textDocument/publishDiagnostics'){diagnostics.set(vscode.Uri.parse(msg.params.uri),msg.params.diagnostics.map(d=>{const v=new vscode.Diagnostic(range(d.range),d.message,d.severity===2?vscode.DiagnosticSeverity.Warning:vscode.DiagnosticSeverity.Error);v.code=d.code;v.source=d.source;v._laymesh=d;return v;}));}}}
const params=(doc,pos)=>({textDocument:{uri:doc.uri.toString()},position:pos});
const supported=doc=>['laymesh','lcss'].includes(doc.languageId)&&doc.uri.scheme==='file';
const locale=()=>{const language=vscode.workspace.getConfiguration('laymesh').get('language','auto');return language==='auto'?vscode.env.language:language;};
const localized=(zh,en)=>/^zh/i.test(locale())?zh:en;
function open(doc){if(supported(doc))send('textDocument/didOpen',{textDocument:{uri:doc.uri.toString(),languageId:doc.languageId,version:doc.version,text:doc.getText()}});}
async function activate(context){
 output=vscode.window.createOutputChannel('LayMesh');diagnostics=vscode.languages.createDiagnosticCollection('LayMesh');context.subscriptions.push(output,diagnostics);
 const configured=vscode.workspace.getConfiguration('laymesh').get('executable');const binary=configured||path.join(context.extensionPath,'bin',process.platform==='win32'?'laymesh.exe':'laymesh');
 activatePreview(context,binary,output);
 process_=spawn(binary,['lsp','--stdio'],{stdio:['pipe','pipe','pipe']});process_.stdout.on('data',receive);process_.stderr.on('data',data=>output.append(data.toString()));process_.on('error',error=>{vscode.window.showErrorMessage(`LayMesh: ${error.message}. Set laymesh.executable to the native Rust CLI.`);for(const p of pending.values())p.reject(error);pending.clear();});process_.on('exit',code=>{if(!closed)output.appendLine(`Language server exited (${code})`);for(const p of pending.values())p.reject(new Error('LayMesh server stopped'));pending.clear();});
 ready=request('initialize', {
  processId: process.pid,
  rootUri: vscode.workspace.workspaceFolders?.[0]?.uri.toString(),
  workspaceFolders: vscode.workspace.workspaceFolders?.map(folder=>({uri:folder.uri.toString(),name:folder.name})),
  locale: vscode.env.language,
  initializationOptions: {language:vscode.workspace.getConfiguration('laymesh').get('language')},
  capabilities: {workspace:{workspaceFolders:true,workspaceEdit:{documentChanges:true}},textDocument: {
   completion: {completionItem:{documentationFormat:['markdown']}},
   hover: {contentFormat:['markdown']},
   signatureHelp: {signatureInformation:{documentationFormat:['markdown'],parameterInformation:{labelOffsetSupport:true}}}
  }}
 });
 await ready;send('initialized',{});vscode.workspace.textDocuments.forEach(open);
 const select=[{scheme:'file',language:'laymesh'},{scheme:'file',language:'lcss'}];const kinds={3:vscode.CompletionItemKind.Function,10:vscode.CompletionItemKind.Property,6:vscode.CompletionItemKind.Variable,14:vscode.CompletionItemKind.Keyword,7:vscode.CompletionItemKind.Class,1:vscode.CompletionItemKind.Text};
 context.subscriptions.push(vscode.workspace.onDidOpenTextDocument(open),vscode.workspace.onDidChangeTextDocument(e=>{if(supported(e.document))send('textDocument/didChange',{textDocument:{uri:e.document.uri.toString(),version:e.document.version},contentChanges:e.contentChanges.map(c=>({range:wireRange(c.range),text:c.text}))});}),vscode.workspace.onDidCloseTextDocument(doc=>{if(supported(doc))send('textDocument/didClose',{textDocument:{uri:doc.uri.toString()}});}),vscode.workspace.onDidSaveTextDocument(doc=>{if(supported(doc))send('textDocument/didSave',{textDocument:{uri:doc.uri.toString()}});}),vscode.workspace.onDidChangeConfiguration(e=>{if(e.affectsConfiguration('laymesh.language'))send('workspace/didChangeConfiguration',{settings:{laymesh:{language:vscode.workspace.getConfiguration('laymesh').get('language')}}});}),
 vscode.languages.registerCompletionItemProvider(select,{async provideCompletionItems(doc,pos){const list=await request('textDocument/completion',params(doc,pos));return(list||[]).map(c=>{const item=new vscode.CompletionItem(c.label,kinds[c.kind]);item.detail=c.detail;item.documentation=markdown(c.documentation);item.insertText=c.textEdit.newText;item.range=range(c.textEdit.range);return item;});}},'.','(',':','_','"',"'",'=',','),
 vscode.languages.registerHoverProvider(select,{async provideHover(doc,pos){const h=await request('textDocument/hover',params(doc,pos));return h?new vscode.Hover(markdown(h.contents),range(h.range)):undefined;}}),
 vscode.languages.registerSignatureHelpProvider(select,{async provideSignatureHelp(doc,pos){const result=await request('textDocument/signatureHelp',params(doc,pos));if(!result)return;const help=new vscode.SignatureHelp();help.activeSignature=result.activeSignature;help.activeParameter=result.activeParameter??0;help.signatures=result.signatures.map(s=>{const sig=new vscode.SignatureInformation(s.label,markdown(s.documentation));sig.parameters=s.parameters.map(p=>new vscode.ParameterInformation(p.label,markdown(p.documentation)));return sig;});return help;}}),
 vscode.languages.registerDefinitionProvider(select,{async provideDefinition(doc,pos){const d=await request('textDocument/definition',params(doc,pos));return d?new vscode.Location(vscode.Uri.parse(d.uri),range(d.range)):undefined;}}),
 vscode.languages.registerReferenceProvider(select,{async provideReferences(doc,pos,options,token){const list=await request('textDocument/references',{...params(doc,pos),context:{includeDeclaration:options.includeDeclaration}});if(token?.isCancellationRequested)return;return(list||[]).map(d=>new vscode.Location(vscode.Uri.parse(d.uri),range(d.range)));}}),
 vscode.languages.registerDocumentHighlightProvider(select,{async provideDocumentHighlights(doc,pos,token){const list=await request('textDocument/documentHighlight',params(doc,pos));if(token?.isCancellationRequested)return;return(list||[]).map(d=>new vscode.DocumentHighlight(range(d.range),d.kind===3?vscode.DocumentHighlightKind.Write:d.kind===2?vscode.DocumentHighlightKind.Read:vscode.DocumentHighlightKind.Text));}}),
 vscode.languages.registerRenameProvider(select,{
  async prepareRename(doc,pos){const prepared=await request('textDocument/prepareRename',params(doc,pos));if(!prepared)throw Error(localized('此处没有可安全重命名的变量；请先补完整受影响的代码','No binding can safely be renamed here; complete the affected code first'));return {range:range(prepared.range),placeholder:prepared.placeholder};},
  async provideRenameEdits(doc,pos,newName){
   const version=doc.version,result=await request('textDocument/rename',{...params(doc,pos),textDocument:{uri:doc.uri.toString(),version},newName});
   if(doc.version!==version)throw Error(localized('文档已变化，请重新发起重命名','Document changed; request rename again'));
   const edit=new vscode.WorkspaceEdit();
   if(result.documentChanges){
    for(const change of result.documentChanges){
     const target=vscode.Uri.parse(change.textDocument.uri),openDoc=vscode.workspace.textDocuments.find(d=>d.uri.toString()===target.toString());
     if(openDoc&&change.textDocument.version!==null&&openDoc.version!==change.textDocument.version)throw Error(localized('目标文档已变化，请重新发起重命名','A target document changed; request rename again'));
     for(const changeEdit of change.edits)edit.replace(target,range(changeEdit.range),changeEdit.newText);
    }
   }else for(const [uri,changes]of Object.entries(result.changes||{}))for(const change of changes)edit.replace(vscode.Uri.parse(uri),range(change.range),change.newText);
   return edit;
  }
 }),
 vscode.languages.registerCodeActionsProvider(select,{async provideCodeActions(doc,r,ctx){const actions=await request('textDocument/codeAction',{textDocument:{uri:doc.uri.toString()},range:wireRange(r),context:{diagnostics:ctx.diagnostics.map(d=>d._laymesh).filter(Boolean)}});return(actions||[]).map(a=>{const action=new vscode.CodeAction(a.title,vscode.CodeActionKind.QuickFix);const edit=new vscode.WorkspaceEdit();for(const [uri,changes]of Object.entries(a.edit.changes))for(const c of changes)edit.replace(vscode.Uri.parse(uri),range(c.range),c.newText);action.edit=edit;return action;});}},{providedCodeActionKinds:[vscode.CodeActionKind.QuickFix]}));
 context.subscriptions.push(vscode.languages.registerColorProvider(select,{
  async provideDocumentColors(doc){const list=await request('textDocument/documentColor',{textDocument:{uri:doc.uri.toString()}});return(list||[]).map(c=>new vscode.ColorInformation(range(c.range),new vscode.Color(c.color.red,c.color.green,c.color.blue,c.color.alpha)));},
  async provideColorPresentations(color,ctx){const list=await request('textDocument/colorPresentation',{textDocument:{uri:ctx.document.uri.toString()},range:wireRange(ctx.range),color:{red:color.red,green:color.green,blue:color.blue,alpha:color.alpha}});return(list||[]).map(p=>{const item=new vscode.ColorPresentation(p.label);item.textEdit=new vscode.TextEdit(range(p.textEdit.range),p.textEdit.newText);return item;});}
 }));
 context.subscriptions.push(vscode.commands.registerCommand('laymesh.editColor',async()=>{
  const editor=vscode.window.activeTextEditor;if(!editor||!supported(editor.document))return;
  const doc=editor.document,uri=doc.uri.toString(),version=doc.version,at=doc.offsetAt(editor.selection.active);
  const colors=await request('laymesh/documentColors',{textDocument:{uri}});const color=colors.find(c=>c.from<=at&&at<=c.to);if(!color){vscode.window.showInformationMessage(localized('LayMesh：将光标放到颜色上','LayMesh: place the cursor in a color'));return;}
  const selected=new vscode.Range(doc.positionAt(color.from),doc.positionAt(color.to)),old=doc.getText(selected);
  const panel=vscode.window.createWebviewPanel('laymeshColor',localized('LayMesh 颜色','LayMesh Color'),vscode.ViewColumn.Beside,{enableScripts:true,localResourceRoots:[vscode.Uri.joinPath(context.extensionUri,'dist')]});
  const module=panel.webview.asWebviewUri(vscode.Uri.joinPath(context.extensionUri,'dist','color-webview.mjs'));
  panel.webview.html=`<!doctype html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><meta http-equiv="Content-Security-Policy" content="default-src 'none'; script-src ${panel.webview.cspSource}; style-src 'unsafe-inline';"></head><body><script type="module" src="${module}"></script></body></html>`;
  panel.webview.onDidReceiveMessage(async message=>{try{
   if(message.method==='ready'){panel.webview.postMessage({type:'initial',color,locale:locale()});return;}
   if(message.method==='close'){panel.dispose();return;}
   let result;
   if(message.method==='convert'){result=await request('laymesh/colorConvert',message.params);}
   else if(message.method==='apply'){
    if(doc.isClosed||doc.version!==version||doc.getText(selected)!==old)throw Error(localized('文档已变化，请重新打开选色器','Document changed; reopen the color editor'));
    if(message.params.unchanged){panel.webview.postMessage({id:message.id,result:true});return;}
    const [red,green,blue,alpha]=message.params.rgba;const list=await request('textDocument/colorPresentation',{textDocument:{uri},range:wireRange(selected),color:{red,green,blue,alpha}});const space=message.params.space;const choice=list.find(p=>p.label.startsWith(space==='hex'?'#':space+'('));if(!choice)throw Error(localized('不支持的颜色格式','Unsupported color format'));
    let text=choice.textEdit.newText;
    if(space!=='hex'&&message.params.originalSpace===space){const nums=message.params.channels.map(n=>String(n));const body=color.format==='constructor'?`${space}(${nums.join(', ')}, alpha=${alpha})`:`${space}(${nums.join(' ')} / ${alpha})`;text=color.format==='string'?`${color.quote}${body}${color.quote}`:body;}
    if(doc.version!==version)throw Error(localized('文档已变化，请重新打开选色器','Document changed; reopen the color editor'));const edit=new vscode.WorkspaceEdit();edit.replace(doc.uri,selected,text);result=await vscode.workspace.applyEdit(edit);
   }else return;
   panel.webview.postMessage({id:message.id,result});
  }catch(error){panel.webview.postMessage({id:message.id,error:error.message});}});
 }));
 const watcher=vscode.workspace.createFileSystemWatcher('**/*.{lay,lcss}');for(const [event,type]of [['onDidCreate',1],['onDidChange',2],['onDidDelete',3]])context.subscriptions.push(watcher[event](uri=>send('workspace/didChangeWatchedFiles',{changes:[{uri:uri.toString(),type}]})));context.subscriptions.push(watcher);
 context.subscriptions.push(vscode.workspace.onDidChangeWorkspaceFolders(event=>send('workspace/didChangeWorkspaceFolders',{event:{added:event.added.map(folder=>({uri:folder.uri.toString(),name:folder.name})),removed:event.removed.map(folder=>({uri:folder.uri.toString(),name:folder.name}))}})));
}
async function deactivate(){closed=true;if(process_){try{await Promise.race([request('shutdown',null),new Promise(resolve=>setTimeout(resolve,1000))]);send('exit');}catch{}process_.kill();}}
module.exports={activate,deactivate};
