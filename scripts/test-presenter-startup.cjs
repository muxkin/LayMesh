// The offline entry must never inspect or arrange displays, even with extended
// virtual screens and granted permission. Popup blocking stays enabled.
const pw=require(process.env.PLAYWRIGHT_MODULE||'playwright'),assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path');
(async()=>{
 const [file,out='/tmp/laymesh-presenter-v006/startup.json']=process.argv.slice(2);assert(file);fs.mkdirSync(path.dirname(out),{recursive:true});
 const browser=await pw.chromium.launch({headless:true,executablePath:process.env.LAYMESH_BROWSER_EXECUTABLE||undefined,ignoreDefaultArgs:['--disable-popup-blocking'],args:['--enable-unsafe-swiftshader']});const results=[];
 try{for(const mode of (process.env.LAYMESH_STARTUP_CASES||'unready,single,extended,mirrored,unsupported,denied,manual,popup-blocked,fullscreen-denied').split(',')){
  const context=await browser.newContext({viewport:null}),errors=[];
  context.on('page',p=>p.on('pageerror',e=>errors.push(e.message)));
  await context.addInitScript(mode=>{
   const calls=window.screenPolicyCalls={details:0,extended:0,permission:0,move:0,resize:0,popups:[],fullscreen:[]};
   const descriptor=Object.getOwnPropertyDescriptor(Screen.prototype,'isExtended');
   Object.defineProperty(screen,'isExtended',{configurable:true,get(){calls.extended++;return mode==='mirrored'||!!descriptor?.get?.call(screen);}});
   const get=window.getScreenDetails?.bind(window);
   window.getScreenDetails=mode==='unsupported'?undefined:async()=>{calls.details++;if(mode==='denied')throw new DOMException('Denied','NotAllowedError');return get?.();};
   const query=navigator.permissions?.query.bind(navigator.permissions);
   if(query)navigator.permissions.query=permission=>{if(permission.name==='window-management')calls.permission++;return query(permission);};
   for(const [method,key]of [['moveTo','move'],['resizeTo','resize']]){const original=window[method];window[method]=function(...args){calls[key]++;return original.apply(this,args);};}
   const open=window.open;window.allowPopup=mode!=='popup-blocked';window.open=function(...args){calls.popups.push(args[2]||'');return window.allowPopup?open.apply(this,args):null;};
   const fullscreen=Element.prototype.requestFullscreen;window.allowFullscreen=mode!=='fullscreen-denied';Element.prototype.requestFullscreen=function(options){calls.fullscreen.push({active:navigator.userActivation.isActive,screen:!!options?.screen});return window.allowFullscreen?fullscreen.call(this,options):Promise.reject(Error('Test fullscreen denied'));};
  },mode);
  const page=await context.newPage();page.setDefaultTimeout(60000);
  let added,cdp;
  if(['extended','denied','manual'].includes(mode)){
   cdp=await context.newCDPSession(page);added=await cdp.send('Emulation.addScreen',{left:-1920,top:0,width:1920,height:1080,label:'Manual projector',isInternal:false});
   const {targetInfo}=await cdp.send('Target.getTargetInfo');await cdp.send('Browser.setPermission',{permission:{name:'window-management'},setting:mode==='denied'?'denied':'granted',browserContextId:targetInfo.browserContextId});
  }
  await page.goto(/^https?:/.test(file)?file:'file://'+path.resolve(file));await page.waitForFunction(()=>window.__LAYMESH_WEB&&document.querySelector('#app').dataset.painted==='true');
  const assertNoScreenPolicy=async p=>{const calls=await p.evaluate(()=>window.screenPolicyCalls);assert(calls,'instrumentation is installed');for(const key of ['details','extended','permission','move','resize'])assert.equal(calls[key],0,`${mode}: ${key}`);assert(calls.fullscreen.every(c=>c.active&&!c.screen));assert(calls.popups.every(features=>!/(?:left|top)=/.test(features)));return calls;};
  await assertNoScreenPolicy(page);const selected=await page.evaluate(()=>{const pages=__LAYMESH_WEB.pages,index=Math.min(4,pages.length-1);return {index,id:pages[index].id};});await page.selectOption('#page',String(selected.index));
  if(mode==='unready'){
   const state=await page.evaluate(async()=>{const v=__LAYMESH_WEB.presentation;await v.prepare();await v.open({fullscreen:false,deferPreparation:true});v.sync();document.dispatchEvent(new Event('visibilitychange'));const result={duration:v.clock.duration,label:v.label.textContent,disabled:v.scrub.disabled};await v.close();return result;});
   assert.equal(state.duration,0);assert.equal(state.disabled,true);assert.match(state.label,/准备/);results.push({mode,state,passed:true});
  }else if(['manual','popup-blocked'].includes(mode)){
   const popup=mode==='manual'?context.waitForEvent('page'):null;
   await page.click('#preview-more summary');await page.click('#presenter');await page.waitForFunction(()=>__LAYMESH_WEB.presentation.ready);
   assert.equal(await page.evaluate(()=>document.fullscreenElement),null);assert.equal((await assertNoScreenPolicy(page)).fullscreen.length,0);
   let control;
   if(popup)control=await popup;
   else{
    await page.waitForFunction(()=>__LAYMESH_WEB.presenter.notices.has('connection'));assert.match(await page.locator('.presenter-recovery').textContent(),/拦截/);assert.equal(context.pages().length,1);
    await page.evaluate(()=>window.allowPopup=true);const retry=context.waitForEvent('page');await page.click('[data-action=presenter-reconnect]');control=await retry;
   }
   control.setDefaultTimeout(60000);await control.waitForFunction(()=>window.laymeshPresenterConsole?.state?.ready);
   assert.equal(await page.evaluate(()=>laymeshPresentation.deck.pages[laymeshPresentation.index].id),selected.id);
   assert.equal(await control.locator('.presenter-screen-select,[data-presenter-action=swap-screens]').count(),0);assert.equal(await page.locator('.presenter-recovery select,[data-action=swap-screens]').count(),0);
   await control.click('[data-presenter-action=reconnect]');await page.waitForFunction(()=>__LAYMESH_WEB.presenter.notices.has('confirmation'));assert.equal((await assertNoScreenPolicy(page)).fullscreen.length,0);
   await page.bringToFront();await page.click('[data-action=audience-fullscreen]');await page.waitForFunction(()=>document.fullscreenElement===laymeshPresentation.root);
   await page.evaluate(()=>document.exitFullscreen());assert.equal(await page.evaluate(()=>laymeshPresentation.opened),true);
   await control.setViewportSize({width:1366,height:768});await control.screenshot({path:path.join(path.dirname(out),mode+'-console.png')});
   if(added){await cdp.send('Emulation.removeScreen',{screenId:added.screenInfo.id});assert.equal(control.isClosed(),false);assert.equal(await page.evaluate(()=>__LAYMESH_WEB.presenter.opened),true);}
   const calls=await assertNoScreenPolicy(page),consoleCalls=await assertNoScreenPolicy(control);
   await control.click('[data-presenter-action=end]');await page.waitForFunction(()=>!__LAYMESH_WEB.presenter.opened&&!__LAYMESH_WEB.presentation.opened);assert.equal(await page.locator('#page').inputValue(),String(selected.index));
   results.push({mode,passed:true,calls,consoleCalls,manualFullscreen:true,currentPage:true,exitRestoresPreview:true});
  }else{
   await page.click('#full');await page.waitForFunction(()=>__LAYMESH_WEB.presentation.ready);assert.equal(context.pages().length,1);assert.equal(await page.evaluate(()=>!!__LAYMESH_WEB.presenter.opened),false);assert.equal(await page.evaluate(()=>__LAYMESH_WEB.presentation.deck.pages[__LAYMESH_WEB.presentation.index].id),selected.id);
   assert.doesNotMatch(await page.locator('.presentation-warning').textContent(),/无法检测|屏幕配置/);
   if(mode==='fullscreen-denied'){
    assert.equal(await page.evaluate(()=>document.fullscreenElement),null);assert.equal(await page.evaluate(()=>__LAYMESH_WEB.presentation.metrics.fullscreenResult.ok),false);
    await page.evaluate(()=>window.allowFullscreen=true);await page.locator('.presentation').focus();await page.keyboard.press('f');
   }
   await page.waitForFunction(()=>document.fullscreenElement===__LAYMESH_WEB.presentation.root);const calls=await assertNoScreenPolicy(page);assert.equal(calls.popups.length,0);
   await page.evaluate(()=>__LAYMESH_WEB.presentation.close());await page.waitForFunction(()=>!__LAYMESH_WEB.presentation.closing);assert.equal(await page.locator('#page').inputValue(),String(selected.index));results.push({mode,passed:true,calls,ordinaryFullscreen:true,currentPage:true});
  }
  assert.deepEqual(errors,[]);await context.close();console.log(mode,'passed');
 }}finally{await browser.close();}
 fs.writeFileSync(out,JSON.stringify({browser:browser.version(),platform:process.platform,displayEnvironment:'CDP virtual screens; no physical monitor placement claim',results},null,2)+'\n');
})().catch(e=>{console.error(e);process.exitCode=1;});
