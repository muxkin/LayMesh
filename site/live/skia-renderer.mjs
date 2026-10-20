// Shared by the website and the offline VS Code worker. Layout and animation
// sampling remain in Rust; these retained commands never reshape text.
import CanvasKitInit from './skia/canvaskit.mjs';
import {applyDisplayUpdate} from './display-update.mjs';
import {PageTransitions,decodeSkiaImage,opaquePageRect} from './presentation-transitions.mjs';
import {spreadAlpha,spreadRoundedRectPath} from './shadow-mask.mjs';
import {makePresentationCPUSurface,canBlitPresentationLayer,blitPresentationLayer} from './skia-cpu-surface.mjs';
import {supportsNativeComposition,promotePresentationEntry,restorePresentationEntry,composePresentationCanvas} from './presentation-canvas.mjs';

let surfaceRaster,surfaceRegion,surfaceRefine,surfaceMaterial;
export function setSurfaceRasterizer(fn,region,refine,material){surfaceRaster=fn;surfaceRegion=region;surfaceRefine=refine;surfaceMaterial=material;}
async function ensureSurfaceRasterizer(){if(!surfaceRaster||!surfaceRegion||!surfaceRefine||!surfaceMaterial){const runtime=await import('./wasm/laymesh_wasm.js');await runtime.default();surfaceRaster=runtime.raster_surface3d;surfaceRegion=runtime.surface_texture_region;surfaceRefine=runtime.refine_surface3d;surfaceMaterial=runtime.surface_material_texture;}}

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
export function singlePatternCell(bounds,fill){
 if(fill.kind!=='pattern'||!(fill.width>0&&fill.height>0))return false;
 const [a,b,c,d,e,f]=fill.transform,det=a*d-b*c;if(!Number.isFinite(det)||!det)return false;
 const r=transformed(bounds,[d/det,-c/det,(c*f-d*e)/det,-b/det,a/det,(b*e-a*f)/det,0,0,1]),epsilon=1e-9*Math.max(1,fill.width,fill.height);
 return r.every(Number.isFinite)&&r[0]>=-epsilon&&r[1]>=-epsilon&&r[2]<=fill.width+epsilon&&r[3]<=fill.height+epsilon;
}
// Nested setTimeout(0) is clamped to 4 ms by browsers. Yield a real task so
// cancellation/input can run without adding that delay to every resource tile.
function yieldRenderTask(delay=0){
 if(delay>0)return new Promise(resolve=>setTimeout(resolve,delay));
 return new Promise(resolve=>{const channel=new MessageChannel();channel.port1.onmessage=()=>{channel.port1.close();channel.port2.close();resolve();};channel.port2.postMessage(null);});
}
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
 constructor(ck,canvas,{software=false,rasterOnly=false,premultipliedRaster=false,clipToPage=false,nativeComposition=true,preferNativeSoftware=true,preserveDrawingBuffer=false,onPresent=()=>{},onContextChange=()=>{}}={}) {
  this.allowNativeComposition=nativeComposition;
  this.rasterOnly=rasterOnly;this.premultipliedRaster=premultipliedRaster;this.clipToPage=clipToPage;this.pageBounds=null;
  this.ck=ck;this.canvas=canvas;this.onPresent=onPresent;this.transitions=new PageTransitions(this);this.images=new Map();this.imageBytes=new Map();this.commandCache=new Map();this.prepared=new WeakMap();this.geometryCache=new WeakMap();this.contentCache=new WeakMap();this.entries=new Set();this.owned=[];this.animatedNames=new Set();this.pictureBytes=0;this.generation=0;this.frameCount=0;
  this.nextCommandId=0;this.staticTextures=new Map();this.staticTexturePool=new Map();this.staticTextureBytes=0;this.sceneRuns=[];this.revealContents=new WeakMap();
  this.layerTextures=new Map();this.layerTextureBytes=0;
  this.imagePool=new Map();this.imageOwners=new WeakMap();this.imageDecodes=0;this.imageReuses=0;
  // A software WebGL driver runs GPU shader work on the CPU. For presentation
  // its readback/completion and duplicate render targets cost more than native
  // bitmap composition. Probe a disposable context before binding the display
  // canvas: a canvas cannot switch from WebGL to 2D after binding.
  if(!software&&premultipliedRaster&&clipToPage&&nativeComposition&&preferNativeSoftware&&typeof OffscreenCanvas!=='undefined'){
   const started=performance.now(),probe=new OffscreenCanvas(1,1),gl=probe.getContext('webgl2');
   if(gl){const debug=gl.getExtension('WEBGL_debug_renderer_info'),device=debug?gl.getParameter(debug.UNMASKED_RENDERER_WEBGL):'';if(/SwiftShader|llvmpipe|softpipe/i.test(device)){software=true;this.softwareGpuBypass=device;}gl.getExtension('WEBGL_lose_context')?.loseContext();}
   probe.width=probe.height=1;this.backendProbeMs=performance.now()-started;
  }
  this.context=software?0:ck.GetWebGLContext(canvas,{majorVersion:2,alpha:1,antialias:1,preserveDrawingBuffer:Number(preserveDrawingBuffer)});
  this.gr=this.context?ck.MakeWebGLContext(this.context):null;
  this.backend=this.gr?'Skia WebGL2':this.softwareGpuBypass?'Skia CPU + Canvas2D':'Skia CPU';this.updateCacheLimit();this.gl=this.gr?canvas.getContext('webgl2'):null;this.timer=this.gl?.getExtension('EXT_disjoint_timer_query_webgl2');this.queries=[];this.fences=[];const debug=this.gl?.getExtension('WEBGL_debug_renderer_info');this.device=debug?this.gl.getParameter(debug.UNMASKED_RENDERER_WEBGL):'';if(/SwiftShader|llvmpipe|softpipe/i.test(this.device))this.backend+=' (software driver)';
  this.onLost=e=>{e.preventDefault();this.lost=true;this.transitions.cancel();this.transitions.warmed.clear();this.clearStaticTextures();this.clearLayerTextures();this.queries=[];this.fences=[];this.gpuMs=undefined;onContextChange(false);};canvas.addEventListener?.('webglcontextlost',this.onLost);
  this.onRestored=()=>{this.surface?.dispose();this.surface=null;this.gr?.delete();this.gr=ck.MakeWebGLContext(this.context);this.updateCacheLimit();this.timer=this.gl?.getExtension('EXT_disjoint_timer_query_webgl2');this.backend=this.gr?'Skia WebGL2'+(/SwiftShader|llvmpipe|softpipe/i.test(this.device)?' (software driver)':''):'Skia CPU';this.lost=false;onContextChange(true);};canvas.addEventListener?.('webglcontextrestored',this.onRestored);
 }
 own(value){if(value){this.owned.push(value);this.owner?.objects.push(value);}return value;}
 acquireImage(id,bytes,mime){
  const variants=this.imagePool.get(id)||[],shared=variants.find(p=>p.mime===mime&&p.bytes.length===bytes.length&&p.bytes.every((v,i)=>v===bytes[i]));
  if(shared){shared.references++;this.imageReuses++;return shared;}
  const image=decodeSkiaImage(this.ck,bytes,mime);if(!image)throw Error(`Skia image decode failed: ${id}`);
  const entry={id,image,bytes,mime,references:1,identity:++this.imageDecodes};variants.push(entry);this.imagePool.set(id,variants);this.imageOwners.set(image,entry);return entry;
 }
 releaseImage(image){
  const entry=this.imageOwners.get(image);if(!entry){image.delete();return;}
  if(--entry.references)return;
  image.delete();this.imageOwners.delete(image);const variants=this.imagePool.get(entry.id).filter(p=>p!==entry);if(variants.length)this.imagePool.set(entry.id,variants);else this.imagePool.delete(entry.id);
 }
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
   }else if(spec.kind==='conic'){
    const [x,y]=spec.center,a=spec.angle-Math.PI/2,c=Math.cos(a),s=Math.sin(a),[m0,m1,m2,m3,m4,m5]=spec.transform;
    const r=[c,s,-s,c,x-c*x+s*y,y-s*x-c*y];
    const m=matrix([m0*r[0]+m2*r[1],m1*r[0]+m3*r[1],m0*r[2]+m2*r[3],m1*r[2]+m3*r[3],m0*r[4]+m2*r[5]+m4,m1*r[4]+m3*r[5]+m5]);
    shader=ck.Shader.MakeSweepGradient(x,y,spec.stops.map(s=>this.color(s.rgba)),spec.stops.map(s=>s.offset),ck.TileMode.Clamp,m,0,0,360);
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
  const contentKey=JSON.stringify([node.effects,node.hint]);
  const content=!this.transient&&!node.hint?.surface3d&&this.contentCache.get(node.kind)?.get(contentKey);
  const placementKey=content=>'placement/'+(content.textureKey??content.uid)+'/'+JSON.stringify([node.name,node.opacity,node.transform]);
  const alias=content=>{
   const key=placementKey(content),previous=this.commandCache.get(key);
   if(previous&&!previous.dead){frameCache.set(node,previous);this.owner?.dependencies.add(previous);return previous;}
   const canonical=content.canonical||content,value={...content,canonical,uid:++this.nextCommandId,textureKey:content.textureKey??content.uid,matrix:matrix(node.transform),objects:[],dependencies:new Set([canonical]),bound:true};
   Object.defineProperty(value,'picture',{enumerable:true,get:()=>canonical.picture,set:picture=>canonical.picture=picture});this.entries.add(value);frameCache.set(node,value);this.commandCache.set(key,value);this.owner?.dependencies.add(value);return value;
  };
  if(content&&!content.dead)return alias(content);
  const cacheKey=value=>JSON.stringify(value,(key,value)=>key==='_identity'?undefined:value);
  // Outer placement/opacity are composition properties, not painted content.
  const identity={...node,name:'',transform:null,opacity:1};
  const key=this.transient?null:node.kind.kind==='group'?cacheKey({...identity,kind:{...node.kind,children:node.kind.children.map(n=>[this.prepare(n).uid,n.opacity])}}):cacheKey(identity);
  const cached=key&&this.commandCache.get(key);
  if(cached&&!cached.dead)return alias(cached);
  const parent=this.owner,ck=this.ck,k=node.kind,v={uid:++this.nextCommandId,cost:1,matrix:matrix(node.transform),bounds:[0,0,0,0],effects:[],objects:[],dependencies:new Set()};
  if(!this.transient)this.entries.add(v);parent?.dependencies.add(v);this.owner=v;frameCache.set(node,v);if(key){this.commandCache.set(key,v);this.commandCache.set(placementKey(v),v);}
  if(k.kind==='path'){
   v.path=this.path(k.path,k.even_odd);v.bounds=Array.from(v.path.getBounds());v.imageFill=k.fill.kind==='pattern'&&k.fill.nodes.length===1&&imagePattern(k.fill.nodes[0]);
   // CSS background layers commonly wrap a single gradient in several full-
   // size pattern cells. Recording those as nested picture shaders rasterizes
   // whole intermediate textures before even a small output tile can run.
   // Inside one cell, clipped retained commands have identical repeat semantics.
   v.directPattern=!this.gr&&!v.imageFill&&singlePatternCell(v.bounds,k.fill);v.fill=v.imageFill||v.directPattern?null:this.paint(k.fill);
   if(k.stroke){const s=k.stroke,p=v.stroke=this.paint({kind:'solid',rgba:s.rgba});p.setStyle(ck.PaintStyle.Stroke);p.setStrokeWidth(s.width);p.setStrokeCap(s.cap==='round'?ck.StrokeCap.Round:s.cap==='square'?ck.StrokeCap.Square:ck.StrokeCap.Butt);p.setStrokeJoin(s.join==='round'?ck.StrokeJoin.Round:s.join==='bevel'?ck.StrokeJoin.Bevel:ck.StrokeJoin.Miter);p.setStrokeMiter(s.miter);if(s.dash.length){const effect=ck.PathEffect.MakeDash(s.dash,s.offset);p.setPathEffect(effect);effect?.delete();}const pad=s.width*s.miter;v.bounds=[v.bounds[0]-pad,v.bounds[1]-pad,v.bounds[2]+pad,v.bounds[3]+pad];}
  }else if(k.kind==='glyphs'){
   v.paths=k.outlines.map(p=>this.path(p));v.fill=this.paint({kind:'solid',rgba:k.color});v.bounds=v.paths.reduce((b,p)=>union(b,p.getBounds()),null)||v.bounds;
  }else if(k.kind==='image'){v.bounds=rect(k.rect);v.dynamicChildren=!!node.hint?.mediaId;}
  else if(k.kind==='group'){
   v.dynamicChildren=k.children.some(n=>this.animatedNames.has(n.name)||this.prepare(n).dynamicChildren);
   v.bounds=k.children.reduce((b,n)=>{const p=this.prepare(n);return union(b,transformed(p.bounds,p.matrix));},null)||v.bounds;
   if(k.clip)v.clip=this.path(k.clip.path,k.clip.even_odd);
  }else throw Error(`Unsupported Skia command ${k.kind}`);
  v.subjectBounds=Array.from(v.bounds);
  if(node.hint?.surface3d){v.bounds=Array.from(node.hint.surface3d.bounds);v.subjectBounds=Array.from(v.bounds);v.dynamicChildren=true;}
  const softRadius=Math.max(0,...node.effects.map(e=>e.softEdge||0));
  for(const e of node.effects){
   if(e.softEdge!=null){if(e.softEdge>0&&e.softEdge===softRadius&&!v.softPaint){const erosion=ck.ImageFilter.MakeErode(e.softEdge/2,e.softEdge/2,null),filter=ck.ImageFilter.MakeBlur(e.softEdge/2,e.softEdge/2,ck.TileMode.Decal,erosion);erosion?.delete();v.softPaint=this.paint({kind:'solid',rgba:[1,1,1,1]});v.softPaint.setBlendMode(ck.BlendMode.DstIn);v.softPaint.setImageFilter(filter);filter?.delete();}continue;}
   if(e.reflection){const r=e.reflection,b=v.subjectBounds,y=b[3]+r.distance,h=(b[3]-b[1])*r.length,paint=this.paint({kind:'solid',rgba:[1,1,1,1]});if(e.blur){const filter=ck.ImageFilter.MakeBlur(e.blur,e.blur,ck.TileMode.Decal,null);paint.setImageFilter(filter);filter.delete();}const mask=this.paint({kind:'solid',rgba:[1,1,1,1]}),shader=ck.Shader.MakeLinearGradient([0,y],[0,y+h],[this.color([1,1,1,r.opacity]),this.color([1,1,1,r.opacity*(1-r.fade)])],[0,1],ck.TileMode.Clamp);if(shader){mask.setShader(shader);shader.delete();}mask.setBlendMode(ck.BlendMode.DstIn);v.effects.push({reflection:r,paint,mask,inner:false});v.bounds=union(v.bounds,[b[0]-4*e.blur,y-4*e.blur,b[2]+4*e.blur,y+h+4*e.blur]);continue;}

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
   const source=e.sourcePath?this.path(e.sourcePath):null,b=source?Array.from(source.getBounds()):v.subjectBounds,pad=4*e.blur+Math.abs(e.spread);
   const bounds=e.inner?Array.from(b):union(b,[b[0]+e.offset[0]-pad,b[1]+e.offset[1]-pad,b[2]+e.offset[0]+pad,b[3]+e.offset[1]+pad]);
  shadow.delete();v.effects.push({paint,inner:e.inner,source,knockout:e.knockout,bounds,fill:this.paint({kind:'solid',rgba:[1,1,1,1]}),spec:e,sourceKey:e.sourcePath?JSON.stringify(e.sourcePath):null});
   v.bounds=union(v.bounds,bounds);
  }
  v.cost+=k.kind==='group'?k.children.reduce((n,child)=>n+this.prepare(child).cost,0):k.kind==='path'?k.path.length:k.kind==='glyphs'?k.outlines.reduce((n,path)=>n+path.length,0):1;
  v.effectCount=node.effects.length+(k.kind==='group'?k.children.reduce((n,child)=>n+(this.prepare(child).effectCount||0),0):0);v.hasEffects=v.effectCount>0;
  v.cpuMask=node.effects.some(e=>e.sourcePath&&e.spread&&!e.inner)||(k.kind==='group'&&k.children.some(n=>this.prepare(n).cpuMask));v.node=node;
  v.filterOutset=Math.max(0,...node.effects.map(e=>4*(e.blur||0)+Math.abs(e.spread||0)),...(k.kind==='group'?k.children.map(n=>{const p=this.prepare(n);return (p.filterOutset||0)*projectedDensity(p.matrix,p.bounds);}):[]));
  this.owner=parent;if(!this.transient){let values=this.contentCache.get(node.kind);if(!values)this.contentCache.set(node.kind,values=new Map());values.set(contentKey,v);}return v;
 }
 bind(n){const p=this.prepare(n),k=n.kind;if(p.bound)return p;p.bound=true;if(k.kind==='group'){k.children.forEach(n=>p.dependencies.add(this.bind(n)));if(k.mask)p.dependencies.add(this.bind(k.mask.node));}if(k.kind==='path'&&k.fill.kind==='pattern')k.fill.nodes.forEach(n=>p.dependencies.add(this.bind(n)));return p;}
 setMediaFrames(frames=[]){
  for(const {resource,bitmap}of frames){try{
   if(!this.list?.resources[resource])continue;
   let image;if(this.gr&&this.surface)image=this.surface.makeImageFromTextureSource(bitmap);
   else {const c=new OffscreenCanvas(bitmap.width,bitmap.height),ctx=c.getContext('2d');ctx.drawImage(bitmap,0,0);image=this.ck.MakeImage({width:bitmap.width,height:bitmap.height,alphaType:this.ck.AlphaType.Unpremul,colorType:this.ck.ColorType.RGBA_8888,colorSpace:this.ck.ColorSpace.SRGB},ctx.getImageData(0,0,bitmap.width,bitmap.height).data,bitmap.width*4);}
   if(!image)throw Error('Video frame upload failed');if(this.images.has(resource))this.releaseImage(this.images.get(resource));this.images.set(resource,image);
  }finally{bitmap.close();}}
 }
 async update(update,readResource,generation,navigation){
  return this.install(applyDisplayUpdate(this.list,update),readResource,generation,navigation,true);
 }
 async install(list,readResource,generation,navigation,incremental=false){
  if(navigation!==undefined)this.transitions.navigate(list,navigation);
  if(![1,2,3].includes(list.schemaVersion))throw Error('Unsupported Skia display list version');
  const hasSurface=n=>!!n.hint?.surface3d||(n.kind.kind==='group'&&n.kind.children.some(hasSurface));if(list.nodes.some(hasSurface))await ensureSurfaceRasterizer();
  const incoming=new Map(),bytesById=new Map(),old={list:this.list,images:this.images,imageBytes:this.imageBytes,animatedNames:this.animatedNames,generation:this.generation,revealContents:this.revealContents};
  const ownedBefore=this.owned.length,cacheBefore=new Map(this.commandCache),entriesBefore=new Set(this.entries);let resourceChanged=false;
  try{
   for(const [id,meta]of Object.entries(list.resources))if(meta.mime.startsWith('image/')){
    if(incremental&&list.resources===old.list.resources&&this.images.has(id)){incoming.set(id,this.images.get(id));bytesById.set(id,this.imageBytes.get(id));continue;}
    const bytes=await readResource(id,meta),previous=this.imageBytes.get(id);bytesById.set(id,bytes);
    if(previous&&previous.length===bytes.length&&previous.every((v,i)=>v===bytes[i])){incoming.set(id,this.images.get(id));continue;}
    if(previous)resourceChanged=true;
    const shared=this.acquireImage(id,bytes,meta.mime);incoming.set(id,shared.image);bytesById.set(id,shared.bytes);
   }
   // Commands are keyed by content; unchanged paths, glyphs and pictures survive
   // an edit. Resource replacement invalidates pictures referencing that image.
   if(resourceChanged){this.commandCache=new Map();this.prepared=new WeakMap();this.contentCache=new WeakMap();}
   this.images=incoming;this.imageBytes=bytesById;this.list=list;
   const names=new Set(Object.values(list.animation||{}).flatMap(t=>t.steps.flatMap(s=>s.clips.map(c=>c.target))));
   if([...names].sort().join('\0')!==[...this.animatedNames].sort().join('\0')){this.commandCache=new Map();this.prepared=new WeakMap();this.contentCache=new WeakMap();}
   this.animatedNames=names;this.updateCacheLimit();
   const roots=[];let bindingAt=performance.now();for(const node of list.nodes){roots.push(this.bind(node));if(performance.now()-bindingAt>=8){await yieldRenderTask();bindingAt=performance.now();}}
   const reveals=new Set(Object.values(list.animation||{}).flatMap(t=>t.steps.flatMap(s=>s.clips.filter(c=>c.tracks.some(t=>t.property==='-laymesh-reveal')).map(c=>c.target)))),contents=[];
   this.revealContents=new WeakMap();
   const bindReveal=n=>{if(reveals.has(n.name)&&!n.hint?.plotBounds){const inner={...n,name:'',opacity:1,effects:[],transform:[1,0,0,1,0,0],hint:{...n.hint,animationRevealContent:true}};this.revealContents.set(n,inner);contents.push(inner);roots.push(this.bind(inner));}if(n.kind.kind==='group')n.kind.children.forEach(bindReveal);};list.nodes.forEach(bindReveal);
   await this.recordPictures(contents);
   const live=new Set(),keep=p=>{if(live.has(p))return;live.add(p);p.dependencies.forEach(keep);};roots.forEach(keep);
   const objects=new Set([...live].flatMap(p=>p.objects));
   for(const object of this.owned.reverse())if(!objects.has(object))object.delete();
   this.owned=[...objects];this.commandCache=new Map([...this.commandCache].filter(([,p])=>live.has(p)));
   // A caller may retain nodes across installs (local export layers). The old
   // identity map also holds pruned, deleted pictures; rebuild it from live keys.
   for(const entry of this.entries)if(!live.has(entry)){this.dropSurfaceTexture(entry);entry.dead=true;}this.entries=live;
   this.pictureBytes=[...new Set([...live].map(p=>p.picture).filter(Boolean))].reduce((n,p)=>n+(p.approximateBytesUsed()||0),0);this.generation=generation;
   for(const [id,image]of old.images)if(incoming.get(id)!==image)this.releaseImage(image);this.planScene();
  }catch(error){
   this.owner=null;while(this.owned.length>ownedBefore)this.owned.pop().delete();
   for(const entry of this.entries)if(!entriesBefore.has(entry))entry.dead=true;this.entries=entriesBefore;this.geometryCache=new WeakMap();
   this.commandCache=cacheBefore;this.prepared=new WeakMap();this.contentCache=new WeakMap();Object.assign(this,old);this.list?.nodes.forEach(n=>this.bind(n));
   for(const [id,image]of incoming)if(old.images.get(id)!==image)this.releaseImage(image);throw error;
  }
 }

 async recordPictures(contents=[]){
  const recorder=new this.ck.PictureRecorder(),list=this.list;
  this.recordingCommands=true;try{const c=recorder.beginRecording([0,0,list.width,list.height]);let at=performance.now();for(const [i,n]of [...list.nodes,...contents].entries()){this.drawNode(c,n,String(i),{});if(performance.now()-at>=8){await yieldRenderTask();at=performance.now();}}recorder.finishRecordingAsPicture().delete();}finally{this.recordingCommands=false;recorder.delete();}
 }

 // Retain costly static objects at their own visible bounds. Including the
 // cheap page background (or distant header/footer) in a run turned mostly
 // empty space into a full-page texture on every page. Cheap vector commands
 // stay direct; expensive objects still avoid tessellation and blur per frame.
 planScene(){
  this.nativeComposition=supportsNativeComposition(this);
  const background=this.nativeComposition&&opaquePageRect(this.list,{},[0,0,this.list.width,this.list.height],this.list.width,this.list.height);
  this.sceneRuns=[];let run;
  this.list.nodes.forEach((node,i)=>{const p=this.prepare(node),dynamic=this.animatedNames.has(node.name)||p.dynamicChildren,isBackground=i===0&&background&&!dynamic,empty=this.nativeComposition&&!dynamic&&(node.opacity<=0||!node.effects.length&&node.kind.kind==='path'&&!node.kind.stroke&&(node.kind.fill.kind==='none'||node.kind.fill.kind==='solid'&&node.kind.fill.rgba[3]===0)),cacheable=!empty&&!dynamic&&!isBackground&&node.opacity>0&&(this.nativeComposition||p.hasEffects||p.cost>=24);
   if(empty){this.sceneRuns.push({nodes:[{node,id:String(i)}],bounds:null,cost:0,static:true,cacheable:false,key:`${p.uid},`,empty:true});run=null;return;}
   if(dynamic||cacheable||isBackground||!run){run={nodes:[],bounds:null,cost:0,static:!dynamic,cacheable,key:'',background:isBackground?node.kind.fill.rgba:null};this.sceneRuns.push(run);}
   run.nodes.push({node,id:String(i)});run.bounds=union(run.bounds,transformed(p.bounds,p.matrix));run.cost+=p.cost;run.key+=`${p.uid},`;
   if(dynamic||cacheable||isBackground)run=null;
  });
  const resources=[...this.images||[]].map(([id,image])=>[id,this.imageOwners.get(image)?.identity]);
  for(const run of this.sceneRuns)if(run.cacheable)run.contentKey=JSON.stringify([this.list.width,this.list.height,resources,run.nodes.map(n=>n.node)],function(key,value){return key==='_identity'||key==='name'&&this.kind&&this.transform?undefined:value;});
  const live=new Set(this.sceneRuns.filter(r=>r.static).map(r=>r.key));
  for(const [key,entry]of this.staticTextures)if(!live.has(key))this.deleteStaticTexture(key,entry);
  const commands=new Set([...this.entries].map(p=>p.textureKey??p.uid));
  for(const [key,entry]of this.layerTextures)if(!commands.has(key))this.deleteLayerTexture(key,entry);
  this.updateCacheLimit();
 }
 deleteStaticTexture(key,entry){
  this.staticTextureBytes-=entry.bytes;this.staticTextures.delete(key);
  if(--entry.references<=0){if(this.staticTexturePool.get(entry.poolKey)===entry)this.staticTexturePool.delete(entry.poolKey);this.textureReleased=(this.textureReleased||0)+entry.bytes;if(entry.bitmap){entry.bitmap.close();this.nativeBitmapBytes-=entry.bytes;this.nativeBitmapCount--;}for(const tile of entry.tiles)if(tile.image){tile.image.delete();this.textureImages--;}}
  this.updateCacheLimit();
 }
 updateCacheLimit(){
  // Let the driver's reusable working set grow with the actual owned scene.
  const images=[...this.imageBytes.values()].reduce((n,b)=>n+b.byteLength,0);
  // A hidden presentation uses a tiny display framebuffer, but its existing
  // driver cache policy must still be based on the real output dimensions.
  const pixels=this.cacheViewportPixels??this.canvas.width*this.canvas.height;
  this.gr?.setResourceCacheLimitBytes(Math.max(this.gr.getResourceCacheUsageBytes?.()||0,this.pictureBytes+this.staticTextureBytes+this.layerTextureBytes+images+pixels*8));
 }

 clearStaticTextures(){for(const [key,entry]of this.staticTextures)this.deleteStaticTexture(key,entry);}
 deleteLayerTexture(key,entry){this.textureReleased=(this.textureReleased||0)+entry.bytes;if(entry.bitmap){entry.bitmap.close();this.nativeBitmapBytes-=entry.bytes;this.nativeBitmapCount--;}for(const tile of entry.tiles){if(tile.image){tile.image.delete();this.textureImages=(this.textureImages||0)-1;}if(tile.pixels){this.cpuTileBytes-=tile.pixels.byteLength;tile.pixels=null;}}this.layerTextureBytes-=entry.bytes;this.layerTextures.delete(key);this.updateCacheLimit();}
 clearLayerTextures(){for(const [key,entry]of this.layerTextures)this.deleteLayerTexture(key,entry);}
 releaseScratch(){this.scratch?.surface.dispose();this.scratch=null;}
 gpuResourceBytes(){if(!this.gr)return 0;const value=this.gr.getResourceCacheUsageBytes();return value??this.gr._getResourceCacheUsageBytes?.()??0;}
 drawLayer(c,node,id,p,opacity){
  // Reuse GPU content independently of the current transform and opacity.
  // Projection density controls rebuilding; budget pressure never reduces it.
  if(c.quickReject(p.bounds))return true;
  const ck=this.ck,m=c.getTotalMatrix(),required=projectedDensity(m,p.bounds);if(!Number.isFinite(required)||required<=0)return false;
  const entry=this.buildLayer(p,required);if(!entry)return false;
  restorePresentationEntry(this,entry);
  entry.used=this.frameCount;
  if(!entry.cpuBlitDisabled&&this.clipToPage&&this.surface.premultipliedPixels&&canBlitPresentationLayer(id,node,m,entry.density)&&this.cpuLayerPixels(entry)){
   // Native readback can grow the WASM heap. Acquire the output view afterwards.
   blitPresentationLayer(this.surface.premultipliedPixels(),this.canvas.width,this.canvas.height,entry.tiles,m,entry.density,this.pageBounds,opacity);this.cpuLayerBlits=(this.cpuLayerBlits||0)+1;return true;
  }
  this.restoreLayerImages(entry);
  entry.used=this.frameCount;const paint=new ck.Paint();paint.setAlphaf(Math.max(0,Math.min(1,opacity)));
  try{for(const {image,x,y,w,h}of entry.tiles)c.drawImageRectOptions(image,[2,2,w+2,h+2],[x/entry.density,y/entry.density,(x+w)/entry.density,(y+h)/entry.density],ck.FilterMode.Linear,ck.MipmapMode.None,paint);}finally{paint.delete();}
  return true;
 }
 cpuLayerPixels(entry){
  const ck=this.ck;
  for(const tile of entry.tiles){
   if(tile.pixels)continue;
   const width=tile.w+4,height=tile.h+4,pixels=tile.image.readPixels(0,0,{width,height,colorType:ck.ColorType.RGBA_8888,alphaType:ck.AlphaType.Premul,colorSpace:ck.ColorSpace.SRGB});
   if(!pixels){entry.cpuBlitDisabled=true;return false;}
   const bytes=pixels.byteLength;this.textureAllocated=(this.textureAllocated||0)+bytes;this.texturePeak=Math.max(this.texturePeak||0,this.textureAllocated-(this.textureReleased||0));
   // Transfer the retained representation: no second permanent copy of a tile.
   tile.image.delete();tile.image=null;tile.pixels=pixels;this.textureImages--;this.textureReleased=(this.textureReleased||0)+bytes;
   this.cpuTileBytes=(this.cpuTileBytes||0)+bytes;this.cpuTilePromotions=(this.cpuTilePromotions||0)+1;this.cpuTileConversions=(this.cpuTileConversions||0)+1;
   this.cpuTileWorkingPeakBytes=Math.max(this.cpuTileWorkingPeakBytes||0,bytes*2);
  }
  return true;
 }
 restoreLayerImages(entry){
  const ck=this.ck;
  for(const tile of entry.tiles){
   if(!tile.pixels)continue;
   const width=tile.w+4,height=tile.h+4,bytes=tile.pixels.byteLength,image=ck.MakeImage({width,height,colorType:ck.ColorType.RGBA_8888,alphaType:ck.AlphaType.Premul,colorSpace:ck.ColorSpace.SRGB},tile.pixels,width*4);
   if(!image)throw Error('Cached tile restoration failed');
   this.textureAllocated=(this.textureAllocated||0)+bytes;this.texturePeak=Math.max(this.texturePeak||0,this.textureAllocated-(this.textureReleased||0));
   tile.image=image;tile.pixels=null;this.textureImages++;this.textureReleased=(this.textureReleased||0)+bytes;this.cpuTileBytes-=bytes;
   this.cpuTileDemotions=(this.cpuTileDemotions||0)+1;this.cpuTileConversions=(this.cpuTileConversions||0)+1;
   // A later rotation or nested use keeps the general representation. Repeated
   // transform changes must not cause readback/allocation churn.
   entry.cpuBlitDisabled=true;
  }
 }
 buildLayer(p,required){let result;const work=this.layerTasks(p,required);for(;;){const step=work.next();if(step.done){result=step.value;break;}}return result;}
 *layerTasks(p,required){
  const key=p.textureKey??p.uid;let entry=this.layerTextures.get(key);
  if(entry&&entry.density<required){this.deleteLayerTexture(key,entry);entry=null;}
  if(!entry){
   // Plan actual projected density; keep it at its high-water mark. A small
   // numerical margin avoids roundoff rebuilds without sqrt(2) oversampling.
   const density=required*(1+1e-6),b=p.bounds,x0=Math.floor(b[0]*density)-2,y0=Math.floor(b[1]*density)-2,x1=Math.ceil(b[2]*density)+2,y1=Math.ceil(b[3]*density)+2;
   if(![x0,y0,x1,y1].every(Number.isSafeInteger)||x1<=x0||y1<=y0||!Number.isSafeInteger((x1-x0)*(y1-y0)*4))return false;
   entry=this.tileEntry(x0,y0,x1,y1,{density,filterOutset:(p.filterOutset||0)*density,used:this.frameCount});this.layerTextures.set(key,entry);this.layerBuilds=(this.layerBuilds||0)+1;
  }else if(entry.complete)this.layerHits=(this.layerHits||0)+1;
  const ck=this.ck;
  try{yield* this.rasterTiles(entry,t=>{t.scale(entry.density,entry.density);if(!this.gr&&p.node&&p.hasEffects)return this.contentPreparationTasks(t,p.node,'0');this.effectPictureReplays=(this.effectPictureReplays||0)+(p.effectCount||0);t.drawPicture(p.picture);},bytes=>this.layerTextureBytes+=bytes);}
  catch(error){this.deleteLayerTexture(key,entry);this.layerAllocationFailures=(this.layerAllocationFailures||0)+1;throw error;}
  if(this.nativeComposition)promotePresentationEntry(this,entry);
  return entry;
 }
 tileEntry(x0,y0,x1,y1,extra){
  return {...extra,tiles:[],pending:[{x:x0,y:y0,w:x1-x0,h:y1-y0}],bytes:0,complete:false};
 }
 *rasterTiles(entry,draw,account){
  const ck=this.ck;
  while(entry.pending.length){
   // On CPU, a tiny output tile still computes its entire blur input halo.
   // Avoid repeating a large halo for hundreds of tiny outputs. Cancellation
   // still happens between tiles in the independent preparation worker.
   const area=entry.pending.pop(),side=Math.max(this.preparationTileSize||1024,this.gr?0:Math.min(1024,Math.ceil(2*(entry.filterOutset||0)))),tile={x:area.x,y:area.y,w:Math.min(side,area.w),h:Math.min(side,area.h)},width=tile.w+4,height=tile.h+4,bytes=width*height*4;
   if(area.h>tile.h)entry.pending.push({x:area.x,y:area.y+tile.h,w:area.w,h:area.h-tile.h});
   if(area.w>tile.w)entry.pending.push({x:area.x+tile.w,y:area.y,w:area.w-tile.w,h:tile.h});
   if(!this.scratch||this.scratch.width<=width||this.scratch.height<=height){
    // A strictly smaller snapshot is copied out by Skia. It cannot retain the
    // scratch target's MSAA/stencil attachments or trigger copy-on-write on the
    // next tile. Reuse one growing work surface instead of allocating a render
    // target for every differently sized label and boundary tile.
    const workWidth=Math.max(width+1,this.scratch?.width||0),workHeight=Math.max(height+1,this.scratch?.height||0);
    this.releaseScratch();const info={width:workWidth,height:workHeight,colorType:ck.ColorType.RGBA_8888,alphaType:ck.AlphaType.Premul,colorSpace:ck.ColorSpace.SRGB},surface=this.surface.makeSurface(info);if(!surface)throw Error('Cached tile allocation failed');
    this.scratch={surface,width:workWidth,height:workHeight,samples:surface.sampleCnt?.()||0,detached:true};
   }
   const surface=this.scratch.surface,t=surface.getCanvas(),saved=t.getSaveCount();
   try{
    t.save();t.clipRect([0,0,width,height],ck.ClipOp.Intersect,false);t.clear(ck.TRANSPARENT);t.translate(2-tile.x,2-tile.y);const drawing=draw(t);if(drawing)yield* drawing;t.restore();surface.flush();
    const image=surface.makeImageSnapshot([0,0,width,height]);
    // Software presentation can copy an opaque, pixel-aligned tile without
    // blending. Inspect coverage once when creating the cache, never per frame
    // and never through a GPU readback. Other surfaces keep their existing path.
    try{
     const opaque=!this.gr&&!this.nativeComposition&&this.premultipliedRaster&&entry.signature!==undefined&&this.opaqueRasterTile(image,tile.w,tile.h);
     entry.tiles.push({...tile,image,opaque});
    }catch(error){image.delete();throw error;}
   }
   finally{t.restoreToCount(saved);if(!entry.tiles.some(t=>t.x===tile.x&&t.y===tile.y))entry.pending.push(tile);}
   entry.bytes+=bytes;account(bytes);this.textureAllocated=(this.textureAllocated||0)+bytes;this.textureImages=(this.textureImages||0)+1;this.texturePeak=Math.max(this.texturePeak||0,(this.textureAllocated||0)-(this.textureReleased||0));
   if(!entry.pending.length)entry.complete=true;
   this.updateCacheLimit();yield {bytes,pixels:tile.w*tile.h};
  }
 }
 opaqueRasterTile(image,width,height){
  const ck=this.ck,size=width*height;
  if((this.cacheCoverageCapacity||0)<size){
   if(this.cacheCoverageBuffer)ck.Free(this.cacheCoverageBuffer);
   this.cacheCoverageBuffer=null;this.cacheCoverageCapacity=0;
   this.cacheCoverageBuffer=ck.Malloc(Uint8Array,size);this.cacheCoverageCapacity=size;
   this.cacheCoveragePeakBytes=Math.max(this.cacheCoveragePeakBytes||0,size);
  }
  // CanvasKit requires an explicit allocation and row stride for Alpha_8.
  // Reuse one native buffer rather than copying an RGBA array into JavaScript.
  const alpha=image.readPixels(2,2,{width,height,colorType:ck.ColorType.Alpha_8,alphaType:ck.AlphaType.Premul,colorSpace:ck.ColorSpace.SRGB},this.cacheCoverageBuffer,width);
  if(!alpha)return false;
  for(let i=0;i<size;i++)if(alpha[i]!==255)return false;
  return true;
 }
 interruptPreparationWait(){this.preparationWake?.();}
 async yieldPreparationTask(delay){
  if(delay<=0)return yieldRenderTask();
  await new Promise(resolve=>{
   let timer;const wake=()=>{clearTimeout(timer);if(this.preparationWake===wake)this.preparationWake=null;resolve();};
   this.interruptPreparationWait();this.preparationWake=wake;timer=setTimeout(wake,delay);
  });
 }
 async prepareTasks(work,cancelled=()=>false,pause=()=>0){
  let tiles=0,drawMs=0,gpuWaitMs=0,maxTaskMs=0,maxGpuWaitMs=0,tileDrawMs=0,maxTaskStage;const effects=this.effectPictureReplays||0,maskBuilds=this.effectMaskBuilds||0,maskHits=this.effectMaskHits||0;
  let submitted=0,batchStarted=performance.now(),estimatedGpuMs=2;
  const complete=async()=>{if(!submitted)return;const start=performance.now();await this.completeResources();const elapsed=performance.now()-start;gpuWaitMs+=elapsed;maxGpuWaitMs=Math.max(maxGpuWaitMs,elapsed);estimatedGpuMs=Math.max(.25,elapsed/submitted);submitted=0;batchStarted=performance.now();};
  const metrics=()=>({tiles,drawMs,gpuWaitMs,maxTaskMs,maxTaskStage,maxGpuWaitMs,effectPictureReplays:(this.effectPictureReplays||0)-effects,maskBuilds:(this.effectMaskBuilds||0)-maskBuilds,maskHits:(this.effectMaskHits||0)-maskHits,maskWorkingPeakBytes:this.effectMaskPeakBytes||0});
  this.activePreparation=true;
  try{for(;;){
   if(cancelled())return {cancelled:true,...metrics()};
   const start=performance.now(),step=work.next(),taskMs=performance.now()-start;drawMs+=taskMs;tileDrawMs+=taskMs;if(taskMs>maxTaskMs){maxTaskMs=taskMs;maxTaskStage=step.done?'finalize':step.value?.stage||'tile';}
   if(step.done){await complete();return {complete:true,...metrics()};}
   if(step.value?.pixels){tiles++;if(this.gl)submitted++;}
   const delay=Math.max(0,pause());
   // Keep input/cancellation boundaries between tiles, but amortize driver
   // acknowledgement across short submissions instead of fencing every label.
   if(submitted&&(delay>0||performance.now()-batchStarted>=8||submitted*estimatedGpuMs>=8))await complete();
   const cost=tileDrawMs+(this.gl?estimatedGpuMs:0);
   // Completion includes browser/driver acknowledgement latency. Target one
   // bounded submission below a display frame, not a sub-latency four-ms tile
   // (which would converge to tiny textures even for an idle fast GPU).
   if(step.value?.pixels){this.preparationTileSize=Math.round(Math.max(this.gr?256:64,Math.min(1024,Math.sqrt(step.value.pixels*12/Math.max(.25,cost))))/32)*32;tileDrawMs=0;}
   // GPU completion is awaited off the UI thread, then the worker accepts
   // urgent target/cancel messages before submitting the next independent tile.
   await this.yieldPreparationTask(delay);
  }}finally{work.return?.();this.activePreparation=false;this.clearEffectMasks();}
 }
 *sceneTasks(view,width,height,states={}){
  for(const run of this.sceneRuns)if(run.static&&run.cacheable)yield* this.staticTasks(run,view,width,height);
  const sx=width/(view[2]-view[0]),sy=height/(view[3]-view[1]),root=[sx,0,-view[0]*sx,0,sy,-view[1]*sy,0,0,1];
  const multiply=(a,b)=>Array.from({length:9},(_,i)=>{const r=Math.floor(i/3)*3,c=i%3;return a[r]*b[c]+a[r+1]*b[c+3]+a[r+2]*b[c+6];});
  const walk=function*(renderer,nodes,parent,prefix=''){
   for(let i=0;i<nodes.length;i++){
    const node=nodes[i],id=prefix+i,p=renderer.prepare(node),state=states[id],world=multiply(parent,state?.matrix||p.matrix);
    if(renderer.animatedNames.has(node.name)&&!state?.content&&!p.dynamicChildren&&p.picture&&(renderer.nativeComposition||p.hasEffects||node.kind.kind==='group'&&p.cost>=128)){
     const density=projectedDensity(world,p.bounds);if(Number.isFinite(density)&&density>0)yield* renderer.layerTasks(p,density);
    }
    if(node.kind.kind==='group'&&p.dynamicChildren)yield* walk(renderer,node.kind.children,world,id+'/');
   }
  };
  yield* walk(this,this.list.nodes,root);
 }
 async completeResources(){
  // Resource preparation ends at GPU completion, not merely CPU submission.
  // Yield while waiting; no animation is rendered and no pixels are read back.
  if(!this.gl)return;this.surface.flush();const gl=this.gl,fence=gl.fenceSync(gl.SYNC_GPU_COMMANDS_COMPLETE,0);if(!fence)throw Error('GPU resource fence unavailable');gl.flush();const start=performance.now();
  try{for(;;){const status=gl.clientWaitSync(fence,0,0);if(status===gl.ALREADY_SIGNALED||status===gl.CONDITION_SATISFIED)break;if(status===gl.WAIT_FAILED||this.lost)throw Error('GPU resource preparation interrupted');if(performance.now()-start>20000)throw Error('GPU resource preparation timed out');await yieldRenderTask();}}finally{gl.deleteSync(fence);this.resourceWaitMs=performance.now()-start;}
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
  const sx=width/(viewport[2]-viewport[0]),sy=height/(viewport[3]-viewport[1]);
  for(const run of this.sceneRuns){
   if(useCache&&run.static&&run.cacheable&&!run.nodes.some(n=>changedRoots.has(n.id))&&this.drawStaticRun(c,run,viewport,width,height))continue;
   c.save();c.scale(sx,sy);c.translate(-viewport[0],-viewport[1]);
   try{for(const {node,id}of run.nodes)this.drawNode(c,node,id,states);}finally{c.restore();}
  }
 }
 *staticTasks(run,viewport,width,height){
  const sx=width/(viewport[2]-viewport[0]),sy=height/(viewport[3]-viewport[1]),signature=[...viewport,width,height].join(',');
  let entry=this.staticTextures.get(run.key);
  if(entry?.signature!==signature){if(entry)this.deleteStaticTexture(run.key,entry);entry=null;}
  const poolKey=signature+'\0'+(run.contentKey??run.key),shared=this.staticTexturePool.get(poolKey);
  if(shared?.complete&&entry!==shared){
   if(entry)this.deleteStaticTexture(run.key,entry);
   shared.references++;this.staticTextures.set(run.key,shared);this.staticTextureBytes+=shared.bytes;this.staticSharedHits=(this.staticSharedHits||0)+1;this.cacheHits=(this.cacheHits||0)+1;return shared;
  }
  if(!entry){
   const b=run.bounds,x0=Math.max(0,Math.floor((b[0]-viewport[0])*sx)-2),y0=Math.max(0,Math.floor((b[1]-viewport[1])*sy)-2),x1=Math.min(width,Math.ceil((b[2]-viewport[0])*sx)+2),y1=Math.min(height,Math.ceil((b[3]-viewport[1])*sy)+2);
   if(![x0,y0,x1,y1].every(Number.isFinite))return false;if(x1<=x0||y1<=y0)return true;
   const filterOutset=Math.max(0,...run.nodes.map(({node})=>{const p=this.prepare(node);return (p.filterOutset||0)*projectedDensity(p.matrix,p.bounds)*Math.max(sx,sy);}));
   entry=this.tileEntry(x0,y0,x1,y1,{signature,poolKey,references:1,filterOutset,used:this.frameCount});this.staticTextures.set(run.key,entry);this.cacheBuilds=(this.cacheBuilds||0)+1;
  }else if(entry.complete)this.cacheHits=(this.cacheHits||0)+1;
  try{yield* this.rasterTiles(entry,t=>{t.scale(sx,sy);t.translate(-viewport[0],-viewport[1]);if(!this.gr)return this.nodesPreparationTasks(t,run.nodes);for(const {node,id}of run.nodes)this.drawNode(t,node,id,{});},bytes=>this.staticTextureBytes+=bytes);}
  catch(error){this.deleteStaticTexture(run.key,entry);throw error;}
  if(entry.complete)this.staticTexturePool.set(poolKey,entry);
  if(this.nativeComposition)promotePresentationEntry(this,entry);
  return entry;
 }
 drawStaticRun(c,run,viewport,width,height){
  const work=this.staticTasks(run,viewport,width,height);let entry;for(;;){const step=work.next();if(step.done){entry=step.value;break;}}
  if(!entry||entry===true)return entry;
  restorePresentationEntry(this,entry);
  const ck=this.ck;
  // Static tiles already use display pixels. An exact identity matrix and
  // clipped image blit avoid a general resampling pipeline and its source crop.
  entry.used=this.frameCount;c.save();let copyPaint;
  try{for(const {image,x,y,w,h,opaque}of entry.tiles){
   if(opaque&&!copyPaint){copyPaint=new ck.Paint();copyPaint.setBlendMode(ck.BlendMode.Src);}
   c.save();try{c.clipRect([x,y,x+w,y+h],ck.ClipOp.Intersect,false);c.drawImage(image,x-2,y-2,opaque?copyPaint:null);}finally{c.restore();}
   if(opaque)this.opaqueStaticCopies=(this.opaqueStaticCopies||0)+1;
  }}finally{copyPaint?.delete();c.restore();}return true;
 }

 clearCommands(){for(const entry of this.entries){this.dropSurfaceTexture(entry);entry.dead=true;}this.entries.clear();this.geometryCache=new WeakMap();this.contentCache=new WeakMap();for(const object of this.owned.reverse())object.delete();this.owned=[];this.prepared=new WeakMap();this.commandCache.clear();this.pictureBytes=0;}
 content(c,node,id,states){const p=this.prepare(node);if(!p.softPaint)return this.subject(c,node,id,states);const b=!p.dynamicChildren||!Object.keys(states).length?p.subjectBounds:undefined;c.saveLayer(undefined,b);this.subject(c,node,id,states);c.saveLayer(p.softPaint,b);this.subject(c,node,id,states);c.restore();c.restore();}
 subject(c,node,id,states){if(node.hint?.surface3d)return this.surface3d(c,node,id,states);this.rawContent(c,node,id,states);}
 // Surface textures are retained commands, separate from transient frame-owned paints.
 dropSurfaceTexture(p){for(const t of p.surfaceTexture?.tiles||[])t.image.delete();p.surfaceTexture=null;}
 surface3d(c,node,id,states){
  if(this.recordingCommands)return;
  const p=this.prepare(node),base=node.hint.surface3d,ck=this.ck,density=Math.ceil(projectedDensity(c.getTotalMatrix(),base.bounds)*64)/64;
  if(!Number.isFinite(density)||density<=0)throw Error('Invalid three_d rendering scale');
  const tolerance=Math.min(.015,.125/(density*(base.texture_scale||1)));
  if(!p.surfaceGeometry||p.surfaceGeometry.tolerance>tolerance){const at=performance.now();p.surfaceGeometry=JSON.parse(surfaceRefine(JSON.stringify(base),density));this.surfaceMeshBuilds++;this.surfaceGeometryMs+=performance.now()-at;}
  const mesh=p.surfaceGeometry,b=mesh.bounds,sourceDensity=density*(mesh.texture_scale||1);
  if(!p.surfaceTexture||p.surfaceTexture.density!==density||p.surfaceTexture.mesh!==mesh){this.dropSurfaceTexture(p);p.surfaceTexture={density,mesh,tiles:[],bytes:0};}
  const entry=p.surfaceTexture,x0=Math.floor(b[0]*density),y0=Math.floor(b[1]*density),width=Math.ceil(b[2]*density)-x0,height=Math.ceil(b[3]*density)-y0;
  if(width*height>100000000)throw Error('three_d exceeds the 100000000-pixel budget');
  const meshJSON=JSON.stringify({...mesh,coverage:undefined}),cache=this.entries.has(p);
  const draw=t=>c.drawImageRectOptions(t.image,[0,0,t.w,t.h],t.r,ck.FilterMode.Linear,ck.MipmapMode.None);
  // Cull using the canvas clip, including the input expansion of effect layers.
  // Tiles are keyed in object coordinates so panning reuses covered regions.
  const pending=[];for(let y=0;y<height;y+=512)for(let x=0;x<width;x+=512)pending.push([x,y,Math.min(512,width-x),Math.min(512,height-y)]);
  while(pending.length){
   const [x,y,w,h]=pending.pop(),r=[(x0+x)/density,(y0+y)/density,(x0+x+w)/density,(y0+y+h)/density];
   if(c.quickReject(r))continue;
   let tile=entry.tiles.find(t=>t.x===x&&t.y===y&&t.w===w&&t.h===h);
   if(tile){this.surfaceTextureHits++;draw(tile);continue;}
   const source=Array.from(surfaceRegion(meshJSON,new Float64Array(r),sourceDensity));if(!source.length)continue;
   const tw=Math.max(1,Math.round((source[2]-source[0])*sourceDensity)),th=Math.max(1,Math.round((source[3]-source[1])*sourceDensity));
   if(tw*th*32+w*h*132>64*1024*1024){if(w>1||h>1){if(w>=h){const a=Math.floor(w/2);pending.push([x,y,a,h],[x+a,y,w-a,h]);}else{const a=Math.floor(h/2);pending.push([x,y,w,a],[x,y+a,w,h-a]);}continue;}throw Error('three_d texture exceeds the 64 MiB working budget');}
   this.surfaceWorkingPeakBytes=Math.max(this.surfaceWorkingPeakBytes||0,tw*th*32+w*h*132);
   const textureAt=performance.now(),surface=ck.MakeSurface(tw,th);if(!surface)throw Error('three_d texture allocation failed');let texture;
   const info={width:tw,height:th,colorType:ck.ColorType.RGBA_8888,alphaType:ck.AlphaType.Unpremul,colorSpace:ck.ColorSpace.SRGB};
   try{const tc=surface.getCanvas();tc.clear(ck.TRANSPARENT);tc.scale(sourceDensity,sourceDensity);tc.translate(-source[0],-source[1]);this.rawContent(tc,node,id,states);texture=tc.readPixels(0,0,info);if(!texture)throw Error('three_d texture read failed');
    if(mesh.coverage){tc.clear(ck.TRANSPARENT);this.rawContent(tc,mesh.coverage,id,{});const coverage=tc.readPixels(0,0,info);if(!coverage)throw Error('three_d coverage read failed');texture=surfaceMaterial(texture,coverage,tw,th,mesh.material_alpha);}
   }finally{surface.dispose();}
   this.surfaceTextureMs+=performance.now()-textureAt;const rasterAt=performance.now();
   const rgba=surfaceRaster(JSON.stringify({...mesh,coverage:undefined,source}),texture,tw,th,new Float64Array(r),w,h),image=ck.MakeImage({width:w,height:h,colorType:ck.ColorType.RGBA_8888,alphaType:ck.AlphaType.Unpremul,colorSpace:ck.ColorSpace.SRGB},rgba,w*4);if(!image)throw Error('three_d image upload failed');
   this.surfaceRasterMs+=performance.now()-rasterAt;tile={image,r,w,h,x,y};this.surfaceTextureBuilds++;
   let used=[...this.entries].reduce((n,e)=>n+(e.surfaceTexture?.bytes||0),0);
   for(const e of this.entries){if(used+w*h*4<=256*1024*1024)break;if(e!==p&&e.surfaceTexture){used-=e.surfaceTexture.bytes;this.dropSurfaceTexture(e);}}
   const keep=cache&&used+w*h*4<=256*1024*1024;
   if(keep){entry.tiles.push(tile);entry.bytes+=w*h*4;}
   try{draw(tile);}finally{if(!keep)image.delete();}
  }
 }

 rawContent(c,node,id,states){
  const ck=this.ck,k=node.kind,p=this.prepare(node);
  if(k.kind==='path'){if(p.imageFill){c.save();c.clipPath(p.path,ck.ClipOp.Intersect,true);c.concat(matrix(k.fill.transform));this.drawNode(c,k.fill.nodes[0],`${id}/image-fill`,states);c.restore();}
   else if(p.directPattern){c.save();c.clipPath(p.path,ck.ClipOp.Intersect,true);const layer=k.fill.nodes.length>1;if(layer)c.saveLayer(undefined,p.bounds);c.concat(matrix(k.fill.transform));c.clipRect([0,0,k.fill.width,k.fill.height],ck.ClipOp.Intersect,true);k.fill.nodes.forEach((node,i)=>this.drawNode(c,node,`${id}/pattern/${i}`,states));if(layer)c.restore();c.restore();}
   else if(p.fill)c.drawPath(p.path,p.fill);if(p.stroke)c.drawPath(p.path,p.stroke);}
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
  // Cull the complete static object before allocating any effect layers. The
  // bounds include shadows, reflections and glow; animated descendants keep
  // their existing traversal because they may move beyond the static bounds.
  if((!p.dynamicChildren||!Object.keys(states).length)&&c.quickReject(p.bounds)){c.restore();return;}
  if(this.cacheLayers&&(contentOnly||state&&!state.content||!state&&this.animatedNames.has(node.name))&&!p.dynamicChildren&&p.picture&&(p.hasEffects||node.kind.kind==='group'&&p.cost>=128)&&this.drawLayer(c,node,id,p,opacity)){c.restore();return;}
  const ck=this.ck,layer=opacity<1||m[6]!==0||m[7]!==0;
  if(layer){const paint=new ck.Paint();paint.setAlphaf(Math.max(0,Math.min(1,opacity)));c.saveLayer(paint,p.dynamicChildren?undefined:p.bounds);paint.delete();}
  if(!this.transient&&!state?.content&&!p.dynamicChildren&&!p.picture){
   const recorder=new ck.PictureRecorder(),recording=recorder.beginRecording(p.bounds);
   this.localContent(recording,node,id,{});const picture=recorder.finishRecordingAsPicture();recorder.delete();
   const bytes=picture.approximateBytesUsed();p.picture=this.own(picture);(p.canonical||p).objects.push(picture);this.pictureBytes+=bytes;
  }
  if(p.picture&&!state?.content&&(this.gr||!p.cpuMask||this.recordingCommands)){if(!this.recordingCommands)this.effectPictureReplays=(this.effectPictureReplays||0)+(p.effectCount||0);c.drawPicture(p.picture);}else this.localContent(c,node,id,states);
  if(layer)c.restore();c.restore();
 }
 clearEffectMasks(){if(this.cacheCoverageBuffer)this.ck.Free(this.cacheCoverageBuffer);this.cacheCoverageBuffer=null;this.cacheCoverageCapacity=0;for(const value of this.effectMasks?.values()||[]){for(const paint of value.paints.values())paint.delete();value.image.delete();}this.effectMasks?.clear();this.effectMaskBytes=0;for(const path of this.effectPaths?.values()||[])path?.delete();this.effectPaths?.clear();for(const paint of this.effectPathPaints?.values()||[])paint.delete();this.effectPathPaints?.clear();}
 drawPathShadow(c,effect){
  if(this.gr||this.recordingCommands||!effect.source||!effect.spec||effect.inner)return false;
  const m=c.getTotalMatrix(),sx=m[0],sy=m[4],e=effect.spec,ck=this.ck;
  if(sx<=0||sy<=0||m[1]||m[3]||m[6]||m[7]||Math.abs(sx-sy)>1e-6*Math.max(sx,sy))return false;
  let path=effect.source;
  if(e.spread){
   const dx=Math.sign(e.spread)*Math.min(256,Math.round(Math.abs(e.spread)*sx))/sx,dy=Math.sign(e.spread)*Math.min(256,Math.round(Math.abs(e.spread)*sy))/sy,key=JSON.stringify([effect.sourceKey,dx,dy]);
   this.effectPaths??=new Map();
   if(!this.effectPaths.has(key)){
    const elements=spreadRoundedRectPath(e.sourcePath,Array.from(path.getBounds()),dx,dy);let shifted=null;
    if(elements){const builder=new ck.PathBuilder();try{for(const el of elements){if(el==='ClosePath')builder.close();else if(el.MoveTo)builder.moveTo(...point(el.MoveTo));else if(el.LineTo)builder.lineTo(...point(el.LineTo));else builder.cubicTo(...el.CurveTo.flatMap(point));}shifted=builder.detach();}finally{builder.delete();}}
    this.effectPaths.set(key,shifted);this.pathShadowBuilds=(this.pathShadowBuilds||0)+1;
   }else this.pathShadowHits=(this.pathShadowHits||0)+1;
   path=this.effectPaths.get(key);if(!path)return false;
  }
  this.effectPathPaints??=new Map();let paint=this.effectPathPaints.get(effect);
  if(!paint){paint=new ck.Paint();paint.setAntiAlias(true);paint.setColor(this.color(e.rgba));if(e.blur){const blur=ck.MaskFilter.MakeBlur(ck.BlurStyle.Normal,e.blur,true);paint.setMaskFilter(blur);blur.delete();}this.effectPathPaints.set(effect,paint);}
  c.save();try{if(effect.knockout)c.clipPath(effect.source,ck.ClipOp.Difference,true);c.translate(...e.offset);c.drawPath(path,paint);}finally{c.restore();}
  this.pathShadowDraws=(this.pathShadowDraws||0)+1;this.effectDraws=(this.effectDraws||0)+1;return true;
 }
 cpuShadowPaint(c,effect){
  if(this.gr||this.recordingCommands||!effect.source||!effect.spec?.spread||effect.inner)return null;
  const m=c.getTotalMatrix(),sx=m[0],sy=m[4];if(sx<=0||sy<=0||m[1]||m[3]||m[6]||m[7])return null;
  const ck=this.ck,e=effect.spec,rx=Math.min(256,Math.round(Math.abs(e.spread)*sx)),ry=Math.min(256,Math.round(Math.abs(e.spread)*sy));
  const fx=m[2]-Math.floor(m[2]),fy=m[5]-Math.floor(m[5]),key=JSON.stringify([effect.sourceKey,sx,sy,fx,fy,e.spread]);this.effectMasks??=new Map();let mask=this.effectMasks.get(key);
  if(!mask){
   const b=effect.source.getBounds(),x=Math.floor(b[0]*sx+fx)-rx-1,y=Math.floor(b[1]*sy+fy)-ry-1,w=Math.ceil(b[2]*sx+fx)+rx+1-x,h=Math.ceil(b[3]*sy+fy)+ry+1-y;
   // This fast path uses a complete source mask. Large masks keep the existing
   // clipped Skia path instead of adding a large temporary allocation.
   if(!Number.isSafeInteger(w*h)||w*h*16>64*1024*1024)return null;
   const surface=ck.MakeSurface(w,h);if(!surface)return null;let image,coverage;
   try{const t=surface.getCanvas();t.clear(ck.TRANSPARENT);t.concat([sx,0,fx-x,0,sy,fy-y,0,0,1]);t.drawPath(effect.source,effect.fill);
    const info={width:w,height:h,colorType:ck.ColorType.Alpha_8,alphaType:ck.AlphaType.Premul,colorSpace:ck.ColorSpace.SRGB};coverage=ck.Malloc(Uint8Array,w*h);
    const alpha=t.readPixels(0,0,info,coverage,w);if(!alpha)return null;
    image=ck.MakeImage(info,e.spread?spreadAlpha(alpha,w,h,rx,ry,e.spread>0):alpha,w);if(!image)return null;
   }finally{if(coverage)ck.Free(coverage);surface.dispose();}
   mask={image,w,h,bounds:[(x-fx)/sx,(y-fy)/sy,(x+w-fx)/sx,(y+h-fy)/sy],paints:new Map()};this.effectMasks.set(key,mask);this.effectMaskBuilds=(this.effectMaskBuilds||0)+1;this.effectMaskBytes=(this.effectMaskBytes||0)+w*h;this.effectMaskPeakBytes=Math.max(this.effectMaskPeakBytes||0,this.effectMaskBytes+w*h*8+4*(Math.max(w+2*rx,h+2*ry)+1));
  }else this.effectMaskHits=(this.effectMaskHits||0)+1;
  let paint=mask.paints.get(effect);if(!paint){const input=ck.ImageFilter.MakeImage(mask.image,{filter:ck.FilterMode.Nearest,mipmap:ck.MipmapMode.None},[0,0,mask.w,mask.h],mask.bounds),filter=ck.ImageFilter.MakeDropShadowOnly(e.offset[0],e.offset[1],e.blur,e.blur,this.color(e.rgba),input);input.delete();paint=new ck.Paint();paint.setImageFilter(filter);filter.delete();mask.paints.set(effect,paint);}return paint;
 }
 drawEffect(c,node,id,states,p,effect){
  if(effect.reflection){const r=effect.reflection,b=p.subjectBounds,y=b[3]+r.distance,h=(b[3]-b[1])*r.length;if(h<=0||r.opacity<=0)return;c.saveLayer(effect.paint,[b[0],y,b[2],y+h]);c.clipRect([b[0],y,b[2],y+h],this.ck.ClipOp.Intersect,false);c.save();c.translate(0,y+b[3]);c.scale(1,-1);this.content(c,node,id,states);c.restore();c.drawPaint(effect.mask);c.restore();return;}
  if(this.drawPathShadow(c,effect))return;

  c.save();if(effect.knockout&&effect.source)c.clipPath(effect.source,this.ck.ClipOp.Difference,true);
  this.effectDraws=(this.effectDraws||0)+1;
  c.saveLayer(this.cpuShadowPaint(c,effect)||effect.paint,p.dynamicChildren&&Object.keys(states).length?undefined:effect.bounds||p.bounds);
  if(effect.source)c.drawPath(effect.source,effect.fill);else this.content(c,node,id,states);
  c.restore();c.restore();
 }
 localContent(c,node,id,states){const p=this.prepare(node);
  for(const effect of p.effects)if(!effect.inner)this.drawEffect(c,node,id,states,p,effect);
  if(!node.hint?.effectsOnly)this.content(c,node,id,states);
  for(const effect of p.effects)if(effect.inner)this.drawEffect(c,node,id,states,p,effect);
 }
 // Resume CPU preparation between independent effects and children. A tile is
 // published only after all its steps finish; cancellation unwinds every saved
 // canvas and requeues the incomplete tile without retaining a partial image.
 *nodesPreparationTasks(c,nodes){
  for(const {node,id}of nodes){
   if(node.opacity<=0)continue;
   const p=this.prepare(node);if(!p.hasEffects){this.drawNode(c,node,id,{});yield {stage:'content'};continue;}
   const saved=c.getSaveCount();c.save();c.concat(p.matrix);
   try{
    if(c.quickReject(p.bounds))continue;
    if(node.opacity<1||p.matrix[6]||p.matrix[7]){const paint=new this.ck.Paint();paint.setAlphaf(node.opacity);c.saveLayer(paint,p.bounds);paint.delete();}
    yield* this.contentPreparationTasks(c,node,id);
   }finally{c.restoreToCount(saved);}
  }
 }
 *contentPreparationTasks(c,node,id){
  const p=this.prepare(node),k=node.kind;
  for(const effect of p.effects)if(!effect.inner){this.drawEffect(c,node,id,{},p,effect);yield {stage:'effect'};}
  if(!node.hint?.effectsOnly){
   if(k.kind==='group'&&!k.mask&&!p.softPaint&&!node.hint?.surface3d&&!node.hint?.animationRevealWrapper){
    const saved=c.getSaveCount();c.save();if(p.clip)c.clipPath(p.clip,this.ck.ClipOp.Intersect,true);
    try{yield* this.nodesPreparationTasks(c,k.children.map((node,i)=>({node,id:`${id}/${i}`})));}finally{c.restoreToCount(saved);}
   }else{this.content(c,node,id,{});yield {stage:'content'};}
  }
  for(const effect of p.effects)if(effect.inner){this.drawEffect(c,node,id,{},p,effect);yield {stage:'effect'};}
 }
 ensureSurface(width,height){
  const ck=this.ck;
  if(!this.surface||this.canvas.width!==width||this.canvas.height!==height){
   // Tiles belong to the context, not the onscreen surface. Static signatures
   // and projected layer density decide reuse after resize; resizing a surface
   // must not discard otherwise valid content (including another resident page).
   this.releaseScratch();this.surface?.dispose();this.canvas.width=width;this.canvas.height=height;
   this.surface=this.gr?ck.MakeOnScreenGLSurface(this.gr,width,height,ck.ColorSpace.SRGB):this.premultipliedRaster?makePresentationCPUSurface(ck,width,height,this.rasterOnly?null:this.canvas):this.rasterOnly?ck.MakeSurface(width,height):ck.MakeSWCanvasSurface(this.canvas);
   if(!this.surface)throw Error('Skia surface allocation failed');
  }
 }
 render(viewport,width,height,states={},generation=this.generation,allowSkip=false,cacheStatic=allowSkip,cacheLayers=allowSkip,preparing=false,effectsOnly=null,present=true){
  if(generation!==this.generation)return false;
  if(this.lost)return {backend:'Skia restoring',skipped:true,rasterMs:0};
  if(!Number.isSafeInteger(width)||!Number.isSafeInteger(height)||width<=0||height<=0||!Number.isSafeInteger(width*height*4))throw Error('Invalid Skia viewport');
  const start=performance.now(),ck=this.ck;
  this.cacheViewportPixels=width*height;
  // At most two animation draws may be in flight on the GPU. Worker replies
  // acknowledge CPU submission; they must not allow a queue of stale GPU frames.
  while(this.fences.length&&this.gl.clientWaitSync(this.fences[0],0,0)!==this.gl.TIMEOUT_EXPIRED)this.gl.deleteSync(this.fences.shift());
  if(allowSkip&&!preparing&&this.fences.length>=2)return {skipped:true,reason:'gpu-busy',backend:this.backend,rasterMs:0};
  this.ensureSurface(width,height);
  const gl=this.gl,timer=this.timer;
  const gpuSamples=[];while(this.queries.length&&gl.getQueryParameter(this.queries[0],gl.QUERY_RESULT_AVAILABLE)){const query=this.queries.shift();if(!gl.getParameter(timer.GPU_DISJOINT_EXT)){this.gpuMs=gl.getQueryParameter(query,gl.QUERY_RESULT)/1e6;gpuSamples.push(this.gpuMs);}gl.deleteQuery(query);}
  const query=timer&&this.queries.length<8?gl.createQuery():null;if(query)gl.beginQuery(timer.TIME_ELAPSED_EXT,query);
  this.cpuLayerBlits=0;this.cpuTileConversions=0;this.opaqueStaticCopies=0;this.surfaceGeometryMs=0;this.surfaceTextureMs=0;this.surfaceRasterMs=0;this.surfaceWorkingPeakBytes=0;this.surfaceMeshBuilds=0;this.surfaceTextureHits=0;this.surfaceTextureBuilds=0;this.effectDraws=0;this.effectPictureReplays=0;this.cacheLayers=cacheLayers;this.layerHits=0;this.layerBuilds=0;this.layerBudgetMisses=0;this.layerAllocationFailures=0;this.cacheHits=0;this.cacheBuilds=0;this.transient=true;this.transientPrepared=new WeakMap();const count=this.owned.length;let c,saved;try{
  // A fitted presentation viewport includes the area around the page. Clip
  // before subjects, effects and cached layers; their bounds can exceed it.
  this.pageBounds=this.clipToPage?[-viewport[0]*width/(viewport[2]-viewport[0]),-viewport[1]*height/(viewport[3]-viewport[1]),(this.list.width-viewport[0])*width/(viewport[2]-viewport[0]),(this.list.height-viewport[1])*height/(viewport[3]-viewport[1])]:null;
  this.opaqueRect=effectsOnly===null?opaquePageRect(this.list,states,viewport,width,height):null;
  const native=effectsOnly===null&&cacheStatic&&cacheLayers&&composePresentationCanvas(this,viewport,width,height,states,present);
  if(!native){c=this.surface.getCanvas();saved=c.getSaveCount();c.clear(ck.TRANSPARENT);c.save();
  if(this.clipToPage)c.clipRect(this.pageBounds,ck.ClipOp.Intersect,false);
  if(effectsOnly===null)this.drawScene(c,viewport,width,height,states,cacheStatic);
  else { // Static PDF export: only the requested effect layers, never the vector subject.
   c.scale(width/(viewport[2]-viewport[0]),height/(viewport[3]-viewport[1]));c.translate(-viewport[0],-viewport[1]);
   const node=this.list.nodes[0],p=this.prepare(node);
   for(const effect of p.effects)if(effect.inner===effectsOnly)this.drawEffect(c,node,'0',{},p,effect);
  }
  c.restore();}if(!present&&this.surface.flushDrawing)this.surface.flushDrawing();else this.surface.flush();}finally{if(c)c.restoreToCount(saved);if(query){gl.endQuery(timer.TIME_ELAPSED_EXT);this.queries.push(query);}
  // Geometry extensions own temporary commands for this frame only.
  while(this.owned.length>count)this.owned.pop().delete();
  this.transient=false;this.cacheLayers=false;this.clearEffectMasks();}if(gl){const fence=gl.fenceSync(gl.SYNC_GPU_COMMANDS_COMPLETE,0);if(fence)this.fences.push(fence);gl.flush();}this.frameCount++;if(present)this.onPresent();return {cpuLayerBlits:this.cpuLayerBlits,cpuTileConversions:this.cpuTileConversions,effectDraws:this.effectDraws,effectPictureReplays:this.effectPictureReplays,resourceStats:{nativeFallbackReason:this.nativeFallbackReason,pathShadowBuilds:this.pathShadowBuilds||0,pathShadowHits:this.pathShadowHits||0,pathShadowDraws:this.pathShadowDraws||0,nativePromotionMs:this.nativePromotionMs||0,nativeReadbackMs:this.nativeReadbackMs||0,nativeUploadMs:this.nativeUploadMs||0,nativeBitmapBytes:this.nativeBitmapBytes||0,nativeBitmapCount:this.nativeBitmapCount||0,nativePromotions:this.nativePromotions||0,nativeDemotions:this.nativeDemotions||0,nativeCompositions:this.nativeCompositions||0,nativeWorkingPeakBytes:this.nativeWorkingPeakBytes||0,rasterBufferBytes:this.surface?.rasterBufferBytes||0,cpuTileBytes:this.cpuTileBytes||0,cpuTilePromotions:this.cpuTilePromotions||0,cpuTileDemotions:this.cpuTileDemotions||0,cpuTileWorkingPeakBytes:this.cpuTileWorkingPeakBytes||0,cacheCoveragePeakBytes:this.cacheCoveragePeakBytes||0,opaqueStaticCopies:this.opaqueStaticCopies||0,presentationBufferBytes:this.surface?.presentationBufferBytes||0,maskBuilds:this.effectMaskBuilds||0,maskHits:this.effectMaskHits||0,maskWorkingPeakBytes:this.effectMaskPeakBytes||0,sharedStaticEntries:this.staticTexturePool.size,staticSharedHits:this.staticSharedHits||0,imageDecodes:this.imageDecodes,imageReuses:this.imageReuses,uniqueImages:[...this.imagePool.values()].reduce((n,p)=>n+p.length,0),encodedImageBytes:[...this.imagePool.values()].flat().reduce((n,p)=>n+p.bytes.byteLength,0),allocatedTextureBytes:this.textureAllocated||0,releasedTextureBytes:this.textureReleased||0,liveTextureBytes:(this.textureAllocated||0)-(this.textureReleased||0),peakTextureBytes:this.texturePeak||0,liveTextureImages:this.textureImages||0,scratchSamples:this.scratch?.samples||0,retainedSamples:this.scratch?.detached?1:0,scratchBytes:this.scratch?this.scratch.width*this.scratch.height*4*Math.max(1,this.scratch.samples):0},surfaceGeometryMs:this.surfaceGeometryMs,surfaceTextureMs:this.surfaceTextureMs,surfaceRasterMs:this.surfaceRasterMs,surfaceWorkingPeakBytes:this.surfaceWorkingPeakBytes,surfaceMeshBuilds:this.surfaceMeshBuilds,surfaceTextureHits:this.surfaceTextureHits,surfaceTextureBuilds:this.surfaceTextureBuilds,surfaceTextureBytes:[...this.entries].reduce((n,e)=>n+(e.surfaceTexture?.bytes||0),0),backend:this.backend,device:this.device,softwareGpuBypass:this.softwareGpuBypass,backendProbeMs:this.backendProbeMs,rasterMs:performance.now()-start,gpuMs:this.gpuMs,gpuSamples,cacheBytes:this.gpuResourceBytes(),staticTextureBytes:this.staticTextureBytes,staticCacheHits:this.cacheHits,staticCacheBuilds:this.cacheBuilds,layerTextureBytes:this.layerTextureBytes,layerCacheHits:this.layerHits,layerCacheBuilds:this.layerBuilds,layerCacheBudgetMisses:this.layerBudgetMisses,layerCacheAllocationFailures:this.layerAllocationFailures,frameCount:this.frameCount};
 }
 // Explicit readback for static export and test oracles; playback never calls it.
 readPixels(){const image=this.surface.makeImageSnapshot();try{return image.readPixels(0,0,{width:this.canvas.width,height:this.canvas.height,colorType:this.ck.ColorType.RGBA_8888,alphaType:this.ck.AlphaType.Unpremul,colorSpace:this.ck.ColorSpace.SRGB});}finally{image.delete();}}
 trim(){for(const p of this.entries)this.dropSurfaceTexture(p);this.clearStaticTextures();this.clearLayerTextures();if(this.gr){this.gr.setResourceCacheLimitBytes(0);this.updateCacheLimit();}}
 dispose(){this.interruptPreparationWait();this.clearEffectMasks();this.transitions.dispose();this.canvas.removeEventListener?.('webglcontextlost',this.onLost);this.canvas.removeEventListener?.('webglcontextrestored',this.onRestored);this.queries.forEach(q=>this.gl?.deleteQuery(q));this.queries=[];this.fences.forEach(f=>this.gl?.deleteSync(f));this.fences=[];this.clearStaticTextures();this.clearLayerTextures();this.clearCommands();for(const image of this.images.values())this.releaseImage(image);this.images.clear();this.imageBytes.clear();this.releaseScratch();this.surface?.dispose();this.surface=null;this.gr?.releaseResourcesAndAbandonContext();this.gr?.delete();this.gr=null;if(this.context)this.ck.deleteContext(this.context);
  // CanvasKit's deleteContext only removes its JS registry entry. Release the
  // browser's display backing store as well, including the CPU canvas pixels.
  // This renderer is disposed; no retained page can use the context again.
  this.gl?.getExtension('WEBGL_lose_context')?.loseContext();this.gl=null;this.context=0;this.canvas.width=1;this.canvas.height=1;
 }
}
