// Sample every page of a real offline artifact in all three browser engines.
const fs=require('fs'),path=require('path'),assert=require('assert/strict');
const {pathToFileURL}=require('url'),pw=require(process.env.PLAYWRIGHT_MODULE||'playwright');
(async()=>{
 const [file,out='/tmp/laymesh-presentation-pages',screenshotPageId]=process.argv.slice(2);
 if(!file)throw Error('Usage: test-presentation-pages.cjs artifact.zip.html [output] [screenshot-page-id]');
 fs.mkdirSync(out,{recursive:true});const results=[];
 for(const name of ['chromium','firefox','webkit']){
  const browser=await pw[name].launch({headless:true,...(name==='chromium'?{args:['--no-sandbox','--enable-unsafe-swiftshader']}: {})});
  try{
   const p=await browser.newPage({viewport:{width:1280,height:800}}),errors=[];p.setDefaultTimeout(90000);p.on('pageerror',e=>errors.push(e.message));
   await p.goto(pathToFileURL(path.resolve(file)).href);await p.waitForFunction(()=>window.__LAYMESH_WEB&&document.querySelector('#app').dataset.painted==='true');
   const pages=await p.evaluate(async()=>{
    const v=window.__LAYMESH_WEB.presentation,ids=window.__LAYMESH_WEB.pages.map(p=>p.id),selected=ids[Math.max(0,ids.length-2)];
    await v.open({fullscreen:false,startPageId:selected});if(v.deck.pages[v.index].id!==selected)throw Error('Current page lost');
    const results=[];for(let i=0;i<ids.length;i++){
     await v.go(i);v.clock.running=false;await v.preload;const frames=[];
     for(const fraction of [0,.2,.5,.8,1]){const frame=await v.call({type:'paint',serial:v.serial,pageId:ids[i],time:v.clock.duration*fraction,reduced:false,running:false,...v.dimensions()});frames.push({fraction,backend:frame.backend,bytes:frame.residentCacheBytes});}
     results.push({id:ids[i],notes:!!v.deck.pages[i].inspection.speakerNotes,frames});
    }await v.close();return results;
   });
   if(process.env.LAYMESH_REQUIRE_NOTES==='1')assert(pages.every(p=>p.notes));assert.deepEqual(errors,[]);
   results.push({browser:name,version:browser.version(),pages});
   if(name==='chromium'&&screenshotPageId){
    const index=pages.findIndex(p=>p.id===screenshotPageId);assert(index>=0);await p.selectOption('#page',String(index));await p.waitForFunction(()=>document.querySelector('#app').dataset.painted==='true');
    await p.setViewportSize({width:1672,height:941});await p.locator('#preview-more summary').click();const popup=p.context().waitForEvent('page');await p.click('#presenter');const audience=await popup;
    const control=audience;
    await control.waitForFunction(()=>window.laymeshPresenterConsole?.state?.ready);
    await p.waitForFunction(()=>window.laymeshPresentation?.ready);
    // Let all linked steps finish naturally; never seek to the total duration.
    await control.waitForFunction(()=>!laymeshPresenterConsole.state.running,{},{timeout:60000});
    await control.waitForFunction(()=>Number(document.querySelector('.presenter').dataset.frameSequence)>0);
    await control.waitForFunction(()=>[...document.querySelectorAll('.presenter-filmstrip img')].every(i=>i.complete&&i.naturalWidth>0));
    await control.screenshot({path:path.join(out,'presenter.png')});await control.locator('[data-presenter-action=end]').click();
   }
   console.log(name,pages.length,'pages x 5 samples passed');
  }finally{await browser.close();}
 }
 fs.writeFileSync(path.join(out,'pages.json'),JSON.stringify(results,null,2)+'\n');
})().catch(e=>{console.error(e);process.exitCode=1;});
