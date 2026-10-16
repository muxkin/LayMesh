// Shared by the website and the offline VS Code worker. Layout and animation
// sampling remain in Rust; these retained commands never reshape text.
import CanvasKitInit from './skia/canvaskit.mjs';
import {applyDisplayUpdate} from './display-update.mjs';
import {PageTransitions,decodeSkiaImage} from './presentation-transitions.mjs';

export async function initializeSkia(wasmURL) {
 // CanvasKit's pinned loader accepts a URL. A blob created from transferred
 // bytes stays in this worker and never enters VS Code's resource proxy.
 const blob=wasmURL instanceof ArrayBuffer||ArrayBuffer.isView(wasmURL)?URL.createObjectURL(new Blob([wasmURL],{type:'application/wasm'})):null;
 try{
  let url=blob||wasmURL;
  if(blob?.startsWith('blob:null/')){
   // Opaque file:// workers in WebKit cannot fetch or FileReader-read even
   // their own blobs. Build a local data URL from the bytes directly. Keep
   // blob URLs in ordinary hosts (including VS Code's stricter data: CSP).
   const bytes=wasmURL instanceof ArrayBuffer?new Uint8Array(wasmURL):new Uint8Array(wasmURL.buffer,wasmURL.byteOffset,wasmURL.byteLength),chunks=[];
   for(let i=0;i<bytes.length;i+=32768)chunks.push(String.fromCharCode(...bytes.subarray(i,i+32768)));
   url='data:application/wasm;base64,'+btoa(chunks.join(''));
  }
  return await CanvasKitInit({locateFile:()=>url});
 }
 finally{if(blob)URL.revokeObjectURL(blob);}
}
const matrix=([a,b,c,d,e,f])=>[a,c,e,b,d,f,0,0,1];
const point=p=>Array.isArray(p)?p:[p.x,p.y];
const rect=r=>Array.isArray(r)?r:[r.x0,r.y0,r.x1,r.y1];
const imagePattern=n=>n.kind.kind==='image'||(n.kind.kind==='group'&&n.kind.children.length===1&&imagePattern(n.kind.children[0]));
const union=(a,b)=>a?[Math.min(a[0],b[0]),Math.min(a[1],b[1]),Math.max(a[2],b[2]),Math.max(a[3],b[3])]:Array.from(b);
function transformed(b,m){let out;for(const x of [b[0],b[2]])for(const y of [b[1],b[3]]){const w=m[6]*x+m[7]*y+m[8],px=(m[0]*x+m[1]*y+m[2])/w,py=(m[3]*x+m[4]*y+m[5])/w;out=union(out,[px,py,px,py]);}return out;}
// An upper bound on projected pixels per local unit across the entire layer.
// For perspective, each Jacobian numerator is linear in x or y, while its
// denominator is w². Bound both over the rectangle, not just its center.
export function projectedDensity(m,b){
 const ws=[m[6]*b[0]+m[7]*b[1]+m[8],m[6]*b[2]+m[7]*b[1]+m[8],m[6]*b[0]+m[7]*b[3]+m[8],m[6]*b[2]+m[7]*b[3]+m[8]],w=Math.min(...ws);
 if(!Number.isFinite(w)||w<=1e-5)return Infinity;
 const bound=(a,v,c,lo,hi)=>Math.max(Math.abs(a*lo+c),Math.abs(a*hi+c))/(v*v);
 const a=bound(m[0]*m[7]-m[6]*m[1],w,m[0]*m[8]-m[6]*m[2],b[1],b[3]);
 const b0=bound(m[1]*m[6]-m[7]*m[0],w,m[1]*m[8]-m[7]*m[2],b[0],b[2]);
 const c=bound(m[3]*m[7]-m[6]*m[4],w,m[3]*m[8]-m[6]*m[5],b[1],b[3]);
 const d=bound(m[4]*m[6]-m[7]*m[3],w,m[4]*m[8]-m[7]*m[5],b[0],b[2]);
 const sum=a*a+b0*b0+c*c+d*d,det=a*d-b0*c;
 return Math.sqrt((sum+Math.sqrt(Math.max(0,sum*sum-4*det*det)))/2);
}

export class SkiaRenderer {
 constructor(ck,canvas,{software=false,rasterOnly=false,clipToPage=false,preserveDrawingBuffer=false,onPresent=()=>{},onContextChange=()=>{}}={}) {
  this.rasterOnly=rasterOnly;this.clipToPage=clipToPage;this.pageBounds=null;
  this.ck=ck;this.canvas=canvas;this.onPresent=onPresent;this.transitions=new PageTransitions(this);this.images=new Map();this.imageBytes=new Map();this.commandCache=new Map();this.prepared=new WeakMap();this.geometryCache=new WeakMap();this.contentCache=new WeakMap();this.entries=new Set();this.owned=[];this.animatedNames=new Set();this.pictureBytes=0;this.generation=0;this.frameCount=0;
  this.nextCommandId=0;this.staticTextures=new Map();this.staticTextureBytes=0;this.sceneRuns=[];this.revealContents=new WeakMap();
  this.layerTextures=new Map();this.layerTextureBytes=0;
  this.context=software?0:ck.GetWebGLContext(canvas,{majorVersion:2,alpha:1,antialias:1,preserveDrawingBuffer:Number(preserveDrawingBuffer)});
  this.gr=this.context?ck.MakeWebGLContext(this.context):null;
  this.backend=this.gr?'Skia WebGL2':'Skia CPU';this.updateCacheLimit();this.gl=this.gr?canvas.getContext('webgl2'):null;this.timer=this.gl?.getExtension('EXT_disjoint_timer_query_webgl2');this.queries=[];this.fences=[];const debug=this.gl?.getExtension('WEBGL_debug_renderer_info');this.device=debug?this.gl.getParameter(debug.UNMASKED_RENDERER_WEBGL):'';if(/SwiftShader|llvmpipe|softpipe/i.test(this.device))this.backend+=' (software driver)';
  this.onLost=e=>{e.preventDefault();this.lost=true;this.transitions.cancel();this.transitions.warmed.clear();this.clearStaticTextures();this.clearLayerTextures();this.queries=[];this.fences=[];this.gpuMs=undefined;onContextChange(false);};canvas.addEventListener?.('webglcontextlost',this.onLost);
  this.onRestored=()=>{this.surface?.dispose();this.surface=null;this.gr?.delete();this.gr=ck.MakeWebGLContext(this.context);this.updateCacheLimit();this.timer=this.gl?.getExtension('EXT_disjoint_timer_query_webgl2');this.backend=this.gr?'Skia WebGL2'+(/SwiftShader|llvmpipe|softpipe/i.test(this.device)?' (software driver)':''):'Skia CPU';this.lost=false;onContextChange(true);};canvas.addEventListener?.('webglcontextrestored',this.onRestored);
 }
 own(value){if(value){this.owned.push(value);this.owner?.objects.push(value);}return value;}
 path(elements,evenOdd=false){
  if(!this.transient){const old=this.geometryCache.get(elements)?.get(evenOdd);if(old&&!old.dead){this.owner?.dependencies.add(old);return old.path;}}
  const parent=this.owner,entry=!this.transient?{objects:[],dependencies:new Set()}:null;
  if(entry){this.owner=entry;this.entries.add(entry);parent?.dependencies.add(entry);}
  const ck=this.ck,p=new ck.PathBuilder();
  for(const el of elements){if(el==='ClosePath'){p.close();continue;}if(el.MoveTo)p.moveTo(...point(el.MoveTo));else if(el.LineTo)p.lineTo(...point(el.LineTo));else if(el.QuadTo)p.quadTo(...el.QuadTo.flatMap(point));else if(el.CurveTo)p.cubicTo(...el.CurveTo.flatMap(point));}
  p.setFillType(evenOdd?ck.FillType.EvenOdd:ck.FillType.Winding);const path=this.own(p.detach());p.delete();
  if(entry){entry.path=path;let values=this.geometryCache.get(elements);if(!values)this.geometryCache.set(elements,values=new Map());values.set(evenOdd,entry);this.owner=parent;}
  return path;
 }
 color(rgba){return new Float32Array(rgba);}
 paint(spec){
  if(!spec||spec.kind==='none')return null;
  const ck=this.ck,p=this.own(new ck.Paint());p.setAntiAlias(true);
  if(spec.kind==='solid')p.setColor(this.color(spec.rgba));
  else {
   let shader;const tile=spec.spread==='repeat'?ck.TileMode.Repeat:spec.spread==='reflect'?ck.TileMode.Mirror:ck.TileMode.Clamp;
   if(spec.kind==='linear'||spec.kind==='radial'){
    const colors=spec.stops.map(s=>this.color(s.rgba)),stops=spec.stops.map(s=>s.offset),m=matrix(spec.transform);
    shader=spec.kind==='linear'?ck.Shader.MakeLinearGradient(spec.start,spec.end,colors,stops,tile,m):ck.Shader.MakeTwoPointConicalGradient(spec.focal,0,spec.center,spec.radius,colors,stops,tile,m);
   }else if(spec.kind==='pattern'){
    const recorder=new ck.PictureRecorder(),bounds=[0,0,spec.width,spec.height],c=recorder.beginRecording(bounds);
    spec.nodes.forEach((n,i)=>this.drawNode(c,n,`pattern/${i}`,{}));
    const picture=this.own(recorder.finishRecordingAsPicture());recorder.delete();
    shader=picture.makeShader(ck.TileMode.Repeat,ck.TileMode.Repeat,ck.FilterMode.Linear,matrix(spec.transform),bounds);
   }else throw Error(`Unsupported Skia paint ${spec.kind}`);
   if(!shader)throw Error('Skia shader creation failed');p.setShader(shader);shader.delete();
  }return p;
 }
 prepare(node){
  const frameCache=this.transient?this.transientPrepared:this.prepared;
  const bound=this.prepared.get(node)||(this.transient?frameCache.get(node):null);
  if(bound&&!bound.dead){this.owner?.dependencies.add(bound);return bound;}
  const contentKey=JSON.stringify([node.effects,node.hint?.animationRevealWrapper]);
  const content=!this.transient&&this.contentCache.get(node.kind)?.get(contentKey);
  if(content&&!content.dead){const value={...content,uid:++this.nextCommandId,matrix:matrix(node.transform),objects:[],dependencies:new Set([content]),bound:true};this.entries.add(value);frameCache.set(node,value);this.owner?.dependencies.add(value);return value;}
  const key=this.transient?null:node.kind.kind==='group'?JSON.stringify({...node,kind:{...node.kind,children:node.kind.children.map(n=>this.prepare(n).uid)}}):JSON.stringify(node);
  const cached=key&&this.commandCache.get(key);
  if(cached&&!cached.dead){this.owner?.dependencies.add(cached);frameCache.set(node,cached);return cached;}
  const parent=this.owner,ck=this.ck,k=node.kind,v={uid:++this.nextCommandId,cost:1,matrix:matrix(node.transform),bounds:[0,0,0,0],effects:[],objects:[],dependencies:new Set()};
  if(!this.transient)this.entries.add(v);parent?.dependencies.add(v);this.owner=v;frameCache.set(node,v);if(key)this.commandCache.set(key,v);
  if(k.kind==='path'){
   v.path=this.path(k.path,k.even_odd);v.imageFill=k.fill.kind==='pattern'&&k.fill.nodes.length===1&&imagePattern(k.fill.nodes[0]);v.fill=v.imageFill?null:this.paint(k.fill);v.bounds=Array.from(v.path.getBounds());
   if(k.stroke){const s=k.stroke,p=v.stroke=this.paint({kind:'solid',rgba:s.rgba});p.setStyle(ck.PaintStyle.Stroke);p.setStrokeWidth(s.width);p.setStrokeCap(s.cap==='round'?ck.StrokeCap.Round:s.cap==='square'?ck.StrokeCap.Square:ck.StrokeCap.Butt);p.setStrokeJoin(s.join==='round'?ck.StrokeJoin.Round:s.join==='bevel'?ck.StrokeJoin.Bevel:ck.StrokeJoin.Miter);p.setStrokeMiter(s.miter);if(s.dash.length){const effect=ck.PathEffect.MakeDash(s.dash,s.offset);p.setPathEffect(effect);effect?.delete();}const pad=s.width*s.miter;v.bounds=[v.bounds[0]-pad,v.bounds[1]-pad,v.bounds[2]+pad,v.bounds[3]+pad];}
  }else if(k.kind==='glyphs'){
   v.paths=k.outlines.map(p=>this.path(p));v.fill=this.paint({kind:'solid',rgba:k.color});v.bounds=v.paths.reduce((b,p)=>union(b,p.getBounds()),null)||v.bounds;
  }else if(k.kind==='image')v.bounds=rect(k.rect);
  else if(k.kind==='group'){
   v.dynamicChildren=k.children.some(n=>this.animatedNames.has(n.name)||this.prepare(n).dynamicChildren);
   v.bounds=k.children.reduce((b,n)=>{const p=this.prepare(n);return union(b,transformed(p.bounds,p.matrix));},null)||v.bounds;
   if(k.clip)v.clip=this.path(k.clip.path,k.clip.even_odd);
  }else throw Error(`Unsupported Skia command ${k.kind}`);
  for(const e of node.effects){
   let spread=null;if(e.spread)spread=(e.spread>0?ck.ImageFilter.MakeDilate:ck.ImageFilter.MakeErode)(Math.abs(e.spread),Math.abs(e.spread),null);
   const shadow=ck.ImageFilter.MakeDropShadowOnly(e.offset[0],e.offset[1],e.blur,e.blur,this.color(e.rgba),spread);spread?.delete();
   const paint=this.paint({kind:'solid',rgba:[1,1,1,1]});
   if(e.inner){
    // A * (1 - shifted/blurred A), then tint; never an opaque bounding box.
    const erosion=e.spread?(e.spread>0?ck.ImageFilter.MakeErode:ck.ImageFilter.MakeDilate)(Math.abs(e.spread),Math.abs(e.spread),null):null;
    const opaqueShadow=ck.ImageFilter.MakeDropShadowOnly(e.offset[0],e.offset[1],e.blur,e.blur,this.color([0,0,0,1]),erosion);erosion?.delete();
    const inside=ck.ImageFilter.MakeBlend(ck.BlendMode.SrcOut,opaqueShadow,null),tint=ck.ColorFilter.MakeBlend(this.color(e.rgba),ck.BlendMode.SrcIn),filter=ck.ImageFilter.MakeColorFilter(tint,inside);
    paint.setImageFilter(filter);filter.delete();tint.delete();inside.delete();opaqueShadow.delete();
   }else paint.setImageFilter(shadow);
   shadow.delete();v.effects.push({paint,inner:e.inner,source:e.sourcePath?this.path(e.sourcePath):null,knockout:e.knockout,fill:this.paint({kind:'solid',rgba:[1,1,1,1]})});
   if(!e.inner){const pad=4*e.blur+Math.abs(e.spread);v.bounds=union(v.bounds,[v.bounds[0]+e.offset[0]-pad,v.bounds[1]+e.offset[1]-pad,v.bounds[2]+e.offset[0]+pad,v.bounds[3]+e.offset[1]+pad]);}
  }
  v.cost+=k.kind==='group'?k.children.reduce((n,child)=>n+this.prepare(child).cost,0):k.kind==='path'?k.path.length:k.kind==='glyphs'?k.outlines.reduce((n,path)=>n+path.length,0):1;
  this.owner=parent;if(!this.transient){let values=this.contentCache.get(node.kind);if(!values)this.contentCache.set(node.kind,values=new Map());values.set(contentKey,v);}return v;
 }
 bind(n){const p=this.prepare(n),k=n.kind;if(p.bound)return p;p.bound=true;if(k.kind==='group'){k.children.forEach(n=>p.dependencies.add(this.bind(n)));if(k.mask)p.dependencies.add(this.bind(k.mask.node));}if(k.kind==='path'&&k.fill.kind==='pattern')k.fill.nodes.forEach(n=>p.dependencies.add(this.bind(n)));return p;}
 async update(update,readResource,generation,navigation){
  return this.install(applyDisplayUpdate(this.list,update),readResource,generation,navigation,true);
 }
 async install(list,readResource,generation,navigation,incremental=false){
  if(navigation!==undefined)this.transitions.navigate(list,navigation);
  if(list.schemaVersion!==1)throw Error('Unsupported Skia display list version');
  const incoming=new Map(),bytesById=new Map(),old={list:this.list,images:this.images,imageBytes:this.imageBytes,animatedNames:this.animatedNames,generation:this.generation,revealContents:this.revealContents};
  const ownedBefore=this.owned.length,cacheBefore=new Map(this.commandCache),entriesBefore=new Set(this.entries);let resourceChanged=false;
  try{
   for(const [id,meta]of Object.entries(list.resources))if(meta.mime.startsWith('image/')){
    if(incremental&&list.resources===old.list.resources&&this.images.has(id)){incoming.set(id,this.images.get(id));bytesById.set(id,this.imageBytes.get(id));continue;}
    const bytes=await readResource(id,meta),previous=this.imageBytes.get(id);bytesById.set(id,bytes);
    if(previous&&previous.length===bytes.length&&previous.every((v,i)=>v===bytes[i])){incoming.set(id,this.images.get(id));continue;}
    if(previous)resourceChanged=true;
    const image=decodeSkiaImage(this.ck,bytes,meta.mime);
    if(!image)throw Error(`Skia image decode failed: ${id}`);incoming.set(id,image);
   }
   // Commands are keyed by content; unchanged paths, glyphs and pictures survive
   // an edit. Resource replacement invalidates pictures referencing that image.
   if(resourceChanged){this.commandCache=new Map();this.prepared=new WeakMap();this.contentCache=new WeakMap();}
   this.images=incoming;this.imageBytes=bytesById;this.list=list;
   const names=new Set(Object.values(list.animation||{}).flatMap(t=>t.steps.flatMap(s=>s.clips.map(c=>c.target))));
   if([...names].sort().join('\0')!==[...this.animatedNames].sort().join('\0')){this.commandCache=new Map();this.prepared=new WeakMap();this.contentCache=new WeakMap();}
   this.animatedNames=names;this.updateCacheLimit();
   const roots=list.nodes.map(n=>this.bind(n));
   const reveals=new Set(Object.values(list.animation||{}).flatMap(t=>t.steps.flatMap(s=>s.clips.filter(c=>c.tracks.some(t=>t.property==='-laymesh-reveal')).map(c=>c.target)))),contents=[];
   this.revealContents=new WeakMap();
   const bindReveal=n=>{if(reveals.has(n.name)&&!n.hint?.plotBounds){const inner={...n,name:'',opacity:1,effects:[],transform:[1,0,0,1,0,0],hint:{...n.hint,animationRevealContent:true}};this.revealContents.set(n,inner);contents.push(inner);roots.push(this.bind(inner));}if(n.kind.kind==='group')n.kind.children.forEach(bindReveal);};list.nodes.forEach(bindReveal);
   this.recordPictures(contents);
   const live=new Set(),keep=p=>{if(live.has(p))return;live.add(p);p.dependencies.forEach(keep);};roots.forEach(keep);
   const objects=new Set([...live].flatMap(p=>p.objects));
   for(const object of this.owned.reverse())if(!objects.has(object))object.delete();
   this.owned=[...objects];this.commandCache=new Map([...this.commandCache].filter(([,p])=>live.has(p)));
   // A caller may retain nodes across installs (local export layers). The old
   // identity map also holds pruned, deleted pictures; rebuild it from live keys.
   for(const entry of this.entries)if(!live.has(entry))entry.dead=true;this.entries=live;
   this.pictureBytes=[...live].reduce((n,p)=>n+(p.picture?.approximateBytesUsed()||0),0);this.generation=generation;
   for(const [id,image]of old.images)if(incoming.get(id)!==image)image.delete();this.planScene();
  }catch(error){
   this.owner=null;while(this.owned.length>ownedBefore)this.owned.pop().delete();
   for(const entry of this.entries)if(!entriesBefore.has(entry))entry.dead=true;this.entries=entriesBefore;this.geometryCache=new WeakMap();
   this.commandCache=cacheBefore;this.prepared=new WeakMap();this.contentCache=new WeakMap();Object.assign(this,old);this.list?.nodes.forEach(n=>this.bind(n));
   for(const [id,image]of incoming)if(old.images.get(id)!==image)image.delete();throw error;
  }
 }

 recordPictures(contents=[]){
  const recorder=new this.ck.PictureRecorder(),list=this.list;
  try{const c=recorder.beginRecording([0,0,list.width,list.height]);list.nodes.forEach((n,i)=>this.drawNode(c,n,String(i),{}));contents.forEach(n=>this.drawNode(c,n,'',{}));recorder.finishRecordingAsPicture().delete();}finally{recorder.delete();}
 }

 // Consecutive static roots can be composited once without changing their order
 // relative to animated roots. SkPictures alone only cache commands: replaying
 // them still tessellates paths and executes every blur on the GPU each frame.
 planScene(){
  this.sceneRuns=[];let run;
  this.list.nodes.forEach((node,i)=>{const p=this.prepare(node),dynamic=this.animatedNames.has(node.name)||p.dynamicChildren;
   if(dynamic||!run){run={nodes:[],bounds:null,cost:0,static:!dynamic,key:''};this.sceneRuns.push(run);}
   run.nodes.push({node,id:String(i)});run.bounds=union(run.bounds,transformed(p.bounds,p.matrix));run.cost+=p.cost;run.key+=`${p.uid},`;
   if(dynamic)run=null;
  });
  const live=new Set(this.sceneRuns.filter(r=>r.static).map(r=>r.key));
  for(const [key,entry]of this.staticTextures)if(!live.has(key))this.deleteStaticTexture(key,entry);
  const commands=new Set([...this.commandCache.values()].map(p=>p.uid));
  for(const [key,entry]of this.layerTextures)if(!commands.has(key))this.deleteLayerTexture(key,entry);
  this.updateCacheLimit();
 }
 deleteStaticTexture(key,entry){for(const tile of entry.tiles)tile.image.delete();this.staticTextureBytes-=entry.bytes;this.staticTextures.delete(key);this.updateCacheLimit();}
 updateCacheLimit(){
  // Let the driver's reusable working set grow with the actual owned scene.
  const images=[...this.imageBytes.values()].reduce((n,b)=>n+b.byteLength,0);
  this.gr?.setResourceCacheLimitBytes(Math.max(this.gr.getResourceCacheUsageBytes?.()||0,this.pictureBytes+this.staticTextureBytes+this.layerTextureBytes+images+this.canvas.width*this.canvas.height*8));
 }

 clearStaticTextures(){for(const [key,entry]of this.staticTextures)this.deleteStaticTexture(key,entry);}
 deleteLayerTexture(key,entry){for(const tile of entry.tiles)tile.image.delete();this.layerTextureBytes-=entry.bytes;this.layerTextures.delete(key);this.updateCacheLimit();}
 clearLayerTextures(){for(const [key,entry]of this.layerTextures)this.deleteLayerTexture(key,entry);}
 drawLayer(c,node,id,p,opacity){
  // Reuse GPU content independently of the current transform and opacity.
  // Projection density controls rebuilding; budget pressure never reduces it.
  if(c.quickReject(p.bounds))return true;
  const ck=this.ck,required=projectedDensity(c.getTotalMatrix(),p.bounds);if(!Number.isFinite(required)||required<=0)return false;
  const entry=this.buildLayer(p,required);if(!entry)return false;
  entry.used=this.frameCount;const paint=new ck.Paint();paint.setAlphaf(Math.max(0,Math.min(1,opacity)));
  try{for(const {image,x,y,w,h}of entry.tiles)c.drawImageRectOptions(image,[2,2,w+2,h+2],[x/entry.density,y/entry.density,(x+w)/entry.density,(y+h)/entry.density],ck.FilterMode.Linear,ck.MipmapMode.None,paint);}finally{paint.delete();}
  return true;
 }
 buildLayer(p,required){
  const ck=this.ck;let entry=this.layerTextures.get(p.uid);
  const previous=entry&&entry.density<required?entry:null;if(previous)entry=null;
  if(!entry){
   // Round upward and keep the high-water resolution while playing. This avoids
   // reallocating every frame of a scale/perspective animation without ever
   // accepting a texture below the current projection's pixel density.
   const density=2**(Math.ceil(Math.log2(required*1.05)*2)/2),b=p.bounds,x0=Math.floor(b[0]*density)-2,y0=Math.floor(b[1]*density)-2,x1=Math.ceil(b[2]*density)+2,y1=Math.ceil(b[3]*density)+2;
   if(![x0,y0,x1,y1].every(Number.isSafeInteger)||x1<=x0||y1<=y0||!Number.isSafeInteger((x1-x0)*(y1-y0)*4))return false;
   const tiles=[];let bytes=0;for(let y=y0;y<y1;y+=2048)for(let x=x0;x<x1;x+=2048){const w=Math.min(2048,x1-x),h=Math.min(2048,y1-y);tiles.push({x,y,w,h});bytes+=(w+4)*(h+4)*4;}
   if(previous)previous.used=this.frameCount;
   if(previous)this.deleteLayerTexture(p.uid,previous);
   entry={tiles:[],bytes,density,used:this.frameCount};
   try{for(const tile of tiles){const surface=this.surface.makeSurface({width:tile.w+4,height:tile.h+4,colorType:ck.ColorType.RGBA_8888,alphaType:ck.AlphaType.Premul,colorSpace:ck.ColorSpace.SRGB});if(!surface)throw Error('Animated layer allocation failed');
    try{const t=surface.getCanvas();t.clear(ck.TRANSPARENT);t.translate(2-tile.x,2-tile.y);t.scale(density,density);t.drawPicture(p.picture);entry.tiles.push({...tile,image:surface.makeImageSnapshot()});}finally{surface.dispose();}
   }}catch(error){for(const tile of entry.tiles)tile.image.delete();this.layerAllocationFailures++;return false;}
   this.layerTextures.set(p.uid,entry);this.layerTextureBytes+=bytes;this.updateCacheLimit();this.layerBuilds++;
  }else{this.layerHits++;this.layerTextures.delete(p.uid);this.layerTextures.set(p.uid,entry);}
  return entry;
 }
 async completeResources(){
  // Resource preparation ends at GPU completion, not merely CPU submission.
  // Yield while waiting; no animation is rendered and no pixels are read back.
  if(!this.gl)return;this.surface.flush();const gl=this.gl,fence=gl.fenceSync(gl.SYNC_GPU_COMMANDS_COMPLETE,0);if(!fence)throw Error('GPU resource fence unavailable');gl.flush();const start=performance.now();
  try{for(;;){const status=gl.clientWaitSync(fence,0,0);if(status===gl.ALREADY_SIGNALED||status===gl.CONDITION_SATISFIED)break;if(status===gl.WAIT_FAILED||this.lost)throw Error('GPU resource preparation interrupted');if(performance.now()-start>20000)throw Error('GPU resource preparation timed out');await new Promise(resolve=>setTimeout(resolve,0));}}finally{gl.deleteSync(fence);}
 }
 async completeFrame(visible=true){
  await this.completeResources();
  // flush() submits GPU work. The placeholder canvas is updated only in the
  // owner's rendering step, after animation callbacks. A following task lets
  // that step finish before a static edit is acknowledged or copied to an
  // overview. Animation frames keep their existing nonblocking submission.
  if(!visible||typeof requestAnimationFrame!=='function')return false;
  return new Promise(resolve=>{
   let frame;const timer=setTimeout(()=>{cancelAnimationFrame(frame);resolve(false);},250);
   frame=requestAnimationFrame(()=>setTimeout(()=>{clearTimeout(timer);resolve(true);},0));
  });
 }

 drawScene(c,viewport,width,height,states,useCache){
  const changedRoots=new Set(Object.keys(states).map(id=>id.split('/')[0]));
  for(const run of this.sceneRuns){
   if(useCache&&this.gr&&run.static&&run.cost>=24&&!run.nodes.some(n=>changedRoots.has(n.id))&&this.drawStaticRun(c,run,viewport,width,height))continue;
   for(const {node,id}of run.nodes)this.drawNode(c,node,id,states);
  }
 }
 drawStaticRun(c,run,viewport,width,height){
  const ck=this.ck,sx=width/(viewport[2]-viewport[0]),sy=height/(viewport[3]-viewport[1]),signature=[...viewport,width,height].join(',');
  let entry=this.staticTextures.get(run.key);
  if(entry?.signature!==signature){if(entry)this.deleteStaticTexture(run.key,entry);entry=null;}
  if(!entry){
   // Device-aligned tiles: no lower-resolution rasterization, resampling, CPU
   // pixel readback or bitmap messages. A two-pixel gutter avoids tile seams.
   const b=run.bounds,x0=Math.max(0,Math.floor((b[0]-viewport[0])*sx)-2),y0=Math.max(0,Math.floor((b[1]-viewport[1])*sy)-2),x1=Math.min(width,Math.ceil((b[2]-viewport[0])*sx)+2),y1=Math.min(height,Math.ceil((b[3]-viewport[1])*sy)+2);
   if(![x0,y0,x1,y1].every(Number.isFinite))return false;if(x1<=x0||y1<=y0)return true;
   const tiles=[];let bytes=0;
   for(let y=y0;y<y1;y+=2048)for(let x=x0;x<x1;x+=2048){const w=Math.min(2048,x1-x),h=Math.min(2048,y1-y);tiles.push({x,y,w,h});bytes+=(w+4)*(h+4)*4;}
   entry={signature,tiles:[],bytes,used:this.frameCount};
   try{
    for(const tile of tiles){const surface=this.surface.makeSurface({width:tile.w+4,height:tile.h+4,colorType:ck.ColorType.RGBA_8888,alphaType:ck.AlphaType.Premul,colorSpace:ck.ColorSpace.SRGB});if(!surface)throw Error('Static tile allocation failed');
     try{const t=surface.getCanvas();t.clear(ck.TRANSPARENT);t.translate(2-tile.x,2-tile.y);t.scale(sx,sy);t.translate(-viewport[0],-viewport[1]);
      for(const {node,id}of run.nodes)this.drawNode(t,node,id,{});
      entry.tiles.push({...tile,image:surface.makeImageSnapshot()});
     }finally{surface.dispose();}
    }
   }catch(error){for(const tile of entry.tiles)tile.image.delete();throw error;}
   this.staticTextures.set(run.key,entry);this.staticTextureBytes+=bytes;this.updateCacheLimit();this.cacheBuilds++;
  }else{this.cacheHits++;this.staticTextures.delete(run.key);this.staticTextures.set(run.key,entry);}
  entry.used=this.frameCount;c.save();c.concat([1/sx,0,viewport[0],0,1/sy,viewport[1],0,0,1]);
  for(const {image,x,y,w,h}of entry.tiles)c.drawImageRectOptions(image,[2,2,w+2,h+2],[x,y,x+w,y+h],ck.FilterMode.Nearest,ck.MipmapMode.None,null);
  c.restore();return true;
 }

 clearCommands(){for(const entry of this.entries)entry.dead=true;this.entries.clear();this.geometryCache=new WeakMap();this.contentCache=new WeakMap();for(const object of this.owned.reverse())object.delete();this.owned=[];this.prepared=new WeakMap();this.commandCache.clear();this.pictureBytes=0;}
 content(c,node,id,states){
  const ck=this.ck,k=node.kind,p=this.prepare(node);
  if(k.kind==='path'){if(p.imageFill){c.save();c.clipPath(p.path,ck.ClipOp.Intersect,true);c.concat(matrix(k.fill.transform));this.drawNode(c,k.fill.nodes[0],`${id}/image-fill`,states);c.restore();}else if(p.fill)c.drawPath(p.path,p.fill);if(p.stroke)c.drawPath(p.path,p.stroke);}
  else if(k.kind==='glyphs'){for(const path of p.paths)c.drawPath(path,p.fill);}
  else if(k.kind==='image'){
   const image=this.images.get(k.resource);if(!image)throw Error(`Missing Skia image ${k.resource}`);
   c.drawImageRectOptions(image,[0,0,image.width(),image.height()],rect(k.rect),k.nearest?ck.FilterMode.Nearest:ck.FilterMode.Linear,ck.MipmapMode.None,null);
  }else if(k.kind==='group'){
   c.save();if(p.clip)c.clipPath(p.clip,ck.ClipOp.Intersect,true);
   if(k.mask)c.saveLayer(undefined,p.dynamicChildren?undefined:p.bounds);
   if(node.hint?.animationRevealWrapper&&k.children.length===1){const inner=k.children[0];if(inner.hint?.animationRevealContent)this.drawNode(c,inner,id,states,true);else{c.save();c.concat(matrix(inner.transform));this.content(c,inner,id,states);c.restore();}}else k.children.forEach((n,i)=>this.drawNode(c,n,`${id}/${i}`,states));
   if(k.mask){const paint=new ck.Paint();paint.setBlendMode(ck.BlendMode.DstIn);if(k.mask.luminance){const filter=ck.ColorFilter.MakeLuma();paint.setColorFilter(filter);filter.delete();}c.saveLayer(paint,p.dynamicChildren?undefined:p.bounds);this.drawNode(c,k.mask.node,`${id}/mask`,{});c.restore();c.restore();paint.delete();}
   c.restore();
  }
 }
 revealNode(node,reveal){
  const retained=this.revealContents.get(node),base=retained||{...node,name:'',opacity:1,effects:[],hint:{...node.hint,animationRevealContent:true}};
  const inner={...base,transform:reveal.transform};
  if(retained)this.transientPrepared.set(inner,{...this.prepare(retained),matrix:matrix(inner.transform)});
  const [x0,y0,x1,y1]=reveal.clip;
  return {...node,hint:{...node.hint,animationRevealWrapper:true},kind:{kind:'group',children:[inner],clip:{path:[{MoveTo:{x:x0,y:y0}},{LineTo:{x:x1,y:y0}},{LineTo:{x:x1,y:y1}},{LineTo:{x:x0,y:y1}},'ClosePath'],even_odd:false},mask:null}};
 }
 drawNode(c,original,id,states,contentOnly=false){
  const state=contentOnly?null:states[id];let node=state?.content||original;const opacity=state?.opacity??node.opacity;if(opacity<=0)return;
  if(state?.reveal)node=this.revealNode(node,state.reveal);
  const p=this.prepare(node),m=state?.matrix||p.matrix;c.save();c.concat(m);
  if(this.cacheLayers&&this.gr&&(contentOnly||state&&!state.content)&&!p.dynamicChildren&&p.picture&&node.kind.kind==='group'&&p.cost>=128&&this.drawLayer(c,node,id,p,opacity)){c.restore();return;}
  const ck=this.ck,layer=opacity<1||m[6]!==0||m[7]!==0;
  if(layer){const paint=new ck.Paint();paint.setAlphaf(Math.max(0,Math.min(1,opacity)));c.saveLayer(paint,p.dynamicChildren?undefined:p.bounds);paint.delete();}
  if(!this.transient&&!state?.content&&!p.dynamicChildren&&!p.picture){
   const recorder=new ck.PictureRecorder(),recording=recorder.beginRecording(p.bounds);
   this.localContent(recording,node,id,{});const picture=recorder.finishRecordingAsPicture();recorder.delete();
   const bytes=picture.approximateBytesUsed();p.picture=this.own(picture);p.objects.push(picture);this.pictureBytes+=bytes;
  }
  if(p.picture&&!state?.content)c.drawPicture(p.picture);else this.localContent(c,node,id,states);
  if(layer)c.restore();c.restore();
 }
 drawEffect(c,node,id,states,p,effect){
  c.save();if(effect.knockout&&effect.source)c.clipPath(effect.source,this.ck.ClipOp.Difference,true);
  c.saveLayer(effect.paint,p.dynamicChildren?undefined:p.bounds);
  if(effect.source)c.drawPath(effect.source,effect.fill);else this.content(c,node,id,states);
  c.restore();c.restore();
 }
 localContent(c,node,id,states){const p=this.prepare(node);
  for(const effect of p.effects)if(!effect.inner)this.drawEffect(c,node,id,states,p,effect);
  this.content(c,node,id,states);
  for(const effect of p.effects)if(effect.inner)this.drawEffect(c,node,id,states,p,effect);
 }
 ensureSurface(width,height){
  const ck=this.ck;
  if(!this.surface||this.canvas.width!==width||this.canvas.height!==height){
   this.clearStaticTextures();this.clearLayerTextures();this.surface?.dispose();this.canvas.width=width;this.canvas.height=height;
   this.surface=this.gr?ck.MakeOnScreenGLSurface(this.gr,width,height,ck.ColorSpace.SRGB):this.rasterOnly?ck.MakeSurface(width,height):ck.MakeSWCanvasSurface(this.canvas);
   if(!this.surface)throw Error('Skia surface allocation failed');
  }
 }
 render(viewport,width,height,states={},generation=this.generation,allowSkip=false,cacheStatic=true,cacheLayers=true,preparing=false,effectsOnly=null){
  if(generation!==this.generation)return false;
  if(this.lost)return {backend:'Skia restoring',skipped:true,rasterMs:0};
  if(!Number.isSafeInteger(width)||!Number.isSafeInteger(height)||width<=0||height<=0||!Number.isSafeInteger(width*height*4))throw Error('Invalid Skia viewport');
  const start=performance.now(),ck=this.ck;
  // At most two animation draws may be in flight on the GPU. Worker replies
  // acknowledge CPU submission; they must not allow a queue of stale GPU frames.
  while(this.fences.length&&this.gl.clientWaitSync(this.fences[0],0,0)!==this.gl.TIMEOUT_EXPIRED)this.gl.deleteSync(this.fences.shift());
  if(allowSkip&&!preparing&&this.fences.length>=2)return {skipped:true,reason:'gpu-busy',backend:this.backend,rasterMs:0};
  this.ensureSurface(width,height);
  const gl=this.gl,timer=this.timer;
  const gpuSamples=[];while(this.queries.length&&gl.getQueryParameter(this.queries[0],gl.QUERY_RESULT_AVAILABLE)){const query=this.queries.shift();if(!gl.getParameter(timer.GPU_DISJOINT_EXT)){this.gpuMs=gl.getQueryParameter(query,gl.QUERY_RESULT)/1e6;gpuSamples.push(this.gpuMs);}gl.deleteQuery(query);}
  const query=timer&&this.queries.length<8?gl.createQuery():null;if(query)gl.beginQuery(timer.TIME_ELAPSED_EXT,query);
  this.cacheLayers=allowSkip&&cacheLayers;this.layerHits=0;this.layerBuilds=0;this.layerBudgetMisses=0;this.layerAllocationFailures=0;this.cacheHits=0;this.cacheBuilds=0;this.transient=true;this.transientPrepared=new WeakMap();const count=this.owned.length,c=this.surface.getCanvas(),saved=c.getSaveCount();try{c.clear(ck.TRANSPARENT);c.save();
  c.scale(width/(viewport[2]-viewport[0]),height/(viewport[3]-viewport[1]));c.translate(-viewport[0],-viewport[1]);
  // A fitted presentation viewport includes the area around the page. Clip
  // before subjects, effects and cached layers; their bounds can exceed it.
  this.pageBounds=this.clipToPage?[-viewport[0]*width/(viewport[2]-viewport[0]),-viewport[1]*height/(viewport[3]-viewport[1]),(this.list.width-viewport[0])*width/(viewport[2]-viewport[0]),(this.list.height-viewport[1])*height/(viewport[3]-viewport[1])]:null;
  if(this.clipToPage)c.clipRect([0,0,this.list.width,this.list.height],ck.ClipOp.Intersect,false);
  if(effectsOnly===null)this.drawScene(c,viewport,width,height,states,allowSkip&&cacheStatic);
  else { // Static PDF export: only the requested effect layers, never the vector subject.
   const node=this.list.nodes[0],p=this.prepare(node);
   for(const effect of p.effects)if(effect.inner===effectsOnly)this.drawEffect(c,node,'0',{},p,effect);
  }
  c.restore();this.surface.flush();}finally{c.restoreToCount(saved);if(query){gl.endQuery(timer.TIME_ELAPSED_EXT);this.queries.push(query);}
  // Geometry extensions own temporary commands for this frame only.
  while(this.owned.length>count)this.owned.pop().delete();
  this.transient=false;this.cacheLayers=false;}if(gl){const fence=gl.fenceSync(gl.SYNC_GPU_COMMANDS_COMPLETE,0);if(fence)this.fences.push(fence);gl.flush();}this.frameCount++;this.onPresent();return {backend:this.backend,device:this.device,rasterMs:performance.now()-start,gpuMs:this.gpuMs,gpuSamples,cacheBytes:this.gr?.getResourceCacheUsageBytes()||0,staticTextureBytes:this.staticTextureBytes,staticCacheHits:this.cacheHits,staticCacheBuilds:this.cacheBuilds,layerTextureBytes:this.layerTextureBytes,layerCacheHits:this.layerHits,layerCacheBuilds:this.layerBuilds,layerCacheBudgetMisses:this.layerBudgetMisses,layerCacheAllocationFailures:this.layerAllocationFailures,frameCount:this.frameCount};
 }
 // Explicit readback for static export and test oracles; playback never calls it.
 readPixels(){const image=this.surface.makeImageSnapshot();try{return image.readPixels(0,0,{width:this.canvas.width,height:this.canvas.height,colorType:this.ck.ColorType.RGBA_8888,alphaType:this.ck.AlphaType.Unpremul,colorSpace:this.ck.ColorSpace.SRGB});}finally{image.delete();}}
 trim(){this.clearStaticTextures();this.clearLayerTextures();if(this.gr){this.gr.setResourceCacheLimitBytes(0);this.updateCacheLimit();}}
 dispose(){this.transitions.dispose();this.canvas.removeEventListener?.('webglcontextlost',this.onLost);this.canvas.removeEventListener?.('webglcontextrestored',this.onRestored);this.queries.forEach(q=>this.gl?.deleteQuery(q));this.queries=[];this.fences.forEach(f=>this.gl?.deleteSync(f));this.fences=[];this.clearStaticTextures();this.clearLayerTextures();this.clearCommands();for(const image of this.images.values())image.delete();this.images.clear();this.imageBytes.clear();this.surface?.dispose();this.gr?.releaseResourcesAndAbandonContext();this.gr?.delete();if(this.context)this.ck.deleteContext(this.context);}
}
