// Actual browser keyboard events, without seeking, rehearsal, or a deck-ready wait.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),crypto=require('node:crypto'),{pathToFileURL}=require('node:url');
const pw=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const [file,out]=process.argv.slice(2),samples=Number(process.env.SAMPLES||50);assert(file&&out);fs.mkdirSync(out,{recursive:true});
const instrument=require('./presentation-keyboard-instrument.cjs');
const q=(a,p)=>{a=a.filter(Number.isFinite).sort((a,b)=>a-b);return !a.length?null:p===.5?(a[Math.floor((a.length-1)/2)]+a[Math.floor(a.length/2)])/2:a[Math.ceil(a.length*p)-1];};
(async()=>{for(const engine of (process.env.ENGINES||'chromium,firefox,webkit').split(',')){
 const browser=await pw[engine].launch({headless:true});
 try{
  const p=await browser.newPage({viewport:{width:1600,height:1000}}),errors=[],network=[];p.setDefaultTimeout(180000);p.on('pageerror',e=>errors.push(e.message));await p.route(/^https?:\/\//,r=>{network.push(r.request().url());return r.abort();});
  await p.goto(pathToFileURL(path.resolve(file)).href);await p.waitForFunction(()=>window.__LAYMESH_WEB?.presentation&&document.querySelector('#app').dataset.painted==='true');
  await p.evaluate('('+instrument.toString()+')(__LAYMESH_WEB.presentation)');
  await p.evaluate(()=>__LAYMESH_WEB.presentation.open({fullscreen:false}));
  const deadline=Date.now()+1800000;let keys=0;
  while(true){
   const status=await p.evaluate(()=>({count:keyboardMeasure.rows.length,index:__LAYMESH_WEB.presentation.index,last:__LAYMESH_WEB.presentation.deck.pages.length-1,ended:__LAYMESH_WEB.presentation.ended,error:__LAYMESH_WEB.presentation.root.dataset.error,ready:__LAYMESH_WEB.presentation.ready,switching:__LAYMESH_WEB.presentation.switching}));
   if(status.count>=samples)break;if(status.error||!status.ready)throw Error(status.error||'Presentation stopped');if(Date.now()>deadline)throw Error('Keyboard navigation timeout');
   await p.keyboard.press(status.ended?'Home':keys++%2?'ArrowDown':'ArrowRight');
   await p.waitForTimeout(80);
  }
  await p.waitForFunction(()=>!__LAYMESH_WEB.presentation.switching);
  const result=await p.evaluate(()=>({...keyboardMeasure,backend:__LAYMESH_WEB.presentation.metrics.backend,resources:__LAYMESH_WEB.presentation.metrics.resources,resident:[...__LAYMESH_WEB.presentation.resident],dimensions:__LAYMESH_WEB.presentation.dimensions(),rendererFallback:__LAYMESH_WEB.presentation.rendererFallback,usesWorker:!!__LAYMESH_WEB.presentation.worker,usesMain:!!__LAYMESH_WEB.presentation.mainRuntime,memory:__LAYMESH_WEB.presentation.metrics.lastFrame}));
  result.rows=result.rows.slice(0,samples);result.summary={};
  for(const [kind,rows]of [['all',result.rows],['firstTraversal',result.rows.filter(r=>r.round===0)],['repeat',result.rows.filter(r=>r.round>0)],['prepared',result.rows.filter(r=>r.ready)]]){
   const s={samples:rows.length};for(const key of ['firstVisibleMs','completedMs','transitionMs','queueWaitMs'])s[key]={median:q(rows.map(r=>r[key]),.5),p95:q(rows.map(r=>r[key]),.95)};result.summary[kind]=s;
  }
  if(result.rows.some(r=>!Number.isFinite(r.firstVisibleMs)))fs.writeFileSync(path.join(out,engine+'-invalid.json'),JSON.stringify({...result,errors,network},null,2));
  assert(result.rows.every(r=>r.trusted));assert(result.rows.every(r=>Number.isFinite(r.firstVisibleMs)&&r.firstVisibleMs>0),'Every accepted page change needs a visible-frame timestamp');assert.deepEqual(errors,[]);assert.deepEqual(network,[]);
  Object.assign(result,{engine,version:browser.version(),htmlSha256:crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex'),errors,network,definition:'Trusted ArrowDown/ArrowRight every 80 ms through ordinary animation steps; Home only after end; no clock seeking and no all-page readiness wait. First visible = first nonzero transition frame notification followed by browser RAF/task; completion includes authored transition and GPU completion. Input protection remains in effect.'});
  fs.writeFileSync(path.join(out,engine+'.json'),JSON.stringify(result,null,2));await p.screenshot({path:path.join(out,engine+'.png')});console.log(engine,JSON.stringify(result.summary));
 }finally{await browser.close();}
}})().catch(e=>{console.error(e);process.exitCode=1;});
