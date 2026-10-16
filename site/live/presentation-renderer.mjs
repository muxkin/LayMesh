import {SkiaRenderer} from './skia-renderer.mjs';
import {layerPlan} from './presentation-resources.mjs';
const pageFields=['list','images','imageBytes','commandCache','prepared','geometryCache','contentCache','entries','owned','animatedNames','pictureBytes','generation','sceneRuns','staticTextures','staticTextureBytes','layerTextures','layerTextureBytes','pageBounds','revealContents'];
const viewport=(list,w,h)=>{const scale=Math.min(w/list.width,h/list.height),dx=(w/scale-list.width)/2,dy=(h/scale-list.height)/2;return [-dx,-dy,list.width+dx,list.height+dy];};
const blank=()=>({list:null,images:new Map(),imageBytes:new Map(),commandCache:new Map(),prepared:new WeakMap(),geometryCache:new WeakMap(),contentCache:new WeakMap(),entries:new Set(),owned:[],animatedNames:new Set(),pictureBytes:0,generation:0,sceneRuns:[],staticTextures:new Map(),staticTextureBytes:0,layerTextures:new Map(),layerTextureBytes:0,pageBounds:null,revealContents:new WeakMap()});
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
 constructor(ck,canvas,{preserveDrawingBuffer=false,software=false,onPresent=()=>{},onContextChange=()=>{}}={}){
  this.pages=new Map();this.current=null;
  this.skia=new SkiaRenderer(ck,canvas,{clipToPage:true,preserveDrawingBuffer,software,onPresent,onContextChange:ready=>{
   if(!ready){this.save();for(const page of this.pages.values()){this.use(page.id);this.skia.clearStaticTextures();this.skia.clearLayerTextures();this.clearFrames(page);page.ready=false;page.plan=null;this.save();}}
   onContextChange(ready);
  }});
 }
 save(){const p=this.pages.get(this.current);if(p)for(const key of pageFields)p.state[key]=this.skia[key];}
 use(id){this.save();const p=this.pages.get(id);if(!p)throw Error('Page is not resident');Object.assign(this.skia,p.state);this.current=id;return p;}
 cost(p){const s=p.state;return p.baseBytes+(s.pictureBytes||0)+(s.staticTextureBytes||0)+(s.layerTextureBytes||0)+[...p.frames.values()].reduce((n,f)=>n+f.width*f.height*4,0);}
 clearFrames(page){for(const frame of page.frames.values())frame.image.delete();page.frames.clear();}
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
 remove(id){const p=this.pages.get(id);if(!p)return;const previous=this.current;this.use(id);this.clearFrames(p);this.skia.clearStaticTextures();this.skia.clearLayerTextures();this.skia.clearCommands();for(const image of this.skia.images.values())image.delete();this.pages.delete(id);Object.assign(this.skia,blank());this.current=null;if(previous!==id&&this.pages.has(previous))this.use(previous);}
 retain(ids){for(const id of this.pages.keys())if(!ids.has(id))this.remove(id);this.skia.transitions.retain(new Set([...this.pages.values()].map(p=>p.state.list?.pageTransition?.name).filter(Boolean)));}
 // Compatibility with callers that previously invoked count-based eviction.
 evict(){return [];}
 frameKey(width,height,time,reduced){return `${width}/${height}/${time??'static'}/${!!reduced}`;}
 hasFrame(id,width,height,time,reduced){return !!this.pages.get(id)?.frames.has(this.frameKey(width,height,time,reduced));}
 async prepareEntry(id,width,height,time,reduced,sample,cancelled=()=>false,cpu=false){
  const started=performance.now(),page=this.use(id),key=this.frameKey(width,height,time,reduced),s=this.skia;
  if(page.frames.has(key))return {complete:true,frameCacheHit:true,prepareMs:0};
  if(cancelled())return {cancelled:true};
  const duration=sample.duration||0;
  for(const [key,frame]of page.frames)if(frame.width!==width||frame.height!==height||frame.reduced!==reduced||frame.time!==0&&frame.time!==duration){frame.image.delete();page.frames.delete(key);}
  s.ensureSurface(width,height);
  const original=s.surface,scratch=cpu?s.ck.MakeSurface(width,height):original.makeSurface({width,height,colorType:s.ck.ColorType.RGBA_8888,alphaType:s.ck.AlphaType.Premul,colorSpace:s.ck.ColorSpace.SRGB});
  if(!scratch)throw Error('Presentation entry surface allocation failed');
  const notify=s.onPresent;let result;
  try{
   s.surface=scratch;s.onPresent=()=>{};
   const sampled=performance.now(),states=time===null?{}:sample(time,reduced),sampleMs=performance.now()-sampled;
   result=s.render(viewport(s.list,width,height),width,height,states,id,true,true,true,true);
   page.frames.set(key,{image:scratch.makeImageSnapshot(),width,height,bounds:s.pageBounds,time,reduced});
   result.sampleMs=sampleMs;
  }finally{s.surface=original;s.onPresent=notify;scratch.dispose();this.save();}
  await s.completeResources();
  return {...result,complete:true,prepareMs:performance.now()-started};
 }
 async prepareTransition(id,width,height,reduced,cpu=false){
  const page=this.use(id),s=this.skia,frame=page.frames.get(this.frameKey(width,height,0,reduced));
  if(!frame||reduced)return {};
  const start=performance.now();
  // Shader parsing can run offscreen; GPU pipeline warming belongs to display.
  const original=s.surface,scratch=cpu?s.ck.MakeSurface(1,1):null;
  try{if(cpu)s.surface=scratch;s.transitions.warm(s.list.pageTransition,frame.image);}finally{if(cpu){s.surface=original;scratch?.dispose();s.transitions.warmed.delete(s.list.pageTransition?.name);}}
  if(!cpu)await s.completeResources();
  return {transitionPrepareMs:performance.now()-start};
 }
 async prepare(id,width,height,sample,priority=false,view=null,cancelled=()=>false,cpu=false){
  const started=performance.now(),page=this.use(id),s=this.skia;
  if(cancelled())return {cancelled:true};
  if(page.ready&&page.readySize?.[0]===width&&page.readySize?.[1]===height)return {complete:true,prepareMs:0};
  if(!s.gr){page.ready=true;page.readySize=[width,height];this.save();return {backend:s.backend,complete:true,cpuCommandsOnly:true,planMs:0,transformProbes:0,prepareMs:performance.now()-started};}
  s.ensureSurface(width,height);
  const planned=performance.now();
  if(page.planSize?.[0]!==width||page.planSize?.[1]!==height)page.plan=null;
  page.planSize=[width,height];
  const plan=page.plan||await layerPlan(s,width,height,sample,cancelled);
  if(plan.cancelled||cancelled())return {cancelled:true,prepareMs:performance.now()-started};
  page.plan=plan;const planMs=performance.now()-planned;
  for(const {p,density}of plan.layers){
   if(cancelled())return {cancelled:true,prepareMs:performance.now()-started};
   const original=s.surface,scratch=cpu?s.ck.MakeSurface(1,1):null;
   try{if(cpu)s.surface=scratch;s.buildLayer(p,density*1.1);}finally{if(cpu){s.surface=original;scratch?.dispose();}}this.save();
   // No scratch surface is installed across this scheduling boundary.
   if(!priority)await new Promise(resolve=>setTimeout(resolve,0));
  }
  page.ready=plan.unrecorded===0&&plan.layers.every(({p,density})=>s.layerTextures.get(p.uid)?.density>=density);page.readySize=[width,height];
  await s.completeResources();this.save();
  return {backend:s.backend,complete:page.ready,planMs,transformProbes:plan.probes,layerCacheBuilds:s.layerBuilds,layerTextureBytes:s.layerTextureBytes,prepareMs:performance.now()-started};
 }
 show(id,width,height,time,reduced){
  const page=this.use(id),frame=page.frames.get(this.frameKey(width,height,time,reduced));if(!frame)return null;
  const s=this.skia,start=performance.now();s.ensureSurface(width,height);s.pageBounds=frame.bounds;
  const c=s.surface.getCanvas();c.clear(s.ck.TRANSPARENT);c.drawImage(frame.image,0,0);s.surface.flush();s.frameCount++;s.onPresent();
  return {backend:s.backend,device:s.device,rasterMs:performance.now()-start,frameCacheHit:true,frameCount:s.frameCount};
 }
 render(id,width,height,states,running,view=null){this.use(id);const s=this.skia,result=s.render(view||viewport(s.list,width,height),width,height,states,id,running);this.save();return result;}
 dispose(){for(const id of [...this.pages.keys()])this.remove(id);this.skia.dispose();}
}
