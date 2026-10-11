import {initializeSkia,SkiaRenderer} from './skia-renderer.mjs';
import init, {prepare_render,prepare_document_render,prepare_presentation,register_preview_font} from './wasm/laymesh_wasm.js';
let initialized, manifest, base,wasmMemory,fullFonts,skia;
const assets={}, fonts={};
let chain=Promise.resolve(),drawJob,generation,lastPaint;
const key=name=>'/'+name.replace(/^\/+/, '');
const detail=error=>{try{return JSON.parse(String(error));}catch{return {message:error?.message||String(error)};}};
// SubtleCrypto is unavailable on the HTTP Tailscale preview, so checksum bytes here.
function sha256(bytes){
 const k=[0x428a2f98,0x71374491,0xb5c0fbcf,0xe9b5dba5,0x3956c25b,0x59f111f1,0x923f82a4,0xab1c5ed5,0xd807aa98,0x12835b01,0x243185be,0x550c7dc3,0x72be5d74,0x80deb1fe,0x9bdc06a7,0xc19bf174,0xe49b69c1,0xefbe4786,0x0fc19dc6,0x240ca1cc,0x2de92c6f,0x4a7484aa,0x5cb0a9dc,0x76f988da,0x983e5152,0xa831c66d,0xb00327c8,0xbf597fc7,0xc6e00bf3,0xd5a79147,0x06ca6351,0x14292967,0x27b70a85,0x2e1b2138,0x4d2c6dfc,0x53380d13,0x650a7354,0x766a0abb,0x81c2c92e,0x92722c85,0xa2bfe8a1,0xa81a664b,0xc24b8b70,0xc76c51a3,0xd192e819,0xd6990624,0xf40e3585,0x106aa070,0x19a4c116,0x1e376c08,0x2748774c,0x34b0bcb5,0x391c0cb3,0x4ed8aa4a,0x5b9cca4f,0x682e6ff3,0x748f82ee,0x78a5636f,0x84c87814,0x8cc70208,0x90befffa,0xa4506ceb,0xbef9a3f7,0xc67178f2];
 const h=[0x6a09e667,0xbb67ae85,0x3c6ef372,0xa54ff53a,0x510e527f,0x9b05688c,0x1f83d9ab,0x5be0cd19];
 const size=Math.ceil((bytes.length+9)/64)*64,padded=new Uint8Array(size);padded.set(bytes);padded[bytes.length]=0x80;
 const view=new DataView(padded.buffer);view.setUint32(size-8,Math.floor(bytes.length*8/2**32));view.setUint32(size-4,(bytes.length*8)>>>0);
 const rotr=(v,n)=>(v>>>n)|(v<<(32-n)),w=new Uint32Array(64);
 for(let offset=0;offset<size;offset+=64){
  for(let i=0;i<16;i++)w[i]=view.getUint32(offset+i*4);
  for(let i=16;i<64;i++){const a=w[i-15],b=w[i-2];w[i]=(w[i-16]+(rotr(a,7)^rotr(a,18)^(a>>>3))+w[i-7]+(rotr(b,17)^rotr(b,19)^(b>>>10)))>>>0;}
  let [a,b,c,d,e,f,g,q]=h;
  for(let i=0;i<64;i++){const s1=rotr(e,6)^rotr(e,11)^rotr(e,25),ch=(e&f)^(~e&g),t1=(q+s1+ch+k[i]+w[i])>>>0,s0=rotr(a,2)^rotr(a,13)^rotr(a,22),maj=(a&b)^(a&c)^(b&c),t2=(s0+maj)>>>0;q=g;g=f;f=e;e=(d+t1)>>>0;d=c;c=b;b=a;a=(t1+t2)>>>0;}
  for(const [i,v] of [a,b,c,d,e,f,g,q].entries())h[i]=(h[i]+v)>>>0;
 }
 return h.map(v=>v.toString(16).padStart(8,'0')).join('');
}
async function checkedFont(url,expected,timeout,signal){
 const controller=new AbortController(),timer=setTimeout(()=>controller.abort(),timeout);
 const abort=()=>controller.abort();if(signal?.aborted)abort();else signal?.addEventListener('abort',abort,{once:true});
 try{const response=await fetch(url,{signal:controller.signal});if(!response.ok)throw Error(`HTTP ${response.status}`);
  const bytes=new Uint8Array(await response.arrayBuffer());if(sha256(bytes)!==expected)throw Error('Font SHA-256 mismatch');return bytes;
 }finally{clearTimeout(timer);signal?.removeEventListener('abort',abort);}
}
const fontBytes=font=>checkedFont(new URL(font.file,base),font.sha256,30000);
function ensureFullFonts(){
 return fullFonts??=Promise.all(manifest.fonts.filter(font=>font.full).map(async font=>{
  const bytes=await fontBytes(font.full);
  // Replace the startup collection at the same key, preserving fallback order.
  register_preview_font('/@bundled/'+font.filename,bytes);
 })).catch(error=>{fullFonts=null;throw error;});
}
async function resource(name){
 const path=key(name);
 if(Object.hasOwn(assets,path))return;
 const location=manifest.resources[path];
 if(!location)throw new Error(`Resource is not registered: ${name}`);
 const response=await fetch(new URL(location,base));
 if(!response.ok)throw new Error(`Resource loading failed: ${name}`);
 assets[path]=/\.(lay|laydoc|lcss|css|json|csv)$/.test(name)?await response.text():Array.from(new Uint8Array(await response.arrayBuffer()));
}
function sceneStats(list){
 const stats={nodes:0,groups:0,paths:0,pathElements:0,glyphs:0,glyphPathElements:0,images:0,effectLayers:0,maxDepth:0,resources:Object.keys(list.resources).length,animatedTargets:new Set(list.animation?.normal.steps.flatMap(s=>s.clips.map(c=>c.target))||[]).size};
 const walk=(n,depth)=>{const k=n.kind;stats.nodes++;stats.maxDepth=Math.max(depth,stats.maxDepth);stats.effectLayers+=n.effects.length;
  if(k.kind==='group'){stats.groups++;k.children.forEach(c=>walk(c,depth+1));if(k.mask)walk(k.mask.node,depth+1);}
  if(k.kind==='path'){stats.paths++;stats.pathElements+=k.path.length;}
  if(k.kind==='glyphs'){stats.glyphs+=k.glyphs.length;stats.glyphPathElements+=k.outlines.reduce((sum,p)=>sum+p.length,0);}
  if(k.kind==='image')stats.images++;
 };list.nodes.forEach(n=>walk(n,0));return stats;
}
self.onmessage=({data})=>{
 if(data.type==='compile-deck-page'){compileDeckPage(data).catch(error=>self.postMessage({type:'deck-error',error:detail(error)}));return;}
 if(data.type==='deck-context'){deckContext=data;return;}
 if(data.type==='deck-ack'){deckAck?.();return;}
 if(data.type==='prepare-deck'){prepareDeck(data).catch(error=>self.postMessage({type:'deck-error',error:detail(error)}));return;}
 if(data.type==='init'){
  base=data.base;
  initialized=(async()=>{
   const start=performance.now();
   const fontsReady=(async()=>{
    const response=await fetch(new URL('manifest.json',base));
    if(!response.ok)throw new Error('Preview manifest loading failed');
    manifest=await response.json();return Promise.all(manifest.fonts.map(fontBytes));
   })();
   const skiaReady=data.compilerOnly?Promise.resolve(null):(async()=>{const meta=await(await fetch(new URL('skia/manifest.json',base))).json();return initializeSkia(new URL('skia/'+meta.wasm,base).href);})();
   const [wasm,bytes,ck]=await Promise.all([init(),fontsReady,skiaReady]);wasmMemory=wasm.memory;
   if(data.canvas)skia=new SkiaRenderer(ck,data.canvas,{onContextChange:ready=>self.postMessage({type:'context',ready})});
   manifest.fonts.forEach((font,i)=>{register_preview_font('/@bundled/'+font.filename,bytes[i]);self.postMessage({type:'bundled-font',family:font.family,source:'page'});});
   self.postMessage({type:'ready'});
   return performance.now()-start;
  })().catch(error=>{self.postMessage({type:'init-error',error:detail(error)});throw error;});
  initialized.catch(()=>{});
  return;
 }
 // Downloads must not occupy the render queue while an animation is playing.
 if(data.type==='full-fonts'){
  initialized.then(ensureFullFonts).then(()=>self.postMessage({type:'full-fonts-ready'}),error=>self.postMessage({type:'full-fonts-error',error:detail(error)}));return;
 }
 chain=chain.then(async()=>{
  const {id}=data;
  try{
   const initMs=await initialized;
   if(data.type==='font'){
    fonts[`/@local/${id}/${/\.(ttf|otf|ttc|otc)$/i.test(data.name)?data.name:data.name+'.ttf'}`]=Array.from(new Uint8Array(data.bytes));
    self.postMessage({type:'font-added',id,families:[data.alias||data.name]});return;
   }
   if(data.type==='trim'){skia?.trim();return;}
   if(data.type==='capture'){if(!skia||!lastPaint)throw Error('Capture requires an installed canvas');const p=lastPaint,states=p.time==null?{}:JSON.parse(drawJob.frame_state(p.time,!!p.reduced));skia.render(p.rect,p.width,p.height,states,generation);const rgba=skia.readPixels();self.postMessage({type:'captured',id,rgba},[rgba.buffer]);return;}
   if(data.type==='paint'){
    if(!drawJob||data.generation!==generation){self.postMessage({type:'paint-skipped',id:data.id,generation:data.generation});return;}
    const before=performance.now(),sampled=data.time==null?'{}':drawJob.frame_state(data.time,!!data.reduced),states=JSON.parse(sampled),sampleMs=performance.now()-before;
    lastPaint=data;const metrics=skia?.render(data.rect,data.width,data.height,states,generation,!!data.running,data.cacheStatic!==false,data.cacheLayers!==false)||{};if(metrics.skipped){self.postMessage({type:'paint-skipped',id:data.id,generation,reason:metrics.reason});return;}
    self.postMessage({type:'painted',id:data.id,generation,width:data.width,height:data.height,rect:data.rect,...metrics,sampleMs,samplerOutputChars:sampled.length,states:skia?undefined:states,wasmMemoryBytes:wasmMemory.buffer.byteLength});return;
   }
   const edited={};for(const [name,text]of Object.entries(data.files))edited[key(name)]=text;
   let resourcesMs=0, compileMs=0, job;
   const attempted=new Set();
   for(;;){
    const start=performance.now();
    try{
     const files=JSON.stringify({...assets,...fonts,...edited});
     job=data.entry.endsWith('.laydoc')?prepare_document_render(data.files[data.entry],key(data.entry),files,data.pageId||''):prepare_render(data.files[data.entry],key(data.entry),files);
     compileMs+=performance.now()-start;break;
    }catch(error){
     compileMs+=performance.now()-start;
     const request=detail(error),path=request.resource;
     // Requests are discovered by the actual compiler, including transitive
     // imports and computed paths. Never fetch a path outside this manifest or
     // replace an unsaved buffer with its older disk snapshot.
     if(!path||!Object.hasOwn(manifest.resources,path)||Object.hasOwn(edited,path)||attempted.has(path))throw error;
     attempted.add(path);
     self.postMessage({type:'resources',id,waiting:true});
     const before=performance.now();
     try{await resource(path);}finally{
      resourcesMs+=performance.now()-before;
      self.postMessage({type:'resources',id,waiting:false});
     }
    }
   }
   try{
    const before=performance.now();
    const next=job.draw_job();
    const lowerMs=performance.now()-before;
    const result=JSON.parse(job.inspection()),drawingInfo=JSON.parse(next.drawing_info());
    const list=JSON.parse(next.display_list()),read=(id)=>next.resource_bytes(id);
    try{if(skia)await skia.install(list,read,id);}catch(error){next.free();throw error;}
    drawJob?.free();drawJob=next;generation=id;
    if(!skia)self.postMessage({type:'skia-scene',generation,list,resources:Object.fromEntries(Object.entries(list.resources).filter(([,r])=>r.mime.startsWith('image/')).map(([id])=>[id,read(id)]))});
    self.postMessage({type:'result',id,generation,protocol:3,...result,drawingInfo,sceneStats:data.stats?sceneStats(list):undefined,resources:Object.entries(drawingInfo.resources).map(([id,meta])=>({id,...meta,resident:true})),metrics:{initialization:initMs,resources:resourcesMs,compile:compileMs,lower:lowerMs,raster:0,total:compileMs+lowerMs}});
   }finally{job.free();}
  }catch(error){self.postMessage({type:data.type==='paint'?'paint-error':'error',id,generation:data.generation,error:detail(error)});}
 });
};

let deckAck,deckContext;
async function compileDeckPage({page,index,total,resourceIds=[]}){
 const started=performance.now();await initialized;await chain;
 if(deckContext.fullFonts)await ensureFullFonts();
 const ready=performance.now(),edited=Object.fromEntries(Object.entries(deckContext.files).map(([k,v])=>[key(k),v]));
 let job,draw;const tried=new Set();let compiled,lowered,serialized;
 try{
  for(;;)try{
   const path=page.src||key(deckContext.entry);if(!Object.hasOwn(edited,path)&&!Object.hasOwn(assets,path))await resource(path);
   job=prepare_render(edited[path]??assets[path],path,JSON.stringify({...assets,...fonts,...edited}));break;
  }catch(e){const d=detail(e);if(!d.resource||tried.has(d.resource)||Object.hasOwn(edited,d.resource)||!manifest.resources[d.resource])throw e;tried.add(d.resource);await resource(d.resource);}
  compiled=performance.now();draw=job.draw_job();lowered=performance.now();
  const list=JSON.parse(draw.display_list()),resources=Object.fromEntries(Object.keys(list.resources).filter(id=>!resourceIds.includes(id)).map(id=>[id,draw.resource_bytes(id)])),inspection=JSON.parse(job.inspection()).inspection;serialized=performance.now();
  self.postMessage({type:'deck-page',page:{id:page.id,name:page.name||page.id,list,resources,inspection},timings:{index,fontsMs:ready-started,compileMs:compiled-ready,lowerMs:lowered-compiled,serializeMs:serialized-lowered}},Object.values(resources).map(b=>b.buffer));
 }catch(e){const error=detail(e);error.message=`${index+1}/${total} ${page.name||page.id}: ${error.message}`;throw JSON.stringify(error);}finally{draw?.free();job?.free();}
}
async function prepareDeck(data){
 await initialized;await chain;if(data.fullFonts)await ensureFullFonts();
 const edited={};for(const [name,text]of Object.entries(data.files))edited[key(name)]=text;
 const encoded=()=>JSON.stringify({...assets,...fonts,...edited});
 async function attempt(fn){const tried=new Set();for(;;)try{return fn();}catch(e){const d=detail(e);if(!d.resource||tried.has(d.resource)||Object.hasOwn(edited,d.resource)||!manifest.resources[d.resource])throw e;tried.add(d.resource);await resource(d.resource);}}
 let plan;const started=performance.now();
 try{
  if(data.entry.endsWith('.laydoc'))plan=await attempt(()=>prepare_presentation(data.files[data.entry],key(data.entry),encoded()));
  const seen=new Map();const metadata=plan?JSON.parse(plan.metadata()):{pages:[{id:'page',name:data.entry.split('/').at(-1)}]};
  self.postMessage({type:'deck-plan',...metadata});
  if(data.planOnly)return;
  for(let i=0;i<metadata.pages.length;i++){
   const page=metadata.pages[i];if(page.src&&seen.has(page.src)){const ack=new Promise(resolve=>deckAck=resolve);self.postMessage({type:'deck-alias',page,source:seen.get(page.src)});await ack;continue;}if(page.src)seen.set(page.src,page.id);self.postMessage({type:'deck-progress',completed:i,total:metadata.pages.length,page:page.name||page.id});
   let job,draw;try{job=await attempt(()=>plan?plan.page(i,encoded()):prepare_render(data.files[data.entry],key(data.entry),encoded()));draw=job.draw_job();const list=JSON.parse(draw.display_list()),resources=Object.fromEntries(Object.keys(list.resources).map(id=>[id,draw.resource_bytes(id)]));
    const ack=new Promise(resolve=>deckAck=resolve);self.postMessage({type:'deck-page',page:{id:page.id,name:page.name||page.id,list,resources,inspection:JSON.parse(job.inspection()).inspection}},Object.values(resources).map(b=>b.buffer));await ack;
   }catch(e){const error=detail(e);error.message=`${i+1}/${metadata.pages.length} ${page.name||page.id}: ${error.message}`;throw JSON.stringify(error);}finally{draw?.free();job?.free();}
  }
  self.postMessage({type:'deck-ready',prepareMs:performance.now()-started});
 }finally{plan?.free();deckAck=null;}
}
