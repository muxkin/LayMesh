// Native rendering is isolated from the LSP and from the webview UI thread.
const vscode=require('vscode');
const {NativePresentation}=require('./presentation-host.cjs');
const {spawn}=require('child_process');
const path=require('path'),fs=require('fs'),os=require('os');
const keyOf=uri=>path.normalize(uri.fsPath).replace(/\\/g,'/');
const messageError=(message,messageZh)=>({code:'E_PREVIEW',message,...(messageZh?{messageZh}:{})});

// Compact replay history without parsing/copying the complete drawing. A later
// ancestor replacement supersedes descendants; a descendant updates an already
// retained ancestor. The live worker still receives the original native patch.
function compactReplay(previous,frame){
 const update=structuredClone(previous?.displayListUpdate?.update||frame.displayListUpdate.update);
 if(previous){
  const incoming=frame.displayListUpdate.update;
  update.media=structuredClone(incoming.media);update.animation=structuredClone(incoming.animation);update.width=incoming.width;update.height=incoming.height;update.schemaVersion=incoming.schemaVersion;
  if(incoming.resources)update.resources=structuredClone(incoming.resources);
  if(incoming.roots){update.roots=structuredClone(incoming.roots);update.nodes=[];}
  const prefix=(a,b)=>a.length<=b.length&&a.every((v,i)=>v===b[i]);
  const replace=(nodes,path,node)=>{for(const i of path.slice(0,-1)){if(nodes[i]?.kind.kind!=='group')throw Error('Invalid retained drawing path');nodes=nodes[i].kind.children;}if(path.at(-1)>=nodes.length)throw Error('Invalid retained drawing index');nodes[path.at(-1)]=structuredClone(node);};
  for(const change of incoming.nodes){
   if(update.roots){replace(update.roots,change.path,change.node);continue;}
   const ancestor=update.nodes.find(old=>prefix(old.path,change.path));
   if(ancestor){const tail=change.path.slice(ancestor.path.length);if(!tail.length)ancestor.node=structuredClone(change.node);else{if(ancestor.node.kind.kind!=='group')throw Error('Invalid retained drawing ancestor');replace(ancestor.node.kind.children,tail,change.node);}}
   else{update.nodes=update.nodes.filter(old=>!prefix(change.path,old.path));update.nodes.push(structuredClone(change));}
  }
 }
 return {...frame,displayListUpdate:{base:previous?.displayListUpdate.base||frame.displayListUpdate.base,update}};
}

class PreviewWorker {
 constructor(binary,output,resourceRoot=os.tmpdir()){this.binary=binary;this.output=output;this.queue=new Map();this.sequence=0;this.resourceDir=fs.mkdtempSync(path.join(resourceRoot,'laymesh-preview-'));}
 enqueue(job){this.enabled=true;this.queue.set(job.file,job);this.start();this.pump();}
 start(){
  if(this.process)return;
  fs.mkdirSync(this.resourceDir,{recursive:true});
  const proc=spawn(this.binary,['preview','--stdio'],{stdio:['pipe','pipe','pipe']});
  this.process=proc;if(this.background)try{os.setPriority(proc.pid,os.constants.priority.PRIORITY_BELOW_NORMAL);}catch{}this.ready=false;let parts=[],stderr='';
  proc.stdout.setEncoding('utf8');
  this.startTimer=setTimeout(()=>this.fail(messageError('Preview worker did not start; update the configured LayMesh executable','预览进程未启动，请更新 LayMesh 可执行文件')),5000);
  proc.stdout.on('data',data=>{
   if(this.process!==proc)return;let start=0,at;
   // Scan only the incoming chunk. Repeatedly searching a growing 10 MB
   // response flattens ropes and copies hundreds of megabytes per refresh.
   while((at=data.indexOf('\n',start))>=0){parts.push(data.slice(start,at));const line=parts.join('');parts=[];start=at+1;let result;
    try{result=JSON.parse(line);}catch{this.fail(messageError('Invalid response from LayMesh preview worker','预览进程返回无效数据'));return;}
    if(result.type==='ready'&&result.protocol===1){clearTimeout(this.startTimer);this.capabilities=result.capabilities||[];this.ready=true;this.pump();}
    else if(this.active&&result.id===this.active.id){const job=this.active;clearTimeout(this.renderTimer);this.active=null;job.done(result);this.pump();}
   }
   if(start<data.length)parts.push(data.slice(start));
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
  const request={protocol:3,resource_dir:this.resourceDir,...job.request};if(request.protocol===3&&!this.capabilities.includes('display-list-v3')){clearTimeout(this.renderTimer);this.active=null;job.done({error:{code:'E_PREVIEW_PROTOCOL',message:'Native engine does not support drawing protocol 3'},dependencies:[]});this.pump();return;}
  if(request.type==='animate'&&!this.capabilities.includes('animation-v1')){clearTimeout(this.renderTimer);this.active=null;job.done({error:messageError('Update LayMesh to support animation export','请更新 LayMesh 以支持动画导出')});this.pump();return;}
  if((job.source?.includes('@laymesh/')||Object.values(job.overlays||{}).some(s=>s.includes('@laymesh/')))&&!this.capabilities.includes('bundled-stylesheets-v1')){clearTimeout(this.renderTimer);this.active=null;job.done({error:messageError('The configured LayMesh engine lacks bundled stylesheets. Update laymesh.executable or use the engine bundled with the current extension.','配置的 LayMesh 引擎不支持内置样式库，请更新 laymesh.executable 或使用当前扩展内置引擎。')});this.pump();return;}
  if(request.protocol===3&&this.capabilities.includes('display-list-gzip-v3'))request.compressed_display_list=true;
  if(!request.presentation_revision&&request.protocol===3&&this.capabilities.includes('display-list-updates-v3'))request.display_list_updates=true;
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
  'ffmpeg.executable':'export.ffmpeg.executable','ffmpeg.directory':'export.ffmpeg.directory','ffmpeg.searchPaths':'export.ffmpeg.search_paths','ffmpeg.downloadSource':'export.ffmpeg.download_source','ffmpeg.mirrorBaseUrl':'export.ffmpeg.mirror_base_url',
  'export.animation.background':'export.animation.background','export.animation.media':'export.animation.media','export.animation.width':'export.animation.width','export.animation.fps':'export.animation.fps','export.animation.reducedMotion':'export.animation.reduced_motion','export.animation.crf':'export.animation.crf','export.animation.preset':'export.animation.preset','export.dpi':'export.dpi','export.quality':['export.jpeg.quality','export.webp.quality'],
  'export.tiffCompression':'export.tiff.compression','export.pngCompression':'export.png.compression',
  'export.background':['export.jpeg.background','export.gif.background','export.ppm.background','export.pgm.background','export.pbm.background'],
  'export.webpQuality':'export.webp.quality','export.webpLossless':'export.webp.webp_lossless','export.webpMethod':'export.webp.webp_method',
  'export.webpAlphaQuality':'export.webp.webp_alpha_quality','export.webpNearLossless':'export.webp.webp_near_lossless',
  'export.html.embedFonts':'export.html.html_embed_fonts','export.html.imageCompression':'export.html.html_image_compression','export.html.imageQuality':'export.html.html_image_quality','export.pdf.imageCompression':'export.pdf.pdf_image_compression','export.pdf.jpegQuality':'export.pdf.pdf_jpeg_quality',
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
 function ffmpegCommand(command,uri,format,provided={},progress,token,cancelFile){
  return new Promise((resolve,reject)=>{
   const args=['ffmpeg',command,'--file',keyOf(uri),'--format',format,'--json','--settings-stdin'];
   if(provided.config)args.push('--config',String(provided.config));
   if(command==='install')args.push('--yes','--progress-json','--cancel-file',cancelFile);
   const proc=spawn(binary,args,{stdio:['pipe','pipe','pipe'],windowsHide:true,env:{...process.env,LAYMESH_FFMPEG_LOCALE:locale()}});
   let stdout='',stderr='',lines='',previous=0,settled=false;
   const finish=(error,value)=>{if(settled)return;settled=true;cancellation?.dispose();error?reject(error):resolve(value);};
   const cancellation=token?.onCancellationRequested(()=>{try{fs.writeFileSync(cancelFile,'cancel');}catch{}});
   proc.stdout.setEncoding('utf8');proc.stdout.on('data',s=>{stdout+=s;if(stdout.length>1024*1024){proc.kill();finish(Error('Invalid FFmpeg manager response'));}});
   proc.stderr.setEncoding('utf8');proc.stderr.on('data',s=>{
    stderr=(stderr+s).slice(-8000);lines+=s;let end;
    while((end=lines.indexOf('\n'))>=0){const line=lines.slice(0,end);lines=lines.slice(end+1);try{const data=JSON.parse(line);if(data.type==='progress'){const percent=data.total?100*data.bytes/data.total:previous;progress?.report({increment:Math.max(0,percent-previous),message:text('正在下载 FFmpeg…','Downloading FFmpeg…')+` ${(data.bytes/1048576).toFixed(1)} MiB`});previous=percent;}}catch{output.appendLine(line);}}
   });
   proc.on('error',e=>finish(e));proc.stdin.on('error',e=>finish(e));
   proc.on('exit',code=>{if(code!==0){finish(Error(stderr.trim()||text('请更新 LayMesh 引擎以支持 FFmpeg 管理','Update LayMesh to support FFmpeg management')));return;}try{const result=JSON.parse(stdout);if(typeof result.available!=='boolean')throw Error('Invalid FFmpeg status');finish(null,result);}catch(e){finish(e);}});
   proc.stdin.end(JSON.stringify(editorDefaults(uri)));
  });
 }
 async function ensureFFmpeg(uri,destination,provided={}){
  const format=path.extname(destination.fsPath).slice(1).toLowerCase();
  const status=await ffmpegCommand('status',uri,format,provided);
  if(status.available)return true;
  if(!status.download?.supported)throw Error(text('当前平台没有自动下载构建，请配置 laymesh.ffmpeg.executable','No download for this platform; configure laymesh.ffmpeg.executable'));
  const accept=text('下载并继续','Download and continue');
  const source=status.download.source==='auto'?text('自动选择上游或中国镜像','Automatically select upstream or China mirror'):status.download.source;
  const detail=text('将下载到共用私有目录：','Shared private directory: ')+status.directory+'\n'+text('下载发行包：ffmpeg-static b','Download bundle: ffmpeg-static b')+status.download.version+'\n'+text('下载源：','Download source: ')+source+'\n'+text('缺失时才下载，扩展、CLI 和 Python 共用。','Downloads only when missing; shared by VS Code, CLI and Python.');
  const answer=await vscode.window.showWarningMessage(text('未找到可用 FFmpeg，是否下载后继续导出动画？','No usable FFmpeg found. Download it to continue exporting?'),{modal:true,detail},accept);
  if(answer!==accept)return false;
  const cancelFile=path.join(os.tmpdir(),'laymesh-ffmpeg-cancel-'+process.pid+'-'+Date.now());
  try{await vscode.window.withProgress({cancellable:true,location:vscode.ProgressLocation.Notification,title:text('LayMesh：准备 FFmpeg','LayMesh: Prepare FFmpeg')},(progress,token)=>ffmpegCommand('install',uri,format,provided,progress,token,cancelFile));return true;}
  finally{try{fs.unlinkSync(cancelFile);}catch{}}
 }
 async function exportFigure(uri,provided,currentPage){
  uri=uri?.scheme?uri:vscode.window.activeTextEditor?.document.uri;
  if(!uri||uri.scheme!=='file'||! /\.(lay|laydoc)$/.test(uri.fsPath)||!vscode.workspace.isTrusted){
   vscode.window.showInformationMessage(text('LayMesh：请在受信任工作区保存 .lay 或 .laydoc 文件后导出','LayMesh: save a .lay or .laydoc file in a trusted workspace before exporting'));return;
  }
  try{
   let destination,options,animated=!!provided?.animated;
   if(provided){
    destination=typeof provided.output==='string'?vscode.Uri.file(provided.output):provided.output;
    options=provided.options||{};
    if(!destination||destination.scheme!=='file')throw new Error(text('导出目标必须是本地文件','Export destination must be a local file'));
   }else{
    const formats=[['svg','SVG'],['pdf','PDF'],['pptx','PowerPoint (PPTX)'],['png','PNG'],['jpg','JPEG'],['tif','TIFF'],['webp','WebP'],['bmp','BMP'],['gif','GIF'],['ico','ICO'],['pam','PAM / PNM'],['ppm','PPM'],['pgm','PGM'],['pbm','PBM'],['tga','TGA'],['html','HTML 播放版 / Playback',false,false],['html','HTML 可编辑版 / Editable',false,true],['zip','ZIP 播放包 / Playback',false,false],['zip','ZIP 可编辑包 / Editable',false,true],['mp4','MP4 动画 / Animation',true],['gif','GIF 动画 / Animation',true]];
    const format=await vscode.window.showQuickPick(formats.map(([extension,label,animated,editable])=>({label,extension,animated,editable})),{title:text('LayMesh：导出图形','LayMesh: Export Figure'),placeHolder:text('选择导出格式','Choose an export format')});
    if(!format)return;animated=!!format.animated;
    options=format.editable===undefined?{}:{html_editable:format.editable};
    destination=await vscode.window.showSaveDialog({defaultUri:vscode.Uri.file(uri.fsPath.replace(/\.(lay|laydoc)$/,(format.extension==='html'?'.zip.html':'.'+format.extension))),filters:{[format.label]:[format.extension]},saveLabel:text('导出','Export')});
    if(!destination)return;
    const actual=path.extname(destination.fsPath).toLowerCase();
    const aliases={jpg:['.jpg','.jpeg'],tif:['.tif','.tiff'],pam:['.pam','.pnm']};
    if(!(aliases[format.extension]||['.'+format.extension]).includes(actual))throw new Error(text('文件后缀与选择的格式不一致','File extension does not match the selected format'));
   }
   // Snapshot after the dialogs: export the current unsaved entry and imported buffers.
   if(animated&&!await ensureFFmpeg(uri,destination,provided||{}))return;
   const doc=await vscode.workspace.openTextDocument(uri),overlays={};
   for(const buffer of vscode.workspace.textDocuments)if(buffer.uri.scheme==='file'&&buffer.isDirty)overlays[keyOf(buffer.uri)]=buffer.getText();
   const cancelFile=path.join(os.tmpdir(),'laymesh-animation-cancel-'+process.pid+'-'+Date.now());
   const result=await vscode.window.withProgress({cancellable:animated,location:vscode.ProgressLocation.Notification,title:text('LayMesh：正在导出…','LayMesh: Exporting…')},(_progress,token)=>new Promise(resolve=>{
    const cancellation=token.onCancellationRequested(()=>{try{fs.writeFileSync(cancelFile,'cancel');}catch{}});
    const exporter=new PreviewWorker(binary,output);
    exporter.enqueue({file:keyOf(uri),source:doc.getText(),overlays,request:{type:animated?'animate':'export',cancel_file:cancelFile,output:keyOf(destination),options,...(uri.fsPath.endsWith('.laydoc')?{page_id:currentPage||provided?.pageId||panels.get(keyOf(uri))?.pageId,document_all:provided?.documentAll??(/\.(html|zip)$/i.test(destination.fsPath)||(!currentPage&&destination.fsPath.toLowerCase().endsWith('.pdf')))}:{}),...editorDefaults(uri),...(provided?.config?{config:provided.config}:{})},timeout:animated?3600000:300000,done:result=>{cancellation.dispose();try{fs.unlinkSync(cancelFile);}catch{}exporter.dispose();resolve(result);}});
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
 function watch(state,dependencies,failed,deckOnly=false){
  if(!deckOnly)state.previewDependencies=new Set(failed?[...(state.previewDependencies||[]),...dependencies]:dependencies);
  const next=new Set([...state.previewDependencies,...(state.documentPages||[]).map(p=>p.src),...(state.deck?.dependencies||[])]);next.add(state.file);
  if([...next].sort().join('\n')===[...state.dependencies].sort().join('\n'))return;
  state.dependencies=next;for(const item of state.watchers)item.dispose();state.watchers=[];
  state.signatures=new Map([...next].map(file=>[file,signature(file)]));
  for(const file of next){const watcher=vscode.workspace.createFileSystemWatcher(new vscode.RelativePattern(vscode.Uri.file(path.dirname(file)),path.basename(file)));
   for(const event of ['onDidCreate','onDidChange','onDidDelete'])state.watchers.push(watcher[event](()=>{state.signatures.set(file,signature(file));sourceChanged(state,[file]);}));
   state.watchers.push(watcher);
  }
 }
 function invalidateDeck(state,background=false){if(!state.deckPinned&&!state.deckBrowserPinned){state.deck?.dispose();state.deck=null;post(state,{type:'deck-invalidated',background});}else state.deckStale=true;}
 function sourceChanged(state,files){if(files.some(file=>file===state.file||state.previewDependencies?.has(file)))invalidate(state);else invalidateDeck(state,true);}
 function post(state,message){if(!state.disposed)state.panel.webview.postMessage(message);}
 function invalidate(state,immediate=false){
  invalidateDeck(state);
  state.generation++;state.dirty=true;for(const other of panels.values())other.deck?.interruptForPreview();clearTimeout(state.timer);worker.cancel(state.file);
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
   worker.enqueue({file:state.file,source:doc.getText(),overlays,request:{protocol:3,generation,page_id:state.pageId,resource_dir:worker.resourceDir,known_resources:[...state.resources.keys()],known_display_list:state.history.length?state.displayListId:undefined,...editorDefaults(state.uri)},timeout:config().get('renderTimeoutMs',30000),done:result=>{
    if(state.disposed||generation!==state.generation)return;
    if(result.document)state.documentPages=result.document.pages;watch(state,result.dependencies||[],!!result.error);
    if(result.document?.pageId)state.pageId=result.document.pageId;
    try{
     if(!result.error){if(result.protocol!==3||(!result.displayList&&!result.displayListCompressed&&!result.displayListUpdate))throw new Error('Expected drawing protocol 3');for(const item of result.resources||[])resource(state,item);result.resources=Object.entries(result.drawingResources||result.displayList.resources).map(([id,meta])=>{const uri=state.resources.get(id);if(!uri)throw Error('Missing drawing resource');return {id,...meta,data:undefined,uri};});}
     const historyBytes=Buffer.byteLength(JSON.stringify(result));
     if(result.displayListCompressed){const bytes=Buffer.from(result.displayListCompressed.data,'base64');result.displayListCompressed.data=new Uint8Array(bytes.buffer,bytes.byteOffset,bytes.byteLength);}
     const frame={type:'result',...result,generation};state.pending=false;state.last=frame;if(result.displayList||result.displayListCompressed||result.displayListUpdate){state.lastSuccess=frame;
      if(!result.displayListUpdate){state.history=[frame];state.historyBytes=historyBytes;state.replayBytes=0;}
      else{
       const compacted=compactReplay(state.history[1],frame);
       state.history=[state.history[0],compacted];
       state.replayBytes=Buffer.byteLength(JSON.stringify(compacted));
      }
      if(state.historyBytes+(state.replayBytes||0)>(result.previewSettings?.cache_mb??256)*1024*1024){state.history=[];state.historyBytes=0;state.replayBytes=0;state.displayListId=undefined;}
     }post(state,frame);
    }catch(error){state.pending=false;post(state,{type:'result',error:messageError(error.message)});}
   }});
  }catch(error){if(!state.disposed&&generation===state.generation){state.last={type:'result',error:messageError(error.message)};post(state,state.last);}}
 }
 async function open(uri){
  uri=uri?.scheme?uri:vscode.window.activeTextEditor?.document.uri;
  if(!uri||uri.scheme!=='file'||! /\.(lay|laydoc)$/.test(uri.fsPath)){vscode.window.showInformationMessage(text('LayMesh：请先保存 .lay 或 .laydoc 文件再打开预览','LayMesh: save a .lay or .laydoc file before opening preview'));return;}
  if(!vscode.workspace.isTrusted){vscode.window.showInformationMessage(text('LayMesh：请在受信任的工作区使用预览','LayMesh: preview requires a trusted workspace'));return;}
  const file=keyOf(uri),existing=panels.get(file);
  if(existing){existing.panel.reveal(vscode.ViewColumn.Beside,true);return;}
  const panel=vscode.window.createWebviewPanel('laymeshPreview',`LayMesh · ${path.basename(file)}`,{viewColumn:vscode.ViewColumn.Beside,preserveFocus:true},{enableScripts:true,localResourceRoots:[vscode.Uri.joinPath(context.extensionUri,'dist'),vscode.Uri.file(worker.resourceDir)]});
  const state={uri,file,panel,generation:0,dirty:true,dependencies:new Set(),watchers:[],disposed:false,resources:new Map(),history:[],historyBytes:0};panels.set(file,state);
  const asset=name=>panel.webview.asWebviewUri(vscode.Uri.joinPath(context.extensionUri,'dist',name));
  panel.webview.html=`<!doctype html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src ${panel.webview.cspSource} blob: data:; font-src ${panel.webview.cspSource} data:; media-src ${panel.webview.cspSource} blob:; script-src ${panel.webview.cspSource} 'wasm-unsafe-eval'; connect-src ${panel.webview.cspSource} blob:; worker-src blob:; style-src ${panel.webview.cspSource} 'unsafe-inline';"><link rel="stylesheet" href="${asset('preview.css')}"><link rel="stylesheet" href="${asset('presentation.css')}"></head><body><script type="module" src="${asset('preview-webview.mjs')}"></script></body></html>`;
  panel.webview.onDidReceiveMessage(async message=>{
   if(message?.type==='ready'){state.displayListId=undefined;post(state,{type:'initial',locale:locale(),showRulers:config().get('showRulers',true)});
    if(state.history.length&&state.lastSuccess)post(state,{...state.lastSuccess,displayListReplay:state.history});
    if(state.last?.error)post(state,state.last);
    if(state.dirty||!state.history.length)invalidate(state,true);
   }
   else if(message?.type==='deck-prepare'){
    state.deck?.dispose();const createWorker=()=>{const w=new PreviewWorker(binary,output,worker.resourceDir);w.background=true;return w;},compiler=createWorker();
    const deck=state.deck=new NativePresentation({worker:compiler,createWorker,canPrepare:()=>!worker.active&&!worker.queue.size&&![...panels.values()].some(p=>p.panel.visible&&(p.dirty||p.pending)),dist:path.join(context.extensionUri.fsPath,'dist'),uri:file=>state.panel.webview.asWebviewUri(vscode.Uri.file(file)).toString(),notify:data=>{if(state.deck===deck)post(state,{...data,requestId:message.requestId});},onBrowserClose:()=>{state.deckBrowserPinned=false;if(state.deck===deck&&!state.deckPinned){deck.dispose();state.deck=null;state.deckStale=false;post(state,{type:'deck-invalidated'});}}});
    deck.clientRequest=message.requestId;const doc=await vscode.workspace.openTextDocument(state.uri),overlays={};for(const doc of vscode.workspace.textDocuments)if(doc.uri.scheme==='file'&&doc.isDirty)overlays[keyOf(doc.uri)]=doc.getText();
    deck.prepare(file,doc.getText(),overlays,editorDefaults(state.uri)).then(()=>{if(state.deck===deck)watch(state,[],false,true);},e=>{if(state.deck===deck)post(state,{type:'deck-error',error:e.message});});
   }
   else if(message?.type==='deck-ack'&&state.deck?.revision===message.revision){state.deck.ack(message.pageId);}
   else if(message?.type==='deck-priority'&&state.deck?.revision===message.revision){state.deck.priority(message.pageId,message.retry);}
   else if(message?.type==='deck-cancel'&&(!message.requestId||state.deck?.clientRequest===message.requestId)&&(!message.revision||state.deck?.revision===message.revision)){state.deck?.dispose();state.deck=null;}
   else if(message?.type==='deck-pin'){state.deckPinned=true;}
   else if(message?.type==='deck-release'){state.deckPinned=false;if(state.deckStale&&!state.deckBrowserPinned){state.deckStale=false;state.deck?.dispose();state.deck=null;post(state,{type:'deck-invalidated'});}}
   else if(message?.type==='deck-browser'){try{const url=await state.deck.browser();state.deckBrowserPinned=true;await vscode.env.openExternal(vscode.Uri.parse(url));}catch(e){post(state,{type:'deck-browser-error',error:e.message});}}
   else if(message?.type==='painted'&&message.generation===state.generation){state.displayListId=message.displayListId;}
   else if(message?.type==='request-full'){state.displayListId=undefined;invalidate(state,true);}
   else if(message?.type==='refresh')invalidate(state,true);
   else if(message?.type==='export')await exportFigure(state.uri,undefined,state.pageId);
   else if(message?.type==='select-page'&&state.last?.document?.pages.some(page=>page.id===message.pageId)){state.pageId=message.pageId;state.displayListId=undefined;invalidate(state,true);}
   else if(message?.type==='open-page'){const page=state.last?.document?.pages.find(page=>page.id===state.pageId);if(page){const doc=await vscode.workspace.openTextDocument(vscode.Uri.file(page.src));await vscode.window.showTextDocument(doc,{viewColumn:vscode.ViewColumn.One});}}
   else if(message?.type==='export-document'){const destination=await vscode.window.showSaveDialog({defaultUri:vscode.Uri.file(state.file.replace(/\.laydoc$/,'.pdf')),filters:{PDF:['pdf']}});if(destination)await exportFigure(state.uri,{output:destination,documentAll:true});}
   else if(message?.type==='source'&&state.last?.error){const error=state.last.error,file=error.file||state.file;
    if(file!==state.file&&!state.dependencies.has(file))return;
    try{const doc=await vscode.workspace.openTextDocument(vscode.Uri.file(file));const line=Math.max(0,(error.loc?.line||1)-1),column=Math.max(0,(error.loc?.column||1)-1);const position=doc.validatePosition(new vscode.Position(line,column));await vscode.window.showTextDocument(doc,{viewColumn:vscode.ViewColumn.One,selection:new vscode.Range(position,position)});}catch(error){vscode.window.showErrorMessage(error.message);}
   }
  });
  panel.onDidChangeViewState(()=>{if(panel.visible&&state.dirty)invalidate(state,true);else if(!panel.visible){state.dirty=state.dirty||state.pending;state.generation++;clearTimeout(state.timer);worker.cancel(file);}});
  panel.onDidDispose(()=>{state.disposed=true;state.deck?.dispose();clearInterval(state.poll);clearTimeout(state.timer);worker.cancel(file);for(const item of state.watchers)item.dispose();panels.delete(file);if(!panels.size)worker.dispose();});
  watch(state,[file],false);
  // Poll exact dependency files as a fallback for absent config directories and
  // operating-system watcher limits, without watching ancestor trees.
  state.poll=setInterval(async()=>{if(state.disposed||state.polling)return;state.polling=true;try{const changed=[];await Promise.all([...state.dependencies].map(async file=>{const before=state.signatures.get(file);let value='missing';try{const stat=await fs.promises.stat(file);value=`${stat.mtimeMs}:${stat.ctimeMs}:${stat.size}:${stat.ino}`;}catch{}if(!state.disposed&&state.dependencies.has(file)&&state.signatures.get(file)===before&&before!==value){state.signatures.set(file,value);changed.push(file);}}));if(changed.length&&!state.disposed)sourceChanged(state,changed);}finally{state.polling=false;}},1000);state.poll.unref?.();
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
  vscode.workspace.onDidChangeTextDocument(e=>{for(const state of panels.values())if(relevant(state,e.document))sourceChanged(state,[keyOf(e.document.uri)]);}),
  vscode.workspace.onDidSaveTextDocument(doc=>{for(const state of panels.values())if(relevant(state,doc))sourceChanged(state,[keyOf(doc.uri)]);}),
  vscode.workspace.onDidCloseTextDocument(doc=>{for(const state of panels.values())if(relevant(state,doc))sourceChanged(state,[keyOf(doc.uri)]);}),
  vscode.workspace.onDidChangeConfiguration(e=>{for(const state of panels.values()){if(e.affectsConfiguration('laymesh.language')||e.affectsConfiguration('laymesh.preview.showRulers'))post(state,{type:'initial',locale:locale(),showRulers:config().get('showRulers',true),forceRulers:e.affectsConfiguration('laymesh.preview.showRulers')});if(e.affectsConfiguration('laymesh.preview')||e.affectsConfiguration('laymesh.export')||e.affectsConfiguration('laymesh.ffmpeg'))invalidate(state);}}),
  {dispose(){for(const state of [...panels.values()])state.panel.dispose();worker.dispose();}}
 );
}
module.exports={activatePreview,PreviewWorker,compactReplay};
