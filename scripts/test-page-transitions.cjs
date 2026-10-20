// Exercise real offline packages and the shared presentation runtime in 3 engines.
const fs=require('fs'),path=require('path'),cp=require('child_process'),assert=require('assert/strict');
const engines=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const root=path.resolve(__dirname,'..'),out=path.resolve(process.argv[2]||'/tmp/laymesh-page-navigation'),binary=process.env.LAYMESH_TEST_BINARY||root+'/target/release/laymesh';fs.mkdirSync(out,{recursive:true});
(async()=>{
  const fixture=out+'/source';fs.cpSync(root+'/examples/animation/transitions',fixture,{recursive:true});
  fs.writeFileSync(fixture+'/styles/transitions.lcss',fs.readFileSync(fixture+'/styles/transitions.lcss','utf8').replace('700ms','180ms'));
  fs.writeFileSync(fixture+'/demo.laydoc',fs.readFileSync(fixture+'/demo.laydoc','utf8').replace('700ms','180ms'));
  cp.execFileSync(binary,['render',fixture+'/demo.laydoc','-o',out+'/demo.html','--warnings','hide']);
  const {webUnzip,webPayload}=await import(root+'/site/live/web-archive.mjs'),files=webUnzip(webPayload(fs.readFileSync(out+'/demo.html')));
  assert.ok(files['runtime/presentation-transitions-data.mjs']);assert.ok(files['licenses/manifest.json']);
  const catalog=JSON.parse(new TextDecoder().decode(files['runtime/presentation-transitions-data.mjs']).split('export const transitions = ')[1].trim().replace(/;$/,''));
  assert.deepEqual(Object.keys(catalog).sort(),['InvertedPageCurl','cube','luma']);
  const results=[];
  for(const engine of (process.env.LAYMESH_TEST_ENGINES||'chromium,firefox,webkit').split(',')){
    const browser=await engines[engine].launch({headless:true});try{
      const page=await browser.newPage({viewport:{width:1280,height:800}}),errors=[],requests=[];
      page.on('pageerror',e=>errors.push(String(e)));page.on('request',r=>{if(/^https?:/.test(r.url()))requests.push(r.url());});page.setDefaultTimeout(60000);
      await page.addInitScript(()=>{const match=window.matchMedia.bind(window);window.__motionQueries=[];window.matchMedia=query=>{const result=match(query);if(query==='(prefers-reduced-motion: reduce)')__motionQueries.push(result);return result;};});
      await page.goto('file://'+out+'/demo.html');await page.waitForFunction(()=>window.__LAYMESH_WEB&&document.querySelector('#app').dataset.painted==='true');
      assert.ok(!await page.evaluate(()=>__LAYMESH_WEB.metrics.transitionName),'First display is static');
      const select=async(n)=>{await page.locator('#page').selectOption(String(n));await page.waitForFunction(n=>document.querySelector('#app').dataset.painted==='true'&&__LAYMESH_WEB.metrics.pageId===__LAYMESH_WEB.pages[n].id,n);return page.evaluate(()=>({...__LAYMESH_WEB.metrics}));};
      let m=await select(1);assert.equal(m.transitionName,'InvertedPageCurl');assert.ok(!m.transitionError,m.transitionError);assert.ok(m.transitionFrames>1,JSON.stringify(m));assert.ok(!m.transitionError,m.transitionError);
      m=await select(2);assert.equal(m.transitionName,'cube');m=await select(3);assert.equal(m.transitionName,'luma');assert.ok(!m.transitionError,m.transitionError);
      m=await select(2);assert.equal(m.transitionName,'luma','Backward uses the departing boundary');assert.ok(!m.transitionError,m.transitionError);
      await page.evaluate(()=>{for(const n of [1,3,0,2]){const e=document.querySelector('#page');e.value=n;e.dispatchEvent(new Event('change'));}});
      await page.waitForFunction(()=>document.querySelector('#app').dataset.painted==='true'&&document.querySelector('#page').value==='2');
      const warm=(await select(3)).skiaMemoryBytes;
      for(let n=0;n<12;n++)await select(n%2?3:2);
      m=await select(2);assert.ok(m.skiaMemoryBytes<=warm+32*1048576,'Repeated navigation keeps memory bounded');
      // WebKit can update a new query before the player's existing query lists.
      // Observe the actual lists returned to the runtime, without replacing
      // their state or dispatching synthetic changes, before testing navigation.
      const motion=async value=>{await page.emulateMedia({reducedMotion:value?'reduce':'no-preference'});await page.waitForFunction(value=>__motionQueries.length>0&&__motionQueries.every(query=>query.matches===value),value);};
      await motion(true);m=await select(1);assert.ok(!m.transitionFrames,'Reduced motion renders no intermediate effect frames '+JSON.stringify(m));await motion(false);
      console.log(engine+' preview checks passed');
      await page.evaluate(async()=>{const p=__LAYMESH_WEB.presentation;p.fullscreen=false;await p.open();});await page.waitForFunction(()=>__LAYMESH_WEB.presentation.ready);
      assert.equal(await page.evaluate(()=>__LAYMESH_WEB.presentation.index),1,'Presentation starts at the current preview page');
      assert.ok(!await page.evaluate(()=>__LAYMESH_WEB.presentation.metrics.lastFrame.transitionName),'Initial current page has no transition');
      await page.evaluate(async()=>{await __LAYMESH_WEB.presentation.go(0);});
      assert.equal(await page.evaluate(()=>__LAYMESH_WEB.presentation.metrics.lastFrame.transitionName),'InvertedPageCurl','Backward crosses the current page boundary');
      await page.evaluate(async()=>{await __LAYMESH_WEB.presentation.go(1);});
      const presentation=await page.evaluate(()=>structuredClone(__LAYMESH_WEB.presentation.metrics.lastFrame));assert.equal(presentation.transitionName,'InvertedPageCurl');assert.ok(presentation.transitionFrames>1,JSON.stringify(presentation));assert.ok(!presentation.transitionError);
      await page.evaluate(async()=>{const p=__LAYMESH_WEB.presentation;await Promise.all([p.go(2),p.go(3),p.go(0)]);});assert.equal(await page.evaluate(()=>__LAYMESH_WEB.presentation.index),0);
      await page.evaluate(()=>{const p=__LAYMESH_WEB.presentation;p.go(1);});await page.waitForTimeout(40);await page.setViewportSize({width:1100,height:760});
      await page.waitForFunction(()=>!__LAYMESH_WEB.presentation.switching);await page.screenshot({path:out+'/'+engine+'-presentation.png'});
      await page.evaluate(async()=>{const p=__LAYMESH_WEB.presentation;p.go(2);await p.close();});assert.equal(await page.locator('.presentation').count(),0);
      console.log(engine+' window/exit checks passed');
      await page.evaluate(async()=>{const p=__LAYMESH_WEB.presentation;p.fullscreen=true;await p.open();});await page.waitForFunction(()=>__LAYMESH_WEB.presentation.ready);
      await page.evaluate(async()=>{await __LAYMESH_WEB.presentation.go(2);});const fullscreen=await page.evaluate(()=>!!document.fullscreenElement);await page.evaluate(()=>__LAYMESH_WEB.presentation.close());
      assert.equal(await page.evaluate(()=>__LAYMESH_WEB.metrics.compiles),0);assert.deepEqual(errors,[]);assert.deepEqual(requests,[]);
      results.push({engine,backend:m.backend,transitionFrames:presentation.transitionFrames,transitionMs:presentation.transitionMs,transitionRasterSize:presentation.transitionRasterSize,fullscreen,skiaMemoryBytes:m.skiaMemoryBytes,checks:['first display','forward','reverse texture','jump','rapid navigation','bounded memory','reduced motion','window presentation','resize cancellation','exit cancellation','fullscreen','offline/no requests','zero playback compiles']});fs.writeFileSync(out+'/navigation.json',JSON.stringify(results,null,2));console.log(JSON.stringify(results.at(-1)));
    }finally{await browser.close();}
  }
  fs.writeFileSync(out+'/navigation.json',JSON.stringify(results,null,2));
})().catch(error=>{console.error(error);process.exitCode=1;});
