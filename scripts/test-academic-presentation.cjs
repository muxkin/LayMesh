// Full narrative academic deck through the real compiler and presentation UI.
// All frame construction is measured; no warmup, rehearsal or excluded round.
const fs=require('fs'),path=require('path'),assert=require('assert/strict');
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const [url,out]=process.argv.slice(2);if(!out)throw Error('Usage: test-academic-presentation.cjs URL OUTPUT');fs.mkdirSync(out,{recursive:true});
(async()=>{const browser=await chromium.launch({headless:true,executablePath:process.env.CHROMIUM_PATH,args:['--no-sandbox','--no-proxy-server']}),p=await browser.newPage({viewport:{width:1920,height:1080}}),errors=[];p.setDefaultTimeout(180000);p.on('pageerror',e=>errors.push(String(e)));
try{
 await p.addInitScript(()=>{window.qa={compiles:0,pixels:0,pages:[]};const Native=Worker;window.Worker=class extends Native{postMessage(data,...args){if(data.type==='compile-deck-page')qa.pages.push(data.page.id);if(['prepare-deck','run'].includes(data.type))qa.compiles++;super.postMessage(data,...args);}constructor(...args){super(...args);this.addEventListener('message',({data})=>{if(data.rgba)qa.pixels++;});}};});
 await p.goto(url);await p.waitForFunction(()=>!!(window.presentation?.deck||window.laymeshPresentation?.deck));await p.evaluate(()=>window.show=window.presentation||window.laymeshPresentation);
 assert.equal(await p.evaluate(()=>show.deck.pages.length),32);await p.locator('[data-action=present]').click();await p.waitForFunction(()=>show.ready);await p.evaluate(()=>show.preload);
 const before=await p.evaluate(()=>qa.compiles);const wheel=async dy=>{await p.mouse.move(100,200);await p.mouse.wheel(0,dy);};
 // Browser protocol round trips can separate mouse.wheel calls by more than
 // the 180 ms gesture boundary. Follow one real wheel event with its tail in
 // the same browser turn, and inspect the state before natural animation time
 // can advance. Timed inertia and separate gestures are covered by the unit
 // wheel test; backward navigation and menu scrolling below use real input.
 await p.evaluate(()=>{
  const input=show.wheelInput,consume=input.consume,decisions=[];
  input.consume=function(...args){const direction=consume.apply(this,args);decisions.push(direction);return direction;};
  show.root.addEventListener('wheel',event=>queueMicrotask(()=>{
   try{
    for(let i=0;i<2;i++)show.canvas.dispatchEvent(new WheelEvent('wheel',{deltaY:event.deltaY,bubbles:true,cancelable:true}));
    qa.wheelBurst={decisions,completed:show.clock.completed,running:show.clock.running};
   }finally{input.consume=consume;}
  }),{once:true});
 });
 await wheel(120);await p.waitForFunction(()=>qa.wheelBurst);
 const burst=await p.evaluate(()=>qa.wheelBurst);
 assert.deepEqual(burst.decisions,[1,0,0],'one wheel burst accepts exactly one navigation action');
 assert.equal(burst.completed,0,'one wheel burst cannot finish a step');assert(burst.running,'the accepted wheel event starts the animation');
 await p.waitForTimeout(220);await wheel(-120);await p.waitForFunction(()=>!show.clock.running&&show.clock.time===0);
 await p.mouse.wheel(180,0);assert.equal(await p.evaluate(()=>show.clock.time),0);
 await p.locator('.presentation-canvas').click({button:'right',position:{x:700,y:350}});await p.getByRole('menuitem',{name:'跳到页面…',exact:true}).click();
 const beforeMenu=await p.evaluate(()=>show.index);await p.locator('.presentation-menu').hover();await p.mouse.wheel(0,380);assert.equal(await p.evaluate(()=>show.index),beforeMenu);await p.waitForFunction(()=>document.querySelector('.presentation-menu').scrollTop>0);await p.keyboard.press('Escape');await p.keyboard.press('Escape');
 // Each real page is activated and every step sampled at its middle and end.
 const inventory=[];let sampled=0;const effects=new Set();
 for(let i=0;i<32;i++){
  await p.evaluate(async i=>{await show.go(i,{end:true});await show.preload;},i);
  const entry=await p.evaluate(()=>({id:show.deck.pages[show.index].id,steps:show.clock.steps,resident:show.metrics.lastFrame.resident,backend:show.metrics.backend}));assert(entry.resident.includes(entry.id));for(const previous of inventory)assert(entry.resident.includes(previous.id),'Unchanged visited page was discarded: '+previous.id);inventory.push({id:entry.id,steps:entry.steps.length,resident:entry.resident});
  if([0,6,10,17,20,24,28,31].includes(i))await p.screenshot({path:path.join(out,`page-${String(i+1).padStart(2,'0')}.png`)});
  for(const step of entry.steps){
   for(const a of step.animations||[])effects.add(a.name);
   for(const fraction of [.47,1]){await p.evaluate(async time=>{show.clock.seek(time);await show.paint();show.sync();},step.start+step.duration*fraction);sampled++;}
  }
  if(i%8===7)console.log(`Verified ${i+1}/32 pages; ${effects.size} effects sampled.`);
 }
 assert.equal([...effects].filter(x=>!x.startsWith('laymesh-')).length,97);assert(effects.has('laymesh-draw')&&effects.has('laymesh-reveal'));assert.equal(await p.evaluate(()=>qa.compiles),before);assert.equal(await p.evaluate(()=>qa.pixels),0);const compiled=await p.evaluate(()=>qa.pages);if(compiled.length){assert.equal(compiled.length,32);assert.equal(new Set(compiled).size,32);}
 // Real-time automatic playback across an animated/static boundary and all
 // eight effect pages, including six nonplanar transforms. No hidden trial.
 await p.evaluate(()=>show.go(22));await p.evaluate(()=>{show.clock.mode='auto';show.clock.speed=2;show.clock.running=true;show.last=performance.now();show.sync();});
 await p.waitForFunction(()=>show.ended,{},{timeout:180000});assert.equal(await p.evaluate(()=>show.index),31);
 assert.equal(await p.evaluate(()=>qa.compiles),before);assert.equal(await p.evaluate(()=>qa.pixels),0);
 // Real 4K buffer for dense-page revisits; the CSS viewport remains unchanged.
 await p.evaluate(()=>show.dimensions=()=>({width:3840,height:2160}));
 // Random seek/backwards, retained-page revisit, rapid latest-target navigation.
 for(const i of [17,2,28,0])await p.evaluate(async i=>{await show.go(i);await show.preload;},i);
 await p.evaluate(async()=>{await Promise.all([show.go(17),show.go(30),show.go(0)]);await show.preload;});assert.equal(await p.evaluate(()=>show.root.dataset.page),'p01');
 const result=await p.evaluate(()=>({metrics:show.metrics,pages:show.deck.pages.length,buffer:[show.canvas.width,show.canvas.height],playbackCompiles:qa.compiles,backgroundPageCompiles:qa.pages,pixelTransfers:qa.pixels}));
 result.compilationRequests=result.playbackCompiles;result.playbackCompiles-=before;assert.deepEqual(result.buffer,[3840,2160]);
 Object.assign(result,{sampledFrames:sampled,effects:[...effects].sort(),inventory,errors,hardwareAcceptance:false,checks:['32 distinct academic pages','97 effects sampled within charts/formulas/groups','real-time auto across static page and 8 effect pages','wheel advance/back and inertia','horizontal wheel isolation','long menu scroll without navigation','random/back/latest navigation','each page compiles once; no playback-triggered recompile or pixel transfer','all unchanged visited pages remain resident until close']});
 await p.keyboard.press('Escape');assert.equal(await p.locator('.presentation').count(),0);assert(await p.evaluate(()=>!show.worker&&!show.mainRuntime&&show.resident.size===0),'Closing presentation releases renderers and resident pages');assert.deepEqual(errors,[]);fs.writeFileSync(path.join(out,'results.json'),JSON.stringify(result,null,2));console.log('Academic presentation passed.');
}catch(error){await p.screenshot({path:path.join(out,'failure.png')});console.error((await p.locator('body').innerText()).slice(-2500));throw error;}finally{await browser.close();}})().catch(e=>{console.error(e);process.exitCode=1;});
