// Device-resolution cache oracle. Timings on software drivers are functional QA only.
const fs=require('fs');const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const [url,output]=process.argv.slice(2);if(!url||!output)throw Error('Usage: test-skia-static-cache.cjs SITE_URL OUTPUT.json');
(async()=>{const browser=await chromium.launch({headless:true,executablePath:process.env.CHROMIUM_PATH,args:['--no-sandbox','--no-proxy-server']});try{const page=await browser.newPage();await page.goto(new URL('site/live/manifest.json',url).href);const results=await page.evaluate(async lifecycleOnly=>{
 const base=new URL('./',location),{initializeSkia,SkiaRenderer}=await import(new URL('skia-renderer.mjs',base)),meta=await(await fetch(new URL('skia/manifest.json',base))).json(),ck=await initializeSkia(new URL('skia/'+meta.wasm,base).href),wasm=await import(new URL('wasm/laymesh_wasm.js',base));await wasm.default();
 const manifest=await(await fetch(base+'manifest.json')).json();await Promise.all(manifest.fonts.map(async f=>wasm.register_preview_font('/@bundled/'+f.filename,new Uint8Array(await(await fetch(new URL(f.full?.file||f.file,base))).arrayBuffer()))));
 const {stressFiles,stressCases}=await import('/stress-scenes.mjs'),results=[];
 for(const index of (lifecycleOnly?[2]:[2,6,9])){const spec=stressCases[index],input=stressFiles(spec),job=wasm.prepare_render(input.files[input.entry],'/'+input.entry,JSON.stringify(Object.fromEntries(Object.entries(input.files).map(([k,v])=>['/'+k,v])))),draw=job.draw_job(),list=JSON.parse(draw.display_list()),r=new SkiaRenderer(ck,new OffscreenCanvas(3840,2160));await r.install(list,id=>draw.resource_bytes(id),1);
 const state=JSON.parse(draw.frame_state(.65,false)),viewport=[0,0,320,180],paint=async cache=>{r.gl?.finish();await new Promise(resolve=>setTimeout(resolve,20));return r.render(viewport,3840,2160,state,1,true,cache,false);};
 const plain=await paint(false),before=r.readPixels(),first=await paint(true),cached=r.readPixels(),again=await paint(true);let sum=0,max=0,n=0;for(let i=0;i<before.length;i++){const delta=Math.abs(before[i]-cached[i]);sum+=delta;max=Math.max(max,delta);if(delta>8)n++;}
 const result={scene:spec.id,plainMs:plain.rasterMs,firstMs:first.rasterMs,warmMs:again.rasterMs,textureBytes:first.staticTextureBytes,hits:again.staticCacheHits,builds:again.staticCacheBuilds,maxDelta:max,meanDelta:sum/before.length,channelsOver8:n};results.push(result);console.log(JSON.stringify(result));if(!again.staticCacheHits||again.staticCacheBuilds)throw Error('No warm cache reuse');if(sum/before.length>.6||n/before.length>.004)throw Error('Cache changed pixels');
 if(index===2){
  const clone=structuredClone(list);clone.nodes.at(-1).opacity=.4;
  await r.install(clone,id=>draw.resource_bytes(id),1);const edited=await paint(true);
  if(edited.staticCacheHits!==1||edited.staticCacheBuilds!==1)throw Error('Edit did not invalidate only the changed static run');
  viewport[2]=240;viewport[3]=135;const zoomed=await paint(true);if(zoomed.staticCacheBuilds!==2||zoomed.staticCacheHits)throw Error('Zoom reused stale-resolution tiles');
  await r.install(clone,id=>draw.resource_bytes(id),1,1);const limited=await paint(true);if(limited.staticTextureBytes>524288)throw Error('Cache exceeded half the configured budget');
  await r.install(clone,id=>draw.resource_bytes(id),1,0);const disabled=await paint(true);if(disabled.staticTextureBytes||disabled.staticCacheHits)throw Error('Zero budget retained textures');
  await r.install(clone,id=>draw.resource_bytes(id),1);await paint(true);
  const lose=r.gl?.getExtension('WEBGL_lose_context');if(lose){lose.loseContext();for(let i=0;i<100&&!r.lost;i++)await new Promise(resolve=>setTimeout(resolve,10));if(!r.lost||r.staticTextureBytes)throw Error('Context loss retained GPU tiles');lose.restoreContext();for(let i=0;i<100&&r.lost;i++)await new Promise(resolve=>setTimeout(resolve,10));if(r.lost)throw Error('Context did not restore');const recovered=await paint(true);if(!recovered.staticCacheBuilds)throw Error('Context restoration did not rebuild GPU tiles');}
  result.lifecycle=['partial edit invalidation','viewport rebuild','bounded eviction','zero budget','context loss and restoration'];
 }
 r.trim();if(r.staticTextureBytes||r.staticTextures.size)throw Error('Pause did not release textures');r.dispose();draw.free();job.free();
 }return results;
},process.argv.includes('--lifecycle-only'));fs.writeFileSync(output,JSON.stringify(results,null,2));console.log(JSON.stringify(results));}finally{await browser.close();}})().catch(e=>{console.error(e);process.exitCode=1});
