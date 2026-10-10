// Real site regression and five cold/warm startup samples. Build docs first.
// PLAYWRIGHT_MODULE / CHROMIUM_PATH select existing test installations.
const fs=require('fs'),path=require('path'),crypto=require('crypto'),assert=require('assert/strict');
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const url=process.argv[2],out=process.argv[3];
if(!url||!out)throw Error('Usage: node scripts/test-browser-animation-startup.cjs URL OUTPUT_DIR [--corpus]');
fs.mkdirSync(out,{recursive:true});
const root=path.resolve(__dirname,'..'),manifest=JSON.parse(fs.readFileSync(path.join(root,'site/dist/site/live/manifest.json')));
const cjk=manifest.fonts.find(f=>f.full),base=new URL('../site/live/',url).href;
const hash=bytes=>crypto.createHash('sha256').update(bytes).digest('hex');
const result={environment:{url,viewport:[1440,1000],deviceScaleFactor:1},fonts:manifest.fonts.map(f=>({family:f.family,bytes:f.bytes,fullBytes:f.full?.bytes})),checks:[],cold:[],warm:[]};
const errors=[];
function instrument(){
 localStorage.setItem('laymesh-toc-desktop','closed');
 window.__qa={posts:[],replies:[]};const Native=Worker;
 window.Worker=class extends Native{
  constructor(...args){super(...args);this.addEventListener('message',({data})=>{const {type,id,generation,metrics}=data;window.__qa.replies.push({type,id,generation,metrics,at:performance.now()});});}
  postMessage(data,...args){const {type,id,time,generation}=data;window.__qa.posts.push({type,id,time,generation,at:performance.now()});super.postMessage(data,...args);}
 };
}
const state=p=>p.evaluate(()=>{const m=document.querySelector('.example-module');return {data:{...m.dataset},label:m.querySelector('.animation-controls output')?.textContent,status:m.querySelector('.live-status').textContent,diagnostics:m.querySelector('.live-diagnostics').textContent,canvasHidden:m.querySelector('.live-image').hidden};});
async function ready(p,previous){await p.waitForFunction(previous=>{const m=document.querySelector('.example-module');if(m?.dataset.liveState==='error')throw Error(m.querySelector('.live-diagnostics').textContent);return m?.dataset.painted==='true'&&m.dataset.liveState==='success'&&!m.querySelector('.live-image').hidden&&(!previous||m.dataset.generation!==previous);},previous,{timeout:45000});}
async function settled(p){await p.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));await p.waitForFunction(()=>{const q=window.__qa,post=q.posts.filter(x=>x.type==='paint').at(-1);return post&&q.replies.some(x=>x.type==='painted'&&x.id===post.id&&x.generation===post.generation);});}
async function pixels(p){await settled(p);return hash(Buffer.from(await p.locator('.live-image').evaluate(c=>c.toDataURL().split(',')[1]),'base64'));}
const action=(p,id)=>p.locator(`[data-action=animation-${id}]`).click();
async function seek(p,time){await p.getByLabel('Animation time',{exact:true}).fill(String(time));await settled(p);}
const edit=(p,text)=>p.locator('.source-panel:not([hidden]) .cm-content').fill(text);
async function sample(p){await ready(p);return p.evaluate(()=>{const m=document.querySelector('.example-module'),r=window.__qa.replies.find(r=>r.type==='result');return {ready:Number(m.dataset.readyMs),firstFrame:Number(m.dataset.firstFrameMs),initialization:r.metrics.initialization,compile:r.metrics.compile,lower:r.metrics.lower,resources:r.metrics.resources};});}
(async()=>{
 const browser=await chromium.launch({headless:true,...(process.env.CHROMIUM_PATH?{executablePath:process.env.CHROMIUM_PATH}:{}),args:['--no-sandbox','--no-proxy-server']});result.environment.browser=browser.version();
 async function session(){const context=await browser.newContext({viewport:{width:1440,height:1000},deviceScaleFactor:1});await context.addInitScript(instrument);const p=await context.newPage();p.on('pageerror',e=>errors.push(String(e)));return {context,p};}
 try{
  // No request interception in the timing runs: it would disable the HTTP cache.
  for(let i=0;i<5;i++){const {context,p}=await session();await p.goto(url);result.cold.push(await sample(p));await p.reload();result.warm.push(await sample(p));await context.close();}
  for(const mode of ['cold','warm'])result[mode+'Median']=Object.fromEntries(Object.keys(result[mode][0]).map(k=>[k,result[mode].map(x=>x[k]).sort((a,b)=>a-b)[2]]));
  assert(result.coldMedian.ready<=650&&result.warmMedian.ready<=650,JSON.stringify(result));assert(result.coldMedian.firstFrame<=900&&result.warmMedian.firstFrame<=900,JSON.stringify(result));
  {
   const {context,p}=await session();await p.goto(url);await ready(p);const initial=await state(p),staticHash=await pixels(p);
   assert.equal(initial.data.mode,'summary');assert.equal(initial.data.fullFonts,'idle');assert.equal(initial.label,'完整排版');assert(!initial.canvasHidden);
   await action(p,'play');await p.waitForFunction(()=>Number(document.querySelector('[aria-label="Animation time"]').value)>.25);await action(p,'play');const paused=await state(p);assert.notEqual(await pixels(p),staticHash);
   await p.locator('[data-source-mode=source]').click();await p.locator('[data-source-mode=summary]').click();assert.equal((await state(p)).label,paused.label);assert.equal((await state(p)).data.generation,initial.data.generation);
   await seek(p,.2);assert.match((await state(p)).label,/0.20/);await action(p,'static');assert.equal(await pixels(p),staticHash);
   await seek(p,7.7);await action(p,'play');await p.waitForFunction(()=>document.querySelector('.document-pages').selectedIndex===1);await ready(p,initial.data.generation);await action(p,'play');assert.match((await state(p)).label,/区域揭示/);
   await seek(p,0);await action(p,'previous');await p.waitForFunction(()=>document.querySelector('.document-pages').selectedIndex===0);await ready(p);await action(p,'static');await settled(p);assert.equal(await pixels(p),staticHash);
   await p.screenshot({path:path.join(out,'summary-static.png')});result.checks.push('Summary play/pause/seek, source tabs preserve time, multipage forward/back, static pixel restoration');await context.close();
  }
  {
   const {context,p}=await session();let release;const gate=new Promise(r=>release=r);await context.route('**/'+cjk.file,async route=>{await gate;await route.continue();});
   await p.goto(url);await p.locator('.startup-play').click();assert.equal((await state(p)).data.playRequested,'true');await p.waitForTimeout(100);assert.equal((await state(p)).data.generation,undefined);release();await ready(p);await p.waitForFunction(()=>document.querySelector('[data-action=animation-play]').textContent==='暂停');
   const firstTime=await p.evaluate(()=>window.__qa.posts.find(x=>x.type==='paint').time);assert.equal(firstTime,0);await action(p,'play');result.checks.push('Play requested during loading starts with a rendered frame at time zero');await context.close();
  }
  {
   const {context,p}=await session();let failures=0;await context.route('**/'+cjk.file,route=>failures++===0?route.fulfill({body:'bad checksum'}):route.continue());await p.goto(url);
   await p.waitForFunction(()=>document.querySelector('.example-module').dataset.liveState==='error');assert(await p.locator('.startup-retry').isVisible());assert.match((await state(p)).diagnostics,/SHA-256/);await p.locator('.startup-retry').click();await ready(p);assert.equal((await state(p)).label,'完整排版');result.checks.push('Startup checksum failure is visible in summary; Retry recovers without autoplay');await context.close();
  }
  {
   const {context,p}=await session();let release,requested=0;const gate=new Promise(r=>release=r);await context.route('**/'+cjk.full.file,async route=>{requested++;await gate;await route.continue();});
   await p.goto(new URL('animation-second.html',url).href);await ready(p);const before=await state(p);assert.equal(requested,0);await p.locator('[data-source-mode=source]').click();await p.waitForFunction(()=>document.querySelector('.example-module').dataset.fullFonts==='loading');const unchanged=await pixels(p);
   const source='page=canvas(size=(80,50),background="#ffffff")\nfor i in range(2) {\n    page.add(text("新增龘麤"+str(i),font_family="Noto Sans CJK SC",font_size=12),offset=(4,8+18*i))\n}';
   await edit(p,source.replace('新增','旧稿'));await p.waitForTimeout(300);await edit(p,source);await p.waitForTimeout(300);assert.equal((await state(p)).data.generation,before.data.generation);assert.equal(await pixels(p),unchanged);assert.match((await state(p)).status,/编辑字体/);
   release();await ready(p,before.data.generation);assert(!/W_FONT|missing glyph/i.test((await state(p)).diagnostics));assert.equal(await p.evaluate(()=>window.__qa.posts.filter(x=>x.type==='run').length),2);assert.equal(requested,1);await p.screenshot({path:path.join(out,'new-glyphs.png')});result.checks.push('Deferred full fonts preserve last frame, compile only latest draft, support new Chinese, generated labels and explicit family');await context.close();
  }
  {
   const {context,p}=await session();await p.goto(url+'#example-animation-presentation-source');await ready(p);await p.waitForFunction(()=>document.querySelector('.example-module').dataset.fullFonts==='ready');assert.equal((await state(p)).data.mode,'source');assert.equal((await state(p)).label,'完整排版');
   const times=await p.evaluate(()=>({first:Number(document.querySelector('.example-module').dataset.firstFrameMs),request:window.__qa.posts.find(x=>x.type==='full-fonts').at}));assert(times.request>=times.first);result.checks.push('Direct source link starts static; full fonts requested only after first frame');await context.close();
  }
  {
   const {context,p}=await session();let failures=0;await context.route('**/'+cjk.full.file,route=>failures++===0?route.fulfill({body:'bad checksum'}):route.continue());
   await p.goto(new URL('animation-second.html',url).href);await ready(p);await p.locator('[data-source-mode=source]').click();await p.waitForFunction(()=>document.querySelector('.example-module').dataset.fullFonts==='error');
   const previous=(await state(p)).data.generation;await edit(p,'page=canvas(size=(80,50))\npage.add(text("重试龘",font_family="Noto Sans CJK SC"))');await p.waitForFunction(()=>document.querySelector('.example-module').dataset.liveState==='error');assert(await p.locator('.startup-retry').isVisible());await p.locator('.startup-retry').click();await ready(p,previous);assert(!/W_FONT/.test((await state(p)).diagnostics));result.checks.push('Failed full-font download remains retryable after editing and recovers the latest draft');await context.close();
  }
  // Render the shipped corpus at fixed dimensions with the real worker, first
  // with startup fonts, then with complete fonts. Compare SHA-256 of raw RGBA.
  if(process.argv.includes('--corpus')){
   const {context,p}=await session();await p.goto(url);await ready(p);await p.exposeFunction('pixelHash',bytes=>hash(Buffer.from(bytes,'base64')));
   result.parity=await p.evaluate(async({base,manifest})=>{
    const worker=new Worker(new URL('worker.js',base),{type:'module'});let id=1000;const pending=new Map();
    worker.onmessage=({data})=>{if(!['ready','init-error','result','error','painted','paint-error','full-fonts-ready','full-fonts-error'].includes(data.type))return;const k=data.type==='ready'?'init':data.type.startsWith('full-fonts')?'fonts':data.id;pending.get(k)?.(data);pending.delete(k);};
    const request=(data,k=data.id)=>new Promise((resolve,reject)=>{const timer=setTimeout(()=>reject(Error('Worker timeout: '+JSON.stringify(data))),45000);pending.set(k,data=>{clearTimeout(timer);/error/.test(data.type)?reject(Error(JSON.stringify(data))):resolve(data);});worker.postMessage(data);});
    await request({type:'init',base},'init');
    async function render(entry){const files={};for(const f of entry.files){const response=await fetch(new URL(manifest.resources['/'+f],base));if(!response.ok)throw Error(`HTTP ${response.status}: ${f}`);files[f]=await response.text();}const run=await request({type:'run',id:++id,entry:entry.entry,files}),p=run.inspection.page,width=640,height=Math.max(1,Math.round(width*p.height/p.width));const frame=await request({type:'paint',id:++id,generation:run.generation,rect:[0,0,p.width,p.height],width,height,time:null});let binary='';for(let i=0;i<frame.rgba.length;i+=32768)binary+=String.fromCharCode(...frame.rgba.subarray(i,i+32768));return {hash:await window.pixelHash(btoa(binary)),warnings:run.warnings};}
    const pairs=[];for(const [name,entry]of Object.entries(manifest.examples))pairs.push({name,subset:await render(entry)});
    await request({type:'full-fonts'},'fonts');for(const pair of pairs){pair.full=await render(manifest.examples[pair.name]);pair.equal=pair.full.hash===pair.subset.hash;}
    worker.terminate();return pairs;
   },{base,manifest});assert(result.parity.every(x=>x.equal),JSON.stringify(result.parity.filter(x=>!x.equal)));result.checks.push('All example pixels match startup and full fonts at 640px');await context.close();
  }
  assert.deepEqual(errors,[]);console.log(JSON.stringify({checks:result.checks,cold:result.coldMedian,warm:result.warmMedian,parity:result.parity?.length},null,2));
 }finally{fs.writeFileSync(path.join(out,'results.json'),JSON.stringify({...result,errors},null,2));await browser.close();}
})().catch(error=>{console.error(error);process.exitCode=1;});
