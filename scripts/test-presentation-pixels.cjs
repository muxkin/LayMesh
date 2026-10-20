// Actual Skia alpha conversion, snapshot lifetime and opaque-region fade oracle.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const pw=require(process.env.PLAYWRIGHT_MODULE||'playwright');
(async()=>{const [base,out]=process.argv.slice(2),rows=[];
for(const engine of (process.env.ENGINES||'chromium,firefox,webkit').split(',')){
 const browser=await pw[engine].launch({headless:true});try{const page=await browser.newPage();await page.goto(base+'/site/live/manifest.json');
 const result=await page.evaluate(async base=>{
  const {initializeSkia}=await import(base+'/site/live/skia-renderer.mjs'),{makePresentationCPUSurface}=await import(base+'/site/live/skia-cpu-surface.mjs'),{drawOpaqueFade,opaquePageRect}=await import(base+'/site/live/presentation-transitions.mjs'),{transitions}=await import(base+'/site/live/presentation-transitions-data.mjs');
  const ck=await initializeSkia(base+'/site/live/skia/canvaskit.wasm'),width=256,height=256,canvas=new OffscreenCanvas(width,height),surface=makePresentationCPUSurface(ck,width,height,canvas),info={width,height,colorType:ck.ColorType.RGBA_8888,alphaType:ck.AlphaType.Unpremul,colorSpace:ck.ColorSpace.SRGB},read=image=>image.readPixels(0,0,info),get=()=>{const image=surface.makeImageSnapshot();try{return read(image);}finally{image.delete();}};
  const bytes=new Uint8Array(width*height*4);for(let i=0;i<bytes.length;i+=4){const a=Math.floor(i/4/256),c=Math.min(a,(i/4)%256);bytes[i]=c;bytes[i+1]=a-c;bytes[i+2]=a;bytes[i+3]=a;}
  const input=ck.MakeImage({...info,alphaType:ck.AlphaType.Premul},bytes,width*4);surface.getCanvas().drawImage(input,0,0);surface.flush();const nativeReference=get(),presented=canvas.getContext('2d').getImageData(0,0,width,height).data;
  const originalCanvas=new OffscreenCanvas(width,height),original=ck.MakeSWCanvasSurface(originalCanvas);original.getCanvas().drawImage(input,0,0);original.flush();const reference=originalCanvas.getContext('2d').getImageData(0,0,width,height).data;original.dispose();
  let displayWeightedMax=0,worst;for(let i=0;i<reference.length;i++){const delta=Math.abs(reference[i]-presented[i])*(i%4===3?1:reference[i-i%4+3]/255);if(delta>displayWeightedMax){displayWeightedMax=delta;const start=i-i%4;worst={pixel:start/4,reference:[...reference.slice(start,start+4)],presented:[...presented.slice(start,start+4)]};}}
  const snapshot=surface.makeImageSnapshot(),snapshotBefore=read(snapshot);surface.getCanvas().clear(ck.RED);surface.flush();const snapshotAfter=read(snapshot);if(snapshotBefore.some((v,i)=>v!==snapshotAfter[i]))throw Error('Direct surface snapshot aliases mutable storage');
  // A new WASM buffer must not leave presentation reading a detached view.
  const growth=ck.Malloc(Uint8Array,ck.HEAPU8.byteLength+65536);surface.flush();ck.Free(growth);const red=canvas.getContext('2d').getImageData(5,5,1,1).data;if(red[0]!==255||red[3]!==255)throw Error('Detached presentation buffer');
  surface.dispose();const snapshotDisposed=read(snapshot);if(snapshotBefore.some((v,i)=>v!==snapshotDisposed[i]))throw Error('Snapshot freed with external raster storage');snapshot.delete();input.delete();
  const holder=ck.MakeSurface(1,1),output=holder.makeSurface({...info,alphaType:ck.AlphaType.Premul});holder.dispose();const paint=new ck.Paint(),effect=ck.RuntimeEffect.Make(transitions.fade.sksl),checks=[];
  for(const kind of ['opaque','mixed','transparent']){
   const rect=kind==='transparent'?null:[8,8,width-8,height-8],images=[0,1].map(n=>{const b=new Uint8Array(width*height*4);for(let y=0;y<height;y++)for(let x=0;x<width;x++){const i=(y*width+x)*4;b[i]=(x*7+n*31)%256;b[i+1]=(y*11+n*63)%256;b[i+2]=(x+y*3+n*19)%256;b[i+3]=kind==='opaque'||kind==='mixed'&&x>=8&&x<width-8&&y>=8&&y<height-8?255:(x*3+y*5+n*17)%256;}return ck.MakeImage(info,b,width*4);});
   for(const reverse of [false,true])for(const progress of [.01,.25,.5,.75,.99]){
    const ordered=reverse?[...images].reverse():images,children=ordered.map(i=>i.makeShaderOptions(ck.TileMode.Clamp,ck.TileMode.Clamp,ck.FilterMode.Linear,ck.MipmapMode.None)),u=new Float32Array(effect.getUniformFloatCount()),values={_resolution:[width,height],ratio:width/height,progress};for(let i=0;i<effect.getUniformCount();i++){const d=effect.getUniform(i);[].concat(values[effect.getUniformName(i)]??0).forEach((v,j)=>u[d.slot+j]=v);}
    const shader=effect.makeShaderWithChildren(u,children),c=output.getCanvas(),pixels=[];
    for(const optimized of [false,true]){c.clear(ck.TRANSPARENT);c.save();if(optimized&&rect){drawOpaqueFade(ck,c,...ordered,rect,progress,paint);c.clipRect(rect,ck.ClipOp.Difference,false);}paint.setShader(shader);c.drawPaint(paint);c.restore();output.flush();const image=output.makeImageSnapshot();pixels.push(read(image));image.delete();}
    let max=0,sum=0;for(let i=0;i<pixels[0].length;i++){const d=Math.abs(pixels[0][i]-pixels[1][i]);max=Math.max(max,d);sum+=d;}checks.push({kind,reverse,progress,max,mean:sum/pixels[0].length});shader.delete();children.forEach(c=>c.delete());
   }images.forEach(i=>i.delete());
  }
  paint.delete();effect.delete();output.dispose();let proven=0;
  for(let i=0;i<26;i++){const d=await(await fetch(base+'/fixtures/pages/'+i+'.json')).json();if(opaquePageRect(d.list,{},[0,0,d.list.width,d.list.height],1920,1080))proven++;}
  return {displayWeightedMax,worst,checks,snapshotSurvives:true,heapGrowth:true,opaquePages:proven};
 },base);
 fs.mkdirSync(path.dirname(out),{recursive:true});fs.writeFileSync(out,JSON.stringify([...rows,{engine,...result}],null,2));assert(result.displayWeightedMax<=1,JSON.stringify({max:result.displayWeightedMax,worst:result.worst}));assert(result.checks.every(c=>c.max<=1),JSON.stringify(result.checks));assert.equal(result.opaquePages,26);rows.push({engine,version:browser.version(),...result});console.log(engine,'pixel conversion, snapshot lifetime and 30 fade comparisons passed');
 }finally{await browser.close();}
}fs.mkdirSync(path.dirname(out),{recursive:true});fs.writeFileSync(out,JSON.stringify(rows,null,2));})().catch(e=>{console.error(e);process.exitCode=1});
