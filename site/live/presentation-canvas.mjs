// Native composition only: all contours, glyphs, effects and image crops are
// still rasterized by the shared Skia renderer at the planned display density.
import {unpremultiplyPixels,cpuUnpremultiplyTable} from './skia-cpu-surface.mjs';
export function supportsNativeComposition(s){
 if(!s.allowNativeComposition||s.gr||!s.premultipliedRaster||!s.clipToPage||s.rasterOnly)return false;
 for(const timeline of [s.list.animation?.normal,s.list.animation?.reduced])for(const step of timeline?.steps||[])for(const clip of step.clips)for(const track of clip.tracks)if(!['transform','transform-origin','opacity'].includes(track.property))return false;
 return s.list.nodes.every(node=>!s.prepare(node).dynamicChildren);
}

export function promotePresentationEntry(s,entry){
 if(entry.bitmap||entry.nativeDisabled||!entry.complete||!entry.tiles.length)return;
 const started=performance.now();let readMs=0,uploadMs=0;
 const ck=s.ck,table=cpuUnpremultiplyTable(ck),x=Math.min(...entry.tiles.map(t=>t.x)),y=Math.min(...entry.tiles.map(t=>t.y)),w=Math.max(...entry.tiles.map(t=>t.x+t.w))-x,h=Math.max(...entry.tiles.map(t=>t.y+t.h))-y;
 let canvas,bitmap;
 try{
  canvas=new OffscreenCanvas(w+4,h+4);const c=canvas.getContext('2d');if(!c)throw Error('Native presentation bitmap unavailable');
  for(const tile of entry.tiles){
   let at=performance.now();
   // Read native premultiplied storage directly. Asking software Skia for
   // straight alpha invokes its generic pixel pipeline for the whole image.
   const rgba=tile.image.readPixels(0,0,{width:tile.w+4,height:tile.h+4,colorType:ck.ColorType.RGBA_8888,alphaType:ck.AlphaType.Premul,colorSpace:ck.ColorSpace.SRGB});
   readMs+=performance.now()-at;
   if(!rgba)throw Error('Presentation cache conversion failed');
   unpremultiplyPixels(rgba,rgba,table);
   // Copy each core once; adjacent tile halos overlap and must not overwrite
   // one another. The merged image eliminates transformed tile clip seams.
   at=performance.now();c.putImageData(new ImageData(new Uint8ClampedArray(rgba.buffer,rgba.byteOffset,rgba.byteLength),tile.w+4,tile.h+4),tile.x-x,tile.y-y,2,2,tile.w,tile.h);uploadMs+=performance.now()-at;
   s.nativeWorkingPeakBytes=Math.max(s.nativeWorkingPeakBytes||0,(w+4)*(h+4)*4+rgba.byteLength);
  }
  bitmap=canvas.transferToImageBitmap();
 }catch(error){bitmap?.close();entry.nativeDisabled=true;s.nativeFallbackReason=String(error);return;}
 finally{if(canvas)canvas.width=canvas.height=1;}
 const old=entry.bytes,bytes=(w+4)*(h+4)*4;
 for(const tile of entry.tiles)tile.image.delete();
 s.textureImages-=entry.tiles.length;s.textureReleased=(s.textureReleased||0)+old;
 entry.tiles=[{x,y,w,h}];entry.bitmap=bitmap;entry.bytes=bytes;
 s.textureAllocated=(s.textureAllocated||0)+bytes;s.nativeBitmapBytes=(s.nativeBitmapBytes||0)+bytes;s.nativeBitmapCount=(s.nativeBitmapCount||0)+1;s.nativePromotions=(s.nativePromotions||0)+1;
 s.texturePeak=Math.max(s.texturePeak||0,s.textureAllocated-s.textureReleased);
 if(entry.signature!==undefined)s.staticTextureBytes+=bytes-old;else s.layerTextureBytes+=bytes-old;
 s.nativePromotionMs=(s.nativePromotionMs||0)+performance.now()-started;s.nativeReadbackMs=(s.nativeReadbackMs||0)+readMs;s.nativeUploadMs=(s.nativeUploadMs||0)+uploadMs;
}

export function restorePresentationEntry(s,entry){
 if(!entry.bitmap)return;
 const bitmap=entry.bitmap,canvas=new OffscreenCanvas(bitmap.width,bitmap.height),c=canvas.getContext('2d'),ck=s.ck;
 try{
  c.drawImage(bitmap,0,0);const rgba=c.getImageData(0,0,bitmap.width,bitmap.height).data;
  const image=ck.MakeImage({width:bitmap.width,height:bitmap.height,colorType:ck.ColorType.RGBA_8888,alphaType:ck.AlphaType.Unpremul,colorSpace:ck.ColorSpace.SRGB},rgba,bitmap.width*4);if(!image)throw Error('Presentation cache restore failed');
  entry.tiles[0].image=image;entry.nativeDisabled=true;bitmap.close();entry.bitmap=null;s.nativeBitmapBytes-=entry.bytes;s.nativeBitmapCount--;s.textureImages++;s.nativeDemotions=(s.nativeDemotions||0)+1;
 }finally{canvas.width=canvas.height=1;}
}

export function composePresentationCanvas(s,view,width,height,states,present=true){
 if(!s.nativeComposition||!s.surface.beginNative)return false;
 if(Object.entries(states).some(([id,v])=>id.includes('/')||v.content||v.reveal))return false;
 const commands=[],sx=width/(view[2]-view[0]),sy=height/(view[3]-view[1]);
 for(const run of s.sceneRuns){
  if(run.empty)continue;
  if(run.background){if(states[run.nodes[0].id])return false;commands.push({color:run.background});continue;}
  if(run.static){
   if(run.nodes.some(({id})=>states[id]))return false;
   const work=s.staticTasks(run,view,width,height);let entry;for(;;){const step=work.next();if(step.done){entry=step.value;break;}}
   if(entry===true)continue;if(!entry)return false;promotePresentationEntry(s,entry);if(!entry.bitmap)return false;
   commands.push({entry});
  }else for(const {node,id}of run.nodes){
   const state=states[id],p=s.prepare(node),m=state?.matrix||p.matrix,opacity=state?.opacity??node.opacity;
   if(m[6]||m[7]||m[8]!==1||!m.every(Number.isFinite))return false;if(opacity<=0)continue;
   // layerTasks checks actual density even after peak-density preparation.
   const a=m[0]*sx,b=m[1]*sx,c=m[3]*sy,d=m[4]*sy,sum=a*a+b*b+c*c+d*d,det=a*d-b*c;
   const entry=s.buildLayer(p,Math.sqrt((sum+Math.sqrt(Math.max(0,sum*sum-4*det*det)))/2));if(!entry)return false;
   promotePresentationEntry(s,entry);if(!entry.bitmap)return false;commands.push({entry,m,opacity});
  }
 }
 const c=s.surface.beginNative(present);c.save();
 try{
  c.resetTransform();c.globalAlpha=1;c.globalCompositeOperation='source-over';c.clearRect(0,0,width,height);c.imageSmoothingEnabled=true;c.imageSmoothingQuality='low';
  const b=s.pageBounds.map(v=>Math.ceil(v-.5));c.beginPath();c.rect(b[0],b[1],b[2]-b[0],b[3]-b[1]);c.clip();
  for(const {entry,m,opacity,color}of commands){
   if(color){const r=s.pageBounds;c.fillStyle=`rgba(${color.slice(0,3).map(v=>v*255).join(',')},${color[3]})`;c.fillRect(r[0],r[1],r[2]-r[0],r[3]-r[1]);continue;}
   const {x,y}=entry.tiles[0],bitmap=entry.bitmap;entry.used=s.frameCount;c.save();
   try{if(m){c.setTransform(sx*m[0],sy*m[3],sx*m[1],sy*m[4],sx*(m[2]-view[0]),sy*(m[5]-view[1]));c.globalAlpha=Math.max(0,Math.min(1,opacity));c.drawImage(bitmap,(x-2)/entry.density,(y-2)/entry.density,bitmap.width/entry.density,bitmap.height/entry.density);}else c.drawImage(bitmap,x-2,y-2);}finally{c.restore();}
  }
 }finally{c.restore();}
 s.nativeCompositions=(s.nativeCompositions||0)+1;return true;
}
