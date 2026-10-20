// Diagnostic: identical retained pixels, alternate CanvasKit image placement.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const pw=require(process.env.PLAYWRIGHT_MODULE||'playwright'),[base,out]=process.argv.slice(2);
(async()=>{const browser=await pw.firefox.launch({headless:true});try{
 const page=await browser.newPage();await page.goto(base+'/site/live/manifest.json');
 const results=await page.evaluate(async base=>{
  const root=base+'/site/live/',{initializeSkia,SkiaRenderer,projectedDensity}=await import(root+'skia-renderer-v23.mjs'),wasm=await import(root+'wasm/laymesh_wasm.js');await wasm.default();const ck=await initializeSkia(root+'skia/canvaskit.wasm'),results=[];
  for(const index of[0,11,16]){
   const doc=await(await fetch(base+'/fixtures/pages/'+index+'.json')).json(),job=wasm.prepare_display_list(JSON.stringify(doc.list)),width=3840,height=1840,scale=Math.min(width/doc.list.width,height/doc.list.height),dx=(width/scale-doc.list.width)/2,dy=(height/scale-doc.list.height)/2,view=[-dx,-dy,doc.list.width+dx,doc.list.height+dy],r=new SkiaRenderer(ck,new OffscreenCanvas(width,height),{software:true,premultipliedRaster:true,clipToPage:true});
   await r.install(doc.list,()=>{throw Error('Unexpected image');},1);r.ensureSurface(width,height);r.preparationTileSize=256;
   const states=JSON.parse(job.presentation_state(.65,false)),work=r.sceneTasks(view,width,height,states),fixed=function*(){for(;;){r.preparationTileSize=256;const step=work.next();if(step.done)return;yield step.value;}};await r.prepareTasks(fixed());
   const original=r.drawLayer,alternate=function(c,node,id,p,opacity){
    if(c.quickReject(p.bounds))return true;const required=projectedDensity(c.getTotalMatrix(),p.bounds);if(!Number.isFinite(required)||required<=0)return false;const entry=this.buildLayer(p,required);if(!entry)return false;
    const paint=new ck.Paint();paint.setAlphaf(Math.max(0,Math.min(1,opacity)));entry.used=this.frameCount;c.save();
    try{c.scale(1/entry.density,1/entry.density);for(const {image,x,y,w,h}of entry.tiles){c.save();try{c.clipRect([x,y,x+w,y+h],ck.ClipOp.Intersect,false);c.drawImageOptions(image,x-2,y-2,ck.FilterMode.Linear,ck.MipmapMode.None,paint);}finally{c.restore();}}}finally{c.restore();paint.delete();}return true;
   };
   const checks=[],timings=[];
   for(const time of[.65,doc.inspection.animation.duration]){
    const s=JSON.parse(job.presentation_state(time,false)),pixels=[];
    for(const fn of[original,alternate]){r.drawLayer=fn;r.render(view,width,height,s,1,false,true,true);pixels.push(r.readPixels());}
    let max=0,sum=0,large=0;for(let i=0;i<pixels[0].length;i++){const d=Math.abs(pixels[0][i]-pixels[1][i]);max=Math.max(max,d);sum+=d;large+=d>8;}checks.push({time,max,mean:sum/pixels[0].length,over8:large/pixels[0].length});
    for(let round=0;round<6;round++)for(const variant of round%2?[1,0]:[0,1]){r.drawLayer=variant?alternate:original;const v=r.render(view,width,height,s,1,false,true,true);timings.push({time,round,variant,rasterMs:v.rasterMs,layerBuilds:v.layerCacheBuilds,staticBuilds:v.staticCacheBuilds});}
   }
   r.dispose();job.free();results.push({page:doc.id,checks,timings});
  }
  return results;
 },base);fs.mkdirSync(path.dirname(out),{recursive:true});fs.writeFileSync(out,JSON.stringify(results,null,2));
 for(const r of results){const mean=v=>{const xs=r.timings.filter(x=>x.variant===v&&x.round>0);return xs.reduce((s,x)=>s+x.rasterMs,0)/xs.length;};console.log(r.page,JSON.stringify({checks:r.checks,beforeMs:mean(0),afterMs:mean(1)}));assert(r.timings.every(x=>!x.layerBuilds&&!x.staticBuilds));}
 }finally{await browser.close();}})().catch(e=>{console.error(e);process.exitCode=1;});
