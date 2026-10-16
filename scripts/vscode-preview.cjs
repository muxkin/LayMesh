// End-to-end bridge tests run in VS Code's supplied extension host.
const vscode=require('vscode'),fs=require('fs'),path=require('path'),os=require('os'),assert=require('assert/strict');
const {pathToFileURL}=require('url');
const pause=ms=>new Promise(r=>setTimeout(r,ms));
const drawings=new Map();
let applyDisplayUpdate;
const observed=message=>{
 let list=message.displayList;
 if(message.displayListCompressed){
  const data=message.displayListCompressed.data;
  const bytes=require('zlib').gunzipSync(typeof data==='string'?Buffer.from(data,'base64'):Buffer.from(data));
  assert.equal(bytes.length,message.displayListCompressed.uncompressedBytes);
  assert.equal(require('crypto').createHash('sha256').update(bytes).digest('hex'),message.displayListCompressed.id);
  list=JSON.parse(bytes.toString('utf8'));
 }else if(message.displayListUpdate){
  const {base,update}=message.displayListUpdate;assert(drawings.has(base),'Patch must name an observed CPU baseline');
  list=applyDisplayUpdate(drawings.get(base),update);
 }
 if(!list)return message;
 if(message.displayListId)drawings.set(message.displayListId,list);
 return {...message,displayList:list};
};
exports.run=async()=>{
 const evidence={status:'passed',host:'VS Code actual preview webview',checks:[]};
 const directory=fs.mkdtempSync(path.join(os.tmpdir(),'laymesh-preview-'));
 const file=path.join(directory,'main.lay'),dependency=path.join(directory,'values.lay');
 const gate=process.env.LAYMESH_PREVIEW_GATE;
 let source='page=canvas(size=(12,9),unit="cm",background="#ffffff")\nimport {top} from "./values.lay"\np=plot(size=(9,6),plot_area=box(offset=(1.5,1),size=(6,4)),x=axis(range=(0,10)),y=axis(range=(-1,1)))\np.line(x=[0,2,4,6,8,10],y=[0,0.6,-0.2,0.8,-0.5,0],line_color="#0072b2",line_width=1.5pt)\npage.add(p,offset=(1,1))\npage.add(rect(size=(top,0.3),fill="#d55e00"),offset=(1,8))\npage.add(text("LayMesh preview",font_size=14pt),offset=(1,0.3))'+(process.env.LAYMESH_PREVIEW_BMP?'\npage.add(image(src='+JSON.stringify(process.env.LAYMESH_PREVIEW_BMP)+'),offset=(10.5,0.3),size=(1,0.5))':'');
 if(process.env.LAYMESH_PREVIEW_TRANSPARENT)source+='\npage.add(image(src='+JSON.stringify(process.env.LAYMESH_PREVIEW_TRANSPARENT)+'),offset=(11.6,0.3),size=(0.3,0.5))';
 if(process.env.LAYMESH_PREVIEW_16BIT)source+='\npage.add(image(src='+JSON.stringify(process.env.LAYMESH_PREVIEW_16BIT)+'),offset=(10.5,1),size=(1,0.5))';
 fs.writeFileSync(file,source);fs.writeFileSync(dependency,'export top=3');
 const doc=await vscode.workspace.openTextDocument(file);await vscode.window.showTextDocument(doc);
 const extension=vscode.extensions.getExtension(require('../extensions/vscode/package.json').publisher+'.laymesh-language');
 ({applyDisplayUpdate}=await import(pathToFileURL(path.join(extension.extensionPath,'dist/display-update.mjs')).href));
 const api=require('module')._load('vscode',{filename:path.join(extension.extensionPath,'dist/client.cjs')});
 let host,panels=0;const create=api.window.createWebviewPanel;
 api.window.createWebviewPanel=(...args)=>{const panel=create(...args);if(args[0]==='laymeshPreview'){
  panels++;const previewHost=host={panel,messages:[],received:[]};const listen=panel.webview.onDidReceiveMessage,post=panel.webview.postMessage;
  panel.webview.onDidReceiveMessage=(handler,...rest)=>{previewHost.receive=handler;return listen.call(panel.webview,message=>{previewHost.received.push(message);return handler(message)},...rest);};
  panel.webview.postMessage=message=>{previewHost.messages.push(observed(message));return post.call(panel.webview,message);};
 }return panel;};
 const until=async(fn,label,attempts=600)=>{for(let i=0;i<attempts;i++){if(fn())return fn();await pause(50);}throw Error('Timeout: '+label+' '+String(JSON.stringify(host?.messages.slice(-8).map(m=>({type:m.type,error:m.error,previewSettings:m.previewSettings,dependencies:m.dependencies,generation:m.generation})))).slice(0,4000));};
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
 await until(()=>host?.messages.find(m=>m.displayList),'Initial preview');
 await vscode.commands.executeCommand('laymesh.openPreview',doc.uri);assert.equal(panels,1);assert.equal(host.messages.find(m=>m.displayList).inspection.page.unit,'cm');
 evidence.checks.push('Open and reuse beside the editor; native canvas unit and Canvas');
 if(gate){fs.writeFileSync(gate,JSON.stringify({phase:'open'}));const phase=()=>{try{return JSON.parse(fs.readFileSync(gate)).phase}catch{return null}};
  const renderCount=()=>new Set(host.messages.filter(m=>m.displayList).map(m=>m.id)).size;
  await until(()=>phase()==='mouse-start','Mouse measurement gate',2400);await pause(500);const before=renderCount(),version=doc.version;fs.writeFileSync(gate,JSON.stringify({phase:'mouse-ready'}));
  await until(()=>phase()==='ui-verified','UI interaction gate',2400);assert.equal(doc.version,version,'UI gesture must not edit figure source');assert.equal(renderCount(),before,'Mouse must not render; messages: '+JSON.stringify(host.received));evidence.checks.push('Actual webview mouse/zoom/pan/rulers/resize do not compile');
  await vscode.workspace.getConfiguration('laymesh').update('language','en',vscode.ConfigurationTarget.Global);fs.writeFileSync(gate,JSON.stringify({phase:'english-ready'}));await until(()=>phase()==='english-verified','English UI gate',2400);
  await vscode.workspace.getConfiguration('laymesh').update('language','auto',vscode.ConfigurationTarget.Global);assert.equal(vscode.env.language,'en');fs.writeFileSync(gate,JSON.stringify({phase:'auto-ready'}));await until(()=>phase()==='auto-verified','Automatic language gate',2400);
  assert.equal(renderCount(),before,'Changing language must not compile');evidence.checks.push('English manifest commands/settings; explicit and automatic English UI switching without rendering');
  await vscode.workspace.getConfiguration('workbench').update('colorTheme','VS Code Light',vscode.ConfigurationTarget.Global);await until(()=>vscode.window.activeColorTheme.kind===vscode.ColorThemeKind.Light,'Light theme');fs.writeFileSync(gate,JSON.stringify({phase:'light-ready'}));await until(()=>phase()==='light-verified','Light/narrow gate',2400);
 }
 const configuration=path.join(directory,'.laymesh.json');let configCount=host.messages.length;
 fs.writeFileSync(configuration,JSON.stringify({preview:{jpeg_quality:88}}));
 await until(()=>host.messages.slice(configCount).find(m=>m.previewSettings?.jpeg_quality===88),'Created project config');
 configCount=host.messages.length;fs.unlinkSync(configuration);await until(()=>host.messages.slice(configCount).find(m=>m.previewSettings?.jpeg_quality===90),'Removed project config');
 evidence.checks.push('Created/deleted project configuration refreshes effective preview quality');
 if(process.env.LAYMESH_PREVIEW_BMP){const asset=process.env.LAYMESH_PREVIEW_BMP,original=fs.readFileSync(asset),changed=Buffer.from(original),offset=changed.readUInt32LE(10);changed[offset]^=4;
  const identity=m=>m.displayList?.nodes.find(n=>n.kind.kind==='image')?.kind.resource;
  const before=identity(host.messages.filter(m=>m.displayList).at(-1));assert(before);const count=host.messages.length;fs.writeFileSync(asset,changed);await until(()=>host.messages.slice(count).find(m=>identity(m)&&identity(m)!==before),'Changed bitmap on disk');
  const changedCount=host.messages.length;fs.writeFileSync(asset,original);await until(()=>host.messages.slice(changedCount).find(m=>identity(m)===before),'Restored bitmap on disk');evidence.checks.push('Disk bitmap modification changes resource identity and restores correctly');
 }
 const dependencyDoc=await vscode.workspace.openTextDocument(dependency);let count=host.messages.length;await replace(dependencyDoc,'export top=5');
 let result=await until(()=>host.messages.slice(count).find(m=>m.displayList),'Unsaved dependency');assert(fs.readFileSync(dependency,'utf8').includes('top=3'));assert(JSON.stringify(result.displayList)!==JSON.stringify(host.messages.find(m=>m.displayList).displayList));
 evidence.checks.push('Unsaved imported buffer wins without changing disk');
 const inverseSource=source.replace('size=(12,9)','size=(12,12*7/3cm)');
 count=host.messages.length;await replace(doc,inverseSource);const inverseError=await until(()=>host.messages.slice(count).find(m=>m.error),'Located unit failure');
 assert.equal(inverseError.error.code,'E_UNIT');assert(inverseError.error.message.includes('12cm*7/3'));assert(inverseError.error.message.includes('(12*7)/(3cm)'));
 if(gate){fs.writeFileSync(gate,JSON.stringify({phase:'english-error'}));await until(()=>JSON.parse(fs.readFileSync(gate)).phase==='error-verified','English stale error gate',2400);}
 await host.receive({type:'source'});assert.equal(vscode.window.activeTextEditor.document.uri.toString(),doc.uri.toString());
 count=host.messages.length;await replace(doc,source);await until(()=>host.messages.slice(count).find(m=>m.displayList),'Recovery');
 evidence.checks.push('Inverse-unit error explains the actual expression, jumps to source, and corrected buffer recovers');
 // Use the workbench's real composition commands and document changes.
 await vscode.window.showTextDocument(doc,{viewColumn:vscode.ViewColumn.One});
 await pause(100);count=host.messages.length;
 await vscode.commands.executeCommand('compositionStart',{});
 await replace(doc,source+'\n# 中');await pause(120);
 await replace(doc,source+'\n# 中文输入');await pause(120);
 assert.equal(host.messages.slice(count).filter(m=>m.type==='result').length,0,'IME intermediate buffers must not be published');
 await vscode.commands.executeCommand('compositionEnd',{});
 await until(()=>host.messages.slice(count).find(m=>m.displayList),'IME commit');
 count=host.messages.length;await replace(doc,source);await until(()=>host.messages.slice(count).find(m=>m.displayList),'IME restoration');
 evidence.checks.push('VS Code composition boundaries pause provisional buffers and publish the committed version');
 count=host.messages.length;await replace(dependencyDoc,'export top=4');await replace(dependencyDoc,'export top=6');await until(()=>host.messages.slice(count).find(m=>m.displayList),'Latest edit');assert.equal(host.messages.slice(count).filter(m=>m.displayList).length,1);
 evidence.checks.push('Rapid edits coalesce into the latest version');
 // Hold one completed native response while a newer generation finishes first.
 const {PreviewWorker,retainPreviewFrame}=require(path.join(extension.extensionPath,'dist/preview-host.cjs'));
 // Replay composition must handle child/ancestor replacement, root-count
 // changes and resource replacement without growing with the edit count.
 const leaf=value=>({kind:{kind:'path',value}}),group=children=>({kind:{kind:'group',children}});
 let expected=[group([leaf(0),group([leaf(1),leaf(2)])]),leaf(3)];
 const baseline={displayListId:'baseline',displayList:{schemaVersion:1,width:10,height:10,nodes:structuredClone(expected),resources:{}}};drawings.set('baseline',baseline.displayList);
 let compacted=baseline;
 const changes=[{path:[0],node:group([leaf(4),group([leaf(5),leaf(6)])])},{path:[0,1,0],node:leaf(7)},{path:[0,1],node:group([leaf(8),leaf(9)])},{path:[1],node:leaf(10)}];
 for(let i=0;i<24;i++){
  const change=changes[i%changes.length],update={schemaVersion:1,width:10,height:10,nodes:[change],roots:null,resources:i===5?{new:{mime:'image/png',data:[]}}:null};
  const frame={displayListId:'composed-'+i,displayListUpdate:{base:i?'composed-'+(i-1):'baseline',update}};
  compacted=retainPreviewFrame(compacted,frame,applyDisplayUpdate);const sequential=observed(frame).displayList;
  assert.deepEqual(observed(compacted).displayList,sequential);assert.equal(compacted.displayListUpdate,undefined);
  assert.equal(compacted.displayList.nodes.length,2);
 }
 const roots={displayListId:'roots',displayListUpdate:{base:compacted.displayListId,update:{schemaVersion:1,width:12,height:10,nodes:[],roots:[group([leaf(11)])],resources:{}}}};
 compacted=retainPreviewFrame(compacted,roots,applyDisplayUpdate);assert.deepEqual(observed(compacted).displayList,observed(roots).displayList);
 const child={displayListId:'child',displayListUpdate:{base:'roots',update:{schemaVersion:1,width:12,height:10,nodes:[{path:[0,0],node:leaf(12)}],roots:null,resources:null}}};
 compacted=retainPreviewFrame(compacted,child,applyDisplayUpdate);assert.deepEqual(observed(compacted).displayList,observed(child).displayList);
 evidence.checks.push('24 replay updates retain one current scene; ancestor/descendant, root-count and resource replacement validated');
 const enqueue=PreviewWorker.prototype.enqueue;let releaseOlder,hold=true;
 PreviewWorker.prototype.enqueue=function(job){if(hold&&job.file===file&&!job.request.presentation_revision){hold=false;const done=job.done;job={...job,done:response=>{releaseOlder=()=>done(response);}};}return enqueue.call(this,job);};
 try{
  await replace(dependencyDoc,'export top=7');await until(()=>releaseOlder,'Held older result');
  count=host.messages.length;await replace(dependencyDoc,'export top=8');await until(()=>host.messages.slice(count).find(m=>m.displayList),'Newer result');
  const beforeRelease=host.messages.filter(m=>m.type==='result').length;releaseOlder();await pause(100);assert.equal(host.messages.filter(m=>m.type==='result').length,beforeRelease,'Older completed preview result must be discarded');
 }finally{PreviewWorker.prototype.enqueue=enqueue;}
 evidence.checks.push('Older native response arriving after a newer generation is discarded');
 // Test the same persistent transport with a deliberately stalled executable.
 const slow=path.join(directory,'slow.cjs');fs.writeFileSync(slow,"process.stdout.write(JSON.stringify({type:'ready',protocol:1,capabilities:['display-list-v3']})+'\\n');process.stdin.resume();");
 const binary=path.join(directory,'slow');if(process.platform!=='win32'){
  const quote=s=>"'"+s.replace(/'/g,"'\\''")+"'",marker=path.join(directory,'started');
  fs.writeFileSync(binary,'#!/bin/sh\nif test -f '+quote(marker)+'; then exec '+quote(path.join(extension.extensionPath,'bin','laymesh'))+' preview --stdio; fi\n: > '+quote(marker)+'\nELECTRON_RUN_AS_NODE=1 exec '+quote(process.execPath)+' '+quote(slow)+'\n');fs.chmodSync(binary,0o755);
  const worker=new PreviewWorker(binary,{append(){}});let response;worker.enqueue({file,source,overlays:{},timeout:100,done:r=>response=r});const firstPid=worker.process.pid;await until(()=>response,'Timeout');assert(response.error.message.includes('timed out'));await until(()=>worker.ready,'Automatic restart');assert.notEqual(worker.process.pid,firstPid);
  assert(!response.error.message.includes('超时'));assert(response.error.messageZh.includes('超时'));
  response=null;worker.enqueue({file,source,overlays:{},timeout:30000,done:r=>response=r});await until(()=>response,'Render after automatic restart');assert(observed(response).displayList);worker.dispose();assert.equal(worker.process,null);evidence.checks.push('Timeout kills isolated worker, automatically starts a new worker, and subsequent rendering succeeds');
 }
 host.panel.dispose();await pause(100);await vscode.commands.executeCommand('laymesh.openPreview',doc.uri);await until(()=>host.messages.find(m=>m.displayList),'Reopen after dispose');assert.equal(panels,2);host.panel.dispose();
 evidence.checks.push('Panel disposal and fresh native worker after reopening');
 // A document keeps ordinary, independently editable .lay sources. Exercise
 // the real selector through the controller, then the host's dirty buffers.
 const bookFile=path.join(directory,'report.laydoc'),coverFile=path.join(directory,'cover.lay'),resultsFile=path.join(directory,'results.lay'),summaryFile=path.join(directory,'summary.lay');
 const coverSource='p=canvas(size=(80,40),background="#fff")\np.add(text("Cover",font_size=16),offset=(4,6))';
 const resultsSource='p=canvas(size=(8,4),unit="cm",background="#fff")\np.add(formula(source="E=mc^2",font_size=18),offset=(0.4,0.6))';
 const summarySource='p=canvas(size=(80,40),background="#fff")\np.add(text("Summary",font_size=16),offset=(4,6))';
 const bookSource='d=document(name="Report"'+(process.env.LAYMESH_PREVIEW_TRANSITIONS?',transition=transition(name="InvertedPageCurl",duration=250ms)':'')+')\ncover=page(src="cover.lay",id="cover")\nresults=page(src="results.lay",id="results")\nsummary=page(src="summary.lay",id="summary")\nfor p in [cover,results,summary] { d.add(p) }';
 fs.writeFileSync(bookFile,bookSource);fs.writeFileSync(coverFile,coverSource);fs.writeFileSync(resultsFile,resultsSource);fs.writeFileSync(summaryFile,summarySource);
 const bookDoc=await vscode.workspace.openTextDocument(bookFile);await vscode.window.showTextDocument(bookDoc);assert.equal(bookDoc.languageId,'laymesh');
 await vscode.commands.executeCommand('laymesh.openPreview',bookDoc.uri);await until(()=>host.messages.find(m=>m.document?.pageId==='cover'&&m.displayList),'Document preview');
 const documentPhase=async(value,next)=>{if(gate){fs.writeFileSync(gate,JSON.stringify({phase:value}));await until(()=>{try{return JSON.parse(fs.readFileSync(gate)).phase===next}catch{return false}},next,2400);}};
 if(gate)await documentPhase('document-ready','document-results');else await host.receive({type:'select-page',pageId:'results'});
 result=await until(()=>host.messages.find(m=>m.document?.pageId==='results'&&m.displayList),'Selected results page');assert.equal(result.inspection.page.unit,'cm');
 const resultsDoc=await vscode.workspace.openTextDocument(resultsFile);count=host.messages.length;
 await replace(resultsDoc,resultsSource.replace('E=mc^2','E=mc^3'));result=await until(()=>host.messages.slice(count).find(m=>m.document?.pageId==='results'&&m.displayList),'Unsaved selected page');
 assert.equal(fs.readFileSync(resultsFile,'utf8'),resultsSource);await documentPhase('document-dirty-ready','document-dirty-verified');
 const coverDoc=await vscode.workspace.openTextDocument(coverFile);count=host.messages.length;await replace(coverDoc,coverSource.replace('Cover','Unsaved cover'));await pause(350);
 assert.equal(host.messages.slice(count).filter(m=>m.displayList).length,0,'Inactive page edits must not compile the selected page');
 count=host.messages.length;await replace(bookDoc,bookSource.replace('[cover,results,summary]','[summary,cover,results]'));
 result=await until(()=>host.messages.slice(count).find(m=>m.document?.pageId==='results'&&m.document.pageNumber===3&&m.displayList),'Reordered stable page ID');
 await documentPhase('document-reordered','document-ui-verified');
 if(gate)await until(()=>vscode.window.activeTextEditor?.document.uri.fsPath===resultsFile,'Edit selected page action');
 const documentPdf=path.join(directory,'document.pdf'),selectedPdf=path.join(directory,'selected.pdf');
 await vscode.commands.executeCommand('laymesh.exportFigure',bookDoc.uri,{output:documentPdf,documentAll:true});
 await vscode.commands.executeCommand('laymesh.exportFigure',bookDoc.uri,{output:selectedPdf,documentAll:false,pageId:'results'});
 assert(fs.readFileSync(documentPdf).includes(Buffer.from('/Count 3')));assert(fs.readFileSync(selectedPdf).includes(Buffer.from('/Count 1')));
 evidence.checks.push('.laydoc language association; real page selection; independent cm units and formula; unsaved selected page; inactive page edits do not compile; stable ID survives reordering; edit-page action; whole/selected vector PDF');
 host.panel.dispose();
 const motionFile=path.join(directory,'animation.lay');
 const motionSource='p=canvas(size=(80,40),stylesheet=["@laymesh/bootstrap.css","@laymesh/open-props.css","@laymesh/animate.css"],background="#fff")\nstyle{.custom-motion{animation-duration:2s}}\na=p.add(rect(size=(25,15),class="bg-primary rounded shadow"),offset=(20,10))\ns=p.step(name="Entrance",hold=500ms)\ns.animate(a,class="animate__animated animate__fadeIn custom-motion")\ns2=p.step(name="Pulse",hold=500ms)\ns2.animate(a,class="animate__animated animate__pulse custom-motion")';
 fs.writeFileSync(motionFile,motionSource);const motionDoc=await vscode.workspace.openTextDocument(motionFile);await vscode.window.showTextDocument(motionDoc);await vscode.commands.executeCommand('laymesh.openPreview',motionDoc.uri);
 const motion=await until(()=>host.messages.find(m=>m.inspection?.animation),'Animation preview');const beforePlayback=host.messages.filter(m=>m.displayList).length;
 assert.equal(motion.inspection.animation.steps[0].duration,2);assert.equal(motion.inspection.animation.steps[1].duration,2);
 assert(!motion.inspection.warnings.some(w=>w.code==='W_ANIMATION'));
 await documentPhase('animation-ready','animation-verified');assert.equal(host.messages.filter(m=>m.displayList).length,beforePlayback,'Playback must not request another layout');
 const animationSettings=vscode.workspace.getConfiguration('laymesh',motionDoc.uri);await animationSettings.update('export.animation.width',160,vscode.ConfigurationTarget.Global);await animationSettings.update('export.animation.fps',5,vscode.ConfigurationTarget.Global);await animationSettings.update('export.animation.background','#f5f0e8',vscode.ConfigurationTarget.Global);
 const video=path.join(directory,'animation.mp4');await vscode.commands.executeCommand('laymesh.exportFigure',motionDoc.uri,{output:video,animated:true});assert(fs.statSync(video).size>500);
 const videoProbe=JSON.parse(require('child_process').execFileSync('ffprobe',['-v','error','-show_streams','-of','json',video]));assert.equal(videoProbe.streams[0].width,160);assert.equal(videoProbe.streams[0].r_frame_rate,'5/1');
 evidence.checks.push('Animate.css preview controls, deterministic seeking, static restoration, no playback recompilation, editor MP4 export');
 const verifyFFmpegDecline=gate&&process.env.LAYMESH_PREVIEW_NORMAL_HOST?async()=>{
  const privateDirectory=path.join(directory,'ffmpeg-private'),cancelledVideo=path.join(directory,'cancelled.mp4'),ffmpegConfig=path.join(directory,'ffmpeg-test.json');
  fs.writeFileSync(ffmpegConfig,JSON.stringify({export:{ffmpeg:{directory:privateDirectory,search_system:false}}}));
  const declined=vscode.commands.executeCommand('laymesh.exportFigure',motionDoc.uri,{output:cancelledVideo,animated:true,config:ffmpegConfig});
  await documentPhase('ffmpeg-dialog-ready','ffmpeg-dialog-cancelled');assert.equal(await declined,undefined);
  assert(!fs.existsSync(privateDirectory));assert(!fs.existsSync(cancelledVideo));
  evidence.checks.push('Actual missing-FFmpeg modal identifies private directory and source; declining creates no installation or output');
 }:null;
 host.panel.dispose();
 if(process.env.LAYMESH_ACADEMIC_DOC){
  const academicDoc=await vscode.workspace.openTextDocument(process.env.LAYMESH_ACADEMIC_DOC);await vscode.window.showTextDocument(academicDoc);await vscode.commands.executeCommand('laymesh.openPreview',academicDoc.uri);
  await until(()=>host.messages.find(m=>m.document?.pages.length===32&&m.displayList),'Academic preview');
  await documentPhase('academic-ready','academic-verified');
  evidence.checks.push('32-page academic deck: prepared native artifacts, embedded wheel navigation, dense charts/formulas and nonresident revisit');host.panel.dispose();
 }
 // Keep two previews alive, close the newer one, then recreate the older
 // webview. Each worker must get resources from its own window throughout.
 const simpleFile=path.join(directory,'simple-line.lay');
 fs.writeFileSync(simpleFile,'p=canvas(size=(100mm,75mm),background="#fff")\np.add(line(length=55mm,angle=20deg,line_width=2mm),offset=(12mm,15mm))');
 const simpleDoc=await vscode.workspace.openTextDocument(simpleFile);await vscode.window.showTextDocument(simpleDoc);
 await vscode.commands.executeCommand('laymesh.openPreview',simpleDoc.uri);const first=host;
 await until(()=>first.received.some(m=>m.type==='painted'),'Simple line preview');
 await vscode.window.showTextDocument(doc);await vscode.commands.executeCommand('laymesh.openPreview',doc.uri);const second=host;
 await until(()=>second.received.some(m=>m.type==='painted'),'Second preview with image resources');
 second.panel.dispose();first.panel.reveal(vscode.ViewColumn.Beside,true);
 const paints=first.received.filter(m=>m.type==='painted').length;first.panel.webview.html+='\n<!-- resource ownership reload -->';
 await until(()=>first.received.filter(m=>m.type==='painted').length>paints,'Older preview reload after newer panel closes');first.panel.dispose();
 evidence.checks.push('Two concurrent previews; simple line and image resources; older webview reloads after closing the newer panel');
 if(verifyFFmpegDecline)await verifyFFmpegDecline();
 // Normal hosts ask to save dirty editors on close. Only persist disposable
 // fixture buffers after all unsaved-buffer assertions have completed.
 if(process.env.LAYMESH_PREVIEW_NORMAL_HOST)for(const doc of vscode.workspace.textDocuments)if(doc.isDirty&&doc.uri.fsPath.startsWith(directory+path.sep))await doc.save();
 await vscode.commands.executeCommand('workbench.action.closeAllEditors');
 fs.writeFileSync(process.env.LAYMESH_PREVIEW_EVIDENCE,JSON.stringify(evidence,null,2));if(gate)fs.writeFileSync(gate,JSON.stringify({phase:'complete'}));fs.rmSync(directory,{recursive:true,force:true});
};
