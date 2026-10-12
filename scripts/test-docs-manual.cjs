// Real documentation interactions: one shared WASM preview for all 97 effects.
const fs=require('fs'),path=require('path'),assert=require('assert/strict'),crypto=require('crypto');
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const [base,out]=process.argv.slice(2);
if(!base||!out)throw Error('Usage: test-docs-manual.cjs SITE_URL OUTPUT_DIRECTORY');
fs.mkdirSync(out,{recursive:true});
const root=path.resolve(__dirname,'..'),effects=JSON.parse(fs.readFileSync(path.join(root,'site/animation-effects.json')));
const url=(relative)=>new URL(relative,base.endsWith('/')?base:base+'/').href;
const checks=[],errors=[];
function instrument(){
 const Original=window.Worker;
 window.manualQA={paints:[],frames:[],presentationFrames:[],workers:[]};
 window.Worker=class extends Original{
  constructor(...args){super(...args);manualQA.workers.push(this);this.qaRequests=new Map();this.qaPresentation=String(args[0]).includes('presentation-worker.mjs');if(this.qaPresentation)manualQA.presentationWorker=this;
   this.addEventListener('message',({data})=>{if(data.type==='result'){manualQA.worker=this;manualQA.inspection=data.inspection;}const request=this.qaRequests.get(data.id);if(data.type==='painted'&&request)manualQA.frames.push(request);if(this.qaPresentation&&data.type==='done'&&request&&!data.skipped)manualQA.presentationFrames.push(request);});
  }
  postMessage(data,...args){if(data.type==='paint'){const request={time:data.time,running:data.running,generation:data.generation};manualQA.paints.push(request);this.qaRequests.set(data.id,request);}super.postMessage(data,...args);}
 };
}
async function frame(page,time){
 await page.waitForFunction(time=>{
  const m=document.querySelector('.example-module'),last=manualQA.frames.at(-1);
  return m?.dataset.painted==='true'&&String(last?.generation)===m.dataset.generation&&last.time===time;
 },time,{timeout:15000});
}
async function ready(page,previous){
 await page.waitForFunction(previous=>{
  const m=document.querySelector('.example-module');
  if(m?.dataset.liveState==='error')throw Error(m.querySelector('.live-diagnostics').textContent);
  return m?.dataset.liveState==='success'&&m.dataset.painted==='true'&&(!previous||m.dataset.generation!==previous);
 },previous,{timeout:90000});
}
async function capture(page,presentation=false){
 const data=await page.evaluate(presentation=>new Promise((resolve,reject)=>{
  const worker=presentation?manualQA.presentationWorker:manualQA.worker,id='manual-capture-'+performance.now();
  const timer=setTimeout(()=>reject(Error('Capture timeout')),15000);
  const listener=({data})=>{if(data.id!==id)return;worker.removeEventListener('message',listener);clearTimeout(timer);if(data.error)reject(Error(JSON.stringify(data.error)));else resolve(Array.from(data.rgba));};
  worker.addEventListener('message',listener);worker.postMessage({type:'capture',id});
 }),presentation);
 return crypto.createHash('sha256').update(Buffer.from(data)).digest('hex');
}
async function navigate(page,relative){
 await page.goto(url(relative));
 const toc=page.locator('.toc-toggle');
 if(await toc.getAttribute('aria-expanded')==='true')await toc.click();
}
async function anchorProject(page){
 await navigate(page,'docs/topics/lcss.zh-CN.html#example-manual-style-project-source');
 await page.locator('.example-module').scrollIntoViewIfNeeded();await ready(page);
 const before=await page.evaluate(()=>manualQA.inspection),original=await capture(page);
 assert((before.warnings||[]).every(w=>w.code!=='W_EFFECT_OVERFLOW'));
 assert.equal(await page.locator('[data-source-mode],.summary-panel').count(),0);
 await page.locator('.source-files [role=tab]').filter({hasText:'style-project.lay'}).click();
 const source=page.locator('.source-panel:not([hidden]) .cm-content');await source.waitFor();
 const text=await source.innerText();assert(text.includes('card_size = (30mm,22mm)'));
 const previous=await page.locator('.example-module').getAttribute('data-generation');
 await source.fill(text.replace('card_size = (30mm,22mm)','card_size = (26mm,22mm)'));await ready(page,previous);
 const after=await page.evaluate(()=>manualQA.inspection);
 for(const [i,id] of ['library','custom','props'].entries()){
  const old=before.decorations.find(d=>d.id===id),now=after.decorations.find(d=>d.id===id);
  assert.equal(old.layout_bounds.width-now.layout_bounds.width,4);
  assert.equal(old.page_transform[4]-now.page_transform[4],i*4,id+' follows changed width');
  assert.equal(old.page_transform[5],now.page_transform[5]);
 }
 assert.notEqual(await capture(page),original);
 await source.blur();await page.screenshot({path:path.join(out,'anchor-width-edit.png')});
 const edited=await page.locator('.example-module').getAttribute('data-generation');
 await page.locator('.live-reset').click();await ready(page,edited);assert.equal(await capture(page),original);
 checks.push('Style project: editing one card width repositions both neighbours through anchors; reset restores exact pixels');
}
async function combinedProjects(page){
 await navigate(page,'docs/topics/style-animation-examples.zh-CN.html');
 assert.match(await page.title(),/综合案例/);
 assert.equal(await page.locator('.example-module').count(),3);
 for(const id of ['styled-cards','art-direction','complex-dashboard'])assert.equal(await page.locator('#'+id).count(),1);
 await navigate(page,'en/docs/topics/style-animation-examples.html');
 assert.equal(await page.locator('.example-module').count(),3);
 for(const [id,steps,duration,time,theme] of [
  ['styled-cards',3,3.5,.4,'styled-theme.lcss'],
  ['art-direction',3,7.6,6,'art-direction.lcss'],
  ['complex-dashboard',1,12,.4,'dashboard-theme.lcss']
 ]){
  await navigate(page,'examples/animation-'+id+'.html');
  await page.locator('.example-module').scrollIntoViewIfNeeded();await ready(page);
  const inspection=await page.evaluate(()=>manualQA.inspection);
  assert.equal(inspection.animation.steps.length,steps);assert.equal(inspection.animation.duration,duration);
  assert((inspection.warnings||[]).every(w=>w.code!=='W_CSS'||w.file.startsWith('@laymesh/')),id);
  const original=await capture(page);
  await page.locator('.preview-options-menu > summary').click();
  await page.locator('.preview-motion-menu > summary').click();
  await page.locator('[data-action="animation-replay"]').click();
  const generation=await page.locator('.example-module').getAttribute('data-generation');
  await page.waitForFunction(generation=>manualQA.frames.some(f=>String(f.generation)===generation&&f.running&&f.time>0),generation,{timeout:30000});
  await page.locator('[aria-label="Animation time"]').fill(String(time));
  await page.locator('[aria-label="Animation time"]').dispatchEvent('input');await frame(page,time);
  assert.notEqual(await capture(page),original,id+' animated pixels');
  await page.locator('.preview-options-menu > summary').click();
  await page.screenshot({path:path.join(out,id+'-motion.png')});
  await page.locator('.preview-options-menu > summary').click();
  await page.locator('.preview-motion-menu > summary').click();
  await page.locator('[data-action="animation-static"]').click();await frame(page,null);
  assert.equal(await capture(page),original,id+' static restoration');
  await page.locator('.preview-options-menu > summary').click();
  await page.evaluate(()=>manualQA.presentationFrames=[]);
  await page.locator('[data-action="present"]').click();
  await page.waitForFunction(()=>{const p=document.querySelector('.presentation');if(p?.dataset.error)throw Error(p.dataset.error);return p?.dataset.painted==='true';},null,{timeout:90000});
  await page.locator('.presentation').focus();
  await page.waitForFunction(()=>manualQA.presentationFrames.some(f=>f.time>.2),null,{timeout:30000});
  await page.keyboard.press('s');
  const moving=await capture(page,true);
  await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
  await page.screenshot({path:path.join(out,id+'-presentation.png')});
  await page.keyboard.press('ArrowRight');
  await page.waitForFunction(()=>Number(document.querySelector('.presentation')?.dataset.completed)>=1);
  await page.keyboard.press('ArrowLeft');
  await page.waitForFunction(()=>document.querySelector('.presentation')?.dataset.time==='0');
  assert.notEqual(await capture(page,true),moving,id+' presentation changes pixels and rewinds');
  await page.keyboard.press('Escape');await page.locator('.presentation').waitFor({state:'detached'});
  await frame(page,null);assert.equal(await capture(page),original,id+' exit restores preview');
  assert.equal(await page.locator('[data-source-mode],.summary-panel').count(),0);
  await page.locator('.source-files [role=tab]').filter({hasText:theme}).click();
  const source=page.locator('.source-panel:not([hidden]) .cm-content');await source.waitFor();
  if(id==='styled-cards'){
   const previous=await page.locator('.example-module').getAttribute('data-generation');
   await source.fill((await source.innerText()).replace('box-shadow-blur:2mm','box-shadow-blur:4mm'));await ready(page,previous);
   assert.notEqual(await capture(page),original,'theme blur changes rendered appearance');
   const edited=await page.locator('.example-module').getAttribute('data-generation');
   await page.locator('.live-reset').click();await ready(page,edited);assert.equal(await capture(page),original);
  }
  await page.keyboard.press('Escape');await source.blur();
  await page.screenshot({path:path.join(out,id+'-theme.png')});
  if(id==='styled-cards'){
   await page.setViewportSize({width:390,height:844});
   assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1));
   await page.screenshot({path:path.join(out,'combined-project-mobile.png')});
   await page.setViewportSize({width:1440,height:1000});
  }
  checks.push(id+': real WASM compilation, playback, presentation advance/back/exit, exact static restoration and editable theme');
 }
 checks.push('Bilingual combined-project entry exposes all three projects; mobile layout fits');
}
(async()=>{
 const browser=await chromium.launch({headless:true,...(process.env.CHROMIUM_PATH?{executablePath:process.env.CHROMIUM_PATH}:{}),args:['--no-sandbox']});
 try{
  const page=await browser.newPage({viewport:{width:1440,height:1000},reducedMotion:'no-preference'});
  page.on('pageerror',e=>errors.push(String(e)));page.on('console',m=>{if(m.type()==='error')errors.push(m.text());});
  await page.addInitScript(instrument);
  page.setDefaultTimeout(60000);
  if(process.argv.includes('--cases-only')){
   await anchorProject(page);await combinedProjects(page);assert.deepEqual(errors,[]);
   fs.writeFileSync(path.join(out,'checks.json'),JSON.stringify({checks,consoleErrors:errors,browser:'Chromium/Playwright',viewports:['1440x1000','390x844']},null,2));
   console.log(JSON.stringify({passed:checks.length,projects:3,consoleErrors:errors}));return;
  }
  await navigate(page,'docs/topics/animation-catalog.zh-CN.html');
  assert.match(await page.title(),/97/);assert.equal(await page.locator('.example-module').count(),1);
  assert.equal(await page.locator('.effect-select option').count(),97);
  await page.locator('.animation-catalog').scrollIntoViewIfNeeded();await ready(page);
  const original=await capture(page);
  for(const effect of effects){
   const previous=await page.locator('.example-module').getAttribute('data-generation');
   if(effect.name===await page.locator('.effect-select').inputValue())continue;
   await page.locator('.effect-select').selectOption(effect.name);await ready(page,previous);
   const inspection=await page.evaluate(()=>manualQA.inspection);
   assert(inspection.animation?.steps.length===1,effect.name);
   assert((await page.locator('.source-panel .cm-content').first().innerText()).includes(effect.class),effect.name);
  }
  checks.push('All 97 effects listed; every selection compiles in the same WASM workspace');
  let previous=await page.locator('.example-module').getAttribute('data-generation');
  await page.locator('.effect-select').selectOption('fadeInUp');await ready(page,previous);
  const catalogSource=page.locator('.source-panel .cm-content').first();
  previous=await page.locator('.example-module').getAttribute('data-generation');
  await catalogSource.fill((await catalogSource.innerText()).replace('duration=1s','duration=2s'));await ready(page,previous);
  previous=await page.locator('.example-module').getAttribute('data-generation');
  await page.locator('.effect-select').selectOption('bounce');await ready(page,previous);
  assert.equal(await page.evaluate(()=>manualQA.inspection.animation.steps[0].duration),2);
  previous=await page.locator('.example-module').getAttribute('data-generation');
  await page.locator('.live-reset').click();await ready(page,previous);
  assert.equal(await page.locator('.effect-select').inputValue(),'fadeInUp');
  assert.equal(await page.locator('.example-module').getAttribute('data-modified'),'false');
  assert.equal(await page.evaluate(()=>manualQA.inspection.animation.steps[0].duration),1);
  checks.push('Effect changes retain edited timing; Restore resets both source and selected effect');
  previous=await page.locator('.example-module').getAttribute('data-generation');
  await page.evaluate(()=>{
   const select=document.querySelector('.effect-select');
   for(const name of ['bounce','zoomIn']){select.value=name;select.dispatchEvent(new Event('change',{bubbles:true}));}
   document.querySelector('.effect-play').click();
  });
  await ready(page,previous);
  assert((await catalogSource.innerText()).includes('animate__zoomIn'));
  const latest=await page.locator('.example-module').getAttribute('data-generation');
  await page.waitForFunction(generation=>manualQA.paints.some(p=>String(p.generation)===generation&&p.running&&p.time>0),latest,{timeout:15000});
  checks.push('Rapid selection uses the last effect; Play during preparation starts the updated animation');
  previous=await page.locator('.example-module').getAttribute('data-generation');
  await page.locator('.effect-select').selectOption('fadeInUp');await ready(page,previous);
  await page.locator('.effect-play').click();
  await page.waitForFunction(()=>manualQA.paints.some(p=>p.running&&p.time>0&&p.time<1),null,{timeout:15000});
  // Seek a reproducible interior frame after stopping playback, then compare actual pixels.
  if(await page.locator('.preview-options-menu').getAttribute('open')===null)await page.locator('.preview-options-menu > summary').click();
  await page.locator('.preview-motion-menu > summary').click();
  await page.locator('[data-action="animation-play"]').click();
  await page.locator('[aria-label="Animation time"]').fill('0.4');
  await page.locator('[aria-label="Animation time"]').dispatchEvent('input');
  await frame(page,.4);
  assert.notEqual(await capture(page),original);
  await page.locator('.effect-static').click();
  await frame(page,null);
  assert.equal(await capture(page),original);
  checks.push('Play produces changing frames; seek produces different pixels; Static restores exact original pixels');
  if(await page.locator('.preview-options-menu').getAttribute('open')!==null)await page.locator('.preview-options-menu > summary').click();
  await page.locator('.effect-select').scrollIntoViewIfNeeded();
  await page.screenshot({path:path.join(out,'catalog-desktop.png')});
  await page.setViewportSize({width:390,height:844});
  await page.locator('.effect-select').scrollIntoViewIfNeeded();
  assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1));
  await page.screenshot({path:path.join(out,'catalog-mobile.png')});
  checks.push('Catalog controls and shared preview fit mobile viewport');
  await page.setViewportSize({width:1440,height:1000});
  await navigate(page,'en/docs/topics/animation-catalog.html');
  assert.match(await page.title(),/97/);assert.equal(await page.locator('.effect-select option').count(),97);
  checks.push('English catalog exposes the same complete effects');
  await navigate(page,'docs/topics/style-start.zh-CN.html');
  await page.locator('.example-module').scrollIntoViewIfNeeded();await ready(page);
  assert.equal(await page.locator('[data-source-mode],.summary-panel').count(),0);
  const theme=page.locator('.source-files [role=tab]').filter({hasText:'style-theme.lcss'});
  await theme.click();
  const source=page.locator('.source-panel:not([hidden]) .cm-content');
  await source.waitFor();assert((await source.innerText()).includes('box-shadow-blur: 2mm'));
  previous=await page.locator('.example-module').getAttribute('data-generation');
  await source.fill((await source.innerText()).replace('box-shadow-blur: 2mm','box-shadow-blur: 4mm'));
  await ready(page,previous);
  await page.keyboard.press('Escape');await source.blur();
  await page.screenshot({path:path.join(out,'style-theme-editor.png')});
  checks.push('Project theme is editable and shadow override recompiles successfully');
  await navigate(page,'docs/topics/animation-start.zh-CN.html');
  await page.locator('.example-module').scrollIntoViewIfNeeded();await ready(page);
  await page.locator('[data-action="present"]').click();
  await page.waitForFunction(()=>{const p=document.querySelector('.presentation');if(p?.dataset.error)throw Error(p.dataset.error);return p?.dataset.painted==='true';},null,{timeout:90000});
  await page.locator('.presentation').focus();
  assert.equal(await page.locator('.presentation').getAttribute('data-running'),'false');
  await page.keyboard.press('Space');
  await page.waitForFunction(()=>Number(document.querySelector('.presentation')?.dataset.time)>.2);
  await page.keyboard.press('ArrowRight');
  await page.waitForFunction(()=>document.querySelector('.presentation')?.dataset.completed==='1');
  await page.keyboard.press('ArrowLeft');
  await page.waitForFunction(()=>document.querySelector('.presentation')?.dataset.time==='0');
  await page.keyboard.press('Escape');
  await page.locator('.presentation').waitFor({state:'detached'});
  checks.push('First-animation manual waits for input, plays, advances, returns to its start and exits');
  for(const topic of ['install','first-layout','style-libraries']){
   await navigate(page,'docs/sections/start.zh-CN.html');
   await page.locator(`article a[href$="${topic}.zh-CN.html"]`).first().click();
   await page.waitForURL('**/'+topic+'.zh-CN.html');
   assert(await page.locator('article h1').isVisible());
  }
  for(const topic of ['style-cascade','style-properties','style-shadows','style-compatibility']){
   await page.locator(`article a[href$="${topic}.zh-CN.html"]`).first().click();
   await page.waitForURL('**/'+topic+'.zh-CN.html');
   assert(await page.locator('article h1').isVisible());
  }
  checks.push('Learning route opens installation, first layout and every style chapter through actual links');
  await anchorProject(page);await combinedProjects(page);
  assert.deepEqual(errors,[]);
  fs.writeFileSync(path.join(out,'checks.json'),JSON.stringify({checks,consoleErrors:errors,browser:'Chromium/Playwright',viewports:['1440x1000','390x844']},null,2));
  console.log(JSON.stringify({passed:checks.length,effects:97,consoleErrors:errors}));
 }finally{await browser.close();}
})().catch(error=>{console.error(error);process.exitCode=1;});
