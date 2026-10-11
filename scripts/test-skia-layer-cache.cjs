// Exercise reuse/admission/lifecycle at 4K, not hardware throughput acceptance.
const fs=require('fs'),{chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const [url,output]=process.argv.slice(2);if(!url||!output)throw Error('Usage: test-skia-layer-cache.cjs SITE_URL OUTPUT.json');
(async()=>{const browser=await chromium.launch({headless:true,executablePath:process.env.CHROMIUM_PATH,args:['--no-sandbox','--no-proxy-server']});try{
 const page=await browser.newPage();await page.goto(new URL('site/live/manifest.json',url).href);
 const results=await page.evaluate(async()=>{
  const base=new URL('./',location),{initializeSkia,SkiaRenderer,projectedDensity}=await import(new URL('skia-renderer.mjs',base)),meta=await(await fetch(new URL('skia/manifest.json',base))).json(),ck=await initializeSkia(new URL('skia/'+meta.wasm,base).href),wasm=await import(new URL('wasm/laymesh_wasm.js',base));await wasm.default();
  const manifest=await(await fetch(new URL('manifest.json',base))).json();await Promise.all(manifest.fonts.map(async f=>wasm.register_preview_font('/@bundled/'+f.filename,new Uint8Array(await(await fetch(new URL(f.full?.file||f.file,base))).arrayBuffer()))));
  const {stressCases,stressFiles}=await import('/stress-scenes.mjs'),results=[];
  if(projectedDensity([2,0,0,0,2,0,0,0,1],[0,0,10,10])!==2||Number.isFinite(projectedDensity([1,0,0,0,1,0,-.2,0,1],[0,0,10,10])))throw Error('Projection density bound');
  for(const index of [7,8,9]){
   const input=stressFiles(stressCases[index]),job=wasm.prepare_render(input.files[input.entry],'/'+input.entry,JSON.stringify(Object.fromEntries(Object.entries(input.files).map(([k,v])=>['/'+k,v])))),draw=job.draw_job(),list=JSON.parse(draw.display_list()),r=new SkiaRenderer(ck,new OffscreenCanvas(3840,2160));await r.install(list,id=>draw.resource_bytes(id),1);
   let states=JSON.parse(draw.frame_state(.65,false)),viewport=[0,0,320,180];
   const paint=async(enabled=true)=>{let metrics;for(let attempt=0;attempt<300;attempt++){r.gl.finish();await new Promise(resolve=>setTimeout(resolve,20));metrics=r.render(viewport,3840,2160,states,1,true,true,enabled);if(!metrics.skipped)break;}if(metrics.skipped||metrics.layerCacheAllocationFailures)throw Error('Cache draw failed '+stressCases[index].id+' '+JSON.stringify(metrics));if(metrics.layerTextureBytes+metrics.staticTextureBytes>r.budget*.75)throw Error('Texture budget exceeded');return metrics;};
   const first=await paint(),warm=await paint();if(!first.layerCacheBuilds||!warm.layerCacheHits||warm.layerCacheBuilds)throw Error('Moving objects not reused');
   states=JSON.parse(draw.frame_state(1.48,false));const moved=await paint(),settled=await paint();if(!settled.layerCacheHits||settled.layerCacheBuilds)throw Error('Unchanged density rebuilt');
   const disabled=await paint(false);if(disabled.layerCacheHits||disabled.layerCacheBuilds)throw Error('A/B off still cached');
   const result={scene:stressCases[index].id,firstBuilds:first.layerCacheBuilds,warmHits:warm.layerCacheHits,warmBuilds:warm.layerCacheBuilds,changedPoseBuilds:moved.layerCacheBuilds,textureBytes:settled.layerTextureBytes,staticTextureBytes:settled.staticTextureBytes};
   if(index===7){
    const changed=structuredClone(list),target=Number(Object.keys(states).find(id=>!id.includes('/')&&changed.nodes[Number(id)].kind.kind==='group'));changed.nodes[target].kind.children[0].opacity=.51;
    await r.install(changed,id=>draw.resource_bytes(id),1);const edited=await paint();if(!edited.layerCacheHits||!edited.layerCacheBuilds)throw Error('Local edit did not preserve other objects');
    viewport=[0,0,240,135];const zoomed=await paint();if(!zoomed.layerCacheBuilds)throw Error('Zoom kept insufficient textures');
    await r.install(changed,id=>draw.resource_bytes(id),1,16);await paint();const limited=await paint();if(!limited.layerCacheBudgetMisses||limited.layerCacheBuilds||!limited.layerCacheHits)throw Error('Budget boundary churned instead of keeping working set');
    result.limitedBudget={hits:limited.layerCacheHits,builds:limited.layerCacheBuilds,budgetMisses:limited.layerCacheBudgetMisses,bytes:limited.layerTextureBytes+limited.staticTextureBytes};
    await r.install(changed,id=>draw.resource_bytes(id),1,0);const zero=await paint();if(zero.layerTextureBytes||zero.staticTextureBytes||zero.layerCacheHits)throw Error('Zero budget retained textures');
    await r.install(changed,id=>draw.resource_bytes(id),1);await paint();const loss=r.gl.getExtension('WEBGL_lose_context');if(loss){loss.loseContext();for(let i=0;i<100&&!r.lost;i++)await new Promise(resolve=>setTimeout(resolve,10));if(!r.lost||r.layerTextureBytes)throw Error('Lost context retained layers');loss.restoreContext();for(let i=0;i<100&&r.lost;i++)await new Promise(resolve=>setTimeout(resolve,10));if(r.lost||!(await paint()).layerCacheBuilds)throw Error('Context failed to restore layers');}
    result.lifecycle=['partial edit','zoom','bounded admission without rebuild cascade','zero budget','context recovery'];
   }
   r.trim();if(r.layerTextureBytes||r.layerTextures.size)throw Error('Pause retained layers');r.dispose();draw.free();job.free();results.push(result);
  }
  return {status:'passed',hardwareAcceptance:false,checks:results};
 });fs.writeFileSync(output,JSON.stringify(results,null,2)+'\n');console.log(JSON.stringify(results,null,2));
}finally{await browser.close();}})().catch(e=>{console.error(e);process.exitCode=1;});
