// The packaging script inlines the generated WASM module here: VS Code
// requires workers loaded from a blob, without external module imports.
import init,{prepare_display_list,register_render_resource,retain_render_resources,set_render_cache_budget} from './wasm/laymesh_wasm.js';
let rasterInitialized,rasterJob,rasterJobId='',rasterGeneration,rasterMemory,rasterPage,overviewReady=false,rasterChain=Promise.resolve();
const rasterResources=new Set();
self.onmessage=({data})=>{
 if(data.type==='init'){rasterInitialized=init({module_or_path:data.wasmURL}).then(exports=>{rasterMemory=exports.memory;self.postMessage({requestId:data.requestId,type:'ready'});});rasterInitialized.catch(error=>self.postMessage({type:'error',requestId:data.requestId,error:String(error)}));return;}
 rasterChain=rasterChain.then(async()=>{
  try{
   const beforeReady=performance.now();await rasterInitialized;
   if(data.type==='install'){
    const beforeResources=performance.now();
    const versions=data.displayListReplay||[data];
    let reused=true,resourcesMs=0;
    for(const version of versions){
    const resourceStart=performance.now();
    await Promise.all((version.resources||[]).map(async resource=>{if(rasterResources.has(resource.id))return;
     const response=await fetch(resource.uri);if(!response.ok)throw Error('Drawing resource loading failed');
     register_render_resource(resource.id,new Uint8Array(await response.arrayBuffer()));rasterResources.add(resource.id);
    }));resourcesMs+=performance.now()-resourceStart;
    const compressed=version.displayListCompressed,id=version.displayListId||compressed?.id||'',same=!!rasterJob&&id&&id===rasterJobId;
    reused&&=!!same;
    if(version.displayListUpdate&&!same){
     if(!rasterJob||version.displayListUpdate.base!==rasterJobId)throw Error('Drawing update baseline mismatch');
     rasterJob.update_display_list(JSON.stringify(version.displayListUpdate.update));rasterJobId=id;overviewReady=false;
    }else if(!same){
     let json;
     if(compressed){
      if(compressed.codec!=='gzip'||!Number.isSafeInteger(compressed.uncompressedBytes)||compressed.uncompressedBytes<0)throw Error('Invalid compressed drawing commands');
      const bytes=typeof compressed.data==='string'?Uint8Array.from(atob(compressed.data),c=>c.charCodeAt(0)):compressed.data;
      const decoded=new Uint8Array(await new Response(new Blob([bytes]).stream().pipeThrough(new DecompressionStream('gzip'))).arrayBuffer());
      if(decoded.byteLength!==compressed.uncompressedBytes)throw Error('Drawing command length mismatch');
      json=new TextDecoder('utf-8',{fatal:true}).decode(decoded);
     }else json=JSON.stringify(version.displayList);
     const next=prepare_display_list(json);rasterJob?.free();rasterJob=next;rasterJobId=id;overviewReady=false;
    }
    }
    const beforeInstall=beforeResources;
    const budget=data.previewSettings?.cache_mb??256;set_render_cache_budget(budget);rasterJob.set_cache_budget(budget);
    rasterGeneration=data.generation;
    rasterPage=data.page;
    const ids=Object.keys(data.drawingResources||data.displayList.resources);retain_render_resources(JSON.stringify(ids));
    for(const id of rasterResources)if(!ids.includes(id))rasterResources.delete(id);
    self.postMessage({type:'installed',requestId:data.requestId,generation:rasterGeneration,reused,wasmMemoryBytes:rasterMemory.buffer.byteLength,initializationWaitMs:beforeResources-beforeReady,resourcesMs,installMs:performance.now()-beforeInstall-resourcesMs});
   }else if(data.type==='paint'){
    if(!rasterJob||data.generation!==rasterGeneration){self.postMessage({type:'paint-skipped',requestId:data.requestId,generation:data.generation});return;}
    const before=performance.now(),pixels=(data.time==null?rasterJob.raster_retained(JSON.stringify(data.crop),data.width,data.height):rasterJob.raster_animation(JSON.stringify(data.crop),data.width,data.height,data.time,!!data.reduced)),rgba=pixels.into_rgba();
    let overview;const transfer=[rgba.buffer];
    if(data.crop[0]===0&&data.crop[1]===0&&data.crop[2]===rasterPage.width&&data.crop[3]===rasterPage.height)overviewReady=true;
    if(!overviewReady&&data.time==null){const p=rasterPage,s=Math.min(768/p.width,2048/p.height,Math.sqrt(500000/(p.width*p.height))),width=Math.max(1,Math.floor(p.width*s)),height=Math.max(1,Math.floor(p.height*s));const frame=rasterJob.raster_retained(JSON.stringify([0,0,p.width,p.height]),width,height);overview={width,height,rgba:frame.into_rgba()};transfer.push(overview.rgba.buffer);overviewReady=true;}
    self.postMessage({type:'painted',requestId:data.requestId,generation:rasterGeneration,width:data.width,height:data.height,crop:data.crop,rgba,overview,rasterMs:performance.now()-before,wasmMemoryBytes:rasterMemory.buffer.byteLength},transfer);
   }
  }catch(error){self.postMessage({type:'error',requestId:data.requestId,generation:data.generation,error:String(error)});}
 });
};
