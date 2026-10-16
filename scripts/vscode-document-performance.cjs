// Actual VS Code: edit shared LCSS or .laydoc and wait for the visible page and
// the ordinary background presentation barrier (all pages/resources accepted).
const vscode=require('vscode'),fs=require('node:fs'),path=require('node:path'),os=require('node:os'),assert=require('node:assert/strict'),cases=require('./semantic-document-cases.cjs');
const pause=ms=>new Promise(r=>setTimeout(r,ms)),quantile=(a,q)=>{const s=[...a].sort((x,y)=>x-y);return q===.5?(s[Math.floor((s.length-1)/2)]+s[Math.floor(s.length/2)])/2:s[Math.ceil(s.length*q)-1];};
exports.run=async()=>{
 const input=process.env.LAYMESH_PERFORMANCE_SOURCE,kind=process.env.LAYMESH_DOCUMENT_CASE;assert(cases.names.includes(kind));
 const dir=fs.mkdtempSync(path.join(os.tmpdir(),'laymesh-doc-perf-'));fs.cpSync(path.dirname(input),dir,{recursive:true});
 const file=path.join(dir,path.basename(input)),original={doc:fs.readFileSync(file,'utf8'),css:fs.readFileSync(path.join(dir,'theme.lcss'),'utf8')};
 const extension=vscode.extensions.getExtension(require('../extensions/vscode/package.json').publisher+'.laymesh-language'),api=require('module')._load('vscode',{filename:path.join(extension.extensionPath,'dist/client.cjs')});
 let host,receive,lastMessage,lastReceived,active={label:'cold',start:Date.now()},frames=new Map(),decks=new Map(),rows=[];
 const create=api.window.createWebviewPanel;
 api.window.createWebviewPanel=(...args)=>{const panel=create(...args);if(args[0]==='laymeshPreview'){
  host=panel;const post=panel.webview.postMessage,listen=panel.webview.onDidReceiveMessage;
  panel.webview.postMessage=message=>{
   lastMessage={type:message.type,error:message.error};
   const frame=frames.get(message.generation);
   if(frame&&message.type==='result'&&!message.error){frame.directoryReceivedMs=Date.now()-frame.start;frame.directory=message.document;frame.incremental=!!message.displayListUpdate;frame.bytes=Buffer.byteLength(JSON.stringify(message));}
   const sample=decks.get(message.requestId);
   if(message.type==='deck-ready'&&sample){sample.documentMs=Date.now()-sample.start;sample.presentationPreparationMs=message.prepareMs;sample.preparation=message.preparation;sample.documentReady=true;}
   if(message.type==='deck-error'&&sample)sample.error=message.error;
   return post.call(panel.webview,message);
  };
  panel.webview.onDidReceiveMessage=(handler,...rest)=>{receive=handler;return listen.call(panel.webview,message=>{
   lastReceived={type:message.type,generation:message.generation};
   if(message.type==='deck-prepare')decks.set(message.requestId,active);
   const frame=frames.get(message.generation);
   if(message.type==='painted'&&frame&&!frame.visible){Object.assign(frame,{visible:true,currentMs:message.at-frame.start,rasterMs:message.rasterMs,installMs:message.installMs,presentationConfirmed:message.presentationConfirmed});}
   return handler(message);
  },...rest);};
 }return panel;};
 await extension.activate();await vscode.commands.executeCommand('workbench.action.closeAuxiliaryBar');
 const {PreviewWorker}=require(path.join(extension.extensionPath,'dist/preview-host.cjs')),enqueue=PreviewWorker.prototype.enqueue;
 PreviewWorker.prototype.enqueue=function(job){if(job.request?.generation!=null&&!job.request.presentation_revision){const frame=active,start=Date.now(),done=job.done;frames.set(job.request.generation,frame);job.done=result=>{frame.nativeMs=Date.now()-start;frame.compileMetrics=result.compileMetrics;return done(result);};}return enqueue.call(this,job);};
 const until=async(fn,label,timeout=180000)=>{const end=Date.now()+timeout;while(Date.now()<end){if(active.error)throw Error(active.error);if(fn())return;await pause(25);}throw Error('Timeout '+label+' '+JSON.stringify({active,lastMessage,lastReceived}));};
 const doc=await vscode.workspace.openTextDocument(file);await vscode.window.showTextDocument(doc);
 if(process.env.LAYMESH_PERFORMANCE_GATE){fs.writeFileSync(process.env.LAYMESH_PERFORMANCE_GATE,'ready');await until(()=>fs.readFileSync(process.env.LAYMESH_PERFORMANCE_GATE,'utf8')==='start','viewport');}
 await vscode.commands.executeCommand('laymesh.openPreview',doc.uri);await until(()=>active.visible&&active.documentReady,'initial document');
 active={label:'selected',start:Date.now()};await receive({type:'select-page',pageId:'editable'});await until(()=>active.visible,'selected page');
 // The existing all-page store is already current; page selection does not
 // necessarily prepare a second deck. Each subsequent source change does.
 const target=await vscode.workspace.openTextDocument(path.join(dir,kind.startsWith('lcss')?'theme.lcss':path.basename(input)));await vscode.window.showTextDocument(target,{viewColumn:vscode.ViewColumn.One});
 const edits=Number(process.env.LAYMESH_PERFORMANCE_EDITS||51);assert(edits>=(process.env.LAYMESH_SMOKE?6:51));
 for(let i=0;i<edits;i++){
  const change=cases.change(kind,i,original.css,original.doc),edit=new vscode.WorkspaceEdit();edit.replace(target.uri,new vscode.Range(target.positionAt(0),target.positionAt(target.getText().length)),change.source);
  active={label:String(i),operation:change.operation||kind,start:Date.now()};assert(await vscode.workspace.applyEdit(edit));await until(()=>active.visible&&active.documentReady,'edit '+i);
  assert.equal(active.presentationConfirmed,true);assert.equal(active.directory.pageId,'editable');if(change.ids)assert.deepEqual(active.directory.pages.map(p=>p.id),change.ids);if(change.name)assert.equal(active.directory.name,change.name);
  if(i)rows.push({...active});
 }
 const summary={kind,samples:rows.length};for(const key of ['directoryReceivedMs','currentMs','documentMs'])summary[key]={median:quantile(rows.map(r=>r[key]),.5),p95:quantile(rows.map(r=>r[key]),.95)};
 const sha=file=>require('node:crypto').createHash('sha256').update(fs.readFileSync(file)).digest('hex');
 fs.writeFileSync(process.env.LAYMESH_PERFORMANCE_OUTPUT,JSON.stringify({host:'VS Code',smoke:!!process.env.LAYMESH_SMOKE,kind,summary,rows,definition:'Source edit to confirmed current page and normal deck-ready barrier after page/resource acknowledgements; 30 ms default input window; one warmup excluded; active editable page; directoryReceivedMs is host metadata receipt, not a screen timestamp',extensionVersion:extension.packageJSON.version,nativeSha256:sha(process.env.LAYMESH_PERFORMANCE_BINARY),sourceSha256:sha(input)},null,2)+'\n');
 host.dispose();await vscode.commands.executeCommand('workbench.action.closeAllEditors');fs.rmSync(dir,{recursive:true,force:true});
};
