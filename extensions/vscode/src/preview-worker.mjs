import {initializeSkia,SkiaRenderer,setSurfaceRasterizer} from './skia-renderer.mjs';
import {applyDisplayUpdate} from './display-update.mjs';
// The packaging script inlines the generated WASM module here: VS Code
// requires workers loaded from a blob, without external module imports.
import init,{prepare_display_list,register_render_resource,retain_render_resources,raster_surface3d,surface_texture_region,refine_surface3d,surface_material_texture} from './wasm/laymesh_wasm.js';
let installedList,skia, rasterInitialized,rasterJob,rasterJobId='',rasterGeneration,rasterMemory,rasterPage,overviewReady=false,rasterChain=Promise.resolve();
const rasterResources=new Set();
let objectKeys,selectableKeys,selectionContext;
const objectBoundsCache=new WeakMap(),selectionPathBoundsCache=new WeakMap(),selectionPathCache=new WeakMap();

const identityTransform=[1,0,0,1,0,0];
function selectionTransform(parent,child){
 const [a,b,c,d,e,f]=parent,[g,h,i,j,k,l]=child;
 return [a*g+c*h,b*g+d*h,a*i+c*j,b*i+d*j,a*k+c*l+e,b*k+d*l+f];
}
function selectionBoundsTransform(bounds,transform){
 const [x0,y0,x1,y1]=bounds,[a,b,c,d,e,f]=transform;
 return [Math.min(a*x0,a*x1)+Math.min(c*y0,c*y1)+e,Math.min(b*x0,b*x1)+Math.min(d*y0,d*y1)+f,
  Math.max(a*x0,a*x1)+Math.max(c*y0,c*y1)+e,Math.max(b*x0,b*x1)+Math.max(d*y0,d*y1)+f];
}
const selectionAnchorNames=[
 'top_left','top_center','top_right','middle_left','center','middle_right',
 'bottom_left','bottom_center','bottom_right',
];
function selectionProjectAnchor(name,x,y,transform){
 const [a,b,c,d,e,f]=transform;
 return {name,x:a*x+c*y+e,y:b*x+d*y+f};
}
function selectionBoxAnchors(bounds,transform,prefix=''){
 const [x0,y0,x1,y1]=bounds;
 const xs=[x0,(x0+x1)/2,x1],ys=[y0,(y0+y1)/2,y1];
 return selectionAnchorNames.map((name,index)=>
  selectionProjectAnchor(prefix+name,xs[index%3],ys[Math.floor(index/3)],transform));
}
function selectionAnchors(node,parent){
 const {anchorSize,anchorIntrinsicInverse,anchorEndpoints,anchorPlot}=node.hint||{};
 if(!anchorSize)return [];
 const [width,height]=anchorSize;
 const layout=anchorIntrinsicInverse?selectionTransform(node.transform,anchorIntrinsicInverse):node.transform;
 // Standard anchors follow the layout bounding box in the immediate container.
 const anchors=selectionBoxAnchors(selectionBoundsTransform([0,0,width,height],layout),parent);
 const toPage=selectionTransform(parent,layout);
 if(anchorEndpoints){
  const [[startX,startY],[endX,endY]]=anchorEndpoints;
  anchors.push(selectionProjectAnchor('start',startX,startY,toPage),selectionProjectAnchor('end',endX,endY,toPage));
 }
 if(anchorPlot)anchors.push(...selectionBoxAnchors(anchorPlot,toPage,'plot_'));
 return anchors;
}
function selectionUnion(a,b){return !a?b:!b?a:[Math.min(a[0],b[0]),Math.min(a[1],b[1]),Math.max(a[2],b[2]),Math.max(a[3],b[3])];}
function selectionIntersect(a,b){
 if(!a||!b)return null;
 const bounds=[Math.max(a[0],b[0]),Math.max(a[1],b[1]),Math.min(a[2],b[2]),Math.min(a[3],b[3])];
 return bounds[2]>=bounds[0]&&bounds[3]>=bounds[1]?bounds:null;
}
function selectionContains(bounds,x,y,tolerance=0){
 return bounds&&x>=bounds[0]-tolerance&&x<=bounds[2]+tolerance&&y>=bounds[1]-tolerance&&y<=bounds[3]+tolerance;
}
const selectionPoint=point=>Array.isArray(point)?point:[point.x,point.y];
function selectionPathBounds(elements){
 if(selectionPathBoundsCache.has(elements))return selectionPathBoundsCache.get(elements);
 let x0=Infinity,y0=Infinity,x1=-Infinity,y1=-Infinity;
 for(const command of elements){
  const endpoint=command.MoveTo||command.LineTo;
  for(const point of endpoint?[endpoint]:command.QuadTo||command.CurveTo||[]){
   const [x,y]=selectionPoint(point);
   x0=Math.min(x0,x);y0=Math.min(y0,y);x1=Math.max(x1,x);y1=Math.max(y1,y);
  }
 }
 const bounds=Number.isFinite(x0)?[x0,y0,x1,y1]:null;
 selectionPathBoundsCache.set(elements,bounds);
 return bounds;
}
function selectionPath(elements){
 let shape=selectionPathCache.get(elements);
 if(shape)return shape;
 shape=new Path2D();
 for(const command of elements){
  if(command.MoveTo)shape.moveTo(...selectionPoint(command.MoveTo));
  else if(command.LineTo)shape.lineTo(...selectionPoint(command.LineTo));
  else if(command.QuadTo)shape.quadraticCurveTo(...command.QuadTo.flatMap(selectionPoint));
  else if(command.CurveTo)shape.bezierCurveTo(...command.CurveTo.flatMap(selectionPoint));
  else if(command==='ClosePath')shape.closePath();
 }
 selectionPathCache.set(elements,shape);
 return shape;
}
function selectionNodeBounds(node){
 if(objectBoundsCache.has(node))return objectBoundsCache.get(node);
 const kind=node.kind;
 let bounds=null;
 if(node.opacity>0){
  if(node.hint?.surface3d?.bounds)bounds=node.hint.surface3d.bounds;
  else if(kind.kind==='image'){
   const r=kind.rect;bounds=Array.isArray(r)?r:[r.x0,r.y0,r.x1,r.y1];
  }else if(kind.kind==='glyphs'&&kind.color[3]>0){
   for(const path of kind.outlines)bounds=selectionUnion(bounds,selectionPathBounds(path));
  }else if(kind.kind==='path'&&(selectionHasFill(kind)||kind.stroke?.rgba[3]>0)){
   bounds=selectionPathBounds(kind.path);
   if(bounds&&kind.stroke?.rgba[3]>0){
    const pad=kind.stroke.width/2*Math.max(1,kind.stroke.miter);
    bounds=[bounds[0]-pad,bounds[1]-pad,bounds[2]+pad,bounds[3]+pad];
   }
  }else if(kind.kind==='group'){
   for(const child of kind.children){const childBounds=selectionNodeBounds(child);if(childBounds)bounds=selectionUnion(bounds,selectionBoundsTransform(childBounds,child.transform));}
   if(kind.clip)bounds=selectionIntersect(bounds,selectionPathBounds(kind.clip.path));
   if(kind.mask){const n=kind.mask.node,b=selectionNodeBounds(n);bounds=selectionIntersect(bounds,b&&selectionBoundsTransform(b,n.transform));}
  }
 }
 objectBoundsCache.set(node,bounds);
 return bounds;
}
function selectionHasFill(kind){return kind.fill.kind!=='none'&&(kind.fill.kind!=='solid'||kind.fill.rgba[3]>0);}
function selectionContainsPath(elements,evenOdd,x,y){
 return selectionContains(selectionPathBounds(elements),x,y)&&(!selectionContext||selectionContext.isPointInPath(selectionPath(elements),x,y,evenOdd?'evenodd':'nonzero'));
}
function selectionHitsPath(kind,x,y,tolerance){
 if(selectionHasFill(kind)&&selectionContainsPath(kind.path,kind.even_odd,x,y))return true;
 const stroke=kind.stroke;
 if(!stroke||stroke.rgba[3]<=0)return false;
 if(!selectionContext)return true;
 selectionContext.lineWidth=Math.max(stroke.width,2*tolerance);
 selectionContext.lineCap=stroke.cap;
 selectionContext.lineJoin=stroke.join;
 selectionContext.miterLimit=stroke.miter;
 selectionContext.setLineDash(stroke.dash);
 selectionContext.lineDashOffset=stroke.offset;
 return selectionContext.isPointInStroke(selectionPath(kind.path),x,y);
}
function previewObjectHits(point,scale){
 if(!objectKeys?.length)return [];
 selectableKeys??=new Set(objectKeys);
 if(selectionContext===undefined)selectionContext=typeof OffscreenCanvas==='function'&&typeof Path2D==='function'?new OffscreenCanvas(1,1).getContext('2d'):null;
 const hits=[],tolerance=3/scale;
 const [x,y]=point;
 function visit(node,parent,ancestorClip,collect=true){
  const bounds=selectionNodeBounds(node);
  if(!bounds)return false;
  const transform=selectionTransform(parent,node.transform),[a,b,c,d,e,f]=transform,det=a*d-b*c;
  if(!Number.isFinite(det)||Math.abs(det)<1e-20)return false;
  const px=(d*(x-e)-c*(y-f))/det,py=(a*(y-f)-b*(x-e))/det;
  const kind=node.kind,group=kind.kind==='group'&&!node.hint?.surface3d;
  const padding=(group||kind.kind==='path')?tolerance*Math.max(Math.hypot(d,c),Math.hypot(a,b))/Math.abs(det):0;
  if(!selectionContains(bounds,px,py,padding))return false;
  let hit=true,clip=ancestorClip;
  if(group){
   if(kind.clip){
    if(!selectionContainsPath(kind.clip.path,kind.clip.even_odd,px,py))return false;
    clip=selectionIntersect(clip,selectionBoundsTransform(selectionPathBounds(kind.clip.path),transform));
   }
   if(kind.mask&&!visit(kind.mask.node,transform,clip,false))return false;
   hit=false;
   for(let i=kind.children.length-1;i>=0;i--){
    if(visit(kind.children[i],transform,clip,collect)){hit=true;if(!collect)break;}
   }
  }else if(kind.kind==='path'&&!node.hint?.surface3d)hit=selectionHitsPath(kind,px,py,padding);
  const name=node.name,identity=node.hint?._identity;
  if(hit&&collect&&(identity||node.hint?.logicalKind&&name)){
   const key=identity?`id:${identity}`:`name:${name}`;
   if(selectableKeys.has(key)){
    const outline=selectionIntersect(selectionBoundsTransform(bounds,transform),clip);
    if(outline)hits.push({key,name,kind:node.hint.kind||node.hint.logicalKind,bounds:outline,anchors:selectionAnchors(node,parent)});
   }
  }
  return hit;
 }
 const page=[0,0,installedList.width,installedList.height];
 if(selectionContains(page,x,y))for(let i=installedList.nodes.length-1;i>=0;i--)visit(installedList.nodes[i],identityTransform,page);
 return hits;
}
self.onmessage=({data})=>{
 if(['install','cancel-transition'].includes(data.type))skia?.transitions.cancel();
 if(data.type==='cancel-transition')return;
 if(data.type==='init'){rasterInitialized=Promise.all([init({module_or_path:data.wasmBytes}),initializeSkia(data.skiaBytes)]).then(([exports,ck])=>{rasterMemory=exports.memory;setSurfaceRasterizer(raster_surface3d,surface_texture_region,refine_surface3d,surface_material_texture);if(data.canvas)skia=new SkiaRenderer(ck,data.canvas,{onContextChange:ready=>self.postMessage({type:'context',ready})});self.postMessage({requestId:data.requestId,type:'ready'});});rasterInitialized.catch(error=>self.postMessage({type:'error',requestId:data.requestId,error:String(error)}));return;}
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
    objectKeys=data.objectKeys;
    selectableKeys=undefined;
    const ids=Object.keys(data.drawingResources||data.displayList.resources);retain_render_resources(JSON.stringify(ids));
    for(const id of rasterResources)if(!ids.includes(id))rasterResources.delete(id);
    const list=installedList;if(skia)await skia.install(list,id=>rasterJob.resource_bytes(id),rasterGeneration,data.navigation,incremental);
    self.postMessage({mediaList:Object.keys(list.media?.assets||{}).length?list:undefined,scene:skia?undefined:list,sceneResources:skia?undefined:Object.fromEntries(Object.entries(list.resources).filter(([,r])=>r.mime.startsWith('image/')).map(([id])=>[id,rasterJob.resource_bytes(id)])),type:'installed',requestId:data.requestId,generation:rasterGeneration,reused,resourceIds:[...rasterResources],wasmMemoryBytes:rasterMemory.buffer.byteLength,initializationWaitMs:beforeResources-beforeReady,resourcesMs,installMs:performance.now()-beforeInstall-resourcesMs});
   }else if(data.type==='pick-objects'){
    const hits=installedList&&data.generation===rasterGeneration?previewObjectHits(data.point,data.scale):[];
    self.postMessage({type:'object-hits',requestId:data.requestId,generation:rasterGeneration,hits});
   }else if(data.type==='paint'){
    if(!rasterJob||data.generation!==rasterGeneration){self.postMessage({type:'paint-skipped',requestId:data.requestId,generation:data.generation});return;}
    skia?.setMediaFrames(data.mediaFrames);
    const before=performance.now(),states=data.time==null?{}:JSON.parse(rasterJob.frame_state(data.time,!!data.reduced)),sampleMs=performance.now()-before;
    const metrics=skia?.render(data.crop,data.width,data.height,states,rasterGeneration,!!data.running)||{};if(metrics.skipped){self.postMessage({type:'paint-skipped',requestId:data.requestId,generation:rasterGeneration});return;}
    if(skia)Object.assign(metrics,await skia.transitions.play(data.reduced));
    if(skia&&!data.running)metrics.presentationConfirmed=await skia.completeFrame(data.visible!==false);
    self.postMessage({type:'painted',requestId:data.requestId,generation:rasterGeneration,width:data.width,height:data.height,crop:data.crop,...metrics,sampleMs,mediaStates:skia&&Object.keys(skia.list?.media?.assets||{}).length?states:undefined,states:skia?undefined:states,wasmMemoryBytes:rasterMemory.buffer.byteLength});
   }
  }catch(error){self.postMessage({type:'error',requestId:data.requestId,generation:data.generation,error:String(error)});}
 });
};
