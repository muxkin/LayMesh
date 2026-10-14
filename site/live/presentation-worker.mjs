import init,{prepare_display_list,register_render_resource,retain_render_resources} from './wasm/laymesh_wasm.js';
import {initializeSkia} from './skia-renderer.mjs';
import {PresentationRenderer} from './presentation-renderer.mjs';
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
 let renderer,current,lastPaint,presenting,latest=0,jobs=new Map(),resources=new Set(),memory;
 const [wasm,ck]=await Promise.all([wasmRuntime??=init({module_or_path:data.wasmBytes||data.wasmURL}).catch(error=>{wasmRuntime=null;throw error;}),(data.skiaInstance||initializeSkia(data.skiaBytes||data.skiaURL))]);memory=wasm.memory;renderer=new PresentationRenderer(ck,data.canvas,{budget:data.budget,software:data.software,preserveDrawingBuffer:data.preserveDrawingBuffer,onPresent:()=>{if(presenting)data.onPresent?.({...presenting});},onContextChange:ready=>notify({type:'context',ready})});resourceOwners.add(resources);
const prune=()=>{for(const id of [...jobs.keys()])if(!renderer.pages.has(id)){jobs.get(id).job.free();jobs.delete(id);}resources.clear();for(const p of jobs.values())for(const id of Object.keys(p.list.resources))resources.add(id);retainSharedResources();};
 return async data=>{let result={};if(data.type==='target'){latest=data.serial;renderer.skia.transitions.cancel();return {};}if(data.type==='activate')latest=Math.max(latest,data.serial);
  if(data.type==='load'||data.type==='prepare'){
   const p=data.page||{id:data.pageId};result.installed=!jobs.has(p.id);if(!jobs.has(p.id)){
    for(const [id,bytes]of Object.entries(p.resources)){if(!registeredResources.has(id)){register_render_resource(id,bytes);registeredResources.add(id);}resources.add(id);}
    const json=JSON.stringify(p.list),job=prepare_display_list(json);try{await renderer.install(p.id,p.list,id=>job.resource_bytes(id),json.length*2+Object.values(p.resources).reduce((n,b)=>n+b.byteLength,0));jobs.set(p.id,{job,list:p.list,warnings:legacyDrawWarnings(p.list)});}catch(e){job.free();throw e;}
   }
   result.warnings=jobs.get(p.id).warnings;renderer.evict(new Set([current,p.id]));prune();if(!data.installOnly)result.preparation=await renderer.prepare(p.id,data.width,data.height,(time,reduced)=>JSON.parse(jobs.get(p.id).job.frame_state(time,reduced)),!!data.priority,data.viewport);renderer.evict(new Set([current,p.id]));prune();result.resident=[...jobs.keys()];result.readyResident=[...renderer.pages.values()].filter(p=>p.ready&&p.readySize?.[0]===data.width&&p.readySize?.[1]===data.height).map(p=>p.id);
  }else if(data.type==='activate'||data.type==='paint'){
   if(data.serial!==latest){return {type:'stale'};}
   if(!jobs.has(data.pageId))throw Error('Presentation page was evicted');
   if(data.type==='activate'){
    const reverse=!!data.transition?.reverse,spec=reverse?jobs.get(current)?.list.pageTransition:jobs.get(data.pageId).list.pageTransition;
    if(current&&renderer.pages.has(current))renderer.use(current);
    renderer.skia.transitions.capture(data.transition&&!data.reduced?spec:null,reverse);
   }
   presenting={pageId:data.pageId,serial:data.serial,time:data.time,running:!!data.running||data.type==='activate'&&!!data.transition};
   const p=jobs.get(data.pageId),start=performance.now(),states=data.time===null?{}:JSON.parse(p.job.frame_state(data.time,!!data.reduced)),sampleMs=performance.now()-start;
   result=renderer.render(data.pageId,data.width,data.height,states,data.type==='paint'&&data.running,data.viewport);if(!result.skipped){current=data.pageId;lastPaint=data;}
   if(data.type==='activate'){Object.assign(result,await renderer.skia.transitions.play(data.reduced));if(data.serial!==latest)return {type:'stale'};}
   presenting=null;Object.assign(result,{sampleMs,wasmMemoryBytes:memory.buffer.byteLength,skiaMemoryBytes:ck.HEAPU8.byteLength,preserveDrawingBuffer:renderer.skia.gl?.getContextAttributes()?.preserveDrawingBuffer,residentCacheBytes:[...renderer.pages.values()].reduce((n,p)=>n+renderer.cost(p),0),resident:[...jobs.keys()]});
  }else if(data.type==='capture'){if(!lastPaint)throw Error('No presentation frame to capture');const p=lastPaint,states=p.time===null?{}:JSON.parse(jobs.get(p.pageId).job.frame_state(p.time,!!p.reduced));renderer.render(p.pageId,p.width,p.height,states,false,p.viewport);result.rgba=renderer.skia.readPixels();}
  else if(data.type==='dispose'){renderer.dispose();for(const p of jobs.values())p.job.free();jobs.clear();resourceOwners.delete(resources);resources.clear();retainSharedResources();}
  return {id:data.id,type:'done',...result};
 };
}
if(typeof document==='undefined'){
 let runtime,chain=Promise.resolve(),latest=0;
 self.onmessage=({data})=>{
  if(data.type==='target'){latest=data.serial;runtime?.then(run=>run(data));return;}
  if(data.type==='init')runtime=createPresentationRuntime(data,value=>self.postMessage(value));
  chain=chain.then(async()=>{try{const run=await runtime;let result=data.type==='init'?{}:await run(data);if(data.type==='activate'&&data.serial<latest)result={type:'stale'};self.postMessage({id:data.id,type:'done',...result},result.rgba?[result.rgba.buffer]:[]);}catch(error){self.postMessage({id:data.id,type:'error',error:String(error)});}});
 };
}
