// End-to-end bridge tests run in VS Code's supplied extension host.
const vscode=require('vscode'),fs=require('fs'),path=require('path'),os=require('os'),assert=require('assert/strict');
const {pathToFileURL}=require('url');
const pause=ms=>new Promise(r=>setTimeout(r,ms));
exports.run=async()=>{
 const evidence={status:'passed',host:'VS Code actual preview webview',checks:[]};
 const directory=fs.mkdtempSync(path.join(os.tmpdir(),'laymesh-preview-'));
 const file=path.join(directory,'main.lay'),dependency=path.join(directory,'values.lay');
 const gate=process.env.LAYMESH_PREVIEW_GATE;
 const source='page=canvas(size=(12,9),unit="cm",background="#ffffff")\nimport {top} from "./values.lay"\np=plot(size=(9,6),plot_area=box(offset=(1.5,1),size=(6,4)),x=axis(range=(0,10)),y=axis(range=(-1,1)))\np.line(x=[0,2,4,6,8,10],y=[0,0.6,-0.2,0.8,-0.5,0],line_color="#0072b2",line_width=1.5pt)\npage.add(p,offset=(1,1))\npage.add(rect(size=(top,0.3),fill="#d55e00"),offset=(1,8))\npage.add(text("LayMesh preview",font_size=14pt),offset=(1,0.3))'+(process.env.LAYMESH_PREVIEW_BMP?'\npage.add(image(src='+JSON.stringify(process.env.LAYMESH_PREVIEW_BMP)+'),offset=(10.5,0.3),size=(1,0.5))':'');
 fs.writeFileSync(file,source);fs.writeFileSync(dependency,'export top=3');
 const doc=await vscode.workspace.openTextDocument(file);await vscode.window.showTextDocument(doc);
 const extension=vscode.extensions.getExtension(require('../extensions/vscode/package.json').publisher+'.laymesh-language');
 const api=require('module')._load('vscode',{filename:path.join(extension.extensionPath,'dist/client.cjs')});
 let host,panels=0;const create=api.window.createWebviewPanel;
 api.window.createWebviewPanel=(...args)=>{const panel=create(...args);if(args[0]==='laymeshPreview'){
  panels++;host={panel,messages:[],received:[]};const listen=panel.webview.onDidReceiveMessage,post=panel.webview.postMessage;
  panel.webview.onDidReceiveMessage=(handler,...rest)=>{host.receive=handler;return listen.call(panel.webview,message=>{host.received.push(message);return handler(message)},...rest);};
  panel.webview.postMessage=message=>{host.messages.push(message);return post.call(panel.webview,message);};
 }return panel;};
 const until=async(fn,label,attempts=300)=>{for(let i=0;i<attempts;i++){if(fn())return fn();await pause(50);}throw Error('Timeout: '+label+' '+String(JSON.stringify(host?.messages.slice(-2))).slice(0,1000));};
 const replace=async(document,text)=>{const edit=new vscode.WorkspaceEdit();edit.replace(document.uri,new vscode.Range(document.positionAt(0),document.positionAt(document.getText().length)),text);assert(await vscode.workspace.applyEdit(edit));};
 await extension.activate();await vscode.workspace.getConfiguration('laymesh').update('language','zh-CN',vscode.ConfigurationTarget.Global);
 assert.equal(extension.packageJSON.contributes.commands.find(c=>c.command==='laymesh.openPreview').title,'LayMesh: Open Preview');
 assert(extension.packageJSON.contributes.configuration.properties['laymesh.language'].markdownDescription.includes('color editor'));
 await vscode.workspace.getConfiguration('window').update('autoDetectColorScheme',false,vscode.ConfigurationTarget.Global);
 await vscode.workspace.getConfiguration('workbench').update('colorTheme','VS Code Dark',vscode.ConfigurationTarget.Global);
 await until(()=>vscode.window.activeColorTheme.kind===vscode.ColorThemeKind.Dark,'Dark theme');
 await vscode.commands.executeCommand('workbench.action.closeAuxiliaryBar');
 const math=await import(pathToFileURL(path.join(extension.extensionPath,'dist/preview-math.mjs')).href);
 assert.equal(math.formatData(5),'5.00000');assert.equal(math.formatData(null),'—');
 const close=(a,b)=>assert(Math.abs(a-b)<1e-8,`${a} != ${b}`);
 for(const dpi of [72,96,144,300])for(const [unit,factor] of Object.entries({mm:1,cm:10,in:25.4,inch:25.4,pt:25.4/72,px:25.4/dpi})){
  assert.equal(math.mmPerUnit(unit,dpi),factor);
  for(const rect of [{left:16,top:20,width:400,height:320},{left:-180,top:58,width:600,height:480},{left:2,top:0,width:180,height:144}]){
   const point=math.canvasAt({width:100,height:80,unit,layout_dpi:dpi},rect,rect.left+rect.width*.55,rect.top+rect.height*.5);
   close(point.mm[0],55);close(point.mm[1],40);close(point.display[0],55/factor);close(point.display[1],40/factor);
  }
 }
 const axes={x:{side:'bottom',scale:'linear',segments:[{domain:[0,10],range:[0,100]}]},y:{side:'left',scale:'log',segments:[{domain:[1,100],range:[80,0]}]},signed:{side:'right',scale:'symlog',constant:2,segments:[{domain:[-10,10],range:[80,0]}]}};
 const plot={id:'main',path:'0',plot_area:{x:0,y:0,width:100,height:80},page_transform:[0,2,-3,0,250,10],axes};
 const point=math.apply(plot.page_transform,50,40),hit=math.plotAt(math.preparePlots({plots:[plot]}),point,4);close(hit.values.x,5);close(hit.values.y,10);close(hit.values.signed,0);
 const reversed={scale:'linear',segments:[{domain:[0,10],range:[100,0]}]};close(math.axisValue(reversed,20),8);
 const broken={scale:'linear',segments:[{domain:[0,2],range:[0,40]},{domain:[8,10],range:[60,100]}]};assert.equal(math.axisValue(broken,50),null);close(math.axisValue(broken,80),9);
 const clipped={...plot,clips:[{transform:[1,0,0,1,0,0],rect:{x:0,y:0,width:1,height:1}}]};assert.equal(math.plotAt(math.preparePlots({plots:[clipped]}),point,4),null);
 const inset={...plot,id:'inset'};assert.equal(math.plotAt(math.preparePlots({plots:[plot,inset]}),point,4).plot.id,'inset');
 const base={id:'p',path:'0',plot_area:{x:0,y:0,width:100,height:100},page_transform:[1,0,0,1,0,0],projection:{kind:'polar',center:[50,50],innerRadius:0,outerRadius:40,angleUnit:'deg',thetaZero:Math.PI/2,direction:-1,theta:[0,360],radial:{scale:'log',domain:[1,100],reverse:true}}};
 let p=math.plotAt(math.preparePlots({plots:[base]}),[70,50],4);close(p.values.theta,90);close(p.values.r,10);
 p=math.plotAt(math.preparePlots({plots:[base]}),[50,50],4);assert.equal(p.values.theta,null);close(p.values.r,100);
 const rad={...base,projection:{...base.projection,angleUnit:'rad',theta:[0,Math.PI*2]}};close(math.plotAt(math.preparePlots({plots:[rad]}),[70,50],4).values.theta,Math.PI/2);
 const radar={...base,projection:{kind:'radar',center:[50,50],outerRadius:40,thetaZero:Math.PI/2,direction:-1,categories:['A','B','C','D'],ranges:[[0,10],[10,30],[0,1],[-1,1]],radarFrame:'polygon'}};
 close(math.plotAt(math.preparePlots({plots:[radar]}),[50,30],4).values.A,5);
 assert.deepEqual(math.plotAt(math.preparePlots({plots:[radar]}),[60,40],4).values,{});assert.deepEqual(math.plotAt(math.preparePlots({plots:[radar]}),[50,50],4).values,{});
 assert.equal(math.plotAt(math.preparePlots({plots:[radar]}),[80,20],4),null);
 evidence.checks.push('Units, rotated/nonuniform transforms, extra axes, log/symlog/reversed/broken axes, clipping, overlapping inset, polar deg/rad/reversal/center, radar spokes and polygon boundary');
 await vscode.commands.executeCommand('laymesh.openPreview',doc.uri);
 await until(()=>host?.messages.find(m=>m.svg),'Initial preview');
 await vscode.commands.executeCommand('laymesh.openPreview',doc.uri);assert.equal(panels,1);assert.equal(host.messages.find(m=>m.svg).inspection.page.unit,'cm');
 evidence.checks.push('Open and reuse beside the editor; native canvas unit and SVG');
 if(gate){fs.writeFileSync(gate,JSON.stringify({phase:'open'}));const phase=()=>{try{return JSON.parse(fs.readFileSync(gate)).phase}catch{return null}};
  const renderCount=()=>new Set(host.messages.filter(m=>m.svg).map(m=>m.id)).size;
  await until(()=>phase()==='mouse-start','Mouse measurement gate',2400);await pause(500);const before=renderCount(),version=doc.version;fs.writeFileSync(gate,JSON.stringify({phase:'mouse-ready'}));
  await until(()=>phase()==='ui-verified','UI interaction gate',2400);assert.equal(doc.version,version,'UI gesture must not edit figure source');assert.equal(renderCount(),before,'Mouse must not render; messages: '+JSON.stringify(host.received));evidence.checks.push('Actual webview mouse/zoom/pan/rulers/resize do not compile');
  await vscode.workspace.getConfiguration('laymesh').update('language','en',vscode.ConfigurationTarget.Global);fs.writeFileSync(gate,JSON.stringify({phase:'english-ready'}));await until(()=>phase()==='english-verified','English UI gate',2400);
  await vscode.workspace.getConfiguration('laymesh').update('language','auto',vscode.ConfigurationTarget.Global);assert.equal(vscode.env.language,'en');fs.writeFileSync(gate,JSON.stringify({phase:'auto-ready'}));await until(()=>phase()==='auto-verified','Automatic language gate',2400);
  assert.equal(renderCount(),before,'Changing language must not compile');evidence.checks.push('English manifest commands/settings; explicit and automatic English UI switching without rendering');
  await vscode.workspace.getConfiguration('workbench').update('colorTheme','VS Code Light',vscode.ConfigurationTarget.Global);await until(()=>vscode.window.activeColorTheme.kind===vscode.ColorThemeKind.Light,'Light theme');fs.writeFileSync(gate,JSON.stringify({phase:'light-ready'}));await until(()=>phase()==='light-verified','Light/narrow gate',2400);
 }
 const dependencyDoc=await vscode.workspace.openTextDocument(dependency);let count=host.messages.length;await replace(dependencyDoc,'export top=5');
 let result=await until(()=>host.messages.slice(count).find(m=>m.svg),'Unsaved dependency');assert(fs.readFileSync(dependency,'utf8').includes('top=3'));assert(result.svg!==host.messages.find(m=>m.svg).svg);
 evidence.checks.push('Unsaved imported buffer wins without changing disk');
 count=host.messages.length;await replace(doc,source+'\n???');await until(()=>host.messages.slice(count).find(m=>m.error),'Located compilation failure');
 if(gate){fs.writeFileSync(gate,JSON.stringify({phase:'english-error'}));await until(()=>JSON.parse(fs.readFileSync(gate)).phase==='error-verified','English stale error gate',2400);}
 await host.receive({type:'source'});assert.equal(vscode.window.activeTextEditor.document.uri.toString(),doc.uri.toString());
 count=host.messages.length;await replace(doc,source);await until(()=>host.messages.slice(count).find(m=>m.svg),'Recovery');
 evidence.checks.push('Located error jumps to source; corrected buffer recovers');
 count=host.messages.length;await replace(dependencyDoc,'export top=4');await replace(dependencyDoc,'export top=6');await until(()=>host.messages.slice(count).find(m=>m.svg),'Latest edit');assert.equal(host.messages.slice(count).filter(m=>m.svg).length,1);
 evidence.checks.push('Rapid edits coalesce into the latest version');
 // Hold one completed native response while a newer generation finishes first.
 const {PreviewWorker}=require(path.join(extension.extensionPath,'dist/preview-host.cjs'));
 const enqueue=PreviewWorker.prototype.enqueue;let releaseOlder,hold=true;
 PreviewWorker.prototype.enqueue=function(job){if(hold&&job.file===file){hold=false;const done=job.done;job={...job,done:response=>{releaseOlder=()=>done(response);}};}return enqueue.call(this,job);};
 try{
  await replace(dependencyDoc,'export top=7');await until(()=>releaseOlder,'Held older result');
  count=host.messages.length;await replace(dependencyDoc,'export top=8');await until(()=>host.messages.slice(count).find(m=>m.svg),'Newer result');
  const beforeRelease=host.messages.length;releaseOlder();await pause(100);assert.equal(host.messages.length,beforeRelease,'Older completed result must be discarded');
 }finally{PreviewWorker.prototype.enqueue=enqueue;}
 evidence.checks.push('Older native response arriving after a newer generation is discarded');
 // Test the same persistent transport with a deliberately stalled executable.
 const slow=path.join(directory,'slow.cjs');fs.writeFileSync(slow,"process.stdout.write(JSON.stringify({type:'ready',protocol:1})+'\\n');process.stdin.resume();");
 const binary=path.join(directory,'slow');if(process.platform!=='win32'){
  const quote=s=>"'"+s.replace(/'/g,"'\\''")+"'",marker=path.join(directory,'started');
  fs.writeFileSync(binary,'#!/bin/sh\nif test -f '+quote(marker)+'; then exec '+quote(path.join(extension.extensionPath,'bin','laymesh'))+' preview --stdio; fi\n: > '+quote(marker)+'\nELECTRON_RUN_AS_NODE=1 exec '+quote(process.execPath)+' '+quote(slow)+'\n');fs.chmodSync(binary,0o755);
  const worker=new PreviewWorker(binary,{append(){}});let response;worker.enqueue({file,source,overlays:{},timeout:100,done:r=>response=r});const firstPid=worker.process.pid;await until(()=>response,'Timeout');assert(response.error.message.includes('timed out'));await until(()=>worker.ready,'Automatic restart');assert.notEqual(worker.process.pid,firstPid);
  assert(!response.error.message.includes('超时'));assert(response.error.messageZh.includes('超时'));
  response=null;worker.enqueue({file,source,overlays:{},timeout:30000,done:r=>response=r});await until(()=>response,'Render after automatic restart');assert(response.svg);worker.dispose();assert.equal(worker.process,null);evidence.checks.push('Timeout kills isolated worker, automatically starts a new worker, and subsequent rendering succeeds');
 }
 host.panel.dispose();await pause(100);await vscode.commands.executeCommand('laymesh.openPreview',doc.uri);await until(()=>host.messages.find(m=>m.svg),'Reopen after dispose');assert.equal(panels,2);host.panel.dispose();
 evidence.checks.push('Panel disposal and fresh native worker after reopening');
 await vscode.commands.executeCommand('workbench.action.closeAllEditors');
 fs.writeFileSync(process.env.LAYMESH_PREVIEW_EVIDENCE,JSON.stringify(evidence,null,2));if(gate)fs.writeFileSync(gate,JSON.stringify({phase:'complete'}));fs.rmSync(directory,{recursive:true,force:true});
};
