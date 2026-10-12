// Exercise every usable pinned recipe in WASM, then the real shared catalog UI.
const fs=require('fs'),path=require('path'),assert=require('assert/strict'),crypto=require('crypto');
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const [base,output]=process.argv.slice(2);if(!base||!output)throw Error('Usage: test-library-catalog.cjs SITE_URL OUTPUT_DIRECTORY [--recipes-only]');
const root=path.resolve(__dirname,'..'),catalog=JSON.parse(fs.readFileSync(path.join(root,'site/style-libraries.json'),'utf8'));
fs.mkdirSync(output,{recursive:true});const errors=[],checks=[];
function instrument(){
 window.libraryQA={frames:[]};const Original=window.Worker;
 window.Worker=class extends Original {constructor(url,options){super(url,options);if(String(url).endsWith('/worker.js')){this.requests=new Map();this.addEventListener('message',({data})=>{if(data.type==='result'){libraryQA.worker=this;libraryQA.inspection=data.inspection;}if(data.type==='painted'&&this.requests.has(data.id))libraryQA.frames.push(this.requests.get(data.id));});}}
 postMessage(data,...args){if(data.type==='paint')this.requests?.set(data.id,{time:data.time,generation:data.generation});super.postMessage(data,...args);}
 };
}
async function ready(page,old){await page.waitForFunction(old=>{const m=document.querySelector('.style-library-catalog .example-module')||document.querySelector('.example-module');if(m?.dataset.liveState==='error')throw Error(m.querySelector('.live-diagnostics').textContent);return m?.dataset.liveState==='success'&&m.dataset.painted==='true'&&(!old||m.dataset.generation!==old);},old,{timeout:90000});}
async function pixels(page){const bytes=await page.evaluate(()=>new Promise((resolve,reject)=>{const w=libraryQA.worker,id='library-capture-'+performance.now(),timer=setTimeout(()=>reject(Error('Capture timeout')),20000);const handler=({data})=>{if(data.id!==id)return;w.removeEventListener('message',handler);clearTimeout(timer);data.error?reject(Error(JSON.stringify(data.error))):resolve(Array.from(data.rgba));};w.addEventListener('message',handler);w.postMessage({type:'capture',id});}));return crypto.createHash('sha256').update(Buffer.from(bytes)).digest('hex');}
async function select(page,name){const current=await page.locator('.style-library-catalog .example-module').getAttribute('data-generation');const entry=await page.evaluate(name=>JSON.parse(document.querySelector('.library-data').textContent).entries.find(e=>e.name===name&&e.sample)?.id,name);assert(entry,name);await page.locator('.library-items').selectOption(entry);await ready(page,current);assert.equal(await page.locator('.example-module').count(),1);}
(async()=>{const browser=await chromium.launch({headless:true,...(process.env.CHROMIUM_PATH?{executablePath:process.env.CHROMIUM_PATH}:{}),args:['--no-sandbox','--no-proxy-server']});try{
 const page=await browser.newPage({viewport:{width:1440,height:1000},reducedMotion:'no-preference'});page.on('pageerror',e=>errors.push(String(e)));await page.addInitScript(instrument);page.setDefaultTimeout(60000);
 await page.goto(new URL('site/live/manifest.json',base).href);
 if(!process.argv.includes('--ui-only'))for(const library of catalog.libraries){
  const usable=library.entries.filter(e=>e.sample),result=await page.evaluate(async({base,entries})=>{
   const wasm=await import(new URL('wasm/laymesh_wasm.js',base));await wasm.default();const manifest=await(await fetch(new URL('manifest.json',base))).json();
   for(const f of manifest.fonts){const data=await(await fetch(new URL(f.file,base))).arrayBuffer();wasm.register_preview_font(f.filename,new Uint8Array(data));}
   const {initializeSkia,SkiaRenderer}=await import(new URL('skia-renderer.mjs',base)),meta=await(await fetch(new URL('skia/manifest.json',base))).json();
   const skia=await initializeSkia(new URL('skia/'+meta.wasm,base).href),canvas=document.createElement('canvas');canvas.style.width='200px';canvas.style.height='200px';document.body.append(canvas);const renderer=new SkiaRenderer(skia,canvas,{software:true});
   let staticCount=0,motionCount=0;const started=performance.now();
   try{for(const [i,e]of entries.entries()){
    let job,draw;try{job=wasm.prepare_render(e.sample,'/catalog.lay','{}');const info=JSON.parse(job.inspection());
     if(info.warnings?.some(w=>w.file==='/catalog.lay'&&['W_CSS','W_ANIMATION'].includes(w.code)))throw Error(JSON.stringify(info.warnings));
     draw=job.draw_job();await renderer.install(JSON.parse(draw.display_list()),id=>draw.resource_bytes(id),i+1);
     if(info.inspection.animation){const state=JSON.parse(draw.frame_state(.4,false));renderer.render([0,0,100,100],96,96,state);const sample=JSON.parse(draw.animation_sample(.4,false));if(!sample)throw Error('Missing animation sample');motionCount++;}
     else{renderer.render([0,0,100,100],96,96);staticCount++;}
     if(!renderer.readPixels()?.length)throw Error('Empty rendering');
    }catch(error){throw Error(e.id+': '+error);}finally{draw?.free();job?.free();}
    if(i%40===0)await new Promise(resolve=>setTimeout(resolve,0));
   }}finally{renderer.dispose();canvas.remove();}
   return{static:staticCount,motion:motionCount,seconds:(performance.now()-started)/1000};
  },{base:new URL('site/live/',base).href,entries:usable});checks.push({library:library.name,recipes:usable.length,...result});console.log(JSON.stringify(checks.at(-1)));
 }
 if(!process.argv.includes('--recipes-only')){
 for(const lib of catalog.libraries){
  const topic={bootstrap:'style-bootstrap', 'open-props':'style-open-props',animate:'style-animate-library'}[lib.name];
  for(const locale of ['zh-CN','en']){
   await page.goto(new URL((locale==='en'?'en/docs/topics/'+topic+'.html':'docs/topics/'+topic+'.zh-CN.html'),base).href);
   const toc=page.locator('.toc-toggle');if(await toc.getAttribute('aria-expanded')==='true')await toc.click();
   await page.locator('.style-library-catalog').scrollIntoViewIfNeeded();await ready(page);
   const data=await page.locator('.library-data').textContent();assert.equal(JSON.parse(data).entries.length,lib.entries.length);assert.equal(await page.locator('.style-library-catalog .example-module').count(),1);
   const source=page.locator('.style-library-catalog .source-panel .cm-content').first();
   for(const name of {bootstrap:['shadow','rounded-pill','text-success'], 'open-props':['--red-6','--gradient-7','--shadow-3','--radius-4','--animation-fade-in'],animate:['animate__bounce','animate__fadeInUp','animate__slow','animate__repeat-2']}[lib.name])await select(page,name);
   if(lib.name==='bootstrap'){
    await select(page,'shadow');const before=await pixels(page),old=await page.locator('.style-library-catalog .example-module').getAttribute('data-generation');
    await page.locator('.library-tuning summary').click();await page.locator('.library-property').selectOption('box-shadow-blur');await page.locator('.library-value').fill('2mm');await page.locator('.library-dpi').fill('144');await page.locator('.library-apply').click();await ready(page,old);assert.notEqual(await pixels(page),before);assert((await source.innerText()).includes('box-shadow-blur: 2mm !important'));
    const previous=await page.locator('.style-library-catalog .example-module').getAttribute('data-generation');await page.locator('.library-dpi').fill('96');await page.locator('.library-apply').click();await ready(page,previous);assert.equal((await source.innerText()).match(/layout_dpi=/g).length,1);
    await page.locator('.library-tuning summary').click();
   }
   if(lib.name==='animate'){
    await select(page,'animate__fadeInUp');const original=await pixels(page);await page.locator('.library-play').click();
    await page.waitForFunction(()=>libraryQA.frames.some(f=>f.time>0&&f.time<1));
    await page.locator('.preview-options-menu summary').first().click();await page.locator('.preview-motion-menu summary').click();
    await page.locator('[aria-label="Animation time"]').fill('0.4');await page.locator('[aria-label="Animation time"]').dispatchEvent('input');
    await page.waitForFunction(()=>libraryQA.frames.at(-1)?.time===.4);assert.notEqual(await pixels(page),original);
    await page.locator('.library-static').click();await page.waitForFunction(()=>libraryQA.frames.at(-1)?.time===null);assert.equal(await pixels(page),original);
    await page.locator('.preview-options-menu summary').first().click();
   }
   await page.locator('.library-search').fill('shadow');assert((await page.locator('.library-items option').allTextContents()).every(s=>s.includes('shadow')||lib.entries.some(e=>e.name===s.split(' · ')[0]&&JSON.stringify(e).includes('shadow'))));await page.locator('.library-search').fill('');
   const unsupported=lib.entries.find(e=>e.status==='unsupported');if(unsupported){await page.locator('.library-status').selectOption('unsupported');await page.locator('.library-items').selectOption(unsupported.id);assert(await page.locator('.style-library-catalog .example-module').isHidden());await page.locator('.library-status').selectOption('');await select(page,{bootstrap:'bg-primary','open-props':'--gradient-7',animate:'animate__fadeInUp'}[lib.name]);}
   await page.locator('.library-filter').scrollIntoViewIfNeeded();await page.screenshot({path:path.join(output,`${lib.name}-${locale}.png`)});
   await page.setViewportSize({width:390,height:844});assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1));await page.setViewportSize({width:1440,height:1000});
   checks.push({library:lib.name,locale,UI:'selection, source, controls, exact static restore, unsupported reasons and mobile fit'});
  }
 }
 }
 assert.deepEqual(errors,[]);fs.writeFileSync(path.join(output,'verification.json'),JSON.stringify({checks,errors,inventory:catalog.libraries.map(l=>({name:l.name,total:l.entries.length})),browser:'Chromium/Playwright'},null,2));
 console.log(JSON.stringify({passed:checks.length,errors}));
}finally{await browser.close();}})().catch(e=>{console.error(e);process.exitCode=1;});
