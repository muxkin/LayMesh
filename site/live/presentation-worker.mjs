import init,{prepare_display_list,register_render_resource,retain_render_resources} from './wasm/laymesh_wasm.js';
import {initializeSkia} from './skia-renderer.mjs';
import {PresentationRenderer} from './presentation-renderer.mjs';
export async function createPresentationRuntime(data,notify=()=>{}){
 let renderer,current,lastPaint,latest=0,jobs=new Map(),resources=new Set(),memory;
 const [wasm,ck]=await Promise.all([init({module_or_path:data.wasmURL}),initializeSkia(data.skiaURL)]);memory=wasm.memory;renderer=new PresentationRenderer(ck,data.canvas,{budget:data.budget,onContextChange:ready=>notify({type:'context',ready})});
const prune=()=>{for(const id of [...jobs.keys()])if(!renderer.pages.has(id)){jobs.get(id).job.free();jobs.delete(id);}const ids=new Set([...jobs.values()].flatMap(p=>Object.keys(p.list.resources)));retain_render_resources(JSON.stringify([...ids]));resources=ids;};
 return async data=>{let result={};if(data.type==='target'){latest=data.serial;return {};}if(data.type==='activate')latest=Math.max(latest,data.serial);
  if(data.type==='load'||data.type==='prepare'){
   const p=data.page||{id:data.pageId};if(!jobs.has(p.id)){
    for(const [id,bytes]of Object.entries(p.resources))if(!resources.has(id)){register_render_resource(id,bytes);resources.add(id);}
    const json=JSON.stringify(p.list),job=prepare_display_list(json);try{await renderer.install(p.id,p.list,id=>job.resource_bytes(id),json.length*2+Object.values(p.resources).reduce((n,b)=>n+b.byteLength,0));jobs.set(p.id,{job,list:p.list});}catch(e){job.free();throw e;}
   }
   renderer.evict(new Set([current,p.id]));prune();result.preparation=await renderer.prepare(p.id,data.width,data.height,(time,reduced)=>JSON.parse(jobs.get(p.id).job.frame_state(time,reduced)),!!data.priority);renderer.evict(new Set([current,p.id]));prune();result.resident=[...jobs.keys()];result.readyResident=[...renderer.pages.values()].filter(p=>p.ready&&p.readySize?.[0]===data.width&&p.readySize?.[1]===data.height).map(p=>p.id);
  }else if(data.type==='activate'||data.type==='paint'){
   if(data.serial!==latest){return {type:'stale'};}
   if(!jobs.has(data.pageId))throw Error('Presentation page was evicted');
   const p=jobs.get(data.pageId),start=performance.now(),states=data.time===null?{}:JSON.parse(p.job.frame_state(data.time,!!data.reduced)),sampleMs=performance.now()-start;
   result=renderer.render(data.pageId,data.width,data.height,states,data.type==='paint'&&data.running);if(!result.skipped){current=data.pageId;lastPaint=data;}
   Object.assign(result,{sampleMs,wasmMemoryBytes:memory.buffer.byteLength,resident:[...jobs.keys()]});
  }else if(data.type==='capture'){if(!lastPaint)throw Error('No presentation frame to capture');const p=lastPaint,states=p.time===null?{}:JSON.parse(jobs.get(p.pageId).job.frame_state(p.time,!!p.reduced));renderer.render(p.pageId,p.width,p.height,states,false);result.rgba=renderer.skia.readPixels();}
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
