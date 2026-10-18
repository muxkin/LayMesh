import {MediaController} from './media-runtime.mjs';
import {drawPreviewRuler} from './preview-geometry.mjs';
import {PresentationSession} from './presentation-session.mjs';
import {Presentation} from './presentation.mjs';
import {uiIcon,iconLabel} from './presentation-icons.mjs';
import {RetainedPresentationCache} from './presentation-store.mjs';
const retainedDeck=new RetainedPresentationCache();
import {PlaybackStats,waitForPresentation} from './playback-stats.mjs';
import {loadRendererRuntime,loadDrawingResources} from './renderer-resources.mjs';
import {PreviewPlayer} from './preview-player.mjs';
import {mmPerUnit,canvasAt,niceStep,preparePlots,plotAt,formatData} from './preview-math.mjs';

const bridge=typeof acquireVsCodeApi==='function'?acquireVsCodeApi():{getState:()=>null,setState(){},postMessage(){}};
const saved=bridge.getState()||{};
let view={mode:'fit',scale:1,pan:{x:0,y:0},rulers:true,...saved},page,plots=[],locale='en',lastPointer=null,drag,space=false,frame=0,diagnostic,initialized=false;
const el=(tag,className,parent,text)=>{const node=document.createElement(tag);if(className)node.className=className;if(text)node.textContent=text;parent?.append(node);return node;};
const root=el('main','preview',document.body),toolbar=el('div','toolbar',root);
const buttons={};for(const name of ['refresh','export','fit','actual','out','in','rulers']){const button=el('button','',toolbar);button.type='button';button.dataset.action=name;buttons[name]=button;}
const zoom=el('output','zoom',toolbar),status=el('span','status',toolbar);
const pageSelect=el('select','page-select',toolbar),openPage=el('button','',toolbar),exportDocument=el('button','',toolbar);
pageSelect.hidden=openPage.hidden=exportDocument.hidden=true;
openPage.type=exportDocument.type='button';openPage.dataset.action='open-page';exportDocument.dataset.action='export-document';
pageSelect.onchange=()=>{view.mode='fit';view.pan={x:0,y:0};syncPages();bridge.postMessage({type:'select-page',pageId:pageSelect.value});};
openPage.onclick=()=>bridge.postMessage({type:'open-page'});
exportDocument.onclick=()=>bridge.postMessage({type:'export-document'});
const playbackStats=new PlaybackStats();let wasRunning=false,media;
const details=el('details','render-details',root),summary=el('summary','',details,'Skia'),metrics=el('output','',details);details.style.cssText='font-size:11px;padding:0 8px';
const player=new PreviewPlayer(root,()=>{if(player?.running)playbackStats.refresh(performance.now());wasRunning=!!player?.running;schedule();},{locale:()=>locale,page:direction=>{const n=pageSelect.selectedIndex+direction;if(n<0||n>=pageSelect.options.length)return false;pageSelect.selectedIndex=n;pageSelect.onchange();return true;}});
const motionMenu=el('details','motion-menu',toolbar);el('summary','',motionMenu,'播放设置');motionMenu.append(player.bar);
const pageTools=el('div','page-tools',toolbar),previousPage=el('button','',pageTools),nextPage=el('button','',pageTools);previousPage.type=nextPage.type='button';previousPage.dataset.action='previous-page';nextPage.dataset.action='next-page';pageTools.append(previousPage,pageSelect,nextPage);pageTools.hidden=true;
function selectPage(delta){const index=pageSelect.selectedIndex+delta;if(index<0||index>=pageSelect.options.length)return false;player.resume=false;pageSelect.selectedIndex=index;pageSelect.onchange();return true;}
previousPage.onclick=()=>selectPage(-1);nextPage.onclick=()=>selectPage(1);
function syncPages(){pageTools.hidden=pageSelect.hidden;previousPage.disabled=pageSelect.selectedIndex<=0;nextPage.disabled=pageSelect.selectedIndex>=pageSelect.options.length-1;}
const presentationMenu=el('details','presentation-choice',toolbar);el('summary','',presentationMenu,'放映 ▾');
let previewMediaSnapshot,previewResume=false;
let deckResolve,deckReject,deckStore,deckReceive=Promise.resolve(),deckProgress,deckRevision,deckSession,deckRequest=0,deckRestart;
const presentation=new Presentation({parent:root,base:new URL('./',import.meta.url),blobWorker:true,fullscreen:false,compatibleAudio:true,getStartPageId:()=>pageSelect.value,onEnter:()=>{previewMediaSnapshot=[...root.querySelectorAll('details')].find(menu=>menu.open&&menu.mediaSnapshot)?.mediaSnapshot||media.snapshot();media.pause({lock:true});previewResume=player.running;if(previewResume)player.action('play');},provider:({signal,onProgress,startPageId,plannedOnly})=>new Promise((resolve,reject)=>{
 deckRevision=null;const requestId=++deckRequest;deckStore=retainedDeck.session();const store=deckStore;deckSession=new PresentationSession(store,{onProgress,request:(id,retry)=>bridge.postMessage({type:'deck-priority',revision:deckRevision,pageId:id,retry}),stop:()=>bridge.postMessage({type:'deck-cancel',requestId})});deckSession.preparation={};deckResolve=resolve;deckReject=reject;deckProgress=onProgress;deckReceive=Promise.resolve();(plannedOnly?deckSession.planReady:deckSession.firstReady).then(resolve,reject);signal.addEventListener('abort',()=>store.dispose(),{once:true});bridge.postMessage({type:'deck-prepare',requestId,startPageId,knownPages:retainedDeck.known()});
}),onExit:()=>{if(!document.hidden)media.restore(previewMediaSnapshot);bridge.postMessage({type:'deck-release'});if(previewResume)player.start();}});
const presentationPopover=el('div','toolbar-popover',presentationMenu);const presentHere=el('button','',presentationPopover);presentHere.type='button';presentHere.onclick=()=>presentation.button.click();presentation.button.addEventListener('click',()=>{presentationMenu.open=false;bridge.postMessage({type:'deck-pin'});});
const browserPresent=el('button','',presentationPopover,'在浏览器全屏放映');browserPresent.onclick=async()=>{presentationMenu.open=false;try{await presentation.prepare();bridge.postMessage({type:'deck-browser',startPageId:pageSelect.value});}catch(e){diagnostic={message:e.message};showDiagnostic();}};
const speakerPresent=el('button','',presentationPopover,'演讲者视图（浏览器）');speakerPresent.onclick=async()=>{presentationMenu.open=false;try{await presentation.prepare();bridge.postMessage({type:'deck-browser',startPageId:pageSelect.value,presenter:true});}catch(e){diagnostic={message:e.message};showDiagnostic();}};
const fromBeginning=el('button','',presentationPopover,'从头放映');fromBeginning.onclick=()=>{presentationMenu.open=false;presentation.open({startPageId:null});};
window.laymeshPresentation=presentation;
const workspace=el('div','workspace',root),corner=el('span','corner',workspace),hr=el('canvas','ruler horizontal',workspace),vr=el('canvas','ruler vertical',workspace),viewport=el('div','viewport',workspace);
viewport.tabIndex=0;viewport.setAttribute('role','region');
const figure=el('div','figure',viewport),overview=el('canvas','figure-overview',figure);overview.hidden=true;overview.setAttribute('aria-hidden','true');overview.style.pointerEvents='none';let overviewContent='';
let image=el('canvas','figure-pixels',figure);image.style.position='absolute';image.setAttribute('role','img');
let mainSkia,directCanvas=false,rasterWorker,rasterReady,workerSequence=0,wasmMemoryBytes=0,drawReady=false,drawJobId='',drawKey='',drawGeneration=0,presented,pendingPaint,paintInFlight=false,paintTimer=0;
const workerCalls=new Map();
let rasterResourceIds=new Set();
function stopWorker(error){rasterWorker?.terminate();rasterWorker=null;rasterReady=null;drawReady=false;rasterResourceIds.clear();for(const call of workerCalls.values()){clearTimeout(call.timer);call.reject(error);}workerCalls.clear();}
function workerCall(data,transfer=[]){return new Promise((resolve,reject)=>{
 const requestId=++workerSequence,timer=setTimeout(()=>stopWorker(Error(`Canvas worker timed out (${data.type})`)),30000);
 workerCalls.set(requestId,{resolve,reject,timer});rasterWorker.postMessage({...data,requestId},transfer);
});}
function startWorker(){if(rasterReady)return rasterReady;
 rasterReady=(async()=>{const [response,runtime]=await Promise.all([fetch(new URL('preview-worker.mjs',import.meta.url)),loadRendererRuntime(new URL('./',import.meta.url))]);if(!response.ok)throw Error('Canvas worker loading failed');
  const url=URL.createObjectURL(new Blob([await response.text()],{type:'text/javascript'}));
  try{rasterWorker=new Worker(url,{type:'module'});}finally{URL.revokeObjectURL(url);}
  rasterWorker.onmessage=({data})=>{if(data.type==='context'){root.dataset.renderBackend=data.ready?'Skia WebGL2':'Skia restoring';if(data.ready){drawKey='';presented=null;schedule();}return;}const call=workerCalls.get(data.requestId);if(!call)return;workerCalls.delete(data.requestId);clearTimeout(call.timer);if(data.type==='error')call.reject(Error(data.error));else call.resolve(data);};
  rasterWorker.onerror=error=>stopWorker(Error(error.message||'Canvas worker failed'));
  const canvas=image.cloneNode(false);image.replaceWith(canvas);image=canvas;mainSkia?.dispose();mainSkia=null;
  const offscreen=canvas.transferControlToOffscreen?.();directCanvas=!!offscreen;
  await workerCall({type:'init',...runtime,canvas:offscreen},[runtime.wasmBytes,runtime.skiaBytes,...(offscreen?[offscreen]:[])]);
 })().catch(error=>{stopWorker(error);throw error;});return rasterReady;
}
startWorker().catch(()=>{});
const vertical=el('div','crosshair vertical',viewport),horizontal=el('div','crosshair horizontal',viewport);
vertical.hidden=horizontal.hidden=true;
const footer=el('footer','preview-footer',root),diagnosticMenu=el('details','diagnostic-menu',footer),diagnosticSummary=el('summary','',diagnosticMenu);diagnosticSummary.textContent='诊断';footer.append(status,details);
const errors=el('div','errors',diagnosticMenu),errorHeading=el('div','diagnostic-heading',errors),errorTitle=el('strong','',errorHeading),closeError=el('button','',errorHeading),errorNote=el('p','diagnostic-note',errors),errorButton=el('button','error-location',errors),warnings=el('div','warnings',errors),readout=el('output','coordinates',footer);
iconLabel(closeError,'close');closeError.onclick=()=>diagnosticMenu.open=false;const sourceError=el('button','source-diagnostics',errors);sourceError.onclick=()=>bridge.postMessage({type:'source'});errorButton.type='button';errorButton.hidden=true;const copyError=el('button','copy-diagnostics',errors,'复制');copyError.onclick=()=>{const field=el('textarea','',errors);field.value=[errorButton.textContent,warnings.textContent].filter(Boolean).join('\n');field.select();document.execCommand('copy');field.remove();};
const pathContext=document.createElement('canvas').getContext('2d');let pathCache=new WeakMap();
const text=(zh,en)=>/^zh/i.test(locale)?zh:en;
function labels(){
 player.localize();iconLabel(presentationMenu.querySelector('summary'),'down');presentationMenu.querySelector('summary').setAttribute('aria-label',text('放映选项','Presentation options'));iconLabel(presentation.button,'play',text('放映','Present'));presentHere.textContent=text('在 VS Code 中放映','Present in VS Code');summary.textContent=text('详情','Details');closeError.setAttribute('aria-label',text('关闭诊断','Close diagnostics'));sourceError.textContent=text('跳转源码','Go to source');browserPresent.textContent=text('在浏览器全屏放映','Present in browser');copyError.textContent=text('复制','Copy');
 pageSelect.setAttribute('aria-label',text('文档页面','Document page'));
 openPage.textContent=text('编辑页面','Edit page');exportDocument.textContent=text('导出整本 PDF','Export document PDF');
 document.documentElement.lang=/^zh/i.test(locale)?'zh-CN':'en';
 const labels={refresh:text('刷新','Refresh'),export:text('导出','Export'),fit:text('适应','Fit'),actual:'100%',out:'−',in:'+',rulers:text('标尺','Rulers')};
 for(const [name,button] of Object.entries(buttons)){button.textContent=labels[name];button.title=labels[name];button.setAttribute('aria-label',name==='out'?text('缩小','Zoom out'):name==='in'?text('放大','Zoom in'):labels[name]);}
 iconLabel(buttons.refresh,'replay',text('刷新','Refresh'));iconLabel(buttons.export,'export',text('导出','Export'));iconLabel(previousPage,'previous');iconLabel(nextPage,'next');previousPage.setAttribute('aria-label',text('上一页','Previous page'));nextPage.setAttribute('aria-label',text('下一页','Next page'));motionMenu.querySelector('summary').textContent=text('播放设置','Playback settings');if(typeof moreSummary!=='undefined')moreSummary.setAttribute('aria-label',text('更多','More'));syncPlaybackButtons();
 viewport.setAttribute('aria-label',text('图形预览；Ctrl 加滚轮缩放，空格加拖动平移','Figure preview; Ctrl/Command + wheel to zoom, Space + drag to pan'));
 image.alt=text('LayMesh 图形预览','LayMesh figure preview');
 hr.setAttribute('aria-label',text('水平标尺','Horizontal ruler'));vr.setAttribute('aria-label',text('垂直标尺','Vertical ruler'));
 errorButton.title=text('跳转到源码','Go to source');
 buttons.rulers.setAttribute('aria-pressed',String(view.rulers));
}
const more=el('details','preview-more',toolbar);const moreSummary=el('summary','',more);iconLabel(moreSummary,'more');moreSummary.setAttribute('aria-label','More');const morePopover=el('div','toolbar-popover',more);for(const b of [openPage,exportDocument,motionMenu])morePopover.append(b);
const zoomMenu=el('details','zoom-menu',toolbar),zoomSummary=el('summary','',zoomMenu);zoomSummary.append(zoom,uiIcon('down'));const zoomPopover=el('div','toolbar-popover',zoomMenu);zoomPopover.append(buttons.out,buttons.in,buttons.actual,buttons.rulers);
const presentGroup=el('div','present-group',toolbar);presentGroup.append(presentation.button,presentationMenu);
const playbackButtons=el('div','playback-buttons',footer);playbackButtons.setAttribute('role','group');for(const id of ['previous','play','next'])playbackButtons.append(player.buttons[id]);
function syncPlaybackButtons(){
 playbackButtons.hidden=!player.metadata?.steps?.length&&!media?.tracks.length;
 for(const [id,icon,zh,en]of [['previous','previous','上一步','Previous step'],['play',player.running||media?.running?'pause':'play',player.running||media?.running?'暂停':'开始',player.running||media?.running?'Pause':'Play'],['next','next','下一步','Next step']]){const b=player.buttons[id];iconLabel(b,icon);b.title=text(zh,en);b.setAttribute('aria-label',b.title);}
 playbackButtons.setAttribute('aria-label',text('动画播放控制','Animation playback controls'));
}
const originalSync=player.sync.bind(player);player.sync=(...args)=>{originalSync(...args);syncPlaybackButtons();};
toolbar.replaceChildren(buttons.refresh,buttons.fit,zoomMenu,pageTools,presentGroup,buttons.export,more);footer.replaceChildren(playbackButtons,diagnosticMenu,status,readout,details);
viewport.addEventListener('keydown',e=>{if(e.target.closest('.media-controls')||e.ctrlKey||e.metaKey||e.altKey||e.repeat||!player.metadata)return;const action={ArrowRight:'next',ArrowDown:'next',ArrowLeft:'previous',ArrowUp:'previous',Enter:'next',s:'play',S:'play',Escape:'static'}[e.key];if(action){e.preventDefault();player.action(action);}});

for(const menu of [more,zoomMenu,presentationMenu,diagnosticMenu,details])menu.addEventListener('toggle',()=>{if(menu.open){for(const other of [more,zoomMenu,presentationMenu,diagnosticMenu,details])if(other!==menu&&other.open){other.open=false;media.restore(other.mediaSnapshot);other.mediaSnapshot=null;}menu.mediaSnapshot=media.suspend();}else{if(!presentation.opened&&!document.hidden)media.restore(menu.mediaSnapshot);menu.mediaSnapshot=null;}});
document.addEventListener('pointerdown',e=>{for(const menu of [more,zoomMenu,presentationMenu,diagnosticMenu,details])if(menu.open&&!menu.contains(e.target))menu.open=false;});
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
 const style=getComputedStyle(root),bounds=viewport.getBoundingClientRect(),p=lastPointer&&canvasAt(page,r,lastPointer.x-bounds.left,lastPointer.y-bounds.top);
 drawPreviewRuler(canvas,axis,{length:axis==='x'?viewport.clientWidth:viewport.clientHeight,start:axis==='x'?r.left:r.top,scale:view.scale,page,ink:style.getPropertyValue('--ruler-ink').trim(),font:'10px '+style.fontFamily,pointer:p?(axis==='x'?lastPointer.x-bounds.left:lastPointer.y-bounds.top):null});
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
function cancelTransition(){rasterWorker?.postMessage({type:'cancel-transition'});mainSkia?.transitions.cancel();}
player.motion.addEventListener('change',cancelTransition);
new ResizeObserver(()=>{cancelTransition();schedule();}).observe(viewport);

let renderSequence=0,frameTimings={};
media=new MediaController({parent:root,compatibleAudio:true,onFrame:()=>{drawKey='';syncPlaybackButtons();schedule();},onState:()=>syncPlaybackButtons(),getViewport:()=>{
 if(!page)return null;const parent=root.getBoundingClientRect(),box=viewport.getBoundingClientRect(),r=rect();return {visible:!presentation.opened&&!document.hidden,matrix:[view.scale,0,box.left+r.left-parent.left,0,view.scale,box.top+r.top-parent.top,0,0,1],clip:[box.left-parent.left,box.top-parent.top,box.right-parent.left,box.bottom-parent.top]};
},onError:e=>{diagnostic={code:'E_MEDIA',message:String(e)};showDiagnostic();}});
window.__LAYMESH_PREVIEW_MEDIA=media;
let mediaLast=performance.now(),mediaRaf;
function tickMedia(now){const elapsed=(now-mediaLast)/1000;mediaLast=now;media.tick(elapsed,player.clock);mediaRaf=requestAnimationFrame(tickMedia);}mediaRaf=requestAnimationFrame(tickMedia);
const mediaAction=player.action.bind(player);player.action=id=>{
 if(id==='static'){media.seek(0);media.stop();}
 else if(id==='play'&&(media.running||player.running)){media.pause();player.clock.running=false;player.refresh();return;}
 else if(id==='play'){if(player.time!==null&&player.clock.time>=player.clock.duration)media.seek(0,0,{preserveManual:true,preserveDocument:true});media.resume();}
 else if(id==='next')media.begin();
 mediaAction(id);
 if(id==='replay')media.resume();
};
const mediaSeek=player.seek.bind(player);player.seek=(time,...args)=>{if(time!==null)media.seek(time);return mediaSeek(time,...args);};
viewport.addEventListener('click',e=>{if(e.target.closest('.media-controls')||drag||e.button!==0||!page)return;const box=viewport.getBoundingClientRect(),r=rect();if(media.click((e.clientX-box.left-r.left)/view.scale,(e.clientY-box.top-r.top)/view.scale,media.states)){e.preventDefault();e.stopImmediatePropagation();}},true);
const mediaResourceUris=new Map();
async function installFrame(data){
 const sequence=++renderSequence;
 if(data.protocol!==3||(!data.displayList&&!data.displayListCompressed&&!data.displayListUpdate))throw Error('Expected drawing protocol 3');
 await startWorker();if(sequence!==renderSequence)return;
 const resourceStart=performance.now(),resourceBytes=await loadDrawingResources(data.displayListReplay||[data],rasterResourceIds),resourceLoadMs=performance.now()-resourceStart;
 if(sequence!==renderSequence)return;
 const bytes=data.displayListCompressed?.data,transfer=bytes instanceof Uint8Array?[bytes.buffer]:[];
 const installed=await workerCall({type:'install',navigation:{...data.document,reduced:player.reduced},generation:data.generation,page:data.inspection.page,displayListId:data.displayListId,displayListUpdate:data.displayListUpdate,displayListReplay:data.displayListReplay,displayList:data.displayList,displayListCompressed:data.displayListCompressed,drawingResources:data.drawingResources,resources:data.resources,resourceBytes,previewSettings:data.previewSettings},[...transfer,...resourceBytes.map(r=>r.bytes)]);
 rasterResourceIds=new Set(installed.resourceIds);
 installed.resourcesMs+=resourceLoadMs;
 if(sequence!==renderSequence)return;
 for(const version of data.displayListReplay||[data])for(const resource of version.resources||[])if(/^(video|audio)\//.test(resource.mime||""))mediaResourceUris.set(resource.id,resource.uri);
 if(installed.mediaList){await media.install(installed.mediaList,async id=>{const uri=mediaResourceUris.get(id);if(!uri)throw Error("Missing media resource: "+id);const response=await fetch(uri);if(!response.ok)throw Error("Media loading failed");return new Uint8Array(await response.arrayBuffer());});}else{await media.install({nodes:[],media:{assets:{},cues:[]}},()=>{});}
 for(const id of mediaResourceUris.keys())if(!installed.mediaList?.resources?.[id])mediaResourceUris.delete(id);
 if(!directCanvas){const {initializeSkia,SkiaRenderer}=await import('./skia-renderer.mjs');if(!mainSkia){const meta=await(await fetch(new URL('skia/manifest.json',import.meta.url))).json();mainSkia=new SkiaRenderer(await initializeSkia(new URL('skia/'+meta.wasm,import.meta.url).href),image,{onContextChange:ready=>{if(ready){drawKey='';presented=null;schedule();}}});}await mainSkia.install(installed.scene,id=>installed.sceneResources[id],data.generation,{...data.document,reduced:player.reduced});}
 drawReady=true;drawJobId=data.displayListId||data.displayListCompressed?.id||String(data.generation);if(!installed.reused)drawKey='';wasmMemoryBytes=installed.wasmMemoryBytes;
 drawGeneration=data.generation;
 player.resumeAfterTransition=false;const resume=player.resume;player.update(data.inspection.animation);if(resume&&player.metadata){const start=player.restorePage(resume);if(start)player.resumeAfterTransition=true;}
 page=data.inspection.page;plots=preparePlots(data.inspection);pathCache=new WeakMap();
 warnings.dataset.count=String((data.inspection.warnings||[]).length);warnings.textContent=(data.inspection.warnings||[]).map(w=>`${w.code}: ${w.message}`).join('\n');showDiagnostic();
 frameTimings={initializationWaitMs:installed.initializationWaitMs,resourcesMs:installed.resourcesMs,installMs:installed.installMs};
 root.dataset.generation=String(data.generation);root.dataset.painted='false';showStatus('success');
 // Present a newly installed frame immediately. Background webviews can defer
 // animation callbacks; pointer/zoom updates remain coalesced by schedule().
 if(frame)cancelAnimationFrame(frame);
 layout(true);
}
function positionPixels(){if(!presented||!page)return;const c=presented.crop;Object.assign(image.style,{width:(c[2]-c[0])*view.scale+'px',height:(c[3]-c[1])*view.scale+'px',left:c[0]*view.scale+'px',top:c[1]*view.scale+'px'});
 overview.hidden=player.time!==null||overviewContent!==drawJobId;
 if(!overview.hidden){const [x0,y0,x1,y1]=c.map((v,i)=>100*v/(i%2?page.height:page.width));Object.assign(overview.style,{position:'absolute',left:'0',top:'0',width:page.width*view.scale+'px',height:page.height*view.scale+'px',clipPath:`polygon(evenodd,0% 0%,100% 0%,100% 100%,0% 100%,0% 0%,${x0}% ${y0}%,${x1}% ${y0}%,${x1}% ${y1}%,${x0}% ${y1}%,${x0}% ${y0}%)`});}
}
function reportPaint(rasterMs){if(!presentation.opened)presentation.prepare().catch(()=>{});root.dataset.painted='true';root.dataset.rasterMs=String(rasterMs);bridge.postMessage({type:'painted',generation:drawGeneration,displayListId:drawJobId,at:Date.now(),rasterMs,wasmMemoryBytes,...frameTimings,width:image.width,height:image.height});}
function paint(r,force=false){
 if(!drawReady)return;positionPixels();
 const left=Math.max(0,r.left),top=Math.max(0,r.top),right=Math.min(viewport.clientWidth,r.left+r.width),bottom=Math.min(viewport.clientHeight,r.top+r.height);
 if(right<=left||bottom<=top){image.hidden=true;return;}image.hidden=false;
 const visible=[(left-r.left)/view.scale,(top-r.top)/view.scale,(right-r.left)/view.scale,(bottom-r.top)/view.scale],dpr=devicePixelRatio||1;
 if(!media.frames.size&&!media.running&&presented?.confirmed!==false&&presented?.time===player.time&&presented?.reduced===player.reduced&&presented?.content===drawJobId&&presented.scale===view.scale&&presented.dpr===dpr&&visible[0]>=presented.crop[0]-1e-9&&visible[1]>=presented.crop[1]-1e-9&&visible[2]<=presented.crop[2]+1e-9&&visible[3]<=presented.crop[3]+1e-9){
  pendingPaint=null;clearTimeout(paintTimer);paintTimer=0;drawKey=presented.key;root.dataset.painted='true';if(force)reportPaint(0);return;
 }
 const pad=96/view.scale,crop=[Math.max(0,visible[0]-pad),Math.max(0,visible[1]-pad),Math.min(page.width,visible[2]+pad),Math.min(page.height,visible[3]+pad)];
 const width=Math.max(1,Math.ceil((crop[2]-crop[0])*view.scale*dpr)),height=Math.max(1,Math.ceil((crop[3]-crop[1])*view.scale*dpr));
 const key=JSON.stringify([drawJobId,crop,width,height,player.time,player.reduced]);if(key===drawKey)return;drawKey=key;root.dataset.painted='false';
 pendingPaint={running:player.running||media.running,visible:document.visibilityState==='visible',time:player.time,reduced:player.reduced,type:'paint',generation:drawGeneration,crop,width,height,content:drawJobId,scale:view.scale,dpr,key};
 clearTimeout(paintTimer);paintTimer=0;
 // Translate the retained Canvas immediately. Changing zoom temporarily
 // scales its pixels; the worker restores exact DPR pixels after motion settles.
 if(force||presented?.scale===view.scale)flushPaint();else paintTimer=setTimeout(()=>{paintTimer=0;flushPaint();},80);
}
async function flushPaint(){if(paintInFlight||!pendingPaint||!rasterWorker)return;
 const request=pendingPaint;pendingPaint=null;paintInFlight=true;if(request.running)playbackStats.submit(performance.now());
 const mediaFrames=media.takeFrames();
 try{const data=await workerCall(directCanvas?{...request,mediaFrames}:request,directCanvas?mediaFrames.map(f=>f.bitmap):[]);media.states=data.mediaStates||data.states||{};
  if(data.type==='painted'&&data.generation===drawGeneration&&request.content===drawJobId&&(request.key===drawKey||pendingPaint)){
   if(!directCanvas){mainSkia.setMediaFrames(mediaFrames);const metrics=mainSkia.render(request.crop,data.width,data.height,data.states,data.generation,!!request.running);if(metrics.skipped)return;Object.assign(metrics,await mainSkia.transitions.play(player.reduced));if(!request.running)metrics.presentationConfirmed=await mainSkia.completeFrame(request.visible);Object.assign(data,metrics);}if(player.resumeAfterTransition){player.resumeAfterTransition=false;player.start();}root.dataset.renderBackend=data.backend;root.dataset.transitionName=data.transitionName||'';root.dataset.transitionFrames=String(data.transitionFrames||0);root.dataset.transitionError=data.transitionError||'';root.dataset.renderWidth=String(data.width);root.dataset.renderHeight=String(data.height);root.dataset.sampleMs=String(data.sampleMs);
   request.confirmed=false;presented=request;positionPixels();
   if(!request.running){data.presentationConfirmed=data.presentationConfirmed&&await waitForPresentation(request.visible);if(request.generation!==drawGeneration||request.content!==drawJobId)return;}
   request.confirmed=true;
   if(request.time===null&&request.crop[0]===0&&request.crop[1]===0&&request.crop[2]===page.width&&request.crop[3]===page.height){const s=Math.min(768/page.width,2048/page.height,Math.sqrt(500000/(page.width*page.height)));overview.width=Math.max(1,Math.floor(page.width*s));overview.height=Math.max(1,Math.floor(page.height*s));overview.getContext('2d').drawImage(image,0,0,overview.width,overview.height);overviewContent=request.content;}
   if(request.running)requestAnimationFrame(now=>{playbackStats.record(`${request.generation}:${request.time}`,now);});
   summary.textContent=text('详情','Details');summary.title=`${data.backend} · ${data.width}×${data.height}`;
   metrics.textContent=`${data.backend} · ${data.width}×${data.height} · ${text('采样','Sample')} ${data.sampleMs.toFixed(2)} ms · ${text('绘制提交','Draw submission')} ${data.rasterMs.toFixed(2)} ms${data.gpuMs==null?'':` · GPU ${data.gpuMs.toFixed(2)} ms`} · ${text('呈现估计','Presentation estimate')} ${playbackStats.fps.toFixed(1)} fps · ${text('提交','Submissions')} ${playbackStats.submissionFps.toFixed(1)} fps · ${text('错过时隙估计','Missed slots estimate')} ${playbackStats.missedPercent.toFixed(1)}%`;
   presented=request;wasmMemoryBytes=data.wasmMemoryBytes;frameTimings.presentationConfirmed=data.presentationConfirmed;positionPixels();if(request.key===drawKey)reportPaint(data.rasterMs);
  }
 }catch(error){if(request.generation===drawGeneration&&request.key===drawKey){drawKey='';diagnostic={code:'E_PREVIEW_RENDER',message:String(error)};showDiagnostic();showStatus('error');}}
 finally{paintInFlight=false;if(pendingPaint&&!paintTimer)flushPaint();}
}
function rendererError(){return ['E_PREVIEW_RESOURCE','E_PREVIEW_RENDER','E_PREVIEW_PROTOCOL','E_PRESENTATION'].includes(diagnostic?.code);}
function showStatus(value){root.dataset.state=value;status.textContent=({pending:text('等待更新 · 预览已过期','Pending · preview stale'),rendering:text('渲染中 · 预览已过期','Rendering · preview stale'),success:text('已更新','Updated'),error:rendererError()?text('预览加载或渲染失败 · 显示上次结果','Preview failed · last successful preview'):text('源码有错误 · 显示上次结果','Source error · last successful preview')})[value]||'';}
function showDiagnostic(){
 const count=Number(warnings.dataset.count||0);const label=diagnostic?text(`1 个错误${count?' · '+count+' 个警告':''}`,`1 error${count?' · '+count+' warnings':''}`):text(`${count} 个警告`,`${count} warnings`);iconLabel(diagnosticSummary,'warning',label);errorTitle.textContent=label;diagnosticMenu.hidden=!diagnostic&&!count;if(diagnosticMenu.hidden)diagnosticMenu.open=false;errorNote.textContent=diagnostic?(rendererError()?text('预览资源加载或渲染失败，可刷新重试','Preview resource loading or rendering failed; refresh to retry'):text('源码有错误，显示上次结果','Source error; showing last successful result')):'';sourceError.hidden=!diagnostic||rendererError();errorButton.disabled=rendererError();errorButton.title=rendererError()?'':text('跳转到源码','Go to source');
 errorButton.hidden=!diagnostic;
 if(diagnostic)errorButton.textContent=`${diagnostic.code||'E_PREVIEW'} ${diagnostic.file?diagnostic.file+':'+(diagnostic.loc?.line||1)+':'+(diagnostic.loc?.column||1)+' — ':''}${text(diagnostic.messageZh||diagnostic.message,diagnostic.message)}`;
}
window.addEventListener('message',({data})=>{
 if(data?.type?.startsWith('deck-')){
  if(data.type==='deck-browser-error'){diagnostic={code:'E_PRESENTATION',message:data.error};showDiagnostic();return;}
  if(data.type==='deck-invalidated'){clearTimeout(deckRestart);presentation.invalidate();if(data.background)deckRestart=setTimeout(()=>{if(root.dataset.state==='success'&&!presentation.opened)presentation.prepare().catch(()=>{});},250);return;}
  if(data.requestId!==undefined&&data.requestId!==deckRequest)return;
  if(data.type==='deck-plan'){deckRevision=data.revision;deckSession.plan(data.pages);return;}
  if(deckRevision&&data.revision&&data.revision!==deckRevision)return;
  if(data.type==='deck-progress')return; // Stored artifact readiness is reported by the session.
  if(data.type==='deck-error'){deckSession?.fail(data.pageId,Error(data.error));if(!data.pageId)deckReject?.(Error(data.error));return;}
  if(data.type==='deck-page'||data.type==='deck-page-reuse'){const store=deckStore,session=deckSession;deckReceive=deckReceive.then(async()=>{if(store!==deckStore||store.closed)return;const page=data.page;if(data.type==='deck-page-reuse'){store.bind(page,data.artifactId);session.ready(store.pages.find(p=>p.id===page.id));bridge.postMessage({type:'deck-ack',revision:data.revision,pageId:page.id});return;}page.resources={};for(const [id,url]of Object.entries(page.resourceUrls)){if(store.resources.has(id))continue;const r=await fetch(url);if(!r.ok)throw Error('Presentation resource unavailable');page.resources[id]=new Uint8Array(await r.arrayBuffer());}if(store!==deckStore||store.closed)return;delete page.resourceUrls;await store.add(page,data.artifactId);session.ready(store.pages.find(p=>p.id===page.id));bridge.postMessage({type:'deck-ack',revision:data.revision,pageId:page.id});}).catch(e=>{if(store===deckStore&&!store.closed){bridge.postMessage({type:'deck-cancel'});store.dispose();deckReject?.(e);}});}
  if(data.type==='deck-ready'){const store=deckStore;deckReceive.then(()=>{if(store===deckStore&&!store.closed){deckSession.prepareMs=data.prepareMs;Object.assign(deckSession.preparation,data.preparation);retainedDeck.prune().catch(console.error);}});}return;
 }

 if(data?.type==='initial'){locale=data.locale||'en';if((!initialized&&saved.rulers===undefined)||data.forceRulers)view.rulers=data.showRulers!==false;initialized=true;labels();showDiagnostic();if(root.dataset.state)showStatus(root.dataset.state);schedule();}
 else if(data?.type==='status'){if(data.status!=='success'){renderSequence++;cancelTransition();player.resumeAfterTransition=false;}showStatus(data.status);}
 else if(data?.type==='result'){
  if(data.document){
   pageSelect.replaceChildren();for(const [i,p] of data.document.pages.entries()){const option=el('option','',pageSelect,`${i+1}. ${p.name||p.id}`);option.value=p.id;}
   pageSelect.value=data.document.pageId;pageSelect.hidden=openPage.hidden=exportDocument.hidden=false;syncPages();
  }
  diagnostic=data.error;showDiagnostic();
  if(diagnostic){showStatus('error');}
  else if(data.protocol===3&&(data.displayList||data.displayListCompressed||data.displayListUpdate)&&data.inspection){installFrame(data).catch(error=>{if(String(error).includes('baseline mismatch')){bridge.postMessage({type:'request-full'});return;}diagnostic={code:'E_PREVIEW_RESOURCE',message:String(error)};showDiagnostic();showStatus('error');});}
  else{diagnostic={code:'E_PREVIEW_PROTOCOL',message:'Expected drawing protocol 3'};showDiagnostic();showStatus('error');}
 }
});
window.addEventListener('pagehide',()=>{media.dispose();cancelAnimationFrame(mediaRaf);presentation.dispose();renderSequence++;clearTimeout(paintTimer);pendingPaint=null;stopWorker(Error('Preview closed'));});
labels();bridge.postMessage({type:'ready'});

addEventListener("pagehide",()=>retainedDeck.dispose());
