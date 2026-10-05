// Native rendering is isolated from the LSP and from the webview UI thread.
const vscode=require('vscode');
const {spawn}=require('child_process');
const path=require('path'),fs=require('fs'),os=require('os');
const keyOf=uri=>path.normalize(uri.fsPath).replace(/\\/g,'/');
const messageError=(message,messageZh)=>({code:'E_PREVIEW',message,...(messageZh?{messageZh}:{})});

class PreviewWorker {
 constructor(binary,output){this.binary=binary;this.output=output;this.queue=new Map();this.sequence=0;this.resourceDir=fs.mkdtempSync(path.join(os.tmpdir(),'laymesh-preview-'));}
 enqueue(job){this.enabled=true;this.queue.set(job.file,job);this.start();this.pump();}
 start(){
  if(this.process)return;
  fs.mkdirSync(this.resourceDir,{recursive:true});
  const proc=spawn(this.binary,['preview','--stdio'],{stdio:['pipe','pipe','pipe']});
  this.process=proc;this.ready=false;let buffer='',stderr='';
  proc.stdout.setEncoding('utf8');
  this.startTimer=setTimeout(()=>this.fail(messageError('Preview worker did not start; update the configured LayMesh executable','预览进程未启动，请更新 LayMesh 可执行文件')),5000);
  proc.stdout.on('data',data=>{
   if(this.process!==proc)return;buffer+=data.toString();let at;
   while((at=buffer.indexOf('\n'))>=0){const line=buffer.slice(0,at);buffer=buffer.slice(at+1);let result;
    try{result=JSON.parse(line);}catch{this.fail(messageError('Invalid response from LayMesh preview worker','预览进程返回无效数据'));return;}
    if(result.type==='ready'&&result.protocol===1){clearTimeout(this.startTimer);this.capabilities=result.capabilities||[];this.ready=true;this.pump();}
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
  const request={...job.request};if(request.protocol===2&&!this.capabilities.includes('resources-v2')){delete request.protocol;}
  this.process.stdin.write(JSON.stringify({...request,id:this.active.id,file:job.file,source:job.source,overlays:job.overlays})+'\n');
 }
 cancel(file){this.queue.delete(file);}
 fail(error){
  const jobs=[...(this.active?[this.active]:[]),...this.queue.values()];this.queue.clear();this.stop();
  for(const job of jobs)job.done({error,dependencies:[]});
 }
 stop(){clearTimeout(this.startTimer);clearTimeout(this.renderTimer);const proc=this.process;this.process=null;this.ready=false;this.active=null;if(proc)proc.kill();}
 dispose(){this.enabled=false;this.queue.clear();this.stop();fs.rmSync(this.resourceDir,{recursive:true,force:true});}
}

function editorDefaults(uri){
 const cfg=vscode.workspace.getConfiguration('laymesh',uri),user={},workspace={};
 const mappings={
  'export.dpi':'export.dpi','export.quality':['export.jpeg.quality','export.webp.quality'],
  'export.tiffCompression':'export.tiff.compression','export.pngCompression':'export.png.compression',
  'export.background':['export.jpeg.background','export.gif.background','export.ppm.background','export.pgm.background','export.pbm.background'],
  'export.webpQuality':'export.webp.quality','export.webpLossless':'export.webp.webp_lossless','export.webpMethod':'export.webp.webp_method',
  'export.webpAlphaQuality':'export.webp.webp_alpha_quality','export.webpNearLossless':'export.webp.webp_near_lossless',
  'export.pdf.imageCompression':'export.pdf.pdf_image_compression','export.pdf.jpegQuality':'export.pdf.pdf_jpeg_quality',
  'export.pdf.downsample':'export.pdf.pdf_downsample',
  'export.pdf.recompressJpeg':'export.pdf.pdf_recompress_jpeg','export.pdf.autoPaletteLimit':'export.pdf.pdf_auto_palette_limit',
  'export.pdf.preserve16bit':'export.pdf.pdf_preserve_16bit','export.pdf.preserveAlpha':'export.pdf.pdf_preserve_alpha','export.pdf.alphaBackground':'export.pdf.pdf_alpha_background',
  'export.pdf.autoFlatnessThreshold':'export.pdf.pdf_auto_flatness_threshold',
'preview.jpegQuality':'preview.jpeg_quality','preview.webpQuality':'preview.webp_quality','preview.webpMethod':'preview.webp_method',
  'preview.imageThreads':'preview.image_threads','preview.cacheMb':'preview.cache_mb','preview.processingMemoryMb':'preview.processing_memory_mb'
 };
 const set=(target,key,value)=>{if(value===undefined)return;for(const destination of Array.isArray(key)?key:[key]){const parts=destination.split('.');let parent=target;for(const p of parts.slice(0,-1))parent=parent[p]??={};parent[parts.at(-1)]=value;}};
 for(const [key,destination] of Object.entries(mappings)){const value=cfg.inspect(key);if(value){set(user,destination,value.globalValue);set(workspace,destination,value.workspaceValue);set(workspace,destination,value.workspaceFolderValue);}}
 return {editor_user:user,editor_workspace:workspace};
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
    options={};
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
    exporter.enqueue({file:keyOf(uri),source:doc.getText(),overlays,request:{type:'export',output:keyOf(destination),options,...editorDefaults(uri),...(provided?.config?{config:provided.config}:{})},timeout:300000,done:result=>{exporter.dispose();resolve(result);}});
   }));
   if(result.error)throw new Error(result.error.messageZh&&/^zh/i.test(locale())?result.error.messageZh:result.error.message);
   if(!result.exported)throw new Error(text('请更新配置的 LayMesh 可执行文件以支持导出','Update the configured LayMesh executable to support export'));
   if(result.warnings?.length)for(const warning of result.warnings)output.appendLine(JSON.stringify(warning));
   if(!provided)await vscode.window.showInformationMessage(text('已导出：','Exported: ')+destination.fsPath);
   return result;
  }catch(error){if(provided)throw error;vscode.window.showErrorMessage('LayMesh: '+error.message);}
 }

 function resource(state,item){
  if(!item||!/^[-a-z0-9]+$/.test(item.id)||path.dirname(path.resolve(item.path))!==worker.resourceDir)throw new Error('Invalid preview resource');
  const uri=state.panel.webview.asWebviewUri(vscode.Uri.file(item.path)).toString();state.resources.set(item.id,uri);return {...item,path:undefined,uri};
 }
 function signature(file){try{const stat=fs.statSync(file);return `${stat.mtimeMs}:${stat.ctimeMs}:${stat.size}:${stat.ino}`;}catch{return 'missing';}}
 function watch(state,dependencies,failed){
  const next=new Set(failed?[...state.dependencies,...dependencies]:dependencies);next.add(state.file);
  if([...next].sort().join('\n')===[...state.dependencies].sort().join('\n'))return;
  state.dependencies=next;for(const item of state.watchers)item.dispose();state.watchers=[];
  state.signatures=new Map([...next].map(file=>[file,signature(file)]));
  for(const file of next){const watcher=vscode.workspace.createFileSystemWatcher(new vscode.RelativePattern(vscode.Uri.file(path.dirname(file)),path.basename(file)));
   for(const event of ['onDidCreate','onDidChange','onDidDelete'])state.watchers.push(watcher[event](()=>{state.signatures.set(file,signature(file));invalidate(state);} ));
   state.watchers.push(watcher);
  }
 }
 function post(state,message){if(!state.disposed)state.panel.webview.postMessage(message);}
 function invalidate(state,immediate=false){
  state.generation++;state.dirty=true;clearTimeout(state.timer);worker.cancel(state.file);
  post(state,{type:'status',status:'pending'});
  if(state.panel.visible)state.timer=setTimeout(()=>render(state),immediate?0:config().get('debounceMs',100));
 }
 async function render(state){
  if(state.disposed||!state.panel.visible||!vscode.workspace.isTrusted)return;
  const generation=state.generation;
  try{
   const doc=await vscode.workspace.openTextDocument(state.uri);
   if(state.disposed||generation!==state.generation)return;
   const overlays={};for(const doc of vscode.workspace.textDocuments)if(doc.uri.scheme==='file'&&doc.isDirty)overlays[keyOf(doc.uri)]=doc.getText();
   state.dirty=false;state.pending=true;post(state,{type:'status',status:'rendering'});
   worker.enqueue({file:state.file,source:doc.getText(),overlays,request:{protocol:2,generation,resource_dir:worker.resourceDir,known_resources:[...state.resources.keys()],...editorDefaults(state.uri)},timeout:config().get('renderTimeoutMs',30000),done:result=>{
    if(state.disposed||generation!==state.generation)return;
    watch(state,result.dependencies||[],!!result.error);
    try{
     if(result.protocol===2){result.resources=(result.resources||[]).map(item=>resource(state,item));result.svg=result.svg.replace(/laymesh-resource:([-a-z0-9]+)/g,(_,id)=>{const uri=state.resources.get(id);if(!uri)throw new Error('Missing preview resource');return uri.replace(/&/g,'&amp;');});}
     const frame={type:'result',...result,generation};state.pending=false;state.last=frame;if(result.svg)state.lastSuccess=frame;post(state,frame);
    }catch(error){state.pending=false;post(state,{type:'result',error:messageError(error.message)});}
   }});
  }catch(error){if(!state.disposed&&generation===state.generation){state.last={type:'result',error:messageError(error.message)};post(state,state.last);}}
 }
 async function open(uri){
  uri=uri?.scheme?uri:vscode.window.activeTextEditor?.document.uri;
  if(!uri||uri.scheme!=='file'||!uri.fsPath.endsWith('.lay')){vscode.window.showInformationMessage(text('LayMesh：请先保存 .lay 文件再打开预览','LayMesh: save a .lay file before opening preview'));return;}
  if(!vscode.workspace.isTrusted){vscode.window.showInformationMessage(text('LayMesh：请在受信任的工作区使用预览','LayMesh: preview requires a trusted workspace'));return;}
  const file=keyOf(uri),existing=panels.get(file);
  if(existing){existing.panel.reveal(vscode.ViewColumn.Beside,true);return;}
  const panel=vscode.window.createWebviewPanel('laymeshPreview',`LayMesh · ${path.basename(file)}`,{viewColumn:vscode.ViewColumn.Beside,preserveFocus:true},{enableScripts:true,localResourceRoots:[vscode.Uri.joinPath(context.extensionUri,'dist'),vscode.Uri.file(worker.resourceDir)]});
  const state={uri,file,panel,generation:0,dirty:true,dependencies:new Set(),watchers:[],disposed:false,resources:new Map()};panels.set(file,state);
  const asset=name=>panel.webview.asWebviewUri(vscode.Uri.joinPath(context.extensionUri,'dist',name));
  panel.webview.html=`<!doctype html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src ${panel.webview.cspSource} blob: data:; font-src ${panel.webview.cspSource} data:; script-src ${panel.webview.cspSource}; style-src ${panel.webview.cspSource} 'unsafe-inline';"><link rel="stylesheet" href="${asset('preview.css')}"></head><body><script type="module" src="${asset('preview-webview.mjs')}"></script></body></html>`;
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
  panel.onDidDispose(()=>{state.disposed=true;clearInterval(state.poll);clearTimeout(state.timer);worker.cancel(file);for(const item of state.watchers)item.dispose();panels.delete(file);if(!panels.size)worker.dispose();});
  watch(state,[file],false);
  // Poll exact dependency files as a fallback for absent config directories and
  // operating-system watcher limits, without watching ancestor trees.
  state.poll=setInterval(async()=>{if(state.disposed||state.polling)return;state.polling=true;try{let changed=false;await Promise.all([...state.dependencies].map(async file=>{const before=state.signatures.get(file);let value='missing';try{const stat=await fs.promises.stat(file);value=`${stat.mtimeMs}:${stat.ctimeMs}:${stat.size}:${stat.ino}`;}catch{}if(!state.disposed&&state.dependencies.has(file)&&state.signatures.get(file)===before&&before!==value){state.signatures.set(file,value);changed=true;}}));if(changed&&!state.disposed)invalidate(state);}finally{state.polling=false;}},1000);state.poll.unref?.();
 }
 async function configureDebounce(){
  const value=await vscode.window.showInputBox({
   title:text('预览防抖时间','Preview debounce delay'),
   prompt:text('输入 0–5000 毫秒；0 关闭防抖，默认 100。保存到当前工作区，无工作区时保存到用户设置。','Enter 0–5000 ms; 0 disables debounce, default 100. Saved to this workspace, or user settings when no workspace is open.'),
   value:String(config().get('debounceMs',100)),
   validateInput:value=>/^\d+$/.test(value.trim())&&Number(value)<=5000?undefined:text('请输入 0–5000 的整数','Enter an integer from 0 to 5000')
  });
  if(value!==undefined)await config().update('debounceMs',Number(value),vscode.workspace.workspaceFolders?.length?vscode.ConfigurationTarget.Workspace:vscode.ConfigurationTarget.Global);
 }
 context.subscriptions.push(vscode.commands.registerCommand('laymesh.configurePreviewDebounce',configureDebounce),vscode.commands.registerCommand('laymesh.openPreview',open),vscode.commands.registerCommand('laymesh.exportFigure',exportFigure),
  vscode.workspace.onDidChangeTextDocument(e=>{for(const state of panels.values())if(relevant(state,e.document))invalidate(state);}),
  vscode.workspace.onDidSaveTextDocument(doc=>{for(const state of panels.values())if(relevant(state,doc))invalidate(state);}),
  vscode.workspace.onDidCloseTextDocument(doc=>{for(const state of panels.values())if(relevant(state,doc))invalidate(state);}),
  vscode.workspace.onDidChangeConfiguration(e=>{for(const state of panels.values()){if(e.affectsConfiguration('laymesh.language')||e.affectsConfiguration('laymesh.preview.showRulers'))post(state,{type:'initial',locale:locale(),showRulers:config().get('showRulers',true),forceRulers:e.affectsConfiguration('laymesh.preview.showRulers')});if(e.affectsConfiguration('laymesh.preview')||e.affectsConfiguration('laymesh.export'))invalidate(state);}}),
  {dispose(){for(const state of [...panels.values()])state.panel.dispose();worker.dispose();}}
 );
}
module.exports={activatePreview,PreviewWorker};
