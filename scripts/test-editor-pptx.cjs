// Exercise the real export command and native worker with a small VS Code API stub.
const assert=require('node:assert/strict'),fs=require('node:fs'),os=require('node:os'),path=require('node:path'),vm=require('node:vm');
const root=path.resolve(__dirname,'..');
const binary=path.resolve(process.argv[2]||path.join(root,'target','release',process.platform==='win32'?'laymesh.exe':'laymesh'));
const directory=fs.mkdtempSync(path.join(os.tmpdir(),'laymesh-editor-pptx-'));
const uri=file=>({scheme:'file',fsPath:file});
const main=uri(path.join(directory,'main.lay')),dependency=uri(path.join(directory,'label.lay')),destination=uri(path.join(directory,'figure.pptx'));
fs.writeFileSync(main.fsPath,'page=canvas(size=(64mm,36mm))');fs.writeFileSync(dependency.fsPath,'export label="Disk"');
const docs=[{uri:main,isDirty:true,getText:()=> 'import {label} from "./label.lay"\npage=canvas(size=(64mm,36mm))\npage.add(text(label))'},
 {uri:dependency,isDirty:true,getText:()=> 'export label="Unsaved editor text"'}];
const commands=new Map(),dialogs=[],errors=[],warnings=[],context={subscriptions:[]};
const disposable=()=>({dispose(){}});
const vscode={env:{language:'en'},Uri:{file:uri},ProgressLocation:{Notification:1},
 commands:{registerCommand:(name,fn)=>{commands.set(name,fn);return disposable();}},
 workspace:{isTrusted:true,textDocuments:docs,openTextDocument:async()=>docs[0],
 getConfiguration:()=>({get:(_key,fallback)=>fallback,inspect:()=>undefined}),
 onDidChangeTextDocument:disposable,onDidSaveTextDocument:disposable,onDidCloseTextDocument:disposable,onDidChangeConfiguration:disposable},
 window:{activeTextEditor:{document:docs[0]},
 showQuickPick:async choices=>{dialogs.push('format');assert.equal(choices.length,15);const choice=choices.find(c=>c.extension==='pptx');assert.equal(choice.label,'PowerPoint (PPTX)');return choice;},
 showSaveDialog:async options=>{dialogs.push('destination');assert.equal(options.defaultUri.fsPath,path.join(directory,'main.pptx'));assert.deepEqual(Array.from(options.filters['PowerPoint (PPTX)']),['pptx']);return destination;},
 showInputBox:()=>{throw new Error('Export must not ask for encoding options');},
 showInformationMessage:async()=>{},showErrorMessage:message=>errors.push(message),withProgress:async(_options,fn)=>fn()}};
const sandbox={require:name=>name==='vscode'?vscode:require(name),module:{exports:{}},console,setTimeout,clearTimeout};
vm.runInNewContext(fs.readFileSync(path.join(root,'extensions/vscode/src/preview-host.cjs'),'utf8'),sandbox,{filename:'preview-host.cjs'});
(async()=>{
 try{
  sandbox.module.exports.activatePreview(context,binary,{appendLine:line=>warnings.push(line)});
  const exportFigure=commands.get('laymesh.exportFigure');assert.equal(typeof exportFigure,'function');
  const result=await exportFigure(main);
  assert.deepEqual(dialogs,['format','destination']);assert.deepEqual(errors,[]);
  assert.equal(result.exported,destination.fsPath.replace(/\\/g,'/'));
  assert.equal(result.bytes,fs.statSync(destination.fsPath).size);
  assert.equal(fs.readFileSync(destination.fsPath).subarray(0,2).toString(),'PK');
  assert.ok(result.dependencies.includes(dependency.fsPath.replace(/\\/g,'/')));
  const original=fs.readFileSync(destination.fsPath);
  await assert.rejects(()=>exportFigure(main,{output:destination,options:{quality:90}}));
  assert.deepEqual(fs.readFileSync(destination.fsPath),original);
  console.log('PPTX picker, two-dialog flow, unsaved dependencies, native export and failure protection passed.');
 }finally{for(const subscription of context.subscriptions)subscription.dispose();fs.rmSync(directory,{recursive:true,force:true});}
})().catch(error=>{console.error(error);process.exitCode=1;});
