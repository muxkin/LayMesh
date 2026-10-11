// Real presentation UI regression: hidden controls and usable page after exit.
const assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path');
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const [url,out]=process.argv.slice(2);if(!out)throw Error('Usage: test-presentation-controls.cjs SITE_URL OUTPUT');fs.mkdirSync(out,{recursive:true});
(async()=>{
 const browser=await chromium.launch({headless:true,executablePath:process.env.CHROMIUM_PATH,args:['--no-sandbox','--no-proxy-server']}),errors=[],evidence={};
 const p=await browser.newPage({viewport:{width:1366,height:768}});p.setDefaultTimeout(60000);p.on('pageerror',error=>errors.push(String(error)));
 try{
  await p.goto(new URL('presentation-test.html?deck=academic',url).href);assert.match(await p.title(),/多页放映/);
  await p.waitForFunction(()=>window.presentation?.deck);await p.locator('[data-action=present]').click();await p.waitForFunction(()=>presentation.ready);await p.evaluate(async()=>{await presentation.go(0,{end:true});await presentation.preload;});
  const hidden=()=>p.locator('.presentation-bar').evaluate(bar=>bar.inert&&bar.getAttribute('aria-hidden')==='true'&&getComputedStyle(bar).visibility==='hidden');
  assert(await hidden());await p.mouse.move(680,745);assert(await hidden(),'default stays hidden at the bottom');
  await p.screenshot({path:path.join(out,'default-hidden.png')});
  await p.keyboard.press('Tab');assert.equal(await hidden(),false);assert.equal(await p.evaluate(()=>document.activeElement.dataset.action),'present-previous');
  await p.keyboard.press('Tab');assert.equal(await p.evaluate(()=>document.activeElement.dataset.action),'present-pause');
  await p.keyboard.press('Shift+Tab');assert.equal(await p.evaluate(()=>document.activeElement.dataset.action),'present-previous');
  await p.keyboard.press('Shift+Tab');assert.equal(await p.evaluate(()=>document.activeElement.dataset.action),'present-exit');
  await p.screenshot({path:path.join(out,'tab-controls.png')});
  await p.locator('[data-action=present-menu]').click();await p.locator('.presentation-menu').waitFor({state:'visible'});await p.keyboard.press('Escape');
  assert(await hidden(),'mouse activation after Tab must open the menu without losing the click');
  await p.evaluate(()=>presentation.root.focus());assert(await hidden());
  const menu=async()=>{await p.locator('.presentation-canvas').click({button:'right',position:{x:500,y:300}});await p.getByRole('menuitem',{name:'控制栏显示…',exact:true}).click();};
  await menu();assert.equal(await p.getByRole('menuitemradio',{name:'不显示',exact:true}).getAttribute('aria-checked'),'true');await p.screenshot({path:path.join(out,'display-menu.png')});
  await p.getByRole('menuitemradio',{name:'一直显示',exact:true}).click();await p.mouse.move(100,100);assert.equal(await hidden(),false);
  await menu();await p.getByRole('menuitemradio',{name:'鼠标到位置显示',exact:true}).click();await p.mouse.move(100,100);assert(await hidden());
  await p.mouse.move(680,745);assert.equal(await hidden(),false);await p.mouse.move(100,745);assert(await hidden(),'bottom edge outside controls does not reveal');
  await p.keyboard.press('Tab');await p.mouse.move(100,100);assert.equal(await hidden(),false,'keyboard focus remains usable outside hover zone');
  await p.evaluate(()=>presentation.root.focus());assert(await hidden());
  await menu();await p.getByRole('menuitemradio',{name:'不显示',exact:true}).click();await p.mouse.move(680,745);assert(await hidden());
  // Menu focus is trapped within the open panel, never in the invisible bar.
  await menu();await p.keyboard.press('Shift+Tab');assert.equal(await p.evaluate(()=>document.activeElement.dataset.toolbarMode),'hidden');await p.keyboard.press('Escape');await p.keyboard.press('Escape');assert(await hidden());
  // Let the real last page complete automatically before exiting.
  await p.evaluate(async()=>{await presentation.go(31);await presentation.preload;presentation.clock.mode='auto';presentation.clock.speed=2;presentation.clock.running=true;presentation.last=performance.now();});
  await p.waitForFunction(()=>presentation.ended);assert(await hidden(),'end-of-deck does not reveal the bar');
  // Maximum bounded diagnostics from a long session must not become DOM text.
  await p.evaluate(()=>{const row=presentation.metrics.playback[0];presentation.metrics.playback=Array.from({length:512},(_,i)=>({...row,key:'long-session-'+i}));});
  await p.keyboard.press('Escape');await p.waitForFunction(()=>!presentation.closing&&!document.querySelector('.presentation'));
  evidence.exit=await p.evaluate(()=>({summaryChars:document.querySelector('#result').textContent.length,documentHeight:document.body.scrollHeight,pendingCalls:presentation.calls.size,raf:presentation.raf,worker:!!presentation.worker}));
  assert(evidence.exit.summaryChars<1000);assert(evidence.exit.documentHeight<2000);assert.equal(evidence.exit.pendingCalls,0);assert.equal(evidence.exit.raf,null);assert.equal(evidence.exit.worker,false);
  const [download]=await Promise.all([p.waitForEvent('download'),p.locator('#save').click()]);const report=JSON.parse(fs.readFileSync(await download.path(),'utf8'));assert.equal(report.playback.length,512);assert.equal(report.workload,'academic-32-v2');evidence.downloadBytes=fs.statSync(await download.path()).size;
  await p.screenshot({path:path.join(out,'after-exit.png')});
  await p.locator('[data-action=present]').click();await p.waitForFunction(()=>presentation.ready);await p.evaluate(()=>presentation.preload);assert(await hidden());await p.mouse.click(100,100);await p.waitForFunction(()=>presentation.clock.running);await p.keyboard.press('Escape');await p.waitForFunction(()=>!presentation.closing);
  // A close in flight is idempotent, and cannot remove a newer session.
  await p.locator('[data-action=present]').click();await p.waitForFunction(()=>presentation.ready);
  const closing=await p.evaluate(async()=>{const realExit=document.exitFullscreen.bind(document);let release;document.exitFullscreen=()=>new Promise(resolve=>release=()=>realExit().catch(()=>{}).then(resolve));const a=presentation.close(),b=presentation.close();const data={same:a===b,removed:!document.querySelector('.presentation')};await presentation.open();data.reopenedWhileClosing=presentation.opened;release();await a;document.exitFullscreen=realExit;return data;});assert.deepEqual(closing,{same:true,removed:true,reopenedWhileClosing:false});
  await p.setViewportSize({width:760,height:720});await p.locator('[data-action=present]').click();await p.waitForFunction(()=>presentation.ready);await p.keyboard.press('Tab');await p.screenshot({path:path.join(out,'narrow-controls.png')});await p.keyboard.press('Escape');await p.waitForFunction(()=>!presentation.closing);
  // Main-thread CanvasKit fallback has the same lifecycle and controls.
  const fallback=await browser.newPage({viewport:{width:960,height:640}});fallback.on('pageerror',error=>errors.push(String(error)));
  await fallback.addInitScript(()=>{HTMLCanvasElement.prototype.transferControlToOffscreen=undefined;});await fallback.goto(new URL('presentation-test.html?deck=linked',url).href);await fallback.waitForFunction(()=>window.presentation?.deck);
  for(let i=0;i<2;i++){await fallback.locator('[data-action=present]').click();await fallback.waitForFunction(()=>presentation.ready);assert(await fallback.evaluate(()=>!!presentation.mainRuntime&&!presentation.worker));await fallback.keyboard.press('Tab');await fallback.keyboard.press('Escape');await fallback.waitForFunction(()=>!presentation.closing&&!document.querySelector('.presentation'));}
  // Cancel a slow renderer initialization, then reopen before its request
  // returns. The old session must not install a renderer into the new one.
  let delayed,hit;const requested=new Promise(resolve=>hit=resolve);
  await fallback.route('**/skia/manifest.json',route=>{if(!delayed){delayed=route;hit();}else route.continue();});
  await fallback.evaluate(()=>{window.oldOpening=presentation.open();});await requested;
  await fallback.evaluate(()=>presentation.close());await fallback.locator('[data-action=present]').click();await fallback.waitForFunction(()=>presentation.ready);
  await delayed.continue();await fallback.evaluate(()=>oldOpening);assert(await fallback.evaluate(()=>presentation.ready&&!!presentation.mainRuntime));
  await fallback.keyboard.press('Escape');await fallback.waitForFunction(()=>!presentation.closing);await fallback.close();
  assert.deepEqual(errors,[]);evidence.errors=errors;evidence.checks=['default hidden','Tab focus cycle','three context-menu modes','bottom-center hover zone','menu focus isolation','automatic completion then exit','bounded on-page diagnostics; full downloadable data','repeat open/close and closing race','narrow viewport','main-thread fallback','cancel initialization and reopen before the old request returns'];fs.writeFileSync(path.join(out,'checks.json'),JSON.stringify(evidence,null,2));console.log('Presentation controls and exit/reopen regression passed.');
 }catch(error){await p.screenshot({path:path.join(out,'failure.png')});throw error;}finally{await browser.close();}
})().catch(error=>{console.error(error);process.exitCode=1;});
