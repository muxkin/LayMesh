// Compile the exact stress inputs without a renderer; hashing is outside timing.
const fs=require('fs'),crypto=require('crypto'),{chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const [url,output]=process.argv.slice(2);if(!url||!output)throw Error('Usage: profile-stress-compile.cjs SITE_URL OUTPUT.json');
(async()=>{const browser=await chromium.launch({headless:true,executablePath:process.env.CHROMIUM_PATH,args:['--no-sandbox','--no-proxy-server']});try{
 const page=await browser.newPage();await page.goto(new URL('site/live/manifest.json',url).href);
 await page.exposeFunction('digest',text=>crypto.createHash('sha256').update(text).digest('hex'));
 const report=await page.evaluate(async()=>{
  const base=new URL('./',location),wasm=await import(new URL('wasm/laymesh_wasm.js',base));await wasm.default();
  const manifest=await(await fetch(new URL('manifest.json',base))).json();await Promise.all(manifest.fonts.map(async f=>wasm.register_preview_font('/@bundled/'+f.filename,new Uint8Array(await(await fetch(new URL(f.full?.file||f.file,base))).arrayBuffer()))));
  const {stressCases,stressFiles}=await import('/stress-scenes.mjs'),rows=[];
  for(const index of [2,8,9]){const input=stressFiles(stressCases[index]),source=input.files[input.entry],files=JSON.stringify(Object.fromEntries(Object.entries(input.files).map(([k,v])=>['/'+k,v])));
   for(let iteration=0;iteration<3;iteration++){
    let start=performance.now();const job=wasm.prepare_render(source,'/'+input.entry,files),compile=performance.now()-start;
    start=performance.now();const draw=job.draw_job(),lower=performance.now()-start;
    start=performance.now();const display=draw.display_list(),inspection=job.inspection(),serialize=performance.now()-start;
    rows.push({scene:stressCases[index].id,iteration,sourceBytes:new TextEncoder().encode(source).length,compile,lower,serialize,displayListSha256:await window.digest(display),inspectionSha256:await window.digest(inspection)});
    draw.free();job.free();
   }
  }
  return {definition:'Three sequential runs per input, one isolated Chromium/WASM instance, full bundled fonts, no Skia renderer or playback. Hashing and initialization excluded.',rows};
 });fs.writeFileSync(output,JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report));
}finally{await browser.close();}})().catch(error=>{console.error(error);process.exitCode=1;});
