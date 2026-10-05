// Request-to-visible timings in the real extension host and preview webview.
const vscode=require('vscode'),fs=require('fs'),path=require('path'),os=require('os'),assert=require('assert/strict');
const pause=ms=>new Promise(resolve=>setTimeout(resolve,ms));
exports.run=async()=>{
 const root=path.resolve(__dirname,'..'),dir=fs.mkdtempSync(path.join(os.tmpdir(),'laymesh-performance-'));
 const demo=path.join(root,'video/04-demos/main-figure'),file=path.join(dir,'figure.lay');
 fs.cpSync(path.join(demo,'inputs'),path.join(dir,'inputs'),{recursive:true});
 const source=fs.readFileSync(path.join(demo,'figure.lay'),'utf8');fs.writeFileSync(file,source);
 const extension=vscode.extensions.getExtension(require('../extensions/vscode/package.json').publisher+'.laymesh-language');
 const api=require('module')._load('vscode',{filename:path.join(extension.extensionPath,'dist/client.cjs')});
 let host,activeStart,activePayload,lastMessage,lastReceived;const create=api.window.createWebviewPanel;const measurements=[];
 api.window.createWebviewPanel=(...args)=>{const panel=create(...args);if(args[0]==='laymeshPreview'){
  host=panel;const post=panel.webview.postMessage,listen=panel.webview.onDidReceiveMessage;
  panel.webview.postMessage=message=>{lastMessage={...message,svg:message.svg?.slice(0,120),inspection:undefined,resources:message.resources?.map(r=>({mime:r.mime,uri:r.uri}))};if(message.svg)activePayload={bytes:Buffer.byteLength(JSON.stringify(message)),resources:message.resources?.length??null,protocol:message.protocol??1};return post.call(panel.webview,message);};
  panel.webview.onDidReceiveMessage=(handler,...rest)=>listen.call(panel.webview,message=>{lastReceived=message;if(message.type==='painted'&&activeStart){measurements.push({...activePayload,elapsed_ms:message.at-activeStart});activeStart=undefined;}return handler(message)},...rest);
 }return panel;};
 await extension.activate();await vscode.commands.executeCommand('workbench.action.closeAuxiliaryBar');const {PreviewWorker}=require(path.join(extension.extensionPath,'dist/preview-host.cjs'));
 const enqueue=PreviewWorker.prototype.enqueue;PreviewWorker.prototype.enqueue=function(job){activeStart=Date.now();return enqueue.call(this,job);};
 const until=async(fn,label)=>{for(let i=0;i<300;i++){if(fn())return;await pause(50);}throw Error('Timeout '+label+' '+JSON.stringify({lastMessage,lastReceived}));};
 const doc=await vscode.workspace.openTextDocument(file);await vscode.window.showTextDocument(doc);await vscode.commands.executeCommand('laymesh.openPreview',doc.uri);
 await until(()=>measurements.length===1,'cold frame');
 const cold=measurements[0];
 for(let i=0;i<6;i++){const edit=new vscode.WorkspaceEdit();edit.replace(doc.uri,new vscode.Range(doc.positionAt(0),doc.positionAt(doc.getText().length)),source.replace('LayMesh / images','LayMesh '+i+' / images'));assert(await vscode.workspace.applyEdit(edit));await until(()=>measurements.length===i+2,'text refresh');await pause(150);}
 const hot=measurements.slice(2);const sorted=hot.map(v=>v.elapsed_ms).sort((a,b)=>a-b);
 const result={binary:process.env.LAYMESH_PERFORMANCE_BINARY||'bundled',source:'video/04-demos/main-figure/figure.lay',cold,hot,hot_median_ms:sorted[2],definition:'native request enqueue to full images decoded and visible in VS Code; excludes edit debounce'};
 fs.writeFileSync(process.env.LAYMESH_PERFORMANCE_OUTPUT,JSON.stringify(result,null,2));host.dispose();await vscode.commands.executeCommand('workbench.action.closeAllEditors');fs.rmSync(dir,{recursive:true,force:true});
};
