const fs=require('fs'),path=require('path'),assert=require('assert/strict');
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const [base,out]=process.argv.slice(2);if(!base||!out)throw Error('Usage: test-preview-presentation.cjs SITE_URL OUTPUT');fs.mkdirSync(out,{recursive:true});
const source=`page=canvas(size=(80mm,50mm),background="#f6f7fb")
style {
 @keyframes move { to { transform: translateX(10mm); } }
 @keyframes tint { to { opacity: .5; } }
 .move { animation: move .35s linear both; }
 .tint { animation: tint .35s linear both; }
}
card=page.add(rect(size=(20mm,15mm),fill="#1677ee"),offset=(10mm,15mm))
first=page.step(name="Entry",start="after-previous",hold=.1s)
first.animate(card,class="move")
second=page.step(name="Linked",start="after-previous",hold=.1s)
second.animate(card,class="tint")
last=page.step(name="Click")
last.animate(card,class="move",duration=.6s)
`;
(async()=>{
 const browser=await chromium.launch({headless:true,executablePath:process.env.CHROMIUM_PATH,args:['--no-sandbox','--no-proxy-server']});
 const p=await browser.newPage({viewport:{width:1440,height:1000}}),errors=[],checks=[],consoleMessages=[];p.setDefaultTimeout(60000);p.on('pageerror',e=>errors.push(String(e)));
 const consoleHealth=page=>page.on('console',message=>{if(['error','warning'].includes(message.type()))consoleMessages.push({level:message.type(),text:message.text()});});consoleHealth(p);
 const viewport=p.locator('.preview-viewport'),state=()=>viewport.evaluate(e=>({...e.dataset}));
 async function ready(previous){await p.waitForFunction(previous=>{const m=document.querySelector('.example-module');if(m?.dataset.liveState==='error')throw Error(m.querySelector('.live-diagnostics').textContent);return m?.dataset.painted==='true'&&m.dataset.liveState==='success'&&(!previous||m.dataset.generation!==previous);},previous);}
 async function show(fullscreen){if(fullscreen)await p.locator('[data-action=present]').click();else{await p.locator('.presentation-entry-menu > summary').click();await p.locator('[data-action=present-window-entry]').click();}await p.waitForFunction(()=>document.querySelector('.presentation')?.dataset.painted==='true');}
 async function chromeLayout(page=p){
  const layout=await page.evaluate(()=>{
   const header=document.querySelector('.preview-toolbar'),group=header.querySelector('.presentation-entry-group'),main=group.querySelector('button'),arrow=group.querySelector('summary');
   const box=e=>e.getBoundingClientRect().toJSON();
   return {header:box(header),main:box(main),arrow:box(arrow),children:[...header.children].filter(e=>e.getClientRects().length).map(box),wrap:getComputedStyle(header).flexWrap,groupDisplay:getComputedStyle(group).display,groupWrap:getComputedStyle(group).flexWrap};
  });
  assert.equal(layout.wrap,'nowrap');assert.equal(layout.groupDisplay,'flex');assert.equal(layout.groupWrap,'nowrap');
  assert(Math.abs(layout.main.y-layout.arrow.y)<1,'Present button and arrow must share a row');
  assert(Math.abs(layout.main.right-layout.arrow.left)<1,'Present button and arrow must be joined');
  for(const [i,box]of layout.children.entries()){
   assert(box.left>=layout.header.left&&box.right<=layout.header.right,'Toolbar controls stay inside the header');
   assert(box.top>=layout.header.top&&box.bottom<=layout.header.bottom,'Toolbar controls stay in one row');
   if(i)assert(box.left>=layout.children[i-1].right,'Toolbar controls do not overlap');
  }
 }
 async function hiddenChrome(page=p){
  const hidden=await page.locator('.preview-bar').evaluate(bar=>bar.inert&&bar.getAttribute('aria-hidden')==='true'&&getComputedStyle(bar).visibility==='hidden');
  assert(hidden,'Default preview bar must be hidden and inert');
  assert(await page.locator('.preview-menu').evaluate(menu=>menu.hidden&&getComputedStyle(menu).display==='none'&&[...menu.querySelectorAll('*')].every(e=>!e.getClientRects().length)),'Closed menu must hide all descendants');
  assert(await page.locator('.preview-menu button').first().evaluate(button=>{button.focus();return document.activeElement!==button;}),'Hidden menu buttons cannot receive focus');
 }
 async function boundedMenu(){
  await p.waitForFunction(()=>{
   const menu=document.querySelector('.preview-menu'),r=menu.getBoundingClientRect(),v=menu.parentElement.getBoundingClientRect();
   return !menu.hidden&&r.left>=v.left&&r.top>=v.top&&r.right<=v.right+1&&r.bottom<=v.bottom+1;
  });
  assert(await p.locator('.preview-menu').evaluate(menu=>{
   const toggle=menu.querySelector('.presentation-switch'),r=toggle.getBoundingClientRect(),b=toggle.parentElement.getBoundingClientRect(),style=getComputedStyle(menu);
   return style.backgroundColor==='rgb(39, 39, 41)'&&style.color==='rgb(243, 243, 244)'&&toggle.parentElement.textContent.includes('自动播放')&&r.left>=b.left&&r.right<=b.right&&r.top>=b.top&&r.bottom<=b.bottom;
  }),'Automatic playback switch stays inside its labelled menu row');
 }
 async function artDirectionChrome(){
  const crypto=require('crypto'),versions={};
  for(const name of ['styles.css','live/presentation.css'])versions[name]=crypto.createHash('sha256').update(fs.readFileSync(path.resolve(__dirname,'../site',name))).digest('hex').slice(0,16);
  const expectedStyles=async page=>{
   const styles=await page.locator('head link[rel=stylesheet]').evaluateAll(links=>links.map(link=>({href:link.href,loaded:!!link.sheet})));
   for(const [name,version]of Object.entries(versions)){
    const matching=styles.filter(link=>new URL(link.href).pathname.endsWith('/site/'+name));
    assert.equal(matching.length,1,'Shared CSS is loaded exactly once');assert(matching[0].loaded);assert.equal(new URL(matching[0].href).searchParams.get('v'),version);
   }
  };
  await p.goto(new URL('examples/animation-art-direction.html',base).href);await p.locator('.example-module').scrollIntoViewIfNeeded();await ready();
  if(await p.locator('.toc-toggle').getAttribute('aria-expanded')==='true')await p.locator('.toc-toggle').click();
  await expectedStyles(p);
  for(const [name,width,height]of [['screenshot',3840,1840],['desktop',1440,1000],['mobile',390,844]]){
   await p.setViewportSize({width,height});await viewport.scrollIntoViewIfNeeded();await chromeLayout();await hiddenChrome();
   await p.screenshot({path:path.join(out,'art-direction-'+name+'.png')});
   const before=await viewport.evaluate(e=>[e.clientWidth,e.clientHeight]);
   await viewport.focus();await p.keyboard.press('Tab');assert.equal(await p.locator('.preview-bar').getAttribute('aria-hidden'),'false');
   assert.deepEqual(await viewport.evaluate(e=>[e.clientWidth,e.clientHeight]),before,'Showing controls preserves the canvas area');
   await p.locator('[data-action=preview-menu]').click();await boundedMenu();
   await p.screenshot({path:path.join(out,'art-direction-'+name+'-menu.png')});
   if(name==='desktop'){
    await p.setViewportSize({width:390,height:844});await boundedMenu();await p.setViewportSize({width,height});await boundedMenu();
   }
   await p.keyboard.press('Escape');await hiddenChrome();
   assert.deepEqual(await viewport.evaluate(e=>[e.clientWidth,e.clientHeight]),before,'Menus do not reserve canvas space');
   await p.locator('.presentation-entry-menu > summary').click();
   assert(await p.locator('.presentation-entry-popover').evaluate(e=>{const r=e.getBoundingClientRect();return r.left>=0&&r.right<=innerWidth;}));
   await p.keyboard.press('Escape');assert.equal(await p.locator('.presentation-entry-menu').getAttribute('open'),null);
  }
  await p.setViewportSize({width:1440,height:1000});await p.reload();await viewport.scrollIntoViewIfNeeded();await ready();await expectedStyles(p);await hiddenChrome();await chromeLayout();
  await p.locator('.theme-toggle').click();await viewport.click({button:'right',position:{x:30,y:30}});await boundedMenu();await p.screenshot({path:path.join(out,'art-direction-dark-menu.png')});await p.keyboard.press('Escape');await p.locator('.theme-toggle').click();
  checks.push('Art Direction: versioned head styles load once; refresh, 3840/1440/390 widths, joined entry, hidden focus isolation, Tab, labelled bounded menus, resize and dark theme');
  const delayed=await browser.newPage({viewport:{width:1440,height:1000}}),styleRequests=[];delayed.on('pageerror',e=>errors.push(String(e)));consoleHealth(delayed);delayed.on('request',request=>{if(new URL(request.url()).pathname.endsWith('/presentation.css'))styleRequests.push(request.url());});
  let release,requested;const gate=new Promise(resolve=>release=resolve),request=new Promise(resolve=>requested=resolve);
  await delayed.route('**/presentation.css?*',async route=>{requested(route.request().url());await gate;await route.continue();});
  const navigation=delayed.goto(new URL('examples/animation-art-direction.html',base).href,{waitUntil:'domcontentloaded'});
  try{
   const requestedURL=await Promise.race([request,new Promise((_,reject)=>setTimeout(()=>reject(Error('Versioned presentation CSS was not requested')),10000))]);
   assert.equal(new URL(requestedURL).searchParams.get('v'),versions['live/presentation.css']);
   assert.equal(await delayed.locator('.presentation-entry-group').count(),0,'Preview controls wait for critical CSS');
   await new Promise(resolve=>setTimeout(resolve,250));release();await navigation;
   await delayed.locator('.example-module').scrollIntoViewIfNeeded();await delayed.waitForFunction(()=>document.querySelector('.example-module')?.dataset.painted==='true');
   await expectedStyles(delayed);assert.deepEqual(styleRequests,[requestedURL]);await hiddenChrome(delayed);await chromeLayout(delayed);await delayed.screenshot({path:path.join(out,'art-direction-delayed-css.png')});
   checks.push('Delayed presentation CSS blocks control initialization until styled; no duplicate unversioned stylesheet request');
  }finally{release();await navigation.catch(()=>{});await delayed.close();}
 }
 try{
  await p.addInitScript(()=>{window.previewQA={runs:0,frames:[]};const WorkerBase=Worker;window.Worker=class extends WorkerBase{constructor(...args){super(...args);this.addEventListener('message',({data})=>{if(data.type==='result')previewQA.animation=data.inspection.animation;});}postMessage(data,...args){if(data.type==='run')previewQA.runs++;if(data.type==='paint')previewQA.frames.push({time:data.time,running:data.running});return super.postMessage(data,...args);}};});
  await artDirectionChrome();
  await p.goto(new URL('examples/animation-second.html',base).href);if(await p.locator('.toc-toggle').getAttribute('aria-expanded')==='true')await p.locator('.toc-toggle').click();await p.locator('.example-module').scrollIntoViewIfNeeded();await ready();await p.locator('.cm-content').first().waitFor();
  const original=await p.locator('.example-module').getAttribute('data-generation');await p.locator('.cm-content').first().fill(source);await ready(original);
  assert.equal(await p.locator('.preview-toolbar > .presentation-entry').count(),0);assert.equal(await p.locator('.presentation-entry-group > button').innerText(),'放映');assert.equal(await p.locator('.presentation-entry-menu').getAttribute('open'),null);
  assert.equal((await state()).time,'static');await p.waitForTimeout(900);assert.equal((await state()).time,'static');assert.equal((await state()).running,'false');checks.push('Single Present entry; Windowed lives in dropdown; preview suppresses automatic entry');
  const runs=await p.evaluate(()=>previewQA.runs);await viewport.click({position:{x:120,y:130}});await p.waitForFunction(()=>document.querySelector('.preview-viewport').dataset.completed==='2'&&document.querySelector('.preview-viewport').dataset.running==='false');assert.notEqual((await state()).time,'static');checks.push('First preview click starts entry and automatically follows linked step, then waits at click step');
  await p.keyboard.press('ArrowRight');await p.waitForFunction(()=>document.querySelector('.preview-viewport').dataset.running==='true');await viewport.click({position:{x:120,y:130}});assert.equal((await state()).completed,'3');assert.equal((await state()).running,'false');await p.keyboard.press('ArrowLeft');assert.equal((await state()).completed,'2');checks.push('Click/arrow navigation shares finish-current and backward-boundary behavior');
  await p.keyboard.press('ArrowRight');await viewport.click({button:'right',position:{x:120,y:130}});await viewport.locator('.preview-menu').waitFor({state:'visible'});const paused=(await state()).time;await p.waitForTimeout(100);assert.equal((await state()).time,paused);await p.keyboard.press('Escape');assert.equal((await state()).running,'true');await p.keyboard.press('s');assert.equal((await state()).running,'false');
  await p.keyboard.press('b');assert.equal(await viewport.locator('.preview-mask').getAttribute('data-color'),'black');await p.keyboard.press('b');await p.keyboard.press('Escape');assert.equal((await state()).time,'static');checks.push('Preview context menu pauses/resumes; shortcuts and Escape restore static');
  const box=await viewport.boundingBox();await p.mouse.move(box.x+100,box.y+120);await p.mouse.down();await p.mouse.move(box.x+140,box.y+150,{steps:4});await p.mouse.up();assert.equal((await state()).time,'static');assert.equal(await p.evaluate(()=>previewQA.runs),runs);checks.push('Dragging does not advance; preview navigation does not recompile');
  await viewport.click({button:'right',position:{x:120,y:130}});await viewport.getByRole('menuitemcheckbox',{name:'自动播放',exact:true}).click();await p.keyboard.press('Escape');assert.equal((await state()).time,'static');assert.equal((await state()).running,'false');
  await viewport.click({position:{x:120,y:130}});await p.waitForFunction(()=>document.querySelector('.preview-viewport').dataset.completed==='3'&&document.querySelector('.preview-viewport').dataset.running==='false');await p.keyboard.press('Escape');
  await viewport.click({button:'right',position:{x:120,y:130}});await viewport.getByRole('menuitemcheckbox',{name:'自动播放',exact:true}).click();await viewport.getByRole('menuitem',{name:'时间定位',exact:true}).click();await viewport.getByLabel('预览动画时间',{exact:true}).fill('0.2');assert.notEqual((await state()).time,'static');await p.keyboard.press('Escape');await p.keyboard.press('Escape');await p.keyboard.press('Escape');assert.equal((await state()).time,'static');checks.push('Selecting Auto keeps initial preview static; explicit click runs automatically; timeline sampling works from static');
  await p.screenshot({path:path.join(out,'preview.png')});await show(true);assert(await p.evaluate(()=>document.fullscreenElement===document.querySelector('.presentation')));await p.waitForFunction(()=>Number(document.querySelector('.presentation').dataset.time)>.1);await p.keyboard.press('Escape');await p.locator('.presentation').waitFor({state:'detached'});assert.equal(await p.evaluate(()=>document.fullscreenElement),null);assert.equal((await state()).time,'static');checks.push('Present starts fullscreen with automatic entry; Escape ends it directly');
  await show(true);await p.evaluate(()=>document.exitFullscreen());await p.locator('.presentation').waitFor({state:'detached'});checks.push('Browser fullscreen exit also ends presentation without a windowed overlay');
  await show(false);assert.equal(await p.evaluate(()=>document.fullscreenElement),null);await p.keyboard.press('Escape');await p.locator('.presentation').waitFor({state:'detached'});checks.push('Dropdown starts windowed presentation and restores preview on exit');
  await p.locator('.workspace-expand').click();await p.locator('.workspace-dialog').waitFor();await show(false);await p.locator('.presentation-canvas').click({position:{x:100,y:120}});await p.keyboard.press('Escape');await p.locator('.presentation').waitFor({state:'detached'});assert(await p.locator('.workspace-dialog').isVisible());await p.locator('.workspace-expand').click();checks.push('Windowed presentation works above the expanded workspace');
  await p.setViewportSize({width:390,height:844});await p.locator('.example-module').scrollIntoViewIfNeeded();await p.locator('.presentation-entry-menu > summary').click();const bounded=await p.locator('.presentation-entry-popover').evaluate(e=>{const r=e.getBoundingClientRect();return r.x>=0&&r.right<=innerWidth;});assert(bounded);await p.screenshot({path:path.join(out,'mobile.png')});checks.push('Mobile dropdown remains within viewport');
  assert.deepEqual(errors,[]);assert.deepEqual(consoleMessages.filter(message=>message.level==='error'),[]);fs.writeFileSync(path.join(out,'checks.json'),JSON.stringify({checks,errors,consoleMessages},null,2));console.log(JSON.stringify({checks,errors,consoleMessages}));
 }catch(error){await p.screenshot({path:path.join(out,'failure.png')}).catch(()=>{});console.error(await state().catch(()=>({})));throw error;}finally{await browser.close();}
})().catch(error=>{console.error(error);process.exitCode=1;});
