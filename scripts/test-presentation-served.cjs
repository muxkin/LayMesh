// Native session -> external browser -> authoritative audience window.
const fs=require('fs'),path=require('path'),assert=require('assert/strict');
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
(async()=>{
 const [url,out]=process.argv.slice(2),browser=await chromium.launch({headless:true,ignoreDefaultArgs:['--disable-popup-blocking'],executablePath:process.env.CHROMIUM_PATH,args:['--no-sandbox','--no-proxy-server']});
 fs.mkdirSync(out,{recursive:true});
 try{
  const context=await browser.newContext({viewport:{width:1280,height:800}}),errors=[];
  context.on('page',p=>p.on('pageerror',e=>errors.push(String(e))));
  const p=await context.newPage(),entry=new URL(url);entry.searchParams.set('page','second');
  await p.goto(entry.href);await p.waitForFunction(()=>window.laymeshPresentation?.deck);
  await p.locator('[data-action=present]').click();await p.waitForFunction(()=>laymeshPresentation.ready);
  assert(await p.evaluate(()=>!!document.fullscreenElement));
  assert.equal(context.pages().length,1);
  assert.equal(await p.evaluate(()=>laymeshPresentation.deck.pages[laymeshPresentation.index].id),'second');
  const pages=await p.evaluate(()=>laymeshPresentation.deck.pages.length);
  await p.locator('.presentation-canvas').click({position:{x:50,y:50}});await p.keyboard.press('End');
  await p.waitForFunction(()=>laymeshPresentation.index===laymeshPresentation.deck.pages.length-1&&!laymeshPresentation.switching);
  await p.screenshot({path:path.join(out,'external-presentation.png')});await p.keyboard.press('Escape');
  await p.waitForFunction(()=>!laymeshPresentation.opened);
  await p.locator('.presentation-entry-menu summary').click();const popup=context.waitForEvent('page');
  await p.locator('[data-action=presenter-entry]').click();const control=await popup;
  await control.waitForFunction(()=>window.laymeshPresenterConsole?.state?.ready);
  assert.equal(await p.evaluate(()=>document.fullscreenElement),null);
  await p.waitForFunction(()=>laymeshPresentation.presenter?.state?.pageId==='second');
  assert.equal(await p.evaluate(()=>laymeshPresentation.deck.pages[laymeshPresentation.index].id),'second');
  await control.locator('[data-presenter-action=black]').click();await p.waitForFunction(()=>!laymeshPresentation.mask.hidden);
  await control.locator('[data-presenter-action=end]').click();await p.waitForFunction(()=>!laymeshPresentation.opened);
  assert.deepEqual(errors,[]);
  fs.writeFileSync(path.join(out,'external.json'),JSON.stringify({pages,errors,checks:['prepared native artifacts','selected-page fullscreen browser presentation','navigate without compiler','presenter and audience from selected page','audience blackout and clean exit','no console errors']},null,2));
 }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
