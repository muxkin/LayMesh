// Approved presentation UI, failure recovery, fallback and version boundaries.
const fs=require('fs'),path=require('path'),assert=require('assert/strict');
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const [url,out]=process.argv.slice(2);if(!url||!out)throw Error('Usage: test-presentation-ui.cjs SITE_URL OUTPUT');fs.mkdirSync(out,{recursive:true});
(async()=>{
 const browser=await chromium.launch({headless:true,executablePath:process.env.CHROMIUM_PATH,args:['--no-sandbox','--no-proxy-server']});
 const p=await browser.newPage({viewport:{width:1672,height:941}}),errors=[];p.setDefaultTimeout(90000);p.on('pageerror',e=>errors.push(String(e)));
 try{
  await p.goto(new URL('examples/animation-presentation.html',url).href);await p.waitForFunction(()=>document.querySelector('.example-module')?.dataset.painted==='true');
  await p.locator('[data-action=present]').click();await p.waitForFunction(()=>document.querySelector('.presentation')?.dataset.painted==='true');
  assert(await p.evaluate(()=>document.fullscreenElement===document.querySelector('.presentation')));
  await p.keyboard.press('Tab');await p.locator('[data-action=present-menu]').click();await p.getByRole('menuitem',{name:'时间定位',exact:true}).click();await p.getByLabel('动画时间',{exact:true}).fill('7.7');await p.keyboard.press('Escape');await p.keyboard.press('Escape');
  await p.keyboard.press('Tab');await p.mouse.move(50,60);await p.screenshot({path:path.join(out,'fullscreen.png')});
  const chrome=await p.evaluate(()=>{const root=document.querySelector('.presentation'),c=root.querySelector('canvas'),b=root.querySelector('.presentation-bar');return{screen:[innerWidth,innerHeight],buffer:[c.width,c.height],bar:b.getBoundingClientRect().toJSON(),canvas:c.getBoundingClientRect().toJSON(),visibleLabels:b.innerText};});
  assert.deepEqual(chrome.buffer,chrome.screen);assert.equal(chrome.canvas.x,0);assert.equal(chrome.canvas.y,0);assert.equal(chrome.screen[1]-chrome.bar.bottom,24);assert.equal(chrome.bar.height,44);assert.equal(chrome.visibleLabels.trim(),'1 / 2');
  await p.locator('.presentation').focus();await p.waitForFunction(()=>document.querySelector('.presentation-bar').classList.contains('idle'));await p.mouse.move(51,60);assert(await p.locator('.presentation-bar').evaluate(e=>e.inert));await p.keyboard.press('Tab');await p.waitForFunction(()=>!document.querySelector('.presentation-bar').classList.contains('idle'));
  await p.keyboard.press('Tab');await p.locator('[data-action=present-menu]').click();await p.screenshot({path:path.join(out,'menu.png')});await p.getByRole('menuitemcheckbox',{name:'自动播放'}).click();assert.equal(await p.getByRole('menuitemcheckbox').getAttribute('aria-checked'),'true');await p.keyboard.press('Escape');await p.keyboard.press('s');
  await p.keyboard.press('Tab');await p.locator('[data-action=present-pages]').click();await p.locator('.presentation-pages button').nth(1).click();await p.waitForFunction(()=>document.querySelector('.presentation').dataset.page==='second');
  await p.locator('.presentation').focus();await p.keyboard.press('w');assert.equal(await p.locator('.presentation-mask').getAttribute('data-color'),'white');await p.keyboard.press('w');await p.keyboard.press('Escape');
  // Preserve an already prepared version even when the source becomes invalid.
  await p.evaluate(async()=>{const base=new URL('/site/live/',location),{Presentation}=await import(new URL('presentation.mjs',base)),{webPresentationProvider}=await import(new URL('presentation-web.mjs',base)),m=document.querySelector('.example-module'),files={};for(const panel of m.querySelectorAll('[data-edit-file]'))files[panel.dataset.editFile]=panel.querySelector('.code-raw').defaultValue;window.testEditor={files,entry:m.dataset.entry,loadedFonts:[],module:{dataset:{}}};window.qaShow=new Presentation({base,fullscreen:false,provider:webPresentationProvider(testEditor,base)});await qaShow.prepare();window.qaDeck=qaShow.deck;await qaShow.open();testEditor.files[testEditor.entry]='invalid source for next version';qaShow.invalidate();});
  assert(await p.evaluate(()=>qaShow.deck===qaDeck&&qaShow.stale));await p.evaluate(async()=>{await qaShow.go(1);await qaShow.close();});
  await p.evaluate(()=>qaShow.open());await p.locator('.presentation-progress').waitFor({state:'visible'});await p.waitForFunction(()=>!qaShow.retry.hidden);await p.screenshot({path:path.join(out,'prepare-failure.png')});
  await p.evaluate(()=>{const panel=document.querySelector(`[data-edit-file="${testEditor.entry}"]`);testEditor.files[testEditor.entry]=panel.querySelector('.code-raw').defaultValue;});await p.getByRole('button',{name:'重试',exact:true}).click();await p.waitForFunction(()=>qaShow.ready);assert(await p.evaluate(()=>qaShow.deck!==qaDeck));await p.evaluate(()=>qaShow.close());
  // Same prepared document on the main-thread CanvasKit fallback, no compiler.
  await p.evaluate(()=>{Object.defineProperty(HTMLCanvasElement.prototype,'transferControlToOffscreen',{configurable:true,value:undefined});const get=HTMLCanvasElement.prototype.getContext;HTMLCanvasElement.prototype.getContext=function(type,...args){const result=get.call(this,type,...args);if(type.startsWith('webgl')&&result)window.qaGL=result;return result;};});
  await p.evaluate(()=>qaShow.open());await p.waitForFunction(()=>qaShow.ready);assert(await p.evaluate(()=>!!qaShow.mainRuntime&&!qaShow.worker));await p.evaluate(()=>qaShow.go(0,{end:true}));await p.evaluate(()=>qaShow.preload);
  const frames=await p.evaluate(()=>qaShow.metrics.transitions.length);
  const restore=await p.evaluate(()=>{const ext=qaGL?.getExtension('WEBGL_lose_context');if(!ext)return false;ext.loseContext();setTimeout(()=>ext.restoreContext(),100);return true;});
  if(restore)await p.waitForFunction(n=>qaShow.metrics.transitions.length>n&&!qaShow.switching,frames);
  await p.evaluate(()=>qaShow.go(1,{end:true}));await p.evaluate(()=>qaShow.close());
  // Narrow viewport retains controls and menus inside the available area.
  await p.setViewportSize({width:360,height:740});await p.evaluate(()=>qaShow.open());await p.waitForFunction(()=>qaShow.ready);await p.keyboard.press('Tab');await p.locator('[data-action=present-menu]').click();
  const bounded=await p.locator('.presentation-menu').evaluate(e=>{const r=e.getBoundingClientRect();return r.x>=0&&r.y>=0&&r.right<=innerWidth&&r.bottom<=innerHeight;});assert(bounded);await p.screenshot({path:path.join(out,'narrow.png')});await p.evaluate(()=>qaShow.dispose());
  assert.deepEqual(errors,[]);fs.writeFileSync(path.join(out,'checks.json'),JSON.stringify({chrome,contextRestored:restore,errors,hardwareAcceptance:false,checks:['actual fullscreen and original resolution','toolbar overlays canvas, no reserved strip','default hiding, pointer isolation and Tab recovery','menus, timeline, auto mode, page list and white screen','fixed version, preparation error and retry','main-thread CanvasKit and context restore','narrow bounded menu']},null,2));
  console.log('Presentation UI, fullscreen, retry, frozen versions, main-thread fallback and context restoration passed.');
 }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
