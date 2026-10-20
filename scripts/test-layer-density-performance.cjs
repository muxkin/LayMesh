const fs=require('node:fs'),path=require('node:path');
const pw=require(process.env.PLAYWRIGHT_MODULE||'playwright'),[base,out]=process.argv.slice(2);
(async()=>{const browser=await pw.firefox.launch({headless:true});try{
 const page=await browser.newPage();await page.goto(base+'/site/live/manifest.json');
 const result=await page.evaluate(async base=>{
  const root=base+'/site/live/',before=await import(root+'skia-renderer-v23.mjs'),exact=await import(root+'skia-renderer-exact-density.mjs'),wasm=await import(root+'wasm/laymesh_wasm.js');await wasm.default();const ck=await before.initializeSkia(root+'skia/canvaskit.wasm');
  const doc=await(await fetch(base+'/fixtures/pages/11.json')).json(),job=wasm.prepare_display_list(JSON.stringify(doc.list)),width=3840,height=1840,scale=Math.min(width/doc.list.width,height/doc.list.height),dx=(width/scale-doc.list.width)/2,dy=(height/scale-doc.list.height)/2,view=[-dx,-dy,doc.list.width+dx,doc.list.height+dy],rs=[];
  const placements=[];
  for(let variant=0;variant<3;variant++){
   const Module=variant?exact:before,r=new Module.SkiaRenderer(ck,new OffscreenCanvas(width,height),{software:true,premultipliedRaster:true,clipToPage:true});rs.push(r);
   if(variant===2)r.drawLayer=function(c,node,id,p,opacity){
    if(c.quickReject(p.bounds))return true;const required=before.projectedDensity(c.getTotalMatrix(),p.bounds);if(!Number.isFinite(required)||required<=0)return false;const entry=this.buildLayer(p,required);if(!entry)return false;
    const paint=new ck.Paint();paint.setAlphaf(Math.max(0,Math.min(1,opacity)));entry.used=this.frameCount;c.save();
    try{c.scale(1/entry.density,1/entry.density);if(placements.length<12)placements.push({matrix:Array.from(c.getTotalMatrix()),density:entry.density,required});for(const {image,x,y,w,h}of entry.tiles){c.save();try{c.clipRect([x,y,x+w,y+h],ck.ClipOp.Intersect,false);c.drawImageOptions(image,x-2,y-2,ck.FilterMode.Linear,ck.MipmapMode.None,paint);}finally{c.restore();}}}finally{c.restore();paint.delete();}return true;
   };
   await r.install(doc.list,()=>{throw Error('Unexpected image');},1);r.ensureSurface(width,height);r.preparationTileSize=256;
   const states=JSON.parse(job.presentation_state(.65,false)),work=r.sceneTasks(view,width,height,states),fixed=function*(){for(;;){r.preparationTileSize=256;const step=work.next();if(step.done)return;yield step.value;}};await r.prepareTasks(fixed());r.render(view,width,height,states,1,false,true,true);
  }
  const checks=[],timings=[];
  for(const r of rs){r.phaseTimes={static:0,layers:0,flush:0};for(const [name,key]of[['drawStaticRun','static'],['drawLayer','layers']]){const fn=r[name].bind(r);r[name]=(...args)=>{const at=performance.now();try{return fn(...args);}finally{r.phaseTimes[key]+=performance.now()-at;}};}const flush=r.surface.flush.bind(r.surface);r.surface.flush=()=>{const at=performance.now();try{return flush();}finally{r.phaseTimes.flush+=performance.now()-at;}};}

  for(const time of[.65,doc.inspection.animation.duration]){
   const s=JSON.parse(job.presentation_state(time,false)),pixels=[];
   for(const r of rs){r.render(view,width,height,s,1,false,true,true);pixels.push(r.readPixels());}
   for(let variant=1;variant<3;variant++){let max=0,sum=0,large=0;for(let i=0;i<pixels[0].length;i++){const d=Math.abs(pixels[0][i]-pixels[variant][i]);max=Math.max(max,d);sum+=d;large+=d>8;}checks.push({time,variant,max,mean:sum/pixels[0].length,over8:large/pixels[0].length});}
   for(let round=0;round<5;round++)for(const variant of round%2?[2,1,0]:[0,1,2]){rs[variant].phaseTimes={static:0,layers:0,flush:0};const v=rs[variant].render(view,width,height,s,1,false,true,true);timings.push({time,round,variant,rasterMs:v.rasterMs,layerBuilds:v.layerCacheBuilds,staticBuilds:v.staticCacheBuilds,phases:{...rs[variant].phaseTimes}});}
  }
  const sceneRuns=rs[0].sceneRuns.map(run=>({static:run.static,cacheable:run.cacheable,bounds:run.bounds,nodes:run.nodes.map(n=>({id:n.id,kind:n.node.kind.kind,name:n.node.name}))}));rs.forEach(r=>r.dispose());job.free();return {sceneRuns,page:doc.id,variants:['margin/rect','exact/rect','exact/image'],checks,timings,placements};
 },base);fs.mkdirSync(path.dirname(out),{recursive:true});fs.writeFileSync(out,JSON.stringify(result,null,2));for(let variant=0;variant<3;variant++){const rows=result.timings.filter(x=>x.variant===variant&&x.round>0);console.log(result.variants[variant],rows.reduce((s,x)=>s+x.rasterMs,0)/rows.length);}console.log(JSON.stringify(result.checks));
 }finally{await browser.close();}})().catch(e=>{console.error(e);process.exitCode=1;});
