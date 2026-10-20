// Isolated original-shader versus production opaque-region composition.
// Pixel oracles are outside timings; this does not replace natural playback.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const pw=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const [base,out]=process.argv.slice(2);fs.mkdirSync(out,{recursive:true});
(async()=>{
 const engine=process.env.ENGINE||'chromium',args=engine==='chromium'&&process.env.HARDWARE_GPU==='1'?['--enable-gpu','--use-gl=angle','--use-angle=vulkan','--enable-features=Vulkan']:[];
 const browser=await pw[engine].launch({headless:true,args});try{
 const page=await browser.newPage();await page.goto(base+'/site/live/manifest.json');
 const result=await page.evaluate(async({base,software})=>{
  const url=new URL(base+'/site/live/'),{initializeSkia,SkiaRenderer}=await import(new URL('skia-renderer.mjs',url)),{transitions}=await import(new URL('presentation-transitions-data.mjs',url)),{drawOpaqueFade}=await import(new URL('presentation-transitions.mjs',url)),w=await import(new URL('wasm/laymesh_wasm.js',url));await w.default();
  const ck=await initializeSkia(new URL('skia/canvaskit.wasm',url).href),width=3840,height=1840,r=new SkiaRenderer(ck,new OffscreenCanvas(width,height),{software,clipToPage:true,premultipliedRaster:true});
  const effects=[transitions.fade.sksl].map(source=>{let error='';const v=ck.RuntimeEffect.Make(source,e=>error+=e);if(!v)throw Error(error);return v;}),images=[];
  for(const [index,end]of [[0,true],[1,false]]){
   const d=await(await fetch(base+'/fixtures/pages/'+index+'.json')).json(),job=w.prepare_display_list(JSON.stringify(d.list)),time=end?d.inspection.animation.duration:0,states=JSON.parse(job.presentation_state(time,false));
   await r.install(d.list,()=>{throw Error('Unexpected image resource');},index);
   const scale=Math.min(width/d.list.width,height/d.list.height),dx=(width/scale-d.list.width)/2,dy=(height/scale-d.list.height)/2;
   const view=[-dx,-dy,d.list.width+dx,d.list.height+dy];r.ensureSurface(width,height);r.preparationTileSize=256;await r.prepareTasks(r.sceneTasks(view,width,height,states));r.render(view,width,height,states,index,false,true,true);await r.completeResources();images.push(r.surface.makeImageSnapshot());job.free();
  }
  function setup(images,w,h){const shaders=images.map(i=>i.makeShaderOptions(ck.TileMode.Clamp,ck.TileMode.Clamp,ck.FilterMode.Linear,ck.MipmapMode.None));return {images,shaders,uniforms:effects.map(effect=>{const u=new Float32Array(effect.getUniformFloatCount()),values={_resolution:[w,h],ratio:w/h,progress:.5};for(let i=0;i<effect.getUniformCount();i++){const d=effect.getUniform(i);[].concat(values[effect.getUniformName(i)]??0).forEach((v,j)=>u[d.slot+j]=v);}return u;})};}
  const paint=new ck.Paint();function draw(surface,setup,variant,progress=.5,clip=null){const e=effects[variant===2?0:variant],u=setup.uniforms[variant===2?0:variant];for(let i=0;i<e.getUniformCount();i++)if(e.getUniformName(i)==='progress')u[e.getUniform(i).slot]=progress;const shader=e.makeShaderWithChildren(u,setup.shaders);if(!shader)throw Error('shader');try{const c=surface.getCanvas();c.clear(ck.TRANSPARENT);c.save();if(clip)c.clipRect(clip,ck.ClipOp.Intersect,false);if(variant===2&&setup.opaqueRect){
 drawOpaqueFade(ck,c,...setup.images,setup.opaqueRect,progress,paint);c.clipRect(setup.opaqueRect,ck.ClipOp.Difference,false);
 }paint.setShader(shader);c.drawPaint(paint);c.restore();surface.flush();}finally{shader.delete();}}
  const stage=setup(images,width,height),timings=[];stage.opaqueRect=r.opaqueRect;
  for(let round=0;round<12;round++)for(const variant of round%2?[2,0]:[0,2]){const start=performance.now();draw(r.surface,stage,variant,.5,r.pageBounds);const submitted=performance.now();await r.completeResources();timings.push({round,variant,submitMs:submitted-start,completedMs:performance.now()-start});}
  stage.shaders.forEach(s=>s.delete());images.forEach(i=>i.delete());
  const qw=128,qh=96,surface=r.surface.makeSurface({width:qw,height:qh,colorType:ck.ColorType.RGBA_8888,alphaType:ck.AlphaType.Premul,colorSpace:ck.ColorSpace.SRGB}),checks=[];
  for(const kind of ['opaque','varying-alpha']){
   const imgs=[0,1].map(n=>{const bytes=new Uint8Array(qw*qh*4);for(let y=0;y<qh;y++)for(let x=0;x<qw;x++){const i=(y*qw+x)*4;bytes[i]=(x*7+n*31)%256;bytes[i+1]=(y*11+n*63)%256;bytes[i+2]=(x+y*3+n*19)%256;bytes[i+3]=kind==='opaque'?255:(x*3+y*5+n*17)%256;}return ck.MakeImage({width:qw,height:qh,colorType:ck.ColorType.RGBA_8888,alphaType:ck.AlphaType.Unpremul,colorSpace:ck.ColorSpace.SRGB},bytes,qw*4);}),state=setup(imgs,qw,qh);
   state.opaqueRect=kind==='opaque'?[1,1,qw-1,qh-1]:null;
   for(const progress of [.01,.25,.5,.75,.99]){
    const pixels=[];for(const variant of [0,2]){draw(surface,state,variant,progress);const image=surface.makeImageSnapshot();pixels.push(image.readPixels(0,0,{width:qw,height:qh,colorType:ck.ColorType.RGBA_8888,alphaType:ck.AlphaType.Unpremul,colorSpace:ck.ColorSpace.SRGB}));image.delete();}
    let max=0,sum=0,changed=0;for(let i=0;i<pixels[0].length;i++){const d=Math.abs(pixels[0][i]-pixels[1][i]);max=Math.max(max,d);sum+=d;changed+=d!==0;}checks.push({kind,progress,max,mean:sum/pixels[0].length,changed});
   }
   state.shaders.forEach(s=>s.delete());imgs.forEach(i=>i.delete());
  }
  surface.dispose();paint.delete();effects.forEach(e=>e.delete());const result={backend:r.backend,device:r.device,width,height,timings,checks};r.dispose();return result;
 },{base,software:process.env.SOFTWARE==='1'});
 assert(result.checks.every(c=>c.max<=1),JSON.stringify(result.checks));
 const avg=variant=>{const v=result.timings.filter(t=>t.round>1&&t.variant===variant);return v.reduce((n,t)=>n+t.completedMs,0)/v.length;};result.meanCompletedMs={original:avg(0),optimized:avg(2)};
 fs.writeFileSync(path.join(out,'result.json'),JSON.stringify(result,null,2));console.log(JSON.stringify({backend:result.backend,meanCompletedMs:result.meanCompletedMs,worstPixel:Math.max(...result.checks.map(c=>c.max))}));
 }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
