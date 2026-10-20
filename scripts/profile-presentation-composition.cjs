// Fixed-state diagnostic, separate from natural playback and resource acceptance.
const fs=require('node:fs'),path=require('node:path');
const pw=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const [base,out]=process.argv.slice(2),engine=process.env.ENGINE||'firefox';
(async()=>{
 const browser=await pw[engine].launch({headless:true,args:engine==='chromium'&&process.env.HARDWARE_GPU==='1'?['--enable-gpu','--use-gl=angle','--use-angle=vulkan','--enable-features=Vulkan']:[]});
 try{
  const page=await browser.newPage();await page.goto(base+'/site/live/manifest.json');
  await page.evaluate(async({base,software})=>{
   const root=base+'/site/live/',{initializeSkia}=await import(root+'skia-renderer.mjs'),{PresentationRenderer}=await import(root+'presentation-renderer.mjs'),w=await import(root+'wasm/laymesh_wasm.js');await w.default();
   globalThis.probe={base,software,PresentationRenderer,w,ck:await initializeSkia(root+'skia/canvaskit.wasm')};
  },{base,software:process.env.HARDWARE_GPU!=='1'&&process.env.SOFTWARE_GPU!=='1'});
  const results=[];
  for(const number of (process.env.PAGES||'1,12,17').split(',').map(Number)){
   const result=await page.evaluate(async({number,width,height})=>{
    const {base,software,PresentationRenderer,w,ck}=probe,doc=await(await fetch(base+'/fixtures/pages/'+(number-1)+'.json')).json();
    const r=new PresentationRenderer(ck,new OffscreenCanvas(width,height),{software}),s=r.skia,job=w.prepare_display_list(JSON.stringify(doc.list)),sample=Object.assign((t,reduced)=>JSON.parse(job.presentation_state(t,reduced)),{duration:doc.inspection.animation?.duration||0,transform:(t,reduced)=>JSON.parse(job.transform_state(t,reduced))});
    let phase;const content=[];for(const key of ['content','drawNode']){const original=s[key].bind(s);s[key]=function(c,n,id,...args){const at=performance.now(),r=original(c,n,id,...args),ms=performance.now()-at;if(ms>20)content.push({key,id,name:n.name,kind:n.kind.kind,fill:n.kind.fill?.kind,ms,bounds:s.prepare(n).bounds});return r;};}const effects=[];const effect=s.drawEffect.bind(s);s.drawEffect=function(...args){const at=performance.now(),result=effect(...args);effects.push({id:args[2],ms:performance.now()-at,blur:args[5].spec?.blur,spread:args[5].spec?.spread});return result;};const wrap=(obj,key,label)=>{const original=obj[key].bind(obj);obj[key]=function(...args){const start=performance.now(),value=original(...args);if(phase)phase[label]=(phase[label]||0)+performance.now()-start;return value;};};
    wrap(s,'drawStaticRun','staticMs');const coverage=s.opaqueRasterTile.bind(s);let coverageMs=0,coverageMaxMs=0;s.opaqueRasterTile=function(...args){const at=performance.now(),result=coverage(...args),ms=performance.now()-at;coverageMs+=ms;coverageMaxMs=Math.max(coverageMaxMs,ms);return result;};
    const layer=s.drawLayer.bind(s);s.drawLayer=function(c,node,id,p,opacity){const before=s.cpuLayerBlits||0,m=Array.from(c.getTotalMatrix()),start=performance.now(),value=layer(c,node,id,p,opacity);if(phase)phase.layers.push({id,name:node.name,ms:performance.now()-start,m,opacity,cpuBlit:(s.cpuLayerBlits||0)>before,density:s.layerTextures.get(p.textureKey??p.uid)?.density});return value;};
    const installed=performance.now();await r.install(doc.id,doc.list,()=>{throw Error('Unexpected image');});const installMs=performance.now()-installed;
    const entry=await r.prepareEntry(doc.id,width,height,0,false,sample);wrap(s.surface,'flush','flushMs');
    const frames=[];
    for(const time of [.65,1,sample.duration])for(let round=0;round<4;round++){
     phase={time,round,layers:[]};const rendered=r.render(doc.id,width,height,sample(time,false),false);Object.assign(phase,{rasterMs:rendered.rasterMs,layerBuilds:rendered.layerCacheBuilds,staticBuilds:rendered.staticCacheBuilds,backend:rendered.backend});frames.push(phase);phase=null;
    }
    const result={page:doc.id,width,height,installMs,entry,content:content.sort((a,b)=>b.ms-a.ms).slice(0,20),coverageMs,coverageMaxMs,nativePromotionMs:s.nativePromotionMs,nativeReadbackMs:s.nativeReadbackMs,nativeUploadMs:s.nativeUploadMs,effects:effects.sort((a,b)=>b.ms-a.ms).slice(0,30),frames};r.dispose();job.free();return result;
   },{number,width:Number(process.env.WIDTH||3840),height:Number(process.env.HEIGHT||1840)});
   results.push(result);fs.mkdirSync(path.dirname(out),{recursive:true});fs.writeFileSync(out,JSON.stringify({engine,version:browser.version(),results},null,2));
   const f=result.frames.filter(x=>x.round>0),avg=fn=>f.reduce((n,x)=>n+fn(x),0)/f.length;
   console.log(result.page,JSON.stringify({entry:result.entry.prepareMs,raster:avg(x=>x.rasterMs),static:avg(x=>x.staticMs||0),layers:avg(x=>x.layers.reduce((n,l)=>n+l.ms,0)),flush:avg(x=>x.flushMs||0)}));
  }
 }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
