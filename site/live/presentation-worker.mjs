import init,{prepare_display_list,register_render_resource,retain_render_resources} from './wasm/laymesh_wasm.js';
import {initializeSkia} from './skia-renderer.mjs';
import {PresentationRenderer} from './presentation-renderer.mjs';
export async function createPresentationRuntime(data,notify=()=>{}){
 let renderer,current,lastPaint,latest=0,jobs=new Map(),resources=new Set(),memory;
 const [wasm,ck]=await Promise.all([init({module_or_path:data.wasmBytes||data.wasmURL}),initializeSkia(data.skiaBytes||data.skiaURL)]);memory=wasm.memory;renderer=new PresentationRenderer(ck,data.canvas,{budget:data.budget,preserveDrawingBuffer:data.preserveDrawingBuffer,onContextChange:ready=>notify({type:'context',ready})});
const prune=()=>{for(const id of [...jobs.keys()])if(!renderer.pages.has(id)){jobs.get(id).job.free();jobs.delete(id);}const ids=new Set([...jobs.values()].flatMap(p=>Object.keys(p.list.resources)));retain_render_resources(JSON.stringify([...ids]));resources=ids;};
 return async data=>{let result={};if(data.type==='target'){latest=data.serial;renderer.skia.transitions.cancel();return {};}if(data.type==='activate')latest=Math.max(latest,data.serial);
  if(data.type==='load'||data.type==='prepare'){
   const p=data.page||{id:data.pageId};result.installed=!jobs.has(p.id);if(!jobs.has(p.id)){
    for(const [id,bytes]of Object.entries(p.resources))if(!resources.has(id)){register_render_resource(id,bytes);resources.add(id);}
    const json=JSON.stringify(p.list),job=prepare_display_list(json);try{await renderer.install(p.id,p.list,id=>job.resource_bytes(id),json.length*2+Object.values(p.resources).reduce((n,b)=>n+b.byteLength,0));jobs.set(p.id,{job,list:p.list});}catch(e){job.free();throw e;}
   }
   renderer.evict(new Set([current,p.id]));prune();if(!data.installOnly)result.preparation=await renderer.prepare(p.id,data.width,data.height,(time,reduced)=>JSON.parse(jobs.get(p.id).job.frame_state(time,reduced)),!!data.priority,data.viewport);renderer.evict(new Set([current,p.id]));prune();result.resident=[...jobs.keys()];result.readyResident=[...renderer.pages.values()].filter(p=>p.ready&&p.readySize?.[0]===data.width&&p.readySize?.[1]===data.height).map(p=>p.id);
  }else if(data.type==='activate'||data.type==='paint'){
   if(data.serial!==latest){return {type:'stale'};}
   if(!jobs.has(data.pageId))throw Error('Presentation page was evicted');
   if(data.type==='activate'){
    const reverse=!!data.transition?.reverse,spec=reverse?jobs.get(current)?.list.pageTransition:jobs.get(data.pageId).list.pageTransition;
    if(current&&renderer.pages.has(current))renderer.use(current);
    renderer.skia.transitions.capture(data.transition&&!data.reduced?spec:null,reverse);
   }
   renderer.use(data.pageId);renderer.skia.setMediaFrames(data.mediaFrames);
   const p=jobs.get(data.pageId),start=performance.now(),states=data.time===null?{}:JSON.parse(p.job.frame_state(data.time,!!data.reduced)),sampleMs=performance.now()-start;
   result=renderer.render(data.pageId,data.width,data.height,states,data.type==='paint'&&data.running,data.viewport);if(!result.skipped){current=data.pageId;lastPaint=data;}
   if(data.type==='activate'){Object.assign(result,await renderer.skia.transitions.play(data.reduced));if(data.serial!==latest)return {type:'stale'};}
   Object.assign(result,{mediaStates:p.list.media?.cues?.length?states:undefined,sampleMs,wasmMemoryBytes:memory.buffer.byteLength,skiaMemoryBytes:ck.HEAPU8.byteLength,preserveDrawingBuffer:renderer.skia.gl?.getContextAttributes()?.preserveDrawingBuffer,residentCacheBytes:[...renderer.pages.values()].reduce((n,p)=>n+renderer.cost(p),0),resident:[...jobs.keys()]});
  }else if(data.type==='capture'){if(!lastPaint)throw Error('No presentation frame to capture');const p=lastPaint,states=p.time===null?{}:JSON.parse(jobs.get(p.pageId).job.frame_state(p.time,!!p.reduced));renderer.render(p.pageId,p.width,p.height,states,false,p.viewport);result.rgba=renderer.skia.readPixels();}
  else if(data.type==='dispose'){renderer.dispose();for(const p of jobs.values())p.job.free();jobs.clear();retain_render_resources('[]');}
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
