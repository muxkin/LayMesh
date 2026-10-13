const fs=require('fs'),path=require('path'),assert=require('assert/strict');
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const base=process.argv[2]||'http://100.64.0.6:8786/',out=process.argv[3]||'/tmp/laymesh-site-transitions';fs.mkdirSync(out,{recursive:true});
(async()=>{const browser=await chromium.launch({headless:true});try{
  const page=await browser.newPage({viewport:{width:1440,height:1000}}),errors=[];page.on('pageerror',e=>errors.push(String(e)));page.setDefaultTimeout(60000);
  await page.goto(new URL('examples/animation-transitions-demo.html',base).href);await page.locator('.example-module').scrollIntoViewIfNeeded();
  await page.waitForFunction(()=>document.querySelector('.example-module')?.dataset.painted==='true');
  const results=[];
  for(const [id,effect]of [['curl','InvertedPageCurl'],['cube','cube'],['mask','luma'],['cube','luma']]){
    const old=await page.locator('.example-module').getAttribute('data-generation');await page.locator('.document-pages').selectOption(id);
    await page.waitForFunction(old=>{const m=document.querySelector('.example-module');if(m.dataset.liveState==='error')throw Error(m.querySelector('.live-diagnostics').textContent);return m.dataset.generation!==old&&m.dataset.painted==='true';},old);
    const row=await page.locator('.example-module').evaluate(m=>({...m.dataset}));assert.equal(row.transitionName,effect,JSON.stringify(row));assert.ok(Number(row.transitionFrames)>1);assert.equal(row.transitionError,'');results.push(row);
  }
  const before=results.at(-1).generation;await page.locator('.live-run').click();await page.waitForFunction(before=>{const m=document.querySelector('.example-module');return m.dataset.generation!==before&&m.dataset.painted==='true';},before);
  assert.equal(await page.locator('.example-module').getAttribute('data-transition-name'),'');assert.deepEqual(errors,[]);
  await page.screenshot({path:path.join(out,'preview.png')});fs.writeFileSync(path.join(out,'site.json'),JSON.stringify({results,errors},null,2));console.log('Website preview transitions, reverse texture and same-page refresh passed.');
}finally{await browser.close();}})().catch(e=>{console.error(e);process.exitCode=1;});
