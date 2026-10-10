import init, {prepare_render,prepare_document_render,register_preview_font} from './wasm/laymesh_wasm.js';
let initialized, manifest, base,wasmMemory;
const assets={}, fonts={};
let chain=Promise.resolve(),drawJob,generation,drawPage,overviewReady=false;
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
async function loadBundledFont(font){
 // Give the CDN a short head start, then race the packaged copy. Both paths
 // verify the same checksum; an unreachable CDN must not delay offline use.
 const controller=new AbortController();let timer,local;
 const packaged=()=>local??=checkedFont(new URL(font.file,base),font.sha256,30000,controller.signal).then(bytes=>({bytes,source:'page'}));
 const fallback=new Promise(resolve=>{timer=setTimeout(()=>resolve(packaged()),100);});
 try{
  const {bytes,source}=await Promise.any([checkedFont(font.cdn,font.sha256,5000,controller.signal).then(bytes=>({bytes,source:'cdn'}),packaged),fallback]);
  register_preview_font('/@bundled/'+font.file.split('/').pop(),bytes);
  self.postMessage({type:'bundled-font',family:font.family,source});
 }finally{clearTimeout(timer);controller.abort();}
}
async function resource(name){
 const path=key(name);
 if(Object.hasOwn(assets,path))return;
 const location=manifest.resources[path];
 if(!location)throw new Error(`Resource is not registered: ${name}`);
 const response=await fetch(new URL(location,base));
 if(!response.ok)throw new Error(`Resource loading failed: ${name}`);
 assets[path]=/\.(lay|laydoc|lcss|json|csv)$/.test(name)?await response.text():Array.from(new Uint8Array(await response.arrayBuffer()));
}
self.onmessage=({data})=>{
 if(data.type==='init'){
  base=data.base;
  initialized=(async()=>{
   const start=performance.now();wasmMemory=(await init()).memory;
   const response=await fetch(new URL('manifest.json',base));
   if(!response.ok)throw new Error('Preview manifest loading failed');
   manifest=await response.json();await Promise.all(manifest.fonts.map(loadBundledFont));self.postMessage({type:'ready'});
   return performance.now()-start;
  })().catch(error=>{self.postMessage({type:'init-error',error:detail(error)});throw error;});
  return;
 }
 chain=chain.then(async()=>{
  const {id}=data;
  try{
   const initMs=await initialized;
   if(data.type==='font'){
    fonts[`/@local/${id}/${/\.(ttf|otf|ttc|otc)$/i.test(data.name)?data.name:data.name+'.ttf'}`]=Array.from(new Uint8Array(data.bytes));
    self.postMessage({type:'font-added',id,families:[data.alias||data.name]});return;
   }
   if(data.type==='paint'){
    if(!drawJob||data.generation!==generation){self.postMessage({type:'paint-skipped',id:data.id,generation:data.generation});return;}
    const before=performance.now(),frame=drawJob.raster_retained(JSON.stringify(data.rect),data.width,data.height);
    const width=frame.width,height=frame.height,rgba=frame.into_rgba(),transfer=[rgba.buffer];let overview;
    if(data.rect[0]===0&&data.rect[1]===0&&data.rect[2]===drawPage.width&&data.rect[3]===drawPage.height)overviewReady=true;
    if(!overviewReady){const p=drawPage,s=Math.min(768/p.width,2048/p.height,Math.sqrt(500000/(p.width*p.height))),width=Math.max(1,Math.floor(p.width*s)),height=Math.max(1,Math.floor(p.height*s));const frame=drawJob.raster_retained(JSON.stringify([0,0,p.width,p.height]),width,height);overview={width,height,rgba:frame.into_rgba()};transfer.push(overview.rgba.buffer);overviewReady=true;}
    self.postMessage({type:'painted',id:data.id,generation,rgba,overview,width,height,rasterMs:performance.now()-before,wasmMemoryBytes:wasmMemory.buffer.byteLength,rect:data.rect},transfer);return;
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
    if(drawJob)job.update_draw_job(drawJob);else drawJob=job.draw_job();
    const lowerMs=performance.now()-before;generation=id;overviewReady=false;
    const result=JSON.parse(job.inspection()),drawingInfo=JSON.parse(drawJob.drawing_info());
    drawPage=drawingInfo;
    self.postMessage({type:'result',id,generation,protocol:3,...result,drawingInfo,resources:Object.entries(drawingInfo.resources).map(([id,meta])=>({id,...meta,resident:true})),metrics:{initialization:initMs,resources:resourcesMs,compile:compileMs,lower:lowerMs,raster:0,total:compileMs+lowerMs}});
   }finally{job.free();}
  }catch(error){self.postMessage({type:data.type==='paint'?'paint-error':'error',id,generation:data.generation,error:detail(error)});}
 });
};
