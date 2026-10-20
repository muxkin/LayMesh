// CanvasKit's convenience software surface stores straight alpha. That forces
// general pixel pipelines even for a one-to-one cached image. Keep Skia's native
// premultiplied pixels and convert only when presenting to Canvas2D ImageData.
export function premultiplyPixels(source,target=source) {
 if(source.byteLength!==target.byteLength||source.byteLength%4)throw Error('Invalid presentation pixel buffer');
 if(source!==target)target.set(source);
 for(let i=0;i<source.length;i+=4){const a=source[i+3];if(a===255)continue;for(let j=0;j<3;j++)target[i+j]=Math.floor((source[i+j]*a+127)/255);}
 return target;
}

const cpuUnpremultiplyTables=new WeakMap();
export function cpuUnpremultiplyTable(ck) {
 if(cpuUnpremultiplyTables.has(ck))return cpuUnpremultiplyTables.get(ck);
 // Ask this CanvasKit build for its exact byte conversion once. Float32 ties
 // differ from a JS division, and Canvas2D can magnify a one-byte difference.
 // The table replaces per-frame arithmetic with a 64 KiB immutable lookup.
 const bytes=new Uint8Array(256*256*4),info={width:256,height:256,colorType:ck.ColorType.RGBA_8888,alphaType:ck.AlphaType.Premul,colorSpace:ck.ColorSpace.SRGB};
 for(let a=0;a<256;a++)for(let c=0;c<256;c++){const i=(a*256+c)*4;bytes[i]=bytes[i+1]=bytes[i+2]=Math.min(a,c);bytes[i+3]=a;}
 const image=ck.MakeImage(info,bytes,1024),surface=ck.MakeSurface(256,256);
 try{
  surface.getCanvas().drawImage(image,0,0);
  const rgba=surface.getCanvas().readPixels(0,0,{...info,alphaType:ck.AlphaType.Unpremul});
  if(!rgba)throw Error('Presentation alpha conversion unavailable');
  const table=new Uint8Array(65536);for(let i=0;i<table.length;i++)table[i]=rgba[i*4];
  cpuUnpremultiplyTables.set(ck,table);return table;
 }finally{image?.delete();surface?.dispose();}
}

export function unpremultiplyPixels(source, target, table) {
 if(source.byteLength!==target.byteLength||source.byteLength%4)throw Error('Invalid presentation pixel buffer');
 target.set(source);
 for(let i=0;i<source.length;i+=4){
  const a=source[i+3];
  if(a===255)continue;
  if(!a){target[i]=target[i+1]=target[i+2]=0;continue;}
  const row=a*256;target[i]=table[row+source[i]];target[i+1]=table[row+source[i+1]];target[i+2]=table[row+source[i+2]];
 }
 return target;
}

// A top-level translation has only the page's hard rectangular clip. Blend its
// already-rasterized premultiplied pixels directly into the software surface.
// Keep the real subpixel phase and bilinear sampling; never snap the artwork.
// Other transforms, nested clips and reveal wrappers stay in Skia.
export function canBlitPresentationLayer(id,node,m,density) {
 return /^\d+$/.test(id)&&!node.hint?.animationRevealContent&&!node.hint?.surface3d&&
  m.every(Number.isFinite)&&m[1]===0&&m[3]===0&&m[6]===0&&m[7]===0&&m[8]===1&&
  Math.abs(m[0]/density-1)<1e-5&&Math.abs(m[4]/density-1)<1e-5;
}

export function blitPresentationLayer(dst,width,height,tiles,m,density,clip,opacity) {
 const sx=m[0]/density,sy=m[4]/density,tx=m[2],ty=m[5],alpha=Math.fround(Math.max(0,Math.min(1,opacity)));
 for(const {x,y,w,h,pixels:src}of tiles){
  const sw=w+4,left=Math.max(0,Math.ceil(clip[0]-.5),Math.ceil(tx+x*sx-.5)),right=Math.min(width,Math.ceil(clip[2]-.5),Math.ceil(tx+(x+w)*sx-.5)),top=Math.max(0,Math.ceil(clip[1]-.5),Math.ceil(ty+y*sy-.5)),bottom=Math.min(height,Math.ceil(clip[3]-.5),Math.ceil(ty+(y+h)*sy-.5));
  for(let py=top;py<bottom;py++){
   const v=(py+.5-ty)/sy-y+1.5,iy=Math.floor(v),fy=v-iy;let di=(py*width+left)*4;
   for(let px=left;px<right;px++,di+=4){
    const u=(px+.5-tx)/sx-x+1.5,ix=Math.floor(u),fx=u-ix,a0=(iy*sw+ix)*4,a1=a0+sw*4;
    const topA=src[a0+3]+(src[a0+7]-src[a0+3])*fx,bottomA=src[a1+3]+(src[a1+7]-src[a1+3])*fx,a=(topA+(bottomA-topA)*fy)*alpha;
    if(a<=0)continue;const inv=1-a/255;
    for(let k=0;k<3;k++){const topC=src[a0+k]+(src[a0+k+4]-src[a0+k])*fx,bottomC=src[a1+k]+(src[a1+k+4]-src[a1+k])*fx;dst[di+k]=Math.min(255,Math.floor((topC+(bottomC-topC)*fy)*alpha+dst[di+k]*inv+.5));}
    dst[di+3]=Math.min(255,Math.floor(a+dst[di+3]*inv+.5));
   }
  }
 }
}

function makePresentationRasterSurface(ck,width,height,canvas) {
 const table=canvas?cpuUnpremultiplyTable(ck):null;
 // Establish the OffscreenCanvas presentation context before its first RAF,
 // even when the first composition is deliberately kept offscreen.
 let context=canvas?.getContext('2d');if(canvas&&!context)throw Error('CPU presentation context unavailable');
 let pixels=ck.Malloc(Uint8Array,width*height*4),display;
 const surface=ck.MakeRasterDirectSurface({width,height,colorType:ck.ColorType.RGBA_8888,alphaType:ck.AlphaType.Premul,colorSpace:ck.ColorSpace.SRGB},pixels,width*4);
 if(!surface){ck.Free(pixels);return null;}
 surface.getCanvas().clear(ck.TRANSPARENT);
 const flush=surface.flush.bind(surface);surface.flushDrawing=flush;
 surface.premultipliedPixels=()=>pixels.toTypedArray();
 surface.presentationBufferBytes=0;
 surface.flush=()=>{
  flush();if(!canvas)return;
  if(!display){display=new ImageData(new Uint8ClampedArray(width*height*4),width,height);surface.presentationBufferBytes=display.data.byteLength;}
  // Refresh the view after any WASM heap growth; the reusable ImageData owns
  // its separate buffer and never points into a detached module heap.
  unpremultiplyPixels(pixels.toTypedArray(),display.data,table);context.putImageData(display,0,0);
 };
 surface.dispose=()=>{if(!pixels)return;surface.delete();ck.Free(pixels);pixels=null;display=null;context=null;surface.presentationBufferBytes=0;};
 return surface;
}

// Cached presentation content is already rasterized by Skia. Let Canvas2D
// composite those immutable bitmaps, avoiding a full WASM framebuffer and an
// unpremultiply/upload of every output pixel on every animation frame.
// The raster surface is allocated lazily for unsupported dynamic content.
export function makePresentationCPUSurface(ck,width,height,canvas) {
 if(!canvas)return makePresentationRasterSurface(ck,width,height,null);
 const display=canvas.getContext('2d');if(!display)throw Error('CPU presentation context unavailable');
 let raster,target,context,factory,converted,native=false,nativeCanvas;
 const output=()=>{if(!target){target=new OffscreenCanvas(width,height);context=target.getContext('2d');}return context;};
 const software=()=>{native=false;return raster??=makePresentationRasterSurface(ck,width,height,null);};
 const sync=()=>{if(native)return;const r=software();r.flushDrawing();converted??=new ImageData(width,height);unpremultiplyPixels(r.premultipliedPixels(),converted.data,cpuUnpremultiplyTable(ck));output().putImageData(converted,0,0);};
 return {
  getCanvas:()=>software().getCanvas(),
  makeSurface:info=>(raster||(factory??=ck.MakeSurface(1,1))).makeSurface(info),
  premultipliedPixels:()=>software().premultipliedPixels(),
  beginNative(present=true){native=true;if(present){nativeCanvas=canvas;return display;}nativeCanvas=target||(output(),target);return context;},
  nativeSnapshot(){sync();const copy=new OffscreenCanvas(width,height);copy.getContext('2d').drawImage(native?nativeCanvas:target,0,0);return copy.transferToImageBitmap();},
  makeImageSnapshot(bounds){
   if(!native)return software().makeImageSnapshot(bounds);
   const [x,y,x1,y1]=bounds||[0,0,width,height],data=nativeCanvas.getContext('2d').getImageData(x,y,x1-x,y1-y);
   return ck.MakeImage({width:data.width,height:data.height,colorType:ck.ColorType.RGBA_8888,alphaType:ck.AlphaType.Unpremul,colorSpace:ck.ColorSpace.SRGB},data.data,data.width*4);
  },
  flushDrawing(){sync();},
  flush(){if(native&&nativeCanvas===canvas)return;sync();display.save();try{display.resetTransform();display.globalAlpha=1;display.globalCompositeOperation='copy';display.drawImage(target,0,0);}finally{display.restore();}if(native)nativeCanvas=canvas;},
  get presentationBufferBytes(){return (target?width*height*4:0)+(converted?.data.byteLength||0);},
  get rasterBufferBytes(){return raster?width*height*4:0;},
  dispose(){raster?.dispose();factory?.dispose();if(target){target.width=1;target.height=1;}raster=target=context=factory=converted=null;},
 };
}
