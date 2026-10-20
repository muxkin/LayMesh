// Exercise the shipped documentation catalog, including its real compiler.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const [base,out]=process.argv.slice(2);
(async()=>{
 const browser=await chromium.launch({headless:true});const results=[];
 try{
  for(const route of ['docs/topics/art-effects.zh-CN.html','en/docs/topics/art-effects.html']){
   const page=await browser.newPage({viewport:{width:1440,height:1000}}),errors=[];
   page.on('pageerror',error=>errors.push(String(error)));
   await page.goto(new URL(route,base).href);
   const catalog=page.locator('.style-library-catalog'),module=catalog.locator('.example-module');
   await catalog.scrollIntoViewIfNeeded();
   const ready=previous=>module.evaluate((m,previous)=>new Promise((resolve,reject)=>{
    const start=performance.now(),check=()=>{
     if(m.dataset.liveState==='error')return reject(Error(m.querySelector('.live-diagnostics').textContent));
     if(m.dataset.painted==='true'&&m.dataset.liveState==='success'&&m.dataset.previewState==='success'&&(!previous||m.dataset.generation!==previous))return resolve();
     if(performance.now()-start>90000)return reject(Error('Effects catalog render timed out'));
     setTimeout(check,50);
    };check();
   }),previous);
   await ready();
   assert.equal(await catalog.locator('.library-items option').count(),30);
   let generation=await module.getAttribute('data-generation');
   await catalog.locator('.library-items').selectOption('effects:class:fx-material-plastic');
   await ready(generation);
   assert((await module.locator('.cm-content').innerText()).includes('fx-material-plastic'));
   await catalog.locator('.library-tuning summary').click();
   await catalog.locator('.library-property').selectOption('three-d');
   const config='three_d(depth=6mm,rotation=(20deg,-35deg,0deg),material="metal")';
   await catalog.locator('.library-value').fill(config);
   generation=await module.getAttribute('data-generation');
   await catalog.locator('.library-apply').click();await ready(generation);
   assert((await module.locator('.cm-content').innerText()).includes(config));
   assert.deepEqual(errors,[]);
   fs.mkdirSync(out,{recursive:true});await catalog.screenshot({path:path.join(out,route.startsWith('en/')?'en.png':'zh-CN.png')});
   results.push({route,presets:30,selection:true,threeDOverride:true,errors});await page.close();
  }
  fs.writeFileSync(path.join(out,'result.json'),JSON.stringify({status:'passed',results},null,2));
 }finally{await browser.close();}
})().catch(error=>{console.error(error);process.exitCode=1;});
