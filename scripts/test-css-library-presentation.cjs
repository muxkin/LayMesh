// Regressions from editing the real offline CSS gallery: units, missing
// animation imports, manual advance and automatic entry with shadowed cards.
const fs=require('fs'),path=require('path'),crypto=require('crypto'),assert=require('assert/strict');
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const [url,out]=process.argv.slice(2);
if(!url||!out)throw Error('Usage: test-css-library-presentation.cjs SITE_URL OUTPUT_DIRECTORY');
const source=fs.readFileSync(path.join(__dirname,'../examples/gallery/styles/libraries.lay'),'utf8');
fs.mkdirSync(out,{recursive:true});
const hash=bytes=>crypto.createHash('sha256').update(bytes).digest('hex');
function instrument(){
 window.cssQA={posts:[],replies:[],frames:[]};const Native=Worker;
 window.Worker=class extends Native{
  constructor(...args){super(...args);this.qaRequests=new Map();this.qaPresentation=String(args[0]).includes('presentation-worker.mjs');if(this.qaPresentation)cssQA.presentationWorker=this;
   this.addEventListener('message',({data})=>{if(data.type==='result'){cssQA.previewWorker=this;cssQA.inspection=data.inspection;}cssQA.replies.push({type:data.type,id:data.id,generation:data.generation});const request=this.qaRequests.get(data.id);if(this.qaPresentation&&data.type==='done'&&!data.skipped&&request)cssQA.frames.push(request);});
  }
  postMessage(data,...args){const request={type:data.type,id:data.id,time:data.time,running:data.running,presentation:this.qaPresentation};cssQA.posts.push(request);this.qaRequests.set(data.id,request);super.postMessage(data,...args);}
 };
}
async function ready(p,previous){await p.waitForFunction(previous=>{const m=document.querySelector('.example-module');if(m?.dataset.liveState==='error')throw Error(m.querySelector('.live-diagnostics').textContent);return m?.dataset.painted==='true'&&m.dataset.liveState==='success'&&m.dataset.previewState==='success'&&(!previous||m.dataset.generation!==previous);},previous,{timeout:60000});}
async function edit(p,text){const previous=await p.locator('.example-module').getAttribute('data-generation');await p.locator('.source-panel:not([hidden]) .cm-content').fill(text);return previous;}
async function capture(p,presentation=false){
 const data=await p.evaluate(presentation=>new Promise((resolve,reject)=>{
  const worker=presentation?cssQA.presentationWorker:cssQA.previewWorker,id='css-qa-'+performance.now(),timer=setTimeout(()=>reject(Error('Capture timed out')),10000);
  const listener=({data})=>{if(data.id!==id)return;worker.removeEventListener('message',listener);clearTimeout(timer);if(data.error)return reject(Error(JSON.stringify(data.error)));const canvas=document.querySelector(presentation?'.presentation-canvas':'.live-image');resolve({width:canvas.width,height:canvas.height,rgba:Array.from(data.rgba)});};
  worker.addEventListener('message',listener);worker.postMessage({type:'capture',id});
 }),presentation);return {width:data.width,height:data.height,hash:hash(Buffer.from(data.rgba))};
}
async function present(p){await p.evaluate(()=>cssQA.frames=[]);await p.locator('[data-action=present]').click();await p.waitForFunction(()=>document.querySelector('.presentation')?.dataset.painted==='true');await p.locator('.presentation').focus();}
(async()=>{
 const browser=await chromium.launch({headless:true,...(process.env.CHROMIUM_PATH?{executablePath:process.env.CHROMIUM_PATH}:{}),args:['--no-sandbox','--no-proxy-server']});
 const context=await browser.newContext({viewport:{width:1440,height:1000},reducedMotion:'no-preference'});await context.addInitScript(instrument);
 const p=await context.newPage(),errors=[],checks=[];p.setDefaultTimeout(60000);p.on('pageerror',e=>errors.push(String(e)));
 try{
  await p.goto(new URL('examples/gallery-styles-libraries.html',url).href);
  const toc=p.locator('.toc-toggle');if(await toc.getAttribute('aria-expanded')==='true')await toc.click();await p.locator('.example-module').scrollIntoViewIfNeeded();await ready(p);
  assert.match(await p.title(),/Bootstrap|Open Props/);assert(await p.locator('.live-image').isVisible());
  await p.locator('.source-panel:not([hidden]) .cm-content').waitFor();await ready(p);
  const original=await capture(p);
  await edit(p,source.replace('size=(160mm,100mm)','size=(160mm,160*100/90mm)'));
  await p.waitForFunction(()=>document.querySelector('.example-module').dataset.liveState==='error');
  const diagnostic=await p.locator('.live-diagnostics').textContent();assert.match(diagnostic,/E_UNIT/);assert.match(diagnostic,/160mm\*100\/90/);assert.deepEqual(await capture(p),original);
  checks.push('Invalid inverse length explains unit binding and retains the previous canvas');
  await ready(p,await edit(p,source.replace('size=(160mm,100mm)','size=(160mm,160mm*100/90)')));
  const dimensions=await p.evaluate(()=>cssQA.inspection.page);assert(Math.abs(dimensions.width-160)<1e-8);assert(Math.abs(dimensions.height-160*100/90)<1e-8);
  checks.push('160mm*100/90 compiles as a physical proportional length in WASM');
  await ready(p,await edit(p,source.replace(',"@laymesh/animate.css"','')));
  const warnings=await p.evaluate(()=>cssQA.inspection.warnings.filter(w=>w.code==='W_ANIMATION'));assert.equal(warnings.length,1);assert.match(warnings[0].message,/@laymesh\/animate\.css/);
  checks.push('Missing Animate.css produces one warning at animate(), with an import hint');
  const slow=source.replace('style {','style {\n  .custom-motion { animation-duration: 2s; }').replace('class="animate__animated animate__fadeInUp")','class="animate__animated animate__fadeInUp custom-motion")');
  await ready(p,await edit(p,slow.replace('start="after-previous", ','')));
  assert((await p.evaluate(()=>cssQA.inspection.warnings)).every(w=>w.code!=='W_ANIMATION'));
  assert.equal(await p.evaluate(()=>cssQA.inspection.animation.steps[0].duration),2);
  checks.push('Inline CSS duration overrides the explicitly imported Animate.css 1s default');
  await present(p);assert.equal(await p.locator('.presentation').getAttribute('data-running'),'false');assert.equal(await p.locator('.presentation').getAttribute('data-time'),'0');
  const before=await capture(p,true);await p.keyboard.press('Space');await p.waitForFunction(()=>cssQA.frames.some(f=>f.time>.35&&f.running),null,{timeout:60000});await p.keyboard.press('s');
  const middle=await capture(p,true);assert.notEqual(middle.hash,before.hash);await p.screenshot({path:path.join(out,'manual-reveal.png')});
  await p.keyboard.press('ArrowRight');await p.waitForFunction(()=>document.querySelector('.presentation').dataset.completed==='1');const after=await capture(p,true);assert.notEqual(after.hash,before.hash);assert.notEqual(after.hash,middle.hash);
  checks.push('Manual presentation waits, Space starts the shadowed gradient card, and advance finishes it');
  await p.keyboard.press('Escape');await p.locator('.presentation').waitFor({state:'detached'});
  await ready(p,await edit(p,slow));await p.evaluate(()=>cssQA.posts=[]);await present(p);
  await p.waitForFunction(()=>cssQA.frames.some(f=>f.time>.35&&f.running),null,{timeout:60000});await p.keyboard.press('s');
  const posts=await p.evaluate(()=>cssQA.posts.filter(x=>x.presentation));assert.equal(posts.find(x=>x.type==='activate').time,0);assert(posts.some(x=>x.type==='paint'&&x.time>.25&&x.running));
  await capture(p,true);await p.screenshot({path:path.join(out,'automatic-reveal.png')});
  checks.push('after-previous starts on entering Present, paints time zero, then advances animated frames');
  await p.keyboard.press('Escape');await p.locator('.presentation').waitFor({state:'detached'});
  const previous=await p.locator('.example-module').getAttribute('data-generation');await p.locator('.live-reset').click();await ready(p,previous);assert.equal(await p.locator('.example-module').getAttribute('data-modified'),'false');
  assert.equal((await capture(p)).hash,original.hash);assert.deepEqual(errors,[]);
  fs.writeFileSync(path.join(out,'checks.json'),JSON.stringify({url:p.url(),checks,dimensions,manualPixels:{before,middle,after},consoleErrors:errors},null,2));console.log(JSON.stringify({checks,consoleErrors:errors}));
 }finally{await context.close();await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
