import init,{prepare_display_list,register_render_resource,retain_render_resources} from './wasm/laymesh_wasm.js';
import {initializeSkia} from './skia-renderer.mjs';
import {PresentationRenderer} from './presentation-renderer.mjs';
import {applyDisplayUpdate} from './display-update.mjs';
// Several presenter surfaces share one WASM instance in the same window.
// Retain the union of their resources so disposing/evicting a thumbnail cannot
// remove an image still used by the current or next-page renderer.
let wasmRuntime;
const resourceOwners=new Set(),registeredResources=new Set();
function retainSharedResources(){const ids=new Set([...resourceOwners].flatMap(owner=>[...owner]));retain_render_resources(JSON.stringify([...ids]));for(const id of registeredResources)if(!ids.has(id))registeredResources.delete(id);}
// Legacy endpoint lists contain filled outlines but no editable centerline.
// Diagnose once at installation; frame sampling keeps their static appearance.
function legacyDrawWarnings(list){
 const nodes=new Map();const walk=n=>{if(n.name)nodes.set(n.name,n);if(n.kind.kind==='group')n.kind.children.forEach(walk);};list.nodes.forEach(walk);
 const drawable=n=>!!n?.hint?.drawGeometry||(n?.kind.kind==='path'&&n.kind.fill.kind==='none'&&n.kind.stroke?.width>0)||(n?.kind.kind==='group'&&n.kind.children.some(drawable));
 const targets=new Set(Object.values(list.animation||{}).flatMap(t=>(t.steps||[]).flatMap(s=>s.clips.filter(c=>c.tracks.some(t=>t.property==='-laymesh-draw')).map(c=>c.target))));
 return [...targets].filter(id=>!drawable(nodes.get(id))).map(id=>({code:'W_ANIMATION',message:`${id}: 此文件缺少可绘制的中心线，保留静态对象；请重新导出作品以使用绘制动画。`}));
}
export async function createPresentationRuntime(data,notify=()=>{}){
 let renderer,current,lastPaint,presenting,latest=0,backgroundPaused=false,jobs=new Map(),resources=new Set(),memory,frameDelivery=Promise.resolve(),frameSequence=0;
 const bitmapFrames=!!data.bitmapFrames;
 const onPresent=()=>{if(!presenting)return;data.onPresent?.({...presenting});if(bitmapFrames){
  // Explicit ownership transfer avoids acknowledging a placeholder canvas
  // before the browser has copied the OffscreenCanvas bitmap into it.
  const frame={type:'frame',...presenting,sequence:++frameSequence};
  const bitmap=createImageBitmap(data.canvas).then(value=>({value}),error=>({error}));
  frameDelivery=frameDelivery.catch(()=>{}).then(async()=>{
   const settled=await bitmap;if(settled.error)throw settled.error;
   try{await notify({...frame,bitmap:settled.value});}catch(error){settled.value.close();throw error;}
  });
  // Transitions may still be running when capture fails. The activation awaits
  // this same promise below; observe it now to avoid an unhandled rejection.
  frameDelivery.catch(()=>{});
 }};
 const [wasm,ck]=await Promise.all([wasmRuntime??=init({module_or_path:data.wasmBytes||data.wasmURL}).catch(error=>{wasmRuntime=null;throw error;}),(data.skiaInstance||initializeSkia(data.skiaBytes||data.skiaURL))]);memory=wasm.memory;renderer=new PresentationRenderer(ck,data.canvas,{software:data.software,preserveDrawingBuffer:data.preserveDrawingBuffer,onPresent,onContextChange:ready=>notify({type:'context',ready})});resourceOwners.add(resources);
const prune=()=>{for(const id of [...jobs.keys()])if(!renderer.pages.has(id)){jobs.get(id).job.free();jobs.delete(id);}resources.clear();for(const p of jobs.values())for(const id of Object.keys(p.list.resources))resources.add(id);retainSharedResources();};
 return async data=>{const queueWaitMs=data.sentAt?Math.max(0,performance.timeOrigin+performance.now()-data.sentAt):0;let result={};if(data.type==='target'){latest=data.serial;backgroundPaused=true;renderer.skia.transitions.cancel();return {};}
  if(data.type==='background'){if(data.serial===latest)backgroundPaused=data.paused;return {};}
  if(data.type==='activate')latest=Math.max(latest,data.serial);
  if(data.type==='load'||data.type==='prepare'){
   const cancelled=()=>data.serial!==undefined&&(data.serial!==latest||!data.priority&&backgroundPaused);
   if(cancelled())return {id:data.id,type:'stale'};
   let p=data.page||{id:data.pageId};if(p.packedPage){const raw=p.gzip?await new Response(new Response(p.packedPage).body.pipeThrough(new DecompressionStream('gzip'))).text():new TextDecoder().decode(p.packedPage);p={...JSON.parse(raw),id:p.id,name:p.name,resources:p.resources};if(cancelled())return {id:data.id,type:'stale'};}if(p.listJSON)p.list=JSON.parse(p.listJSON);result.installed=!jobs.has(p.id);if(!jobs.has(p.id)){
    for(const [id,bytes]of Object.entries(p.resources)){if(!registeredResources.has(id)){register_render_resource(id,bytes);registeredResources.add(id);}resources.add(id);}
    if(p.listUpdate){
     const base=jobs.get(p.listUpdate.base);if(!base)throw Error('Presentation drawing baseline mismatch');
     const patch=JSON.stringify(p.listUpdate.update),list=applyDisplayUpdate(base.list,p.listUpdate.update);
     try{base.job.update_display_list(patch);await renderer.update(p.listUpdate.base,p.id,p.listUpdate.update,id=>base.job.resource_bytes(id),patch.length*2);}
     catch(error){base.job.free();jobs.delete(p.listUpdate.base);renderer.remove(p.listUpdate.base);throw error;}
     jobs.delete(p.listUpdate.base);jobs.set(p.id,{job:base.job,list,warnings:legacyDrawWarnings(list)});if(current===p.listUpdate.base)current=p.id;
     result.installed=false;result.patched=true;
    }else{
     const json=p.listJSON||JSON.stringify(p.list),job=prepare_display_list(json);try{await renderer.install(p.id,p.list,id=>job.resource_bytes(id),json.length*2+Object.values(p.resources).reduce((n,b)=>n+b.byteLength,0));jobs.set(p.id,{job,list:p.list,warnings:legacyDrawWarnings(p.list)});}catch(e){job.free();throw e;}
    }
   }
   result.warnings=jobs.get(p.id).warnings;prune();
   if(!data.installOnly)result.preparation=data.stage==='transition'?await renderer.prepareTransition(p.id,data.width,data.height,!!data.reduced,!!data.cpuPreparation):data.stage==='entry'
    ?await renderer.prepareEntry(p.id,data.width,data.height,data.entryTime??0,!!data.reduced,Object.assign((time,reduced)=>JSON.parse(jobs.get(p.id).job.presentation_state(time,reduced)),{duration:jobs.get(p.id).list.animation?.duration||data.duration||0}),cancelled,!!data.cpuPreparation)
    :await renderer.prepare(p.id,data.width,data.height,(time,reduced)=>JSON.parse(jobs.get(p.id).job.transform_state(time,reduced)),!!data.priority,data.viewport,cancelled,!!data.cpuPreparation);
   result.resident=[...jobs.keys()];result.readyResident=[...renderer.pages.values()].filter(p=>p.ready&&p.readySize?.[0]===data.width&&p.readySize?.[1]===data.height).map(p=>p.id);
   result.entryFrames=[...renderer.pages.values()].flatMap(p=>[...p.frames?.keys()||[]].map(key=>p.id+'/'+key));
  }else if(data.type==='activate'||data.type==='paint'){
   if(data.serial!==latest){data.previousFrame?.bitmap.close();for(const f of data.mediaFrames||[])f.bitmap.close();return {type:'stale'};}
   if(!jobs.has(data.pageId))throw Error('Presentation page was evicted');
   if(data.type==='activate'){
    const reverse=!!data.transition?.reverse,spec=reverse?jobs.get(current)?.list.pageTransition:jobs.get(data.pageId).list.pageTransition;
    if(current&&renderer.pages.has(current))renderer.use(current);
    if(data.previousFrame){renderer.skia.ensureSurface(data.width,data.height);renderer.skia.transitions.captureBitmap(data.transition&&!data.reduced?(reverse?data.previousFrame.transition:spec):null,reverse,data.previousFrame);}
    else renderer.skia.transitions.capture(data.transition&&!data.reduced?spec:null,reverse);
   }
   renderer.use(data.pageId);renderer.skia.setMediaFrames(data.mediaFrames);
   presenting={pageId:data.pageId,serial:data.serial,time:data.time,running:!!data.running||data.type==='activate'&&!!data.transition};
   try{
   const p=jobs.get(data.pageId),start=performance.now();let sampleMs=0;let states={};
   result=!data.mediaFrames?.length&&data.usePrepared&&renderer.show(data.pageId,data.width,data.height,data.time??0,!!data.reduced);
   if(!result){states=data.time===null?{}:JSON.parse(p.job.presentation_state(data.time,!!data.reduced));sampleMs=performance.now()-start;result=renderer.render(data.pageId,data.width,data.height,states,data.type==='paint'&&data.running,data.viewport);}if(!result.skipped){current=data.pageId;lastPaint=data;}
   if(data.type==='activate'&&result.skipped)throw Error('Presentation drawing interrupted; waiting for the rendering context');
   if(data.type==='activate'){
    let visible;
    Object.assign(result,await renderer.skia.transitions.play(data.reduced,()=>{visible=renderer.skia.completeResources().then(()=>notify({type:'navigation-visible',serial:data.serial,pageId:data.pageId,at:performance.timeOrigin+performance.now()}));visible.catch(()=>{});}));
    await visible;
   }
   if(bitmapFrames){await frameDelivery;result.presentationConfirmed=!result.skipped&&data.visible!==false;}
   else if(data.type==='activate'||!data.running&&!result.skipped)result.presentationConfirmed=await renderer.skia.completeFrame(data.visible!==false);
   if(data.serial!==latest)return {type:'stale'};
   Object.assign(result,{mediaStates:Object.keys(p.list.media?.assets||{}).length?states:undefined,sampleMs,wasmMemoryBytes:memory.buffer.byteLength,skiaMemoryBytes:ck.HEAPU8.byteLength,preserveDrawingBuffer:renderer.skia.gl?.getContextAttributes()?.preserveDrawingBuffer,residentCacheBytes:[...renderer.pages.values()].reduce((n,p)=>n+renderer.cost(p),0),resident:[...jobs.keys()]});
   }finally{presenting=null;}
  }else if(data.type==='snapshot'){
   if(!lastPaint)throw Error('No presentation frame to transfer');
   const p=lastPaint,s=renderer.skia;
   // The display worker retains its presented buffer. ImageBitmap captures it
   // directly; navigation must not sample or redraw the outgoing page.
   if(s.canvas.width!==p.width||s.canvas.height!==p.height||s.gl&&!s.gl.getContextAttributes()?.preserveDrawingBuffer){await s.completeResources();if(!renderer.show(p.pageId,p.width,p.height,p.time??0,!!p.reduced)){const states=p.time===null?{}:JSON.parse(jobs.get(p.pageId).job.presentation_state(p.time,!!p.reduced));renderer.render(p.pageId,p.width,p.height,states,true,p.viewport);}}
   const transition=jobs.get(p.pageId).list.pageTransition,extra=Object.fromEntries(Object.entries(transition?.textures||{}).map(([name,id])=>[name,{bytes:renderer.skia.imageBytes.get(id),mime:renderer.skia.list.resources[id]?.mime}]));
   result={bitmap:await createImageBitmap(renderer.skia.canvas),transition,extra};
  }else if(data.type==='retain'){renderer.retain(new Set(data.pages));prune();result.resident=[...jobs.keys()];
  }else if(data.type==='capture'){if(!lastPaint)throw Error('No presentation frame to capture');const p=lastPaint,states=p.time===null?{}:JSON.parse(jobs.get(p.pageId).job.presentation_state(p.time,!!p.reduced));await renderer.skia.completeResources();renderer.render(p.pageId,p.width,p.height,states,!!data.cached,p.viewport);result.rgba=renderer.skia.readPixels();}
  else if(data.type==='dispose'){renderer.dispose();for(const p of jobs.values())p.job.free();jobs.clear();resourceOwners.delete(resources);resources.clear();retainSharedResources();}
  return {id:data.id,type:'done',queueWaitMs,...result};
 };
}
if(typeof document==='undefined'){
 let runtime,chain=Promise.resolve(),latest=0;
 self.onmessage=({data})=>{
  if(data.type==='target'||data.type==='background'){if(data.type==='target')latest=data.serial;runtime?.then(run=>run(data));return;}
  if(data.type==='init')runtime=createPresentationRuntime(data,value=>self.postMessage(value,value.bitmap?[value.bitmap]:[]));
  chain=chain.then(async()=>{try{const run=await runtime;let result=data.type==='init'?{}:await run(data);if(data.type==='activate'&&data.serial<latest)result={type:'stale'};self.postMessage({id:data.id,type:'done',...result},result.bitmap?[result.bitmap]:result.rgba?[result.rgba.buffer]:[]);}catch(error){data.previousFrame?.bitmap.close();self.postMessage({id:data.id,type:'error',error:String(error)});}});
 };
}
