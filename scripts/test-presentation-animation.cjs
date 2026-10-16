// Natural playback of each authored animation, driven by trusted navigation keys.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),crypto=require('node:crypto'),{pathToFileURL}=require('node:url');
const pw=require(process.env.PLAYWRIGHT_MODULE||'playwright'),instrument=require('./presentation-animation-instrument.cjs');
const [file,out]=process.argv.slice(2);assert(file&&out);fs.mkdirSync(out,{recursive:true});
(async()=>{for(const engine of (process.env.ENGINES||'chromium,firefox,webkit').split(',')){
 const browser=await pw[engine].launch({headless:true});try{
  const p=await browser.newPage({viewport:{width:Number(process.env.WIDTH||1920),height:Number(process.env.HEIGHT||920)}}),errors=[],network=[];p.setDefaultTimeout(180000);p.on('pageerror',e=>errors.push(e.message));
  await p.route(/^https?:\/\//,r=>{network.push(r.request().url());return r.abort();});
  await p.goto(pathToFileURL(path.resolve(file)).href);await p.waitForFunction(()=>window.__LAYMESH_WEB?.presentation&&document.querySelector('#app').dataset.painted==='true');
  await p.evaluate('('+instrument.toString()+')(__LAYMESH_WEB.presentation)');
  if(process.env.GPU_PREPARATION)await p.evaluate(()=>{const v=__LAYMESH_WEB.presentation,init=v.initRenderer.bind(v);v.initRenderer=async()=>{await init();const request=v.worker.request.bind(v.worker);v.worker.request=(r,data,transfer)=>request(r,{...data,cpuPreparation:false},transfer);};});
  if(process.env.EDIT_LCSS)await p.waitForFunction(()=>!!__LAYMESH_WEB.editor);
  if(process.env.EDIT_LCSS)await p.evaluate(()=>{const w=__LAYMESH_WEB;w.editor.files['/project/theme.lcss']+='\n* {color:#c61717 !important; border-color:#c61717 !important;}';w.editor.schedule(true);});
  await p.locator('#preview-more > summary').click();await p.locator('#from-start').click();
  const rounds=Number(process.env.ROUNDS||2),pages=Number(process.env.PAGES||6);
  for(let round=0;round<rounds;round++)for(let index=0;index<pages;index++){
   if(round||index){await p.keyboard.press(index===0?'Home':index%2?'ArrowDown':'ArrowRight');}
   await p.waitForFunction(index=>{const v=__LAYMESH_WEB.presentation;return v.ready&&!v.switching&&v.index===index;},index);
   assert(await p.evaluate(()=>{const v=__LAYMESH_WEB.presentation,r=v.root.getBoundingClientRect(),c=v.canvas.getBoundingClientRect();return c.x===r.x&&c.y===r.y&&c.width===r.width&&c.height===r.height&&document.elementFromPoint(r.x+r.width/2,r.y+r.height/2)===v.canvas;}),'The active renderer must occupy the visible presentation surface');
   await p.waitForFunction(()=>{const v=__LAYMESH_WEB.presentation;return !v.clock.running&&!v.painting&&!v.switching;});
   console.log(engine,'played',round+1,index+1);
   if(round===0&&index===1)await p.screenshot({path:path.join(out,engine+'-seed.png')});
  }
  const result=await p.evaluate(()=>({...animationMeasure,backend:__LAYMESH_WEB.presentation.metrics.backend,dimensions:__LAYMESH_WEB.presentation.dimensions(),usesWorker:!!__LAYMESH_WEB.presentation.worker,fallback:__LAYMESH_WEB.presentation.rendererFallback,resident:[...__LAYMESH_WEB.presentation.resident],resources:__LAYMESH_WEB.presentation.metrics.resources,initialEdit:__LAYMESH_WEB.previewCompiler?.version?__LAYMESH_WEB.diagnostics().firstEdit:null}));
  assert.equal(result.visits.length,rounds*pages);assert.deepEqual(errors,[]);assert.deepEqual(network,[]);assert(result.calls.every(c=>!c.result.transitionError),'No cross-renderer transition errors');
  Object.assign(result,{engine,version:browser.version(),htmlSha256:crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex'),errors,network,definition:'Trusted navigation keys after each authored animation finishes. No seeking, rehearsal, all-page readiness wait, or animation duration changes. Frame intervals count successful render replies, including intervening skipped attempts; GPU scanout is not measured.'});
  fs.writeFileSync(path.join(out,engine+'.json'),JSON.stringify(result,null,2));console.log(engine,'natural playback passed',result.visits.length,'visits',result.calls.filter(c=>c.type==='paint').length,'frame attempts');
 }finally{await browser.close();}
}})().catch(e=>{console.error(e);process.exitCode=1;});
