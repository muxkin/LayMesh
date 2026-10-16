import {initializeSkia,SkiaRenderer} from './skia-renderer.mjs';
import {applyDisplayUpdate} from './display-update.mjs';
// The packaging script inlines the generated WASM module here: VS Code
// requires workers loaded from a blob, without external module imports.
import init,{prepare_display_list,register_render_resource,retain_render_resources} from './wasm/laymesh_wasm.js';
let installedList,skia, rasterInitialized,rasterJob,rasterJobId='',rasterGeneration,rasterMemory,rasterPage,overviewReady=false,rasterChain=Promise.resolve();
const rasterResources=new Set();
self.onmessage=({data})=>{
 if(['install','cancel-transition'].includes(data.type))skia?.transitions.cancel();
 if(data.type==='cancel-transition')return;
 if(data.type==='init'){rasterInitialized=Promise.all([init({module_or_path:data.wasmBytes}),initializeSkia(data.skiaBytes)]).then(([exports,ck])=>{rasterMemory=exports.memory;if(data.canvas)skia=new SkiaRenderer(ck,data.canvas,{onContextChange:ready=>self.postMessage({type:'context',ready})});self.postMessage({requestId:data.requestId,type:'ready'});});rasterInitialized.catch(error=>self.postMessage({type:'error',requestId:data.requestId,error:String(error)}));return;}
 rasterChain=rasterChain.then(async()=>{
  try{
   const beforeReady=performance.now();await rasterInitialized;
   if(data.type==='trim'){skia?.trim();return;}
   if(data.type==='install'){
    const beforeResources=performance.now();
    const versions=data.displayListReplay||[data];
    const resourceBytes=new Map((data.resourceBytes||[]).map(r=>[r.id,r.bytes]));
    let reused=true,resourcesMs=0,incremental=!!installedList;
    for(const version of versions){
    const resourceStart=performance.now();
    for(const resource of version.resources||[]){if(rasterResources.has(resource.id))continue;
     const bytes=resourceBytes.get(resource.id);if(!bytes)throw Error(`Drawing resource bytes missing: ${resource.id}`);
     register_render_resource(resource.id,new Uint8Array(bytes));rasterResources.add(resource.id);
    }resourcesMs+=performance.now()-resourceStart;
    const compressed=version.displayListCompressed,id=version.displayListId||compressed?.id||'',same=!!rasterJob&&id&&id===rasterJobId;
    reused&&=!!same;
    if(version.displayListUpdate&&!same){
     if(!rasterJob||version.displayListUpdate.base!==rasterJobId)throw Error('Drawing update baseline mismatch');
     const next=applyDisplayUpdate(installedList,version.displayListUpdate.update);rasterJob.update_display_list(JSON.stringify(version.displayListUpdate.update));installedList=next;rasterJobId=id;overviewReady=false;
    }else if(!same){
     let json;
     if(compressed){
      if(compressed.codec!=='gzip'||!Number.isSafeInteger(compressed.uncompressedBytes)||compressed.uncompressedBytes<0)throw Error('Invalid compressed drawing commands');
      const bytes=typeof compressed.data==='string'?Uint8Array.from(atob(compressed.data),c=>c.charCodeAt(0)):compressed.data;
      const decoded=new Uint8Array(await new Response(new Blob([bytes]).stream().pipeThrough(new DecompressionStream('gzip'))).arrayBuffer());
      if(decoded.byteLength!==compressed.uncompressedBytes)throw Error('Drawing command length mismatch');
      json=new TextDecoder('utf-8',{fatal:true}).decode(decoded);
     }else json=JSON.stringify(version.displayList);
     const next=prepare_display_list(json);installedList=JSON.parse(json);incremental=false;rasterJob?.free();rasterJob=next;rasterJobId=id;overviewReady=false;
    }
    }
    const beforeInstall=beforeResources;
    rasterGeneration=data.generation;
    rasterPage=data.page;
    const ids=Object.keys(data.drawingResources||data.displayList.resources);retain_render_resources(JSON.stringify(ids));
    for(const id of rasterResources)if(!ids.includes(id))rasterResources.delete(id);
    const list=installedList;if(skia)await skia.install(list,id=>rasterJob.resource_bytes(id),rasterGeneration,data.navigation,incremental);
    self.postMessage({scene:skia?undefined:list,sceneResources:skia?undefined:Object.fromEntries(Object.entries(list.resources).filter(([,r])=>r.mime.startsWith('image/')).map(([id])=>[id,rasterJob.resource_bytes(id)])),type:'installed',requestId:data.requestId,generation:rasterGeneration,reused,resourceIds:[...rasterResources],wasmMemoryBytes:rasterMemory.buffer.byteLength,initializationWaitMs:beforeResources-beforeReady,resourcesMs,installMs:performance.now()-beforeInstall-resourcesMs});
   }else if(data.type==='paint'){
    if(!rasterJob||data.generation!==rasterGeneration){self.postMessage({type:'paint-skipped',requestId:data.requestId,generation:data.generation});return;}
    const before=performance.now(),states=data.time==null?{}:JSON.parse(rasterJob.frame_state(data.time,!!data.reduced)),sampleMs=performance.now()-before;
    const metrics=skia?.render(data.crop,data.width,data.height,states,rasterGeneration,!!data.running)||{};if(metrics.skipped){self.postMessage({type:'paint-skipped',requestId:data.requestId,generation:rasterGeneration});return;}
    if(skia)Object.assign(metrics,await skia.transitions.play(data.reduced));
    if(skia&&!data.running)metrics.presentationConfirmed=await skia.completeFrame(data.visible!==false);
    self.postMessage({type:'painted',requestId:data.requestId,generation:rasterGeneration,width:data.width,height:data.height,crop:data.crop,...metrics,sampleMs,states:skia?undefined:states,wasmMemoryBytes:rasterMemory.buffer.byteLength});
   }
  }catch(error){self.postMessage({type:'error',requestId:data.requestId,generation:data.generation,error:String(error)});}
 });
};
