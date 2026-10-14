import {SkiaRenderer} from './skia-renderer.mjs';
import {layerPlan} from './presentation-resources.mjs';
const pageFields=['list','images','imageBytes','commandCache','prepared','owned','animatedNames','pictureBytes','generation','sceneRuns','staticTextures','staticTextureBytes','layerTextures','layerTextureBytes'];
const viewport=(list,w,h)=>{const scale=Math.min(w/list.width,h/list.height),dx=(w/scale-list.width)/2,dy=(h/scale-list.height)/2;return [-dx,-dy,list.width+dx,list.height+dy];};
const blank=()=>({list:null,images:new Map(),imageBytes:new Map(),commandCache:new Map(),prepared:new WeakMap(),owned:[],animatedNames:new Set(),pictureBytes:0,generation:0,sceneRuns:[],staticTextures:new Map(),staticTextureBytes:0,layerTextures:new Map(),layerTextureBytes:0});
// One GL context with retained page-local commands. Switching does not install
// or delete the previous page; all pages share one cache admission budget.
export class PresentationRenderer {
 constructor(ck,canvas,{budget=256*1048576,preserveDrawingBuffer=false,software=false,onPresent=()=>{},onContextChange=()=>{}}={}){this.budget=budget;this.pages=new Map();this.current=null;this.skia=new SkiaRenderer(ck,canvas,{cacheMB:budget/1048576,preserveDrawingBuffer,software,onPresent,onContextChange:ready=>{if(!ready){this.save();for(const page of this.pages.values()){this.use(page.id);this.skia.clearStaticTextures();this.skia.clearLayerTextures();page.ready=false;this.save();}}onContextChange(ready);}});}
 save(){const p=this.pages.get(this.current);if(p)for(const key of pageFields)p.state[key]=this.skia[key];}
 use(id){this.save();const p=this.pages.get(id);if(!p)throw Error('Page is not resident');Object.assign(this.skia,p.state);this.current=id;p.used=performance.now();this.skia.budget=Math.max(0,this.budget-[...this.pages.values()].filter(p=>p.id!==id).reduce((n,p)=>n+this.cost(p),0));this.skia.updateCacheLimit();return p;}
 cost(p){const s=p.state;return p.baseBytes+(s.pictureBytes||0)+(s.staticTextureBytes||0)+(s.layerTextureBytes||0);}
 async install(id,list,read,baseBytes=0){if(this.pages.has(id)){this.use(id);return true;}this.save();const previous=this.current,p={id,state:blank(),used:performance.now(),baseBytes};this.pages.set(id,p);this.use(id);try{await this.skia.install(list,read,id,this.skia.budget/1048576);this.save();return false;}catch(e){this.remove(id);if(this.pages.has(previous))this.use(previous);throw e;}}
 remove(id){const p=this.pages.get(id);if(!p)return;const previous=this.current;this.use(id);this.skia.clearStaticTextures();this.skia.clearLayerTextures();this.skia.clearCommands();for(const image of this.skia.images.values())image.delete();this.pages.delete(id);Object.assign(this.skia,blank());this.current=null;if(previous!==id&&this.pages.has(previous))this.use(previous);}
 evict(protect){this.save();const removed=[];for(const p of [...this.pages.values()].sort((a,b)=>a.used-b.used)){const bytes=[...this.pages.values()].reduce((n,p)=>n+this.cost(p),0);if(this.pages.size<=3&&bytes<=this.budget)break;if(protect.has(p.id))continue;this.remove(p.id);removed.push(p.id);}return removed;}
 async prepare(id,width,height,sample,priority=false,view=null){const started=performance.now();this.use(id);const s=this.skia,ck=s.ck,area=view||viewport(s.list,width,height);
  if(priority){for(const page of [...this.pages.values()].filter(p=>p.id!==id).sort((a,b)=>a.used-b.used)){if(s.budget>=this.budget*.75)break;this.remove(page.id);this.use(id);}s.recordPictures();}
  if(!s.surface||s.canvas.width!==width||s.canvas.height!==height){s.surface?.dispose();s.canvas.width=width;s.canvas.height=height;s.surface=s.gr?ck.MakeOnScreenGLSurface(s.gr,width,height,ck.ColorSpace.SRGB):ck.MakeSWCanvasSurface(s.canvas);}
  if(!s.surface)throw Error('Presentation surface allocation failed');const original=s.surface,scratch=original.makeSurface({width,height,colorType:ck.ColorType.RGBA_8888,alphaType:ck.AlphaType.Premul,colorSpace:ck.ColorSpace.SRGB});if(!scratch)throw Error('Presentation resource allocation failed');
  // Build the content once in its layout pose, independently of the timeline.
  // No animation is played, no clock advances, and no pixels leave the GPU.
  let result;
  try{s.surface=scratch;result=s.render(area,width,height,{},id,true,true,true,true);
   const planned=performance.now(),plan=s.gr?layerPlan(s,width,height,sample):{layers:[],probes:0,unrecorded:0};result.planMs=performance.now()-planned;result.transformProbes=plan.probes;
   // A foreground page owns admission priority. Neighbors may retain only CPU
   // artifacts when their GPU textures would crowd out the active animation.
   const plannedBytes=plan.layers.reduce((sum,{p,density})=>{const d=2**(Math.ceil(Math.log2(density*1.1*1.05)*2)/2),w=Math.ceil(p.bounds[2]*d)-Math.floor(p.bounds[0]*d)+4,h=Math.ceil(p.bounds[3]*d)-Math.floor(p.bounds[1]*d)+4;return sum+(w+4*Math.ceil(w/2048))*(h+4*Math.ceil(h/2048))*4;},0);
   if(priority){this.save();for(const page of [...this.pages.values()].filter(p=>p.id!==id).sort((a,b)=>a.used-b.used)){if((s.staticTextureBytes+plannedBytes)<=s.budget*.75)break;this.remove(page.id);this.use(id);}}

   for(const {p,density}of plan.layers)s.buildLayer(p,density*1.1);
   result.layerCacheBuilds=s.layerBuilds;result.layerCacheBudgetMisses=s.layerBudgetMisses;result.layerTextureBytes=s.layerTextureBytes;result.complete=plan.unrecorded===0&&plan.layers.every(({p,density})=>s.layerTextures.get(p.uid)?.density>=density);this.pages.get(id).ready=result.complete;this.pages.get(id).readySize=[width,height];
   await s.completeResources();result.prepareMs=performance.now()-started;
  }finally{s.surface=original;scratch.dispose();this.save();}return result;}
 render(id,width,height,states,running,view=null){this.use(id);const s=this.skia,result=s.render(view||viewport(s.list,width,height),width,height,states,id,running);this.save();return result;}
 dispose(){for(const id of [...this.pages.keys()])this.remove(id);this.skia.dispose();}
}
