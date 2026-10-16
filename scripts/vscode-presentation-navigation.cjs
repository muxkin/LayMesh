// Isolated extension-host harness: the shipped presentation code stays intact.
const vscode=require('vscode'),fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const benchmark=require('./presentation-navigation-benchmark.cjs');
exports.run=async()=>{
 const extension=vscode.extensions.getExtension(require('../extensions/vscode/package.json').publisher+'.laymesh-language');
 const api=require('module')._load('vscode',{filename:path.join(extension.extensionPath,'dist/client.cjs')});
 let host,result,error;const create=api.window.createWebviewPanel;
 api.window.createWebviewPanel=(...args)=>{
  const panel=create(...args);if(args[0]!=='laymeshPreview')return panel;host=panel;
  let proto=panel.webview,descriptor;while(proto&&!descriptor){descriptor=Object.getOwnPropertyDescriptor(proto,'html');proto=Object.getPrototypeOf(proto);}
  assert(descriptor?.set);
  Object.defineProperty(panel.webview,'html',{get:()=>descriptor.get.call(panel.webview),set:html=>{
   const test='const originalAcquire=acquireVsCodeApi;window.acquireVsCodeApi=()=>window.navigationBridge=originalAcquire();'+
    '(async()=>{while(!window.laymeshPresentation?.deck)await new Promise(r=>setTimeout(r,25));'+
    'try{const result=await ('+benchmark.toString()+')(laymeshPresentation,'+JSON.stringify({samples:Number(process.env.SAMPLES||50),visual:false})+');navigationBridge.postMessage({type:"navigation-test-result",result});}'+
    'catch(error){navigationBridge.postMessage({type:"navigation-test-error",error:error.stack||String(error)});}})();';
   html=html.replace('script-src ', "script-src 'nonce-navigation-test' ");
   descriptor.set.call(panel.webview,html.replace('<body>','<body><script nonce="navigation-test">'+test+'</script>'));
  }});
  const listen=panel.webview.onDidReceiveMessage;
  panel.webview.onDidReceiveMessage=(handler,...rest)=>listen.call(panel.webview,message=>{
   if(message.type==='navigation-test-result'){result=message.result;return;}
   if(message.type==='navigation-test-error'){error=message.error;return;}
   return handler(message);
  },...rest);return panel;
 };
 const until=async(fn)=>{const end=Date.now()+1800000;while(!fn()){if(error)throw Error(error);if(Date.now()>end)throw Error('Navigation benchmark timeout');await new Promise(r=>setTimeout(r,25));}};
 await extension.activate();await vscode.commands.executeCommand('workbench.action.closeAuxiliaryBar');
 const doc=await vscode.workspace.openTextDocument(process.env.LAYMESH_PERFORMANCE_SOURCE);await vscode.window.showTextDocument(doc);
 if(process.env.LAYMESH_PERFORMANCE_GATE){fs.writeFileSync(process.env.LAYMESH_PERFORMANCE_GATE,'ready');await until(()=>fs.readFileSync(process.env.LAYMESH_PERFORMANCE_GATE,'utf8')==='start');}
 await vscode.commands.executeCommand('laymesh.openPreview',doc.uri);await until(()=>result);
 const sha=file=>require('node:crypto').createHash('sha256').update(fs.readFileSync(file)).digest('hex');
 result.host='VS Code';result.extensionVersion=extension.packageJSON.version;
 result.nativeSha256=sha(process.env.LAYMESH_PERFORMANCE_BINARY);
 result.wasmSha256=sha(path.join(extension.extensionPath,'dist/wasm',JSON.parse(fs.readFileSync(path.join(extension.extensionPath,'dist/wasm/manifest.json'))).wasm));
 fs.writeFileSync(process.env.LAYMESH_PERFORMANCE_OUTPUT,JSON.stringify(result,null,2));
 host.dispose();await vscode.commands.executeCommand('workbench.action.closeAllEditors');
};
