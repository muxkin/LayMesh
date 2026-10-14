const pw=require(process.env.PLAYWRIGHT_MODULE||'playwright'),assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path');
(async()=>{const [file,out='/tmp/laymesh-presenter-v004/natural']=process.argv.slice(2);fs.mkdirSync(out,{recursive:true});const results=[];
 for(const name of (process.env.LAYMESH_BROWSERS||'chromium,firefox,webkit').split(',')){
 const b=await pw[name].launch({headless:true,ignoreDefaultArgs:['--disable-popup-blocking'],...(name==='chromium'?{args:['--enable-unsafe-swiftshader']}: {})});try{
  const ctx=await b.newContext({viewport:{width:1280,height:800}}),p=await ctx.newPage(),errors=[];ctx.on('page',p=>p.on('pageerror',e=>errors.push(e.message)));p.setDefaultTimeout(90000);await p.goto('file://'+path.resolve(file));await p.waitForFunction(()=>window.__LAYMESH_WEB&&document.querySelector('#app').dataset.painted==='true');await p.selectOption('#page','5');await p.click('#preview-more summary');const popup=ctx.waitForEvent('page');await p.click('#presenter');const control=await popup;control.setDefaultTimeout(90000);await control.waitForFunction(()=>window.laymeshPresenterConsole?.state?.ready);
  const pages=[];
  for(let i=0;i<6;i++){
   await p.evaluate(i=>laymeshPresentation.go(i),i);const deadline=Date.now()+60000;
   while(Date.now()<deadline){const state=await p.evaluate(()=>({running:laymeshPresentation.clock.running,...laymeshPresentation.clock.status()}));if(['complete','static'].includes(state.animationStatus))break;if(!state.running)await p.evaluate(()=>laymeshPresentation.action('next'));await p.waitForTimeout(100);}
   await control.waitForFunction(i=>laymeshPresenterConsole.state.index===i&&['complete','static'].includes(laymeshPresenterConsole.state.animationStatus),i);
   assert.equal(await control.locator('progress').evaluate(n=>n.value),1);
   const state=await p.evaluate(()=>({id:laymeshPresentation.deck.pages[laymeshPresentation.index].id,time:laymeshPresentation.clock.time,duration:laymeshPresentation.clock.duration,...laymeshPresentation.clock.status()}));pages.push(state);
  }
  assert.equal(pages[5].time,6);assert.equal(pages[5].duration,7);
  await control.waitForFunction(()=>document.querySelector('.presenter').dataset.framePage==='editable');await p.waitForTimeout(150);
  // Compare downsized displayed pixels, not two independent scene renders.
  const actual=await control.evaluate(()=>{const c=document.createElement('canvas');c.width=64;c.height=36;const x=c.getContext('2d');x.drawImage(laymeshPresenterConsole.currentCanvas,0,0,64,36);return Array.from(x.getImageData(0,0,64,36).data)});
  const expected=await p.evaluate(()=>{const v=laymeshPresentation,c=document.createElement('canvas');c.width=64;c.height=36;const x=c.getContext('2d'),r=__LAYMESH_WEB.presenter.mirror.rect({pageId:'editable'},v.canvas);x.drawImage(v.canvas,...r,0,0,64,36);return Array.from(x.getImageData(0,0,64,36).data)});
  const pixelError=actual.reduce((n,x,i)=>n+Math.abs(x-expected[i]),0)/actual.length;assert(pixelError<6,`Mirror differs from audience: ${pixelError}`);
  if(name==='chromium'){await control.waitForFunction(()=>[...document.querySelectorAll('.presenter-filmstrip img')].every(i=>i.naturalWidth>0));await control.screenshot({path:path.join(out,'presenter.png')});}
  await p.emulateMedia({reducedMotion:'reduce'});await p.evaluate(()=>laymeshPresentation.go(5));await p.waitForTimeout(150);for(let n=0;n<200;n++){const s=await p.evaluate(()=>({running:laymeshPresentation.clock.running,...laymeshPresentation.clock.status()}));if(s.animationStatus==='complete')break;if(!s.running)await p.evaluate(()=>laymeshPresentation.action('pause'));await p.waitForTimeout(100);}await control.waitForFunction(()=>laymeshPresenterConsole.state.reduced&&laymeshPresenterConsole.state.animationStatus==='complete');assert.equal(await control.locator('progress').evaluate(n=>n.value),1);
  await control.click('[data-presenter-action=end]');await p.waitForFunction(()=>!__LAYMESH_WEB.presenter.opened);assert.deepEqual(errors,[]);results.push({browser:name,pages,pixelMeanAbsoluteError:pixelError,reduced:true,errors});console.log(name,'six pages naturally complete; mirror error',pixelError);
 }finally{await b.close();}}
 fs.writeFileSync(path.join(out,'natural.json'),JSON.stringify(results,null,2)+'\n');
})().catch(e=>{console.error(e);process.exitCode=1});
