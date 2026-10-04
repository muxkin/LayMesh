// Native rendering is isolated from the LSP and from the webview UI thread.
const vscode=require('vscode');
const {spawn}=require('child_process');
const path=require('path');
const keyOf=uri=>path.normalize(uri.fsPath).replace(/\\/g,'/');
const messageError=(message,messageZh)=>({code:'E_PREVIEW',message,...(messageZh?{messageZh}:{})});

class PreviewWorker {
 constructor(binary,output){this.binary=binary;this.output=output;this.queue=new Map();this.sequence=0;}
 enqueue(job){this.enabled=true;this.queue.set(job.file,job);this.start();this.pump();}
 start(){
  if(this.process)return;
  const proc=spawn(this.binary,['preview','--stdio'],{stdio:['pipe','pipe','pipe']});
  this.process=proc;this.ready=false;let buffer='',stderr='';
  proc.stdout.setEncoding('utf8');
  this.startTimer=setTimeout(()=>this.fail(messageError('Preview worker did not start; update the configured LayMesh executable','预览进程未启动，请更新 LayMesh 可执行文件')),5000);
  proc.stdout.on('data',data=>{
   if(this.process!==proc)return;buffer+=data.toString();let at;
   while((at=buffer.indexOf('\n'))>=0){const line=buffer.slice(0,at);buffer=buffer.slice(at+1);let result;
    try{result=JSON.parse(line);}catch{this.fail(messageError('Invalid response from LayMesh preview worker','预览进程返回无效数据'));return;}
    if(result.type==='ready'&&result.protocol===1){clearTimeout(this.startTimer);this.ready=true;this.pump();}
    else if(this.active&&result.id===this.active.id){const job=this.active;clearTimeout(this.renderTimer);this.active=null;job.done(result);this.pump();}
   }
  });
  proc.stderr.on('data',data=>{stderr=(stderr+data.toString()).slice(-1200);this.output.append(data.toString());});
  proc.on('error',error=>{if(this.process===proc)this.fail(messageError(error.message));});
  proc.stdin.on('error',error=>{if(this.process===proc)this.fail(messageError(error.message));});
  proc.on('exit',code=>{if(this.process===proc)this.fail(messageError(`Preview worker stopped (${code}). Update laymesh.executable if it lacks preview support${stderr?'\n'+stderr:''}`,`预览进程退出（${code}），请检查 laymesh.executable 的版本${stderr?'\n'+stderr:''}`));});
 }
 pump(){
  if(!this.ready||this.active||!this.queue.size)return;
  const [key,job]=this.queue.entries().next().value;this.queue.delete(key);
  this.active={...job,id:++this.sequence};
  this.renderTimer=setTimeout(()=>{
   const active=this.active;this.stop();
   active.done({error:messageError('Preview timed out; restarting the worker','预览超时，正在重启渲染进程'),dependencies:[]});
   if(this.enabled)this.start();
  },job.timeout);
  this.process.stdin.write(JSON.stringify({...job.request,id:this.active.id,file:job.file,source:job.source,overlays:job.overlays})+'\n');
 }
 cancel(file){this.queue.delete(file);}
 fail(error){
  const jobs=[...(this.active?[this.active]:[]),...this.queue.values()];this.queue.clear();this.stop();
  for(const job of jobs)job.done({error,dependencies:[]});
 }
 stop(){clearTimeout(this.startTimer);clearTimeout(this.renderTimer);const proc=this.process;this.process=null;this.ready=false;this.active=null;if(proc)proc.kill();}
 dispose(){this.enabled=false;this.queue.clear();this.stop();}
}

function activatePreview(context,binary,output){
 const panels=new Map(),worker=new PreviewWorker(binary,output);
 const config=()=>vscode.workspace.getConfiguration('laymesh.preview');
 const locale=()=>{const language=vscode.workspace.getConfiguration('laymesh').get('language','auto');return language==='auto'?vscode.env.language:language;};
 const text=(zh,en)=>/^zh/i.test(locale())?zh:en;
 const relevant=(state,doc)=>doc.uri.scheme==='file'&&(keyOf(doc.uri)===state.file||state.dependencies.has(keyOf(doc.uri)));
 async function exportFigure(uri,provided){
  uri=uri?.scheme?uri:vscode.window.activeTextEditor?.document.uri;
  if(!uri||uri.scheme!=='file'||!uri.fsPath.endsWith('.lay')||!vscode.workspace.isTrusted){
   vscode.window.showInformationMessage(text('LayMesh：请在受信任工作区保存 .lay 文件后导出','LayMesh: save a .lay file in a trusted workspace before exporting'));return;
  }
  try{
   let destination,options;
   if(provided){
    destination=typeof provided.output==='string'?vscode.Uri.file(provided.output):provided.output;
    options=provided.options||{};
    if(!destination||destination.scheme!=='file')throw new Error(text('导出目标必须是本地文件','Export destination must be a local file'));
   }else{
    const formats=[['svg','SVG'],['pdf','PDF'],['png','PNG'],['jpg','JPEG'],['tif','TIFF'],['webp','WebP'],['bmp','BMP'],['gif','GIF'],['ico','ICO'],['pam','PAM / PNM'],['ppm','PPM'],['pgm','PGM'],['pbm','PBM'],['tga','TGA']];
    const format=await vscode.window.showQuickPick(formats.map(([extension,label])=>({label,extension})),{title:text('LayMesh：导出图形','LayMesh: Export Figure'),placeHolder:text('选择导出格式','Choose an export format')});
    if(!format)return;
    options=await exportOptions(format.extension);if(!options)return;
    destination=await vscode.window.showSaveDialog({defaultUri:vscode.Uri.file(uri.fsPath.replace(/\.lay$/,'.'+format.extension)),filters:{[format.label]:[format.extension]},saveLabel:text('导出','Export')});
    if(!destination)return;
    const actual=path.extname(destination.fsPath).toLowerCase();
    const aliases={jpg:['.jpg','.jpeg'],tif:['.tif','.tiff'],pam:['.pam','.pnm']};
    if(!(aliases[format.extension]||['.'+format.extension]).includes(actual))throw new Error(text('文件后缀与选择的格式不一致','File extension does not match the selected format'));
   }
   // Snapshot after the dialogs: export the current unsaved entry and imported buffers.
   const doc=await vscode.workspace.openTextDocument(uri),overlays={};
   for(const buffer of vscode.workspace.textDocuments)if(buffer.uri.scheme==='file'&&buffer.isDirty)overlays[keyOf(buffer.uri)]=buffer.getText();
   const result=await vscode.window.withProgress({location:vscode.ProgressLocation.Notification,title:text('LayMesh：正在导出…','LayMesh: Exporting…')},()=>new Promise(resolve=>{
    const exporter=new PreviewWorker(binary,output);
    exporter.enqueue({file:keyOf(uri),source:doc.getText(),overlays,request:{type:'export',output:keyOf(destination),options},timeout:300000,done:result=>{exporter.dispose();resolve(result);}});
   }));
   if(result.error)throw new Error(result.error.messageZh&&/^zh/i.test(locale())?result.error.messageZh:result.error.message);
   if(!result.exported)throw new Error(text('请更新配置的 LayMesh 可执行文件以支持导出','Update the configured LayMesh executable to support export'));
   if(result.warnings?.length)for(const warning of result.warnings)output.appendLine(JSON.stringify(warning));
   if(!provided)await vscode.window.showInformationMessage(text('已导出：','Exported: ')+destination.fsPath);
   return result;
  }catch(error){if(provided)throw error;vscode.window.showErrorMessage('LayMesh: '+error.message);}
 }
 async function exportOptions(format){
  if(['svg','pdf'].includes(format))return {};
  const settings=vscode.workspace.getConfiguration('laymesh.export'),previous=context.workspaceState.get('export.'+format,{}),options={};
  const number=async(key,prompt,value,min,max,integer=false)=>{
   const answer=await vscode.window.showInputBox({title:text('LayMesh：导出参数','LayMesh: Export Options'),prompt,value:String(previous[key]??value),validateInput:input=>{
    const n=Number(input);return !input.trim()||!Number.isFinite(n)||n<min||n>max||(integer&&!Number.isInteger(n))?text(`请输入 ${min}–${max}${integer?' 的整数':''}`,`Enter ${integer?'an integer':'a number'} from ${min} to ${max}`):undefined;
   }});
   if(answer===undefined)return false;options[key]=Number(answer);return true;
  };
  const choice=async(key,prompt,items,value)=>{
   const preferred=previous[key]??value;
   const chosen=await vscode.window.showQuickPick(items.map(([id,label])=>({label,id})).sort((a,b)=>Number(b.id===preferred)-Number(a.id===preferred)),{title:text('LayMesh：导出参数','LayMesh: Export Options'),placeHolder:prompt});
   if(!chosen)return false;options[key]=chosen.id;return true;
  };
  if(!await number('dpi',text('分辨率 DPI；决定像素尺寸','Resolution in DPI; sets pixel dimensions'),settings.get('dpi',300),0.01,25400))return;
  if(format==='jpg'){
   if(!await number('quality',text('JPEG 质量；越高文件越大','JPEG quality; higher values produce larger files'),settings.get('quality',90),1,100,true))return;
  }
  if(format==='tif'&&!await choice('compression',text('TIFF 压缩方式（均为无损）','TIFF compression (all lossless)'),[['lzw','LZW'],['deflate','Deflate'],['packbits','PackBits'],['none',text('无压缩','Uncompressed')]],settings.get('tiffCompression','lzw')))return;
  if(format==='png'&&!await choice('compression',text('PNG 压缩（均为无损）','PNG compression (all lossless)'),[['default',text('默认','Default')],['fast',text('快速','Fast')],['best',text('更小文件','Smaller file')]],'default'))return;
  if(format==='webp'){
   if(!await choice('webp_lossless',text('WebP 编码模式','WebP encoding mode'),[[true,text('无损','Lossless')],[false,text('有损','Lossy')]],settings.get('webpLossless',true)))return;
   if(!await number('quality',options.webp_lossless?text('无损压缩力度；越高压缩越充分','Lossless compression effort; higher values compress more'):text('WebP 图像质量','WebP image quality'),options.webp_lossless?100:settings.get('quality',90),0,100))return;
   if(!await number('webp_method',text('编码耗时 0–6；越高通常文件越小','Encoding effort 0–6; higher values usually produce smaller files'),4,0,6,true))return;
   if(options.webp_lossless){if(!await number('webp_near_lossless',text('近无损保真度；100 为完全无损','Near-lossless fidelity; 100 is fully lossless'),100,0,100,true))return;}
   else if(!await number('webp_alpha_quality',text('透明通道质量；100 完整保留','Alpha quality; 100 preserves full precision'),100,0,100,true))return;
  }
  if(['jpg','gif','ppm','pgm','pbm'].includes(format)){
   const background=await vscode.window.showInputBox({title:text('LayMesh：导出参数','LayMesh: Export Options'),prompt:text('透明像素的底色 #RRGGBB；GIF 保留完全透明像素','Matte color #RRGGBB; GIF retains fully transparent pixels'),value:previous.background||'#ffffff',validateInput:value=>/^#[\da-f]{6}$/i.test(value)?undefined:text('请输入 #RRGGBB 颜色','Enter a #RRGGBB color')});
   if(background===undefined)return;options.background=background;
  }
  await context.workspaceState.update('export.'+format,options);
  return options;
 }
 function watch(state,dependencies,failed){
  const next=new Set(failed?[...state.dependencies,...dependencies]:dependencies);next.add(state.file);
  if([...next].sort().join('\n')===[...state.dependencies].sort().join('\n'))return;
  state.dependencies=next;for(const item of state.watchers)item.dispose();state.watchers=[];
  for(const file of next){const watcher=vscode.workspace.createFileSystemWatcher(new vscode.RelativePattern(vscode.Uri.file(path.dirname(file)),path.basename(file)));
   for(const event of ['onDidCreate','onDidChange','onDidDelete'])state.watchers.push(watcher[event](()=>invalidate(state)));
   state.watchers.push(watcher);
  }
 }
 function post(state,message){if(!state.disposed)state.panel.webview.postMessage(message);}
 function invalidate(state,immediate=false){
  state.generation++;state.dirty=true;clearTimeout(state.timer);worker.cancel(state.file);
  post(state,{type:'status',status:'pending'});
  if(state.panel.visible)state.timer=setTimeout(()=>render(state),immediate?0:config().get('debounceMs',250));
 }
 async function render(state){
  if(state.disposed||!state.panel.visible||!vscode.workspace.isTrusted)return;
  const generation=state.generation;
  try{
   const doc=await vscode.workspace.openTextDocument(state.uri);
   if(state.disposed||generation!==state.generation)return;
   const overlays={};for(const doc of vscode.workspace.textDocuments)if(doc.uri.scheme==='file'&&doc.isDirty)overlays[keyOf(doc.uri)]=doc.getText();
   state.dirty=false;state.pending=true;post(state,{type:'status',status:'rendering'});
   worker.enqueue({file:state.file,source:doc.getText(),overlays,timeout:config().get('renderTimeoutMs',30000),done:result=>{
    if(state.disposed||generation!==state.generation)return;
    watch(state,result.dependencies||[],!!result.error);
    const frame={type:'result',...result};state.pending=false;state.last=frame;if(result.svg)state.lastSuccess=frame;post(state,frame);
   }});
  }catch(error){if(!state.disposed&&generation===state.generation){state.last={type:'result',error:messageError(error.message)};post(state,state.last);}}
 }
 async function open(uri){
  uri=uri?.scheme?uri:vscode.window.activeTextEditor?.document.uri;
  if(!uri||uri.scheme!=='file'||!uri.fsPath.endsWith('.lay')){vscode.window.showInformationMessage(text('LayMesh：请先保存 .lay 文件再打开预览','LayMesh: save a .lay file before opening preview'));return;}
  if(!vscode.workspace.isTrusted){vscode.window.showInformationMessage(text('LayMesh：请在受信任的工作区使用预览','LayMesh: preview requires a trusted workspace'));return;}
  const file=keyOf(uri),existing=panels.get(file);
  if(existing){existing.panel.reveal(vscode.ViewColumn.Beside,true);return;}
  const panel=vscode.window.createWebviewPanel('laymeshPreview',`LayMesh · ${path.basename(file)}`,{viewColumn:vscode.ViewColumn.Beside,preserveFocus:true},{enableScripts:true,localResourceRoots:[vscode.Uri.joinPath(context.extensionUri,'dist')]});
  const state={uri,file,panel,generation:0,dirty:true,dependencies:new Set(),watchers:[],disposed:false};panels.set(file,state);
  const asset=name=>panel.webview.asWebviewUri(vscode.Uri.joinPath(context.extensionUri,'dist',name));
  panel.webview.html=`<!doctype html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src blob:; script-src ${panel.webview.cspSource}; style-src ${panel.webview.cspSource} 'unsafe-inline';"><link rel="stylesheet" href="${asset('preview.css')}"></head><body><script type="module" src="${asset('preview-webview.mjs')}"></script></body></html>`;
  panel.webview.onDidReceiveMessage(async message=>{
   if(message?.type==='ready'){post(state,{type:'initial',locale:locale(),showRulers:config().get('showRulers',true)});if(state.last?.error&&state.lastSuccess)post(state,state.lastSuccess);if(state.last)post(state,state.last);if(state.dirty)invalidate(state,true);}
   else if(message?.type==='refresh')invalidate(state,true);
   else if(message?.type==='export')await exportFigure(state.uri);
   else if(message?.type==='source'&&state.last?.error){const error=state.last.error,file=error.file||state.file;
    if(file!==state.file&&!state.dependencies.has(file))return;
    try{const doc=await vscode.workspace.openTextDocument(vscode.Uri.file(file));const line=Math.max(0,(error.loc?.line||1)-1),column=Math.max(0,(error.loc?.column||1)-1);const position=doc.validatePosition(new vscode.Position(line,column));await vscode.window.showTextDocument(doc,{viewColumn:vscode.ViewColumn.One,selection:new vscode.Range(position,position)});}catch(error){vscode.window.showErrorMessage(error.message);}
   }
  });
  panel.onDidChangeViewState(()=>{if(panel.visible&&state.dirty)invalidate(state,true);else if(!panel.visible){state.dirty=state.dirty||state.pending;state.generation++;clearTimeout(state.timer);worker.cancel(file);}});
  panel.onDidDispose(()=>{state.disposed=true;clearTimeout(state.timer);worker.cancel(file);for(const item of state.watchers)item.dispose();panels.delete(file);if(!panels.size)worker.dispose();});
  watch(state,[file],false);
 }
 context.subscriptions.push(vscode.commands.registerCommand('laymesh.openPreview',open),vscode.commands.registerCommand('laymesh.exportFigure',exportFigure),
  vscode.workspace.onDidChangeTextDocument(e=>{for(const state of panels.values())if(relevant(state,e.document))invalidate(state);}),
  vscode.workspace.onDidSaveTextDocument(doc=>{for(const state of panels.values())if(relevant(state,doc))invalidate(state);}),
  vscode.workspace.onDidCloseTextDocument(doc=>{for(const state of panels.values())if(relevant(state,doc))invalidate(state);}),
  vscode.workspace.onDidChangeConfiguration(e=>{for(const state of panels.values()){if(e.affectsConfiguration('laymesh.language')||e.affectsConfiguration('laymesh.preview.showRulers'))post(state,{type:'initial',locale:locale(),showRulers:config().get('showRulers',true),forceRulers:e.affectsConfiguration('laymesh.preview.showRulers')});if(e.affectsConfiguration('laymesh.preview'))invalidate(state);}}),
  {dispose(){for(const state of [...panels.values()])state.panel.dispose();worker.dispose();}}
 );
}
module.exports={activatePreview,PreviewWorker};
