// Trusted interaction while preparation is active, and real offline edit/save/reopen.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),{pathToFileURL}=require('node:url');
const memory=require('./browser-process-memory.cjs');
const pw=require(process.env.PLAYWRIGHT_MODULE||'playwright'),[file,out]=process.argv.slice(2);fs.mkdirSync(out,{recursive:true});
(async()=>{for(const engine of (process.env.ENGINES||'chromium,firefox,webkit').split(',')){
 const server=await pw[engine].launchServer({headless:true}),browser=await pw[engine].connect(server.wsEndpoint());try{
 const context=await browser.newContext({viewport:{width:1920,height:920},acceptDownloads:true});await context.route(/^https?:\/\//,r=>r.abort());await context.addInitScript(()=>window.showSaveFilePicker=undefined);
 const page=await context.newPage(),errors=[],samples=[];page.setDefaultTimeout(120000);page.on('pageerror',e=>errors.push(e.message));await page.goto(pathToFileURL(path.resolve(file)).href);
 await page.evaluate(()=>{window.inputTrace=[];document.addEventListener('keydown',e=>inputTrace.push({key:e.key,target:e.target.tagName,focused:document.activeElement?.className}),true);});
 const state=async(label,fn)=>{try{await page.waitForFunction(fn,null,{timeout:20000});}catch(error){const detail=await page.evaluate(()=>{const p=__LAYMESH_WEB.presentation;return {index:p.index,ready:p.ready,switching:p.switching,painting:p.painting,message:p.message?.textContent,backgroundError:p.backgroundError,lastFrame:p.metrics?.lastFrame,resources:p.metrics?.resources,renderers:p.worker?.renderers.map(r=>({resident:r.resident,pending:r.pending.size})),digits:p.digits,clock:{time:p.clock.time,completed:p.clock.completed,active:p.clock.active},focus:document.activeElement?.outerHTML.slice(0,500),inputTrace};});fs.writeFileSync(path.join(out,engine+'-failure.json'),JSON.stringify({label,detail},null,2));await page.screenshot({path:path.join(out,engine+'-failure.png')});throw error;}};
 const ready=()=>page.waitForFunction(()=>window.__LAYMESH_WEB?.editor&&document.querySelector('#app').dataset.painted==='true'&&document.querySelector('#app').dataset.painting!=='true');await ready();
 for(let i=0;i<Number(process.env.CYCLES||5);i++){
  await page.evaluate(()=>__LAYMESH_WEB.presentation.open({fullscreen:false,startPageId:'P15'}));await page.waitForFunction(()=>__LAYMESH_WEB.presentation.ready&&!__LAYMESH_WEB.presentation.switching);
  await page.keyboard.press('ArrowRight');await page.keyboard.press('ArrowDown');
  // Repeated arrows during transition retain the existing protection. They
  // must not prevent menu/exit, including while the next page is still cold.
  await page.keyboard.press('ArrowLeft');await page.keyboard.press('ArrowUp');
  if(process.env.SKIP_NAV_CHECK!=='1'&&i===Number(process.env.CYCLES||5)-1){
   await page.waitForFunction(()=>!__LAYMESH_WEB.presentation.switching&&!__LAYMESH_WEB.presentation.painting);
   await page.keyboard.type('16');await page.keyboard.press('Enter');await state('numeric target',()=>__LAYMESH_WEB.presentation.index===15&&!__LAYMESH_WEB.presentation.switching);
   await page.keyboard.press('ArrowLeft');await state('backward key',()=>__LAYMESH_WEB.presentation.index===14&&!__LAYMESH_WEB.presentation.switching);
   await page.mouse.click(1400,400);await state('forward left click',()=>__LAYMESH_WEB.presentation.index===15&&!__LAYMESH_WEB.presentation.switching);
  }
  let start=Date.now();await page.mouse.click(1400,400,{button:'right'});await page.waitForFunction(()=>[...document.querySelectorAll('.context-menu-list,.presentation-menu')].some(n=>n.getClientRects().length&&!n.hidden&&getComputedStyle(n).visibility!=='hidden'));
  const menuMs=Date.now()-start;await page.keyboard.press('Escape');start=Date.now();await page.keyboard.press('Escape');await page.waitForFunction(()=>!__LAYMESH_WEB.presentation.opened&&!document.querySelector('.presentation'));
  const exitMs=Date.now()-start;await page.waitForTimeout(Number(process.env.SETTLE_MS||1000));
  const runtime=await page.evaluate(()=>{const p=__LAYMESH_WEB.presentation,d=__LAYMESH_WEB.diagnostics();return {renderers:p.parkedWorker?.renderers.length||0,resident:p.parkedWorker?.state().resident||[],sourceReady:d.pages.filter(p=>p.ready).length,compilerMemoryBytes:d.compilerMemoryBytes,previewFrames:__LAYMESH_WEB.metrics.framesCommitted,previewRestores:__LAYMESH_WEB.metrics.previewRestores||0,backend:p.metrics.lastFrame?.backend,resources:p.metrics.lastFrame?.resourceStats};});
  samples.push({iteration:i,menuMs,exitMs,afterClose:memory(server.process().pid),runtime});fs.writeFileSync(path.join(out,engine+'-progress.json'),JSON.stringify({engine,samples},null,2));console.log(engine,'cycle',i,menuMs,exitMs,samples.at(-1).afterClose.pss);
 }
 // Editing the existing source through CodeMirror dispatch exercises the same
 // change listener as typing, rather than changing an exported bitmap.
 const edit=await page.evaluate(async()=>{const editor=__LAYMESH_WEB.editor,entry='/project/pages/P01.lay';await editor.openFile(entry);const view=editor.views.get(entry)?.view;if(!view)throw Error('Page source editor unavailable');const title='LayMesh 教程',marker=title+' - edited',at=view.state.doc.toString().indexOf(title);if(at<0)throw Error('Source title missing');view.dispatch({changes:{from:at,to:at+title.length,insert:marker}});return {entry,marker};});
 await page.waitForFunction(()=>__LAYMESH_WEB.metrics.compiles>0);await page.waitForFunction(()=>{const d=document.querySelector('#app').dataset;return d.documentState?d.documentState==='ready':d.compileState==='ready';});await ready();
 await page.locator('#export').click();const download=page.waitForEvent('download');await page.locator('#save').click();const saved=path.join(out,engine+'-saved.zip.html');await(await download).saveAs(saved);
 await page.goto(pathToFileURL(path.resolve(saved)).href);await ready();assert.equal(await page.evaluate(()=>__LAYMESH_WEB.pages.length),26);assert(await page.evaluate(({entry,marker})=>__LAYMESH_WEB.editor.files[entry].includes(marker),edit));
 await page.evaluate(()=>__LAYMESH_WEB.presentation.open({fullscreen:false,startPageId:'P15'}));await page.waitForFunction(()=>__LAYMESH_WEB.presentation.ready&&!__LAYMESH_WEB.presentation.switching);await page.keyboard.press('Escape');await page.waitForFunction(()=>!__LAYMESH_WEB.presentation.opened);
 assert.deepEqual(errors,[]);fs.writeFileSync(path.join(out,engine+'.json'),JSON.stringify({status:'passed',memoryProtocol:memory.protocol,engine,version:browser.version(),samples,offlineEditSaved:true,pages:26,errors},null,2));console.log(engine,'cold input, exit/reopen and offline source roundtrip passed');
 }finally{await browser.close();await server.close();}
}})().catch(e=>{console.error(e);process.exitCode=1;});
