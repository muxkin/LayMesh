import {mmPerUnit,canvasAt,niceStep,preparePlots,plotAt,formatData} from './preview-math.mjs';

const bridge=typeof acquireVsCodeApi==='function'?acquireVsCodeApi():{getState:()=>null,setState(){},postMessage(){}};
const saved=bridge.getState()||{};
let view={mode:'fit',scale:1,pan:{x:0,y:0},rulers:true,...saved},page,plots=[],locale='en',lastPointer=null,drag,space=false,frame=0,diagnostic,initialized=false;
const el=(tag,className,parent,text)=>{const node=document.createElement(tag);if(className)node.className=className;if(text)node.textContent=text;parent?.append(node);return node;};
const root=el('main','preview',document.body),toolbar=el('div','toolbar',root);
const buttons={};for(const name of ['refresh','export','fit','actual','out','in','rulers']){const button=el('button','',toolbar);button.type='button';button.dataset.action=name;buttons[name]=button;}
const zoom=el('output','zoom',toolbar),status=el('span','status',toolbar);
const workspace=el('div','workspace',root),corner=el('span','corner',workspace),hr=el('canvas','ruler horizontal',workspace),vr=el('canvas','ruler vertical',workspace),viewport=el('div','viewport',workspace);
viewport.tabIndex=0;viewport.setAttribute('role','region');
const figure=el('div','figure',viewport),overview=el('canvas','figure-overview',figure);overview.hidden=true;overview.setAttribute('aria-hidden','true');overview.style.pointerEvents='none';let overviewContent='';
let image=el('canvas','figure-pixels',figure);image.style.position='absolute';image.setAttribute('role','img');
let rasterWorker,rasterReady,workerSequence=0,wasmMemoryBytes=0,drawReady=false,drawJobId='',drawKey='',drawGeneration=0,presented,pendingPaint,paintInFlight=false,paintTimer=0;
const workerCalls=new Map();
function stopWorker(error){rasterWorker?.terminate();rasterWorker=null;rasterReady=null;drawReady=false;for(const call of workerCalls.values()){clearTimeout(call.timer);call.reject(error);}workerCalls.clear();}
function workerCall(data,transfer=[]){return new Promise((resolve,reject)=>{
 const requestId=++workerSequence,timer=setTimeout(()=>stopWorker(Error('Canvas worker timed out')),30000);
 workerCalls.set(requestId,{resolve,reject,timer});rasterWorker.postMessage({...data,requestId},transfer);
});}
function startWorker(){if(rasterReady)return rasterReady;
 rasterReady=(async()=>{const response=await fetch(new URL('preview-worker.mjs',import.meta.url));if(!response.ok)throw Error('Canvas worker loading failed');
  const url=URL.createObjectURL(new Blob([await response.text()],{type:'text/javascript'}));
  try{rasterWorker=new Worker(url,{type:'module'});}finally{URL.revokeObjectURL(url);}
  rasterWorker.onmessage=({data})=>{const call=workerCalls.get(data.requestId);if(!call)return;workerCalls.delete(data.requestId);clearTimeout(call.timer);if(data.type==='error')call.reject(Error(data.error));else call.resolve(data);};
  rasterWorker.onerror=error=>stopWorker(Error(error.message||'Canvas worker failed'));
  await workerCall({type:'init',wasmURL:new URL('wasm/laymesh_wasm_bg.wasm',import.meta.url).href});
 })().catch(error=>{stopWorker(error);throw error;});return rasterReady;
}
startWorker().catch(()=>{});
const vertical=el('div','crosshair vertical',viewport),horizontal=el('div','crosshair horizontal',viewport);
vertical.hidden=horizontal.hidden=true;
const errors=el('div','errors',root),errorButton=el('button','error-location',errors),warnings=el('div','warnings',errors),readout=el('output','coordinates',root);
errorButton.type='button';errorButton.hidden=true;
const pathContext=document.createElement('canvas').getContext('2d');let pathCache=new WeakMap();
const text=(zh,en)=>/^zh/i.test(locale)?zh:en;
function labels(){
 document.documentElement.lang=/^zh/i.test(locale)?'zh-CN':'en';
 const labels={refresh:text('刷新','Refresh'),export:text('导出','Export'),fit:text('适应窗口','Fit'),actual:'100%',out:'−',in:'+',rulers:text('标尺','Rulers')};
 for(const [name,button] of Object.entries(buttons)){button.textContent=labels[name];button.title=labels[name];button.setAttribute('aria-label',name==='out'?text('缩小','Zoom out'):name==='in'?text('放大','Zoom in'):labels[name]);}
 viewport.setAttribute('aria-label',text('图形预览；Ctrl 加滚轮缩放，空格加拖动平移','Figure preview; Ctrl/Command + wheel to zoom, Space + drag to pan'));
 image.alt=text('LayMesh 图形预览','LayMesh figure preview');
 hr.setAttribute('aria-label',text('水平标尺','Horizontal ruler'));vr.setAttribute('aria-label',text('垂直标尺','Vertical ruler'));
 errorButton.title=text('跳转到源码','Go to source');
 buttons.rulers.setAttribute('aria-pressed',String(view.rulers));
}
function persist(){bridge.setState({mode:view.mode,scale:view.scale,pan:view.pan,rulers:view.rulers});}
function containsPath(shape,p){
 let path=pathCache.get(shape);if(!path){try{path=new Path2D(shape.d);pathCache.set(shape,path);}catch{return false;}}
 return pathContext.isPointInPath(path,...p,shape.fillRule==='evenodd'?'evenodd':'nonzero');
}
function rect(){
 const w=page.width*view.scale,h=page.height*view.scale;
 return {left:viewport.clientWidth/2+view.pan.x-w/2,top:viewport.clientHeight/2+view.pan.y-h/2,width:w,height:h};
}
function drawRuler(canvas,axis,r){
 const length=axis==='x'?viewport.clientWidth:viewport.clientHeight,dpr=window.devicePixelRatio||1;
 const width=axis==='x'?length:24,height=axis==='x'?24:length;
 if(canvas.width!==Math.round(width*dpr))canvas.width=Math.round(width*dpr);if(canvas.height!==Math.round(height*dpr))canvas.height=Math.round(height*dpr);canvas.style.width=width+'px';canvas.style.height=height+'px';
 const ctx=canvas.getContext('2d');ctx.setTransform(dpr,0,0,dpr,0,0);ctx.clearRect(0,0,width,height);
 const style=getComputedStyle(root);ctx.strokeStyle=style.getPropertyValue('--ruler-ink').trim();ctx.fillStyle=ctx.strokeStyle;ctx.font='10px '+style.fontFamily;ctx.lineWidth=1;
 const factor=mmPerUnit(page.unit,page.layout_dpi),pixelsPerUnit=view.scale*factor,step=niceStep(60/pixelsPerUnit),minor=step/5,start=axis==='x'?r.left:r.top,max=(axis==='x'?page.width:page.height)/factor;
 const first=Math.max(0,Math.ceil((-start)/pixelsPerUnit/minor)),last=Math.min(Math.floor(max/minor),Math.floor((length-start)/pixelsPerUnit/minor));
 for(let i=first;i<=last;i++){
  const major=i%5===0;if(!major&&minor*pixelsPerUnit<8)continue;
  const pos=start+i*minor*pixelsPerUnit,size=major?10:5;ctx.beginPath();
  if(axis==='x'){ctx.moveTo(pos,24);ctx.lineTo(pos,24-size);}else{ctx.moveTo(24,pos);ctx.lineTo(24-size,pos);}ctx.stroke();
  if(major){const label=String(Number((i*minor).toPrecision(6)));if(axis==='x')ctx.fillText(label,pos+3,10);else{ctx.save();ctx.translate(10,pos+3);ctx.rotate(-Math.PI/2);ctx.fillText(label,0,0);ctx.restore();}}
 }
 if(lastPointer){const bounds=viewport.getBoundingClientRect(),p=canvasAt(page,r,lastPointer.x-bounds.left,lastPointer.y-bounds.top);if(p){const pos=axis==='x'?lastPointer.x-bounds.left:lastPointer.y-bounds.top;ctx.strokeStyle=style.getPropertyValue('--cursor-ink').trim();ctx.lineWidth=2;ctx.beginPath();if(axis==='x'){ctx.moveTo(pos,0);ctx.lineTo(pos,24);}else{ctx.moveTo(0,pos);ctx.lineTo(24,pos);}ctx.stroke();}}
}
function cursor(r){
 if(!lastPointer||!page){readout.textContent='';vertical.hidden=horizontal.hidden=true;return;}
 const bounds=viewport.getBoundingClientRect(),x=lastPointer.x-bounds.left,y=lastPointer.y-bounds.top,p=canvasAt(page,r,x,y);
 if(!p||x<0||y<0||x>viewport.clientWidth||y>viewport.clientHeight){readout.textContent='';vertical.hidden=horizontal.hidden=true;return;}
 vertical.hidden=horizontal.hidden=false;vertical.style.left=x+'px';horizontal.style.top=y+'px';
 const unit=page.unit||'mm';let label=`${text('画布','Canvas')}: X=${p.display[0].toFixed(3)} ${unit}  Y=${p.display[1].toFixed(3)} ${unit}`;
 const hit=plotAt(plots,p.mm,view.scale,containsPath);
 if(hit&&Object.keys(hit.values).length){label+=`  |  ${text('绘图','Plot')} ${hit.plot.id||hit.plot.path}: `+Object.entries(hit.values).map(([name,value])=>`${name==='theta'?'θ':name}=${formatData(value)}${name==='theta'?(hit.angleUnit==='rad'?' rad':'°'):''}`).join('  ');}
 readout.textContent=label;
}
function layout(force=false){
 frame=0;workspace.classList.toggle('no-rulers',!view.rulers);buttons.rulers.setAttribute('aria-pressed',String(view.rulers));
 if(!page)return;
 if(view.mode==='fit'){view.scale=Math.max(1e-6,Math.min(Math.max(1,viewport.clientWidth-32)/page.width,Math.max(1,viewport.clientHeight-32)/page.height));view.pan={x:0,y:0};}
 const r=rect();Object.assign(figure.style,{width:r.width+'px',height:r.height+'px',left:'0',top:'0',transform:`translate3d(${r.left}px,${r.top}px,0)`,willChange:'transform'});paint(r,force===true);
 zoom.textContent=Math.round(view.scale/(page.layout_dpi||96)*25.4*100)+'%';corner.textContent=page.unit||'mm';
 cursor(r);if(view.rulers){drawRuler(hr,'x',r);drawRuler(vr,'y',r);}
}
function schedule(){if(!frame)frame=requestAnimationFrame(layout);}
function zoomTo(scale,point){
 if(!page)return;
 const native=(page.layout_dpi||96)/25.4;scale=Math.max(native*.05,Math.min(native*32,scale));
 const before=rect(),x=point?.x??viewport.clientWidth/2,y=point?.y??viewport.clientHeight/2;
 const mmx=(x-before.left)/view.scale,mmy=(y-before.top)/view.scale;
 view.mode='manual';view.scale=scale;view.pan={x:x-mmx*scale-viewport.clientWidth/2+page.width*scale/2,y:y-mmy*scale-viewport.clientHeight/2+page.height*scale/2};persist();schedule();
}
buttons.refresh.onclick=()=>bridge.postMessage({type:'refresh'});
buttons.export.onclick=()=>bridge.postMessage({type:'export'});
buttons.fit.onclick=()=>{view.mode='fit';view.pan={x:0,y:0};persist();schedule();};
buttons.actual.onclick=()=>{if(page)zoomTo((page.layout_dpi||96)/25.4);};
buttons.out.onclick=()=>zoomTo(view.scale/1.25);buttons.in.onclick=()=>zoomTo(view.scale*1.25);
buttons.rulers.onclick=()=>{view.rulers=!view.rulers;persist();schedule();};
errorButton.onclick=()=>bridge.postMessage({type:'source'});
viewport.addEventListener('wheel',e=>{if(e.ctrlKey||e.metaKey){e.preventDefault();const b=viewport.getBoundingClientRect();zoomTo(view.scale*Math.exp(-e.deltaY*.002),{x:e.clientX-b.left,y:e.clientY-b.top});}else{e.preventDefault();if(page){view.mode='manual';view.pan.x-=e.deltaX;view.pan.y-=e.deltaY;persist();schedule();}}},{passive:false});
viewport.addEventListener('pointerdown',e=>{viewport.focus({preventScroll:true});if(page&&(e.button===1||e.button===0&&space)){e.preventDefault();view.mode='manual';drag={id:e.pointerId,x:e.clientX,y:e.clientY,pan:{...view.pan}};viewport.setPointerCapture(e.pointerId);viewport.classList.add('panning');}});
viewport.addEventListener('pointermove',e=>{lastPointer={x:e.clientX,y:e.clientY};if(drag){view.pan={x:drag.pan.x+e.clientX-drag.x,y:drag.pan.y+e.clientY-drag.y};}schedule();});
viewport.addEventListener('pointerleave',()=>{if(!drag){lastPointer=null;schedule();}});
viewport.addEventListener('mouseleave',()=>{if(!drag){lastPointer=null;schedule();}});
for(const type of ['pointerout','mouseout'])document.addEventListener(type,e=>{if(!e.relatedTarget&&!drag){lastPointer=null;schedule();}});
const endDrag=()=>{if(drag){drag=null;persist();viewport.classList.remove('panning');}schedule();};
for(const type of ['pointerup','pointercancel','lostpointercapture'])viewport.addEventListener(type,endDrag);
viewport.addEventListener('keydown',e=>{if(e.code==='Space'){space=true;e.preventDefault();}else if(['+','=','-','0'].includes(e.key)){e.preventDefault();if(e.key==='0')buttons.fit.click();else zoomTo(view.scale*(e.key==='-'?1/1.25:1.25));}});
viewport.addEventListener('keyup',e=>{if(e.code==='Space'){space=false;e.preventDefault();}});
window.addEventListener('blur',()=>{space=false;lastPointer=null;endDrag();});
new ResizeObserver(schedule).observe(viewport);

let renderSequence=0,frameTimings={};
async function installFrame(data){
 const sequence=++renderSequence;
 if(data.protocol!==3||(!data.displayList&&!data.displayListCompressed))throw Error('Expected drawing protocol 3');
 await startWorker();if(sequence!==renderSequence)return;
 const bytes=data.displayListCompressed?.data,transfer=bytes instanceof Uint8Array?[bytes.buffer]:[];
 const installed=await workerCall({type:'install',generation:data.generation,page:data.inspection.page,displayList:data.displayList,displayListCompressed:data.displayListCompressed,drawingResources:data.drawingResources,resources:data.resources,previewSettings:data.previewSettings},transfer);
 if(sequence!==renderSequence)return;
 drawReady=true;drawJobId=data.displayListCompressed?.id||String(data.generation);if(!installed.reused)drawKey='';wasmMemoryBytes=installed.wasmMemoryBytes;
 drawGeneration=data.generation;
 page=data.inspection.page;plots=preparePlots(data.inspection);pathCache=new WeakMap();
 warnings.textContent=(data.inspection.warnings||[]).map(w=>`${w.code}: ${w.message}`).join('\n');
 frameTimings={initializationWaitMs:installed.initializationWaitMs,resourcesMs:installed.resourcesMs,installMs:installed.installMs};
 root.dataset.generation=String(data.generation);root.dataset.painted='false';showStatus('success');
 // Present a newly installed frame immediately. Background webviews can defer
 // animation callbacks; pointer/zoom updates remain coalesced by schedule().
 if(frame)cancelAnimationFrame(frame);
 layout(true);
}
function positionPixels(){if(!presented||!page)return;const c=presented.crop;Object.assign(image.style,{width:(c[2]-c[0])*view.scale+'px',height:(c[3]-c[1])*view.scale+'px',left:c[0]*view.scale+'px',top:c[1]*view.scale+'px'});
 overview.hidden=overviewContent!==drawJobId;
 if(!overview.hidden){const [x0,y0,x1,y1]=c.map((v,i)=>100*v/(i%2?page.height:page.width));Object.assign(overview.style,{position:'absolute',left:'0',top:'0',width:page.width*view.scale+'px',height:page.height*view.scale+'px',clipPath:`polygon(evenodd,0% 0%,100% 0%,100% 100%,0% 100%,0% 0%,${x0}% ${y0}%,${x1}% ${y0}%,${x1}% ${y1}%,${x0}% ${y1}%,${x0}% ${y0}%)`});}
}
function reportPaint(rasterMs){root.dataset.painted='true';root.dataset.rasterMs=String(rasterMs);bridge.postMessage({type:'painted',generation:drawGeneration,at:Date.now(),rasterMs,wasmMemoryBytes,...frameTimings,width:image.width,height:image.height});}
function paint(r,force=false){
 if(!drawReady)return;positionPixels();
 const left=Math.max(0,r.left),top=Math.max(0,r.top),right=Math.min(viewport.clientWidth,r.left+r.width),bottom=Math.min(viewport.clientHeight,r.top+r.height);
 if(right<=left||bottom<=top){image.hidden=true;return;}image.hidden=false;
 const visible=[(left-r.left)/view.scale,(top-r.top)/view.scale,(right-r.left)/view.scale,(bottom-r.top)/view.scale],dpr=devicePixelRatio||1;
 if(presented?.content===drawJobId&&presented.scale===view.scale&&presented.dpr===dpr&&visible[0]>=presented.crop[0]-1e-9&&visible[1]>=presented.crop[1]-1e-9&&visible[2]<=presented.crop[2]+1e-9&&visible[3]<=presented.crop[3]+1e-9){
  pendingPaint=null;clearTimeout(paintTimer);paintTimer=0;drawKey=presented.key;root.dataset.painted='true';if(force)reportPaint(0);return;
 }
 const pad=96/view.scale,crop=[Math.max(0,visible[0]-pad),Math.max(0,visible[1]-pad),Math.min(page.width,visible[2]+pad),Math.min(page.height,visible[3]+pad)];
 const width=Math.max(1,Math.ceil((crop[2]-crop[0])*view.scale*dpr)),height=Math.max(1,Math.ceil((crop[3]-crop[1])*view.scale*dpr));
 const key=JSON.stringify([drawJobId,crop,width,height]);if(key===drawKey)return;drawKey=key;root.dataset.painted='false';
 pendingPaint={type:'paint',generation:drawGeneration,crop,width,height,content:drawJobId,scale:view.scale,dpr,key};
 clearTimeout(paintTimer);paintTimer=0;
 // Translate the retained Canvas immediately. Changing zoom temporarily
 // scales its pixels; the worker restores exact DPR pixels after motion settles.
 if(force||presented?.scale===view.scale)flushPaint();else paintTimer=setTimeout(()=>{paintTimer=0;flushPaint();},80);
}
async function flushPaint(){if(paintInFlight||!pendingPaint||!rasterWorker)return;
 const request=pendingPaint;pendingPaint=null;paintInFlight=true;
 try{const data=await workerCall(request);
  if(data.type==='painted'&&data.generation===drawGeneration&&request.content===drawJobId&&(request.key===drawKey||pendingPaint)){
   image.width=data.width;image.height=data.height;image.getContext('2d').putImageData(new ImageData(new Uint8ClampedArray(data.rgba.buffer,data.rgba.byteOffset,data.rgba.byteLength),data.width,data.height),0,0);
   if(data.overview){const o=data.overview;overview.width=o.width;overview.height=o.height;overview.getContext('2d').putImageData(new ImageData(new Uint8ClampedArray(o.rgba.buffer,o.rgba.byteOffset,o.rgba.byteLength),o.width,o.height),0,0);overviewContent=request.content;}
   else if(request.crop[0]===0&&request.crop[1]===0&&request.crop[2]===page.width&&request.crop[3]===page.height){const s=Math.min(768/page.width,2048/page.height,Math.sqrt(500000/(page.width*page.height)));overview.width=Math.max(1,Math.floor(page.width*s));overview.height=Math.max(1,Math.floor(page.height*s));overview.getContext('2d').drawImage(image,0,0,overview.width,overview.height);overviewContent=request.content;}
   presented=request;wasmMemoryBytes=data.wasmMemoryBytes;positionPixels();if(request.key===drawKey)reportPaint(data.rasterMs);
  }
 }catch(error){if(request.generation===drawGeneration&&request.key===drawKey){drawKey='';diagnostic={code:'E_PREVIEW_RENDER',message:String(error)};showDiagnostic();showStatus('error');}}
 finally{paintInFlight=false;if(pendingPaint&&!paintTimer)flushPaint();}
}
function showStatus(value){root.dataset.state=value;status.textContent=({pending:text('等待更新 · 预览已过期','Pending · preview stale'),rendering:text('渲染中 · 预览已过期','Rendering · preview stale'),success:text('已更新','Updated'),error:text('错误 · 预览已过期','Error · preview stale')})[value]||'';}
function showDiagnostic(){
 errorButton.hidden=!diagnostic;
 if(diagnostic)errorButton.textContent=`${diagnostic.code||'E_PREVIEW'} ${diagnostic.file?diagnostic.file+':'+(diagnostic.loc?.line||1)+':'+(diagnostic.loc?.column||1)+' — ':''}${text(diagnostic.messageZh||diagnostic.message,diagnostic.message)}`;
}
window.addEventListener('message',({data})=>{
 if(data?.type==='initial'){locale=data.locale||'en';if((!initialized&&saved.rulers===undefined)||data.forceRulers)view.rulers=data.showRulers!==false;initialized=true;labels();showDiagnostic();if(root.dataset.state)showStatus(root.dataset.state);schedule();}
 else if(data?.type==='status'){if(data.status!=='success')renderSequence++;showStatus(data.status);}
 else if(data?.type==='result'){
  diagnostic=data.error;showDiagnostic();
  if(diagnostic){showStatus('error');}
  else if(data.protocol===3&&(data.displayList||data.displayListCompressed)&&data.inspection){installFrame(data).catch(error=>{diagnostic={code:'E_PREVIEW_RESOURCE',message:String(error)};showDiagnostic();showStatus('error');});}
  else{diagnostic={code:'E_PREVIEW_PROTOCOL',message:'Expected drawing protocol 3'};showDiagnostic();showStatus('error');}
 }
});
window.addEventListener('pagehide',()=>{renderSequence++;clearTimeout(paintTimer);pendingPaint=null;stopWorker(Error('Preview closed'));});
labels();bridge.postMessage({type:'ready'});
