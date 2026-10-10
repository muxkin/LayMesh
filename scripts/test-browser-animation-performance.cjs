// Measure real WASM playback, queue bounds, native density and hold-frame reuse.
const fs=require('fs'),path=require('path'),assert=require('assert/strict');
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const [url,output]=process.argv.slice(2);if(!url||!output)throw Error('Usage: node scripts/test-browser-animation-performance.cjs URL OUTPUT.json');
const percentile=(values,p)=>[...values].sort((a,b)=>a-b)[Math.min(values.length-1,Math.floor(values.length*p))];
(async()=>{
 const browser=await chromium.launch({headless:true,...(process.env.CHROMIUM_PATH?{executablePath:process.env.CHROMIUM_PATH}:{}),args:['--no-sandbox','--no-proxy-server']});const results=[];
 try{for(const dpr of [1,3]){
  const context=await browser.newContext({viewport:{width:2560,height:1440},deviceScaleFactor:dpr});
  await context.addInitScript(()=>{
   localStorage.setItem('laymesh-toc-desktop','closed');window.qa={frames:[],pending:new Map(),inFlight:0,maxInFlight:0,runs:0};const Native=Worker;
   window.Worker=class extends Native{
    constructor(...args){super(...args);this.addEventListener('message',({data})=>{if(['painted','paint-skipped','paint-error'].includes(data.type)){qa.inFlight--;const request=qa.pending.get(data.id);qa.pending.delete(data.id);if(data.type==='painted')qa.frames.push({at:performance.now(),raster:data.rasterMs,time:request.time,running:request.running,width:data.width,density:request.dpr});}});}
    postMessage(data,...args){if(data.type==='paint'){qa.pending.set(data.id,data);qa.maxInFlight=Math.max(qa.maxInFlight,++qa.inFlight);}if(data.type==='run')qa.runs++;super.postMessage(data,...args);}
   };
  });
  const p=await context.newPage();const errors=[];p.on('pageerror',e=>errors.push(String(e)));await p.goto(url);await p.waitForFunction(()=>document.querySelector('.example-module')?.dataset.painted==='true');
  await p.getByLabel('Animation time',{exact:true}).fill('1.8');await p.locator('[data-action=animation-play]').click();await p.waitForFunction(()=>Number(document.querySelector('[aria-label="Animation time"]').value)>=2.9);await p.locator('[data-action=animation-play]').click();
  await p.waitForFunction(dpr=>{const c=document.querySelector('.live-image');return document.querySelector('.example-module').dataset.painted==='true'&&Math.abs(c.width/c.getBoundingClientRect().width-dpr)<.01;},dpr);
  const frames=await p.evaluate(()=>qa.frames.filter(f=>f.running&&f.time>=1.85&&f.time<=2.8));const gaps=frames.slice(1).map((f,i)=>f.at-frames[i].at),costs=frames.map(f=>f.raster);
  const record={viewport:[2560,1440],dpr,frames:frames.length,rasterMedian:percentile(costs,.5),rasterP95:percentile(costs,.95),frameIntervalMedian:percentile(gaps,.5),frameIntervalP95:percentile(gaps,.95),fps:(frames.length-1)*1000/(frames.at(-1).at-frames[0].at),widths:[...new Set(frames.map(f=>f.width))],state:await p.evaluate(()=>({maxInFlight:qa.maxInFlight,runs:qa.runs,displayedFps:document.querySelector('.example-module').dataset.playbackFps})),errors};
  assert.equal(record.state.maxInFlight,1);assert.equal(record.state.runs,1);assert.deepEqual(errors,[]);assert(frames.length>=10);assert(frames.every(frame=>frame.density===dpr),'Playback must keep native density');
  await p.getByLabel('Animation time',{exact:true}).fill('0.9');await p.locator('[data-action=animation-play]').click();await p.waitForFunction(()=>Number(document.querySelector('[aria-label="Animation time"]').value)>1.0);const held=await p.evaluate(()=>qa.frames.length);await p.waitForFunction(()=>Number(document.querySelector('[aria-label="Animation time"]').value)>1.6);assert((await p.evaluate(()=>qa.frames.length))-held<=1,'Hold must reuse the identical boundary frame');await p.locator('[data-action=animation-play]').click();
  results.push(record);await context.close();
 }
 fs.mkdirSync(path.dirname(output),{recursive:true});fs.writeFileSync(output,JSON.stringify({browser:browser.version(),url,results},null,2));console.log(JSON.stringify(results,null,2));
 }finally{await browser.close();}
})().catch(error=>{console.error(error);process.exitCode=1;});
