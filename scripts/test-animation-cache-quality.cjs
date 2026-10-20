// Semantic renderer oracle: direct versus cached drawing on both backends.
// BASELINE_RENDERER_MODULE additionally enables a historical direct comparison.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const pw=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const [url,out]=process.argv.slice(2);fs.mkdirSync(out,{recursive:true});
(async()=>{for(const engine of (process.env.ENGINES||'chromium,firefox,webkit').split(',')){
 const args=process.env.HARDWARE_GPU==='1'&&engine==='chromium'?['--enable-gpu','--use-gl=angle','--use-angle=vulkan','--enable-features=Vulkan']:[];const browser=await pw[engine].launch({headless:true,args});try{
 const page=await browser.newPage(),errors=[];page.on('pageerror',e=>errors.push(e.message));await page.goto(url+'/site/live/manifest.json');
 const result=await page.evaluate(async ({baseURL,baseline})=>{
  const base=new URL(baseURL+'/site/live/'),{initializeSkia,SkiaRenderer}=await import(new URL('skia-renderer.mjs',base)),Previous=baseline?(await import(new URL(baseline,base))).SkiaRenderer:SkiaRenderer;
  const meta=await(await fetch(new URL('skia/manifest.json',base))).json(),ck=await initializeSkia(new URL('skia/'+meta.wasm,base).href),w=await import(new URL('wasm/laymesh_wasm.js',base));await w.default();
  const source=`p=canvas(size=(160mm,90mm),background="#e7eef4")
a=p.add(rect(size=(48mm,29mm),border_radius=4mm,fill="#27999aaa",effects=[shadow(blur=0.4mm,offset=(1mm,1mm),opacity=0.15),shadow(blur=1mm,offset=(2mm,2mm),opacity=0.2),shadow(blur=2mm,offset=(2mm,4mm),opacity=0.2),shadow(blur=4mm,offset=(3mm,7mm),opacity=0.2),glow(blur=1mm,color="#4c81de",opacity=0.2)]),offset=(22mm,26mm))
p.add(ellipse(size=(34mm,26mm),fill="#c36e5599",effects=[shadow(blur=2mm,offset=(-2mm,3mm))]),offset=(95mm,30mm))
style { @keyframes move { from { transform: translateX(-3mm); opacity:0.3; } to { transform: translateX(3mm); opacity:0.9; } } }
s=p.step(hold=3s)
s.animate(a,name="move",duration=1s,easing="linear",fill="both")`;
  const job=w.prepare_render(source,'/cache.lay','{}'),draw=job.draw_job(),list=JSON.parse(draw.display_list()),checks=[];
  function diff(a,b,width,region){let sum=0,max=0,large=0,n=0;for(let y=region[1];y<region[3];y++)for(let x=region[0];x<region[2];x++)for(let k=0;k<4;k++){const i=(y*width+x)*4+k,d=Math.abs(a[i]-b[i]);sum+=d;max=Math.max(max,d);large+=d>8;n++;}return {mean:sum/n,max,over8:large/n};}
  for(const software of [false,true]){
   const r=new SkiaRenderer(ck,new OffscreenCanvas(960,540),{software,clipToPage:true}),old=new Previous(ck,new OffscreenCanvas(960,540),{software,clipToPage:true});
   await r.install(list,id=>draw.resource_bytes(id),'a');await old.install(list,id=>draw.resource_bytes(id),'a');
   let cancelled=false,yields=0;r.ensureSurface(960,540);r.preparationTileSize=256;
   const interrupted=await r.prepareTasks(r.sceneTasks([0,0,160,90],960,540,JSON.parse(draw.presentation_state(0,false))),()=>cancelled,()=>{if(++yields===2)cancelled=true;return 0;});
   if(!interrupted.cancelled)throw Error('Preparation did not yield to cancellation');
   const allocated=r.textureAllocated;await r.prepareTasks(r.sceneTasks([0,0,160,90],960,540,JSON.parse(draw.presentation_state(0,false))));
   if(r.textureReleased)throw Error('Resume rebuilt retained partial tiles');
   const unique=r.textureAllocated;
   for(const time of [0,.5,1,4]){
    const states=JSON.parse(draw.presentation_state(time,false));
    old.render([0,0,160,90],960,540,states,'a',false,false,false);const before=old.readPixels();
    r.render([0,0,160,90],960,540,states,'a',false,false,false);const direct=r.readPixels();
    r.render([0,0,160,90],960,540,states,'a',false,true,true);const cached=r.readPixels();
    const pixels=await r.completeResources();
    const entry={backend:r.backend,time,objects:[[0,0,540,540],[540,0,960,540]].map(region=>({region,boundsCorrection:diff(before,direct,960,region),cached:diff(direct,cached,960,region)}))};
    if(entry.objects.some(o=>o.boundsCorrection.mean>.5||o.cached.mean>1||o.cached.over8>.025))throw Error(JSON.stringify(entry));
    checks.push(entry);
   }
   const settled=r.textureAllocated;for(let i=0;i<4;i++)r.render([0,0,160,90],960,540,JSON.parse(draw.presentation_state(.5,false)),'a',false,true,true);
   if(r.textureAllocated!==settled)throw Error('Placement/opacity duplicated content tiles');
   checks.push({backend:r.backend,device:r.device,scratchSamples:r.scratch?.samples,retainedSamples:r.scratch?.detached?1:0,cancellationResumed:true,partialRetainedBytes:allocated,preparedBytes:unique,settledBytes:settled});
   r.dispose();old.dispose();if(r.textureAllocated!==r.textureReleased||r.textureImages!==0)throw Error('Texture ownership leaked after disposal');
  }
  draw.free();job.free();return checks;
 },{baseURL:url.replace(/\/$/,''),baseline:process.env.BASELINE_RENDERER_MODULE});
 assert.deepEqual(errors,[]);fs.writeFileSync(path.join(out,engine+'.json'),JSON.stringify({status:'passed',engine,reference:process.env.BASELINE_RENDERER_MODULE||'current-direct',result,errors},null,2));console.log(engine,'direct/cached effects, translucency, cancellation, reuse and release passed');
 }finally{await browser.close();}
}})().catch(e=>{console.error(e);process.exitCode=1;});
