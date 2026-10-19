// Verify the public gallery, including its explanation and inspection controls.
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const assert=require('node:assert/strict');
const url=process.argv[2]||'http://127.0.0.1:8769/gradient-gallery.html';
(async()=>{const browser=await chromium.launch({headless:true});try{
 const page=await browser.newPage({viewport:{width:1200,height:900}}),errors=[];
 page.on('pageerror',e=>errors.push(e.message));await page.goto(url);
 // The stable Pages entry redirects to the versioned evidence directory.
 await page.waitForFunction(()=>document.querySelectorAll('article').length===119);
 assert.equal(await page.locator('article').count(),119);
 const evidence=await page.request.get(new URL('verification.json',page.url()).href);
 assert.equal(evidence.status(),200);assert.equal((await evidence.json()).cases.length,119);
 await page.locator('#filter').selectOption('dark');assert.equal(await page.locator('article:visible').count(),4);
 assert((await page.locator('#open-default-16').innerText()).includes('400px'));
 await page.locator('#filter').selectOption('exception');assert.equal(await page.locator('article:visible').count(),8);
 await page.locator('#search').fill('calc-size');assert.equal(await page.locator('article:visible').count(),1);
 assert((await page.locator('#calc-size').innerText()).includes('CSS.supports=false'));
 assert.equal(await page.locator('#calc-size figure').count(),7);
 await page.locator('#calc-size .image').last().click();assert(await page.locator('#zoom').isVisible());
 await page.keyboard.press('Escape');assert(!(await page.locator('#zoom').isVisible()));
 await page.locator('#search').fill('');await page.locator('#filter').selectOption('all');
 await page.setViewportSize({width:390,height:844});
 assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
 assert.deepEqual(errors,[]);console.log('Gradient gallery: 119 cases, 4 dark recipes, 8 explicit reference exceptions, equivalent-size reference, zoom, mobile layout; no page errors.');
}finally{await browser.close();}})().catch(e=>{console.error(e);process.exitCode=1});
