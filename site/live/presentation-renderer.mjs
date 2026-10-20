import {SkiaRenderer} from './skia-renderer.mjs';
import {layerPlan} from './presentation-resources.mjs';
const pageFields=['nativeComposition','list','images','imageBytes','commandCache','prepared','geometryCache','contentCache','entries','owned','animatedNames','pictureBytes','generation','sceneRuns','staticTextures','staticTextureBytes','layerTextures','layerTextureBytes','pageBounds','opaqueRect','revealContents'];
const viewport=(list,w,h)=>{const scale=Math.min(w/list.width,h/list.height),dx=(w/scale-list.width)/2,dy=(h/scale-list.height)/2;return [-dx,-dy,list.width+dx,list.height+dy];};
const blank=()=>({nativeComposition:false,list:null,images:new Map(),imageBytes:new Map(),commandCache:new Map(),prepared:new WeakMap(),geometryCache:new WeakMap(),contentCache:new WeakMap(),entries:new Set(),owned:[],animatedNames:new Set(),pictureBytes:0,generation:0,sceneRuns:[],staticTextures:new Map(),staticTextureBytes:0,layerTextures:new Map(),layerTextureBytes:0,pageBounds:null,opaqueRect:null,revealContents:new WeakMap()});
const sizes=new WeakMap();
function retainedBytes(value){
 if(value===null||value===undefined)return 0;if(typeof value==='string')return value.length*2;if(typeof value!=='object')return 8;
 if(ArrayBuffer.isView(value))return value.byteLength;
 if(sizes.has(value))return sizes.get(value);
 const bytes=32+(Array.isArray(value)?value.reduce((n,v)=>n+retainedBytes(v),0):Object.entries(value).reduce((n,[k,v])=>n+k.length*2+retainedBytes(v),0));sizes.set(value,bytes);return bytes;
}
// Pages own their drawing resources until replaced or removed from the document.
// Entry snapshots and optional animation warming have independent readiness.
export class PresentationRenderer {
 constructor(ck,canvas,{preserveDrawingBuffer=false,software=false,preferNativeSoftware=true,onPresent=()=>{},onContextChange=()=>{}}={}){
  this.pages=new Map();this.current=null;
  this.skia=new SkiaRenderer(ck,canvas,{premultipliedRaster:true,clipToPage:true,preserveDrawingBuffer,software,preferNativeSoftware,onPresent,onContextChange:ready=>{
   if(!ready){this.save();for(const page of this.pages.values()){this.use(page.id);this.skia.clearStaticTextures();this.skia.clearLayerTextures();this.clearFrames(page);page.ready=false;page.plan=null;this.save();}}
   onContextChange(ready);
  }});
 }
 save(){const p=this.pages.get(this.current);if(p)for(const key of pageFields)p.state[key]=this.skia[key];}
 use(id){this.save();const p=this.pages.get(id);if(!p)throw Error('Page is not resident');Object.assign(this.skia,p.state);this.skia.staticTextureBytes=[...this.skia.staticTextures.values()].reduce((n,e)=>n+e.bytes,0);this.current=id;return p;}
 cost(p){const s=p.state;return p.baseBytes+(s.pictureBytes||0)+[...s.staticTextures.values()].reduce((n,e)=>n+e.bytes,0)+(s.layerTextureBytes||0)+[...p.frames.values()].reduce((n,f)=>n+(f.image?f.width*f.height*4:retainedBytes(f.states)),0);}
 residentBytes(){
  const shared=new Set();let total=0;
  for(const page of this.pages.values()){total+=this.cost(page);for(const entry of page.state.staticTextures.values()){if(shared.has(entry))total-=entry.bytes;else shared.add(entry);}}
  return total;
 }
 clearFrames(page){for(const frame of page.frames.values())frame.image?.delete();page.frames.clear();}
 async install(id,list,read,baseBytes=0){
  if(this.pages.has(id)){this.use(id);return true;}
  this.save();const previous=this.current,p={id,state:blank(),baseBytes,frames:new Map()};this.pages.set(id,p);this.use(id);
  try{await this.skia.install(list,read,id);this.save();return false;}catch(e){this.remove(id);if(this.pages.has(previous))this.use(previous);throw e;}
 }
 async update(base,id,update,read){
  const page=this.use(base);await this.skia.update(update,read,id);
  this.clearFrames(page);this.save();this.pages.delete(base);page.id=id;page.ready=false;page.plan=null;
  // Patch traffic is not retained allocation. The scene and resource maps replace
  // their previous values; never accumulate the sizes of historical patches.
  page.baseBytes=retainedBytes(this.skia.list)+[...this.skia.imageBytes.values()].reduce((n,b)=>n+b.byteLength,0);
  this.pages.set(id,page);this.current=id;this.save();
 }
 remove(id){const p=this.pages.get(id);if(!p)return;const previous=this.current;this.use(id);this.clearFrames(p);this.skia.clearStaticTextures();this.skia.clearLayerTextures();this.skia.clearCommands();for(const image of this.skia.images.values())this.skia.releaseImage(image);this.pages.delete(id);Object.assign(this.skia,blank());this.current=null;if(previous!==id&&this.pages.has(previous))this.use(previous);}
 retain(ids){for(const id of this.pages.keys())if(!ids.has(id))this.remove(id);this.skia.transitions.retain(new Set([...this.pages.values()].map(p=>p.state.list?.pageTransition?.name).filter(Boolean)));}
 // Compatibility with callers that previously invoked count-based eviction.
 evict(){return [];}
 frameKey(width,height,time,reduced){return `${width}/${height}/${time??'static'}/${!!reduced}`;}
 hasFrame(id,width,height,time,reduced){return !!this.pages.get(id)?.frames.has(this.frameKey(width,height,time,reduced));}
 async prepareEntry(id,width,height,time,reduced,sample,cancelled=()=>false,cpu=false,pause=()=>0,offscreen=false){
  const started=performance.now(),page=this.use(id),key=this.frameKey(width,height,time,reduced),s=this.skia;
  if(page.frames.has(key))return {complete:true,frameCacheHit:true,prepareMs:0};
  if(cancelled())return {cancelled:true};
  const duration=sample.duration||0;
  for(const [key,frame]of page.frames)if(frame.width!==width||frame.height!==height||frame.reduced!==reduced||frame.time!==0&&frame.time!==duration)page.frames.delete(key);
  // Background preparation needs a context and a reusable tile surface, not a
  // full-size presentation framebuffer. Sampling still uses width/height below.
  s.cacheViewportPixels=width*height;s.ensureSurface(offscreen?1:width,offscreen?1:height);s.preparationTileSize??=256;
  const sampled=performance.now(),states=time===null?{}:sample(time,reduced),sampleMs=performance.now()-sampled;
  try{
   if(page.planSize?.[0]!==width||page.planSize?.[1]!==height)page.plan=null;
   page.planSize=[width,height];
   const plan=page.plan||await layerPlan(s,width,height,sample.transform||sample,cancelled);
   if(plan.cancelled||cancelled())return {cancelled:true};
   page.plan=plan;
   // Plan peak projected density before the first frame. Growing/rotating
   // content must not rebuild a slightly larger texture on every frame.
   const work=function*(){for(const {p,density}of plan.layers)yield* s.layerTasks(p,density);yield* s.sceneTasks(viewport(s.list,width,height),width,height,states);};
   const prepared=await s.prepareTasks(work(),cancelled,pause);
   if(prepared.cancelled)return prepared;
   // A prepared entry is a composition recipe over retained content tiles.
   // Storing another full-page image duplicated those same pixels per endpoint.
   page.frames.set(key,{states,width,height,time,reduced});
   page.ready=plan.unrecorded===0&&plan.layers.every(({p,density})=>s.layerTextures.get(p.textureKey??p.uid)?.complete&&s.layerTextures.get(p.textureKey??p.uid)?.density>=density);page.readySize=[width,height];
   return {...prepared,backend:s.backend,sampleMs,prepareMs:performance.now()-started};
  }finally{this.save();}
 }
 async prepareTransition(id,width,height,reduced,cpu=false,offscreen=false){
  const page=this.use(id),s=this.skia;if(reduced||!s.list.pageTransition)return {};
  const start=performance.now();s.cacheViewportPixels=width*height;s.ensureSurface(offscreen?1:width,offscreen?1:height);
  // Compile with a one-pixel texture, not a retained full-page snapshot.
  const surface=s.surface.makeSurface({width:1,height:1,colorType:s.ck.ColorType.RGBA_8888,alphaType:s.ck.AlphaType.Premul,colorSpace:s.ck.ColorSpace.SRGB});
  if(!surface)return {};
  let image;try{surface.getCanvas().clear(s.ck.TRANSPARENT);image=surface.makeImageSnapshot();s.transitions.warm(s.list.pageTransition,image);await s.completeResources();}finally{image?.delete();surface.dispose();}
  return {transitionPrepareMs:performance.now()-start};
 }
 async prepare(id,width,height,sample,priority=false,view=null,cancelled=()=>false,cpu=false,pause=()=>0,offscreen=false){
  const started=performance.now(),page=this.use(id),s=this.skia;
  if(cancelled())return {cancelled:true};
  if(page.ready&&page.readySize?.[0]===width&&page.readySize?.[1]===height)return {complete:true,prepareMs:0};
  s.cacheViewportPixels=width*height;s.ensureSurface(offscreen?1:width,offscreen?1:height);
  const planned=performance.now();
  if(page.planSize?.[0]!==width||page.planSize?.[1]!==height)page.plan=null;
  page.planSize=[width,height];
  const plan=page.plan||await layerPlan(s,width,height,sample,cancelled);
  if(plan.cancelled||cancelled())return {cancelled:true,prepareMs:performance.now()-started};
  page.plan=plan;const planMs=performance.now()-planned;
  let tiles=0,drawMs=0,gpuWaitMs=0;
  for(const {p,density}of plan.layers){
   if(cancelled())return {cancelled:true,prepareMs:performance.now()-started};
   const result=await s.prepareTasks(s.layerTasks(p,density),cancelled,pause);this.save();
   tiles+=result.tiles;drawMs+=result.drawMs;gpuWaitMs+=result.gpuWaitMs;
   if(result.cancelled)return {...result,prepareMs:performance.now()-started};
  }
  page.ready=plan.unrecorded===0&&plan.layers.every(({p,density})=>s.layerTextures.get(p.textureKey??p.uid)?.complete&&s.layerTextures.get(p.textureKey??p.uid)?.density>=density);page.readySize=[width,height];
  await s.completeResources();this.save();
  return {backend:s.backend,complete:page.ready,tiles,drawMs,gpuWaitMs,planMs,transformProbes:plan.probes,layerCacheBuilds:s.layerBuilds,layerTextureBytes:s.layerTextureBytes,prepareMs:performance.now()-started};
 }
 show(id,width,height,time,reduced,present=true){
  const page=this.use(id),frame=page.frames.get(this.frameKey(width,height,time,reduced));if(!frame)return null;
  const result=this.skia.render(viewport(this.skia.list,width,height),width,height,frame.states,id,false,true,true,false,null,present);this.save();return {...result,frameCacheHit:true};
 }
 render(id,width,height,states,running,view=null,cached=true,present=true){this.use(id);const s=this.skia,result=s.render(view||viewport(s.list,width,height),width,height,states,id,running,cached,cached,false,null,present);this.save();return result;}
 dispose(){for(const id of [...this.pages.keys()])this.remove(id);this.skia.dispose();}
}
