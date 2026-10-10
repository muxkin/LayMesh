// Request-to-visible timings in the real extension host and preview webview.
const vscode=require('vscode'),fs=require('fs'),path=require('path'),os=require('os'),assert=require('assert/strict');
const pause=ms=>new Promise(resolve=>setTimeout(resolve,ms));
exports.run=async()=>{
 const root=path.resolve(__dirname,'..'),dir=fs.mkdtempSync(path.join(os.tmpdir(),'laymesh-performance-'));
 const input=process.env.LAYMESH_PERFORMANCE_SOURCE||path.join(root,'examples/effects/images-groups.lay');
 const file=path.join(dir,path.basename(input));
 for(const item of fs.readdirSync(path.dirname(input))){if(item!==path.basename(input))fs.symlinkSync(path.join(path.dirname(input),item),path.join(dir,item));}
 const source=fs.readFileSync(input,'utf8');fs.writeFileSync(file,source);
 const extension=vscode.extensions.getExtension(require('../extensions/vscode/package.json').publisher+'.laymesh-language');
 const api=require('module')._load('vscode',{filename:path.join(extension.extensionPath,'dist/client.cjs')});
 let host,lastMessage,lastReceived;const create=api.window.createWebviewPanel;const frames=new Map(),measurements=new Map();
 api.window.createWebviewPanel=(...args)=>{const panel=create(...args);if(args[0]==='laymeshPreview'){
  host=panel;const post=panel.webview.postMessage,listen=panel.webview.onDidReceiveMessage;
  panel.webview.postMessage=message=>{lastMessage={generation:message.generation,error:message.error,protocol:message.protocol};const frame=frames.get(message.generation);if(frame&&(message.displayList||message.displayListCompressed||message.svg)){const data=message.displayListCompressed?.data;const bytes=data instanceof Uint8Array?data.byteLength+Buffer.byteLength(JSON.stringify({...message,displayListCompressed:{...message.displayListCompressed,data:undefined}})):Buffer.byteLength(JSON.stringify(message));Object.assign(frame,{bytes,resources:message.resources?.length??null,protocol:message.protocol??1});}return post.call(panel.webview,message);};
  panel.webview.onDidReceiveMessage=(handler,...rest)=>listen.call(panel.webview,message=>{lastReceived=message;const frame=frames.get(message.generation);if(message.type==='painted'&&frame?.bytes&&!measurements.has(frame.label)){measurements.set(frame.label,{...frame,resources_ms:message.resourcesMs,install_ms:message.installMs,initialization_wait_ms:message.initializationWaitMs,raster_ms:message.rasterMs,wasm_memory_bytes:message.wasmMemoryBytes,width:message.width,height:message.height,elapsed_ms:message.at-frame.start});}return handler(message)},...rest);
 }return panel;};
 await extension.activate();await vscode.commands.executeCommand('workbench.action.closeAuxiliaryBar');const {PreviewWorker}=require(path.join(extension.extensionPath,'dist/preview-host.cjs'));
 const enqueue=PreviewWorker.prototype.enqueue;PreviewWorker.prototype.enqueue=function(job){const frame={start:Date.now(),label:job.source.match(/# preview benchmark (\d+)/)?.[1]??'cold',generation:job.request?.generation};frames.set(frame.generation,frame);const done=job.done;job.done=result=>{let rss;try{rss=Number(fs.readFileSync('/proc/'+this.process.pid+'/status','utf8').match(/^VmHWM:\s+(\d+)/m)?.[1]);}catch{}Object.assign(frame,{native_ms:Date.now()-frame.start,native_peak_rss_kib:rss});return done(result)};return enqueue.call(this,job);};
 const until=async(fn,label,loops=300)=>{for(let i=0;i<loops;i++){if(fn())return;await pause(50);}throw Error('Timeout '+label+' '+JSON.stringify({lastMessage,lastReceived}));};
 const doc=await vscode.workspace.openTextDocument(file);await vscode.window.showTextDocument(doc);
 if(process.env.LAYMESH_PERFORMANCE_GATE){fs.writeFileSync(process.env.LAYMESH_PERFORMANCE_GATE,'ready');await until(()=>fs.readFileSync(process.env.LAYMESH_PERFORMANCE_GATE,'utf8')==='start','workbench viewport');await pause(200);}
 await vscode.commands.executeCommand('laymesh.openPreview',doc.uri);
 await until(()=>measurements.has('cold'),'cold frame');
 const cold=measurements.get('cold');
 if(process.env.LAYMESH_PERFORMANCE_MOTION){fs.writeFileSync(process.env.LAYMESH_PERFORMANCE_GATE,'motion');await until(()=>fs.readFileSync(process.env.LAYMESH_PERFORMANCE_GATE,'utf8')==='motion-done','motion probe',1800);}
 const replacement=process.env.LAYMESH_PERFORMANCE_REPLACE?JSON.parse(process.env.LAYMESH_PERFORMANCE_REPLACE):null;
 if(replacement)assert(source.includes(replacement[0]),'Replacement must change the drawing source');
 for(let i=0;i<6;i++){const edited=replacement?source.replace(replacement[0],replacement[1].replace('{i}',String(i))):source.replace('Group','Group '+i);const edit=new vscode.WorkspaceEdit();edit.replace(doc.uri,new vscode.Range(doc.positionAt(0),doc.positionAt(doc.getText().length)),edited+'\n# preview benchmark '+i);assert(await vscode.workspace.applyEdit(edit));await until(()=>measurements.has(String(i)),'text refresh '+i);await pause(150);}
 const hot=[1,2,3,4,5].map(i=>measurements.get(String(i)));const sorted=hot.map(v=>v.elapsed_ms).sort((a,b)=>a-b);
 const result={binary:process.env.LAYMESH_PERFORMANCE_BINARY||'bundled',source:input,replacement,cold,hot,hot_median_ms:sorted[2],definition:((cold.protocol===3)?'native request enqueue to resources loaded and WASM Canvas drawn':'native request enqueue to resources loaded and SVG visible')+' in VS Code; excludes edit debounce'};
 const sha=file=>fs.existsSync(file)?require('crypto').createHash('sha256').update(fs.readFileSync(file)).digest('hex'):null;
 result.fixture_sha256=sha(input);result.extension_version=extension.packageJSON.version;
 const selectedBinary=process.env.LAYMESH_PERFORMANCE_BINARY;
 result.artifact_sha256={native:sha(selectedBinary&&selectedBinary!=='bundled'?selectedBinary:path.join(extension.extensionPath,'bin',process.platform==='win32'?'laymesh.exe':'laymesh')),wasm:sha(path.join(extension.extensionPath,'dist/wasm/laymesh_wasm_bg.wasm'))};
 fs.writeFileSync(process.env.LAYMESH_PERFORMANCE_OUTPUT,JSON.stringify(result,null,2));host.dispose();await vscode.commands.executeCommand('workbench.action.closeAllEditors');fs.rmSync(dir,{recursive:true,force:true});
};
