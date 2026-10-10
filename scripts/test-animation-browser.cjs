// Requires a built animation_reference example and web-target WASM bindings.
// PLAYWRIGHT_MODULE optionally selects an existing Playwright installation.
const fs=require('fs'),path=require('path'),http=require('http'),os=require('os'),cp=require('child_process');
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const root=path.resolve(__dirname,'..'),out=process.argv[2]||fs.mkdtempSync(path.join(os.tmpdir(),'laymesh-animation-'));
fs.mkdirSync(out,{recursive:true});
const reference=cp.execFileSync(path.join(root,'target/release/examples/animation_reference'),{maxBuffer:32*1024*1024});fs.writeFileSync(path.join(out,'native.json'),reference);
const host=process.env.LAYMESH_TEST_HOST||'127.0.0.1';
const routes={'/':path.join(root,'tests/animation/reference.html.in'),'/native.json':path.join(out,'native.json'),'/animation-player.mjs':path.join(root,'site/live/animation-player.mjs')};
const server=http.createServer((req,res)=>{const url=new URL(req.url,'http://localhost').pathname;const file=routes[url]||(url.startsWith('/wasm/')&&!url.includes('..')?path.join(root,'site/dist/site/live',url.slice(1)):null);if(!file||!fs.existsSync(file)){res.writeHead(404).end();return;}res.setHeader('Content-Type',file.endsWith('.wasm')?'application/wasm':/\.m?js$/.test(file)?'text/javascript':file.endsWith('.json')?'application/json':'text/html');fs.createReadStream(file).pipe(res);});
(async()=>{await new Promise(r=>server.listen(0,host,r));const browser=await chromium.launch({headless:true,...(process.env.CHROMIUM_PATH?{executablePath:process.env.CHROMIUM_PATH}:{}),args:['--no-sandbox']});try{
const page=await browser.newPage({viewport:{width:1280,height:800}});const errors=[];page.on('pageerror',e=>{errors.push(String(e));console.error('PAGE ERROR',String(e));});await page.goto(`http://${host}:${server.address().port}/`);await page.waitForFunction(()=>window.animationReady,{},{timeout:120000});
const result=await page.evaluate(()=>runConformance());result.consoleErrors=errors;fs.writeFileSync(path.join(out,'browser.json'),JSON.stringify(result,null,2));
await page.getByRole('button',{name:'Play',exact:true}).click();await page.getByRole('button',{name:'Pause',exact:true}).click();
result.representativePixelMeanError={};
for(const name of ['headShake','flip','flipInX','flipInY','flipOutX','flipOutY']){
 await page.getByLabel('Animation effect').selectOption(name);await page.getByLabel('Animation time',{exact:true}).fill('0.4');
 const css=await page.locator('#css-stage').screenshot({path:path.join(out,name+'-css.png')});
 const wasm=await page.locator('#frame').screenshot({path:path.join(out,name+'-wasm.png')});
 const error=await page.evaluate(async images=>{
  const pixels=await Promise.all(images.map(async base64=>{const blob=await(await fetch('data:image/png;base64,'+base64)).blob(),bitmap=await createImageBitmap(blob),c=document.createElement('canvas');c.width=bitmap.width;c.height=bitmap.height;const ctx=c.getContext('2d');ctx.drawImage(bitmap,0,0);bitmap.close();return ctx.getImageData(0,0,c.width,c.height).data;}));
  if(pixels[0].length!==pixels[1].length)throw new Error('Representative frame dimensions differ');
  let sum=0;for(let i=0;i<pixels[0].length;i++)if(i%4!==3)sum+=Math.abs(pixels[0][i]-pixels[1][i]);return sum/(pixels[0].length/4*3);
 },[css.toString('base64'),wasm.toString('base64')]);
 result.representativePixelMeanError[name]=error;
 if(error>3)result.failures.push(`${name}@0.4 representative RGB mean error ${error} exceeds 3/255`);
 if(name==='flipInY')await page.screenshot({path:path.join(out,'flipInY.png')});
}
await page.getByRole('button',{name:'Static',exact:true}).click();result.staticRestored=(await page.getByLabel('Current step').textContent())==='Static';result.performance=await page.evaluate(()=>benchmark());
await page.setViewportSize({width:640,height:700});await page.screenshot({path:path.join(out,'controls-640.png')});
fs.writeFileSync(path.join(out,'browser.json'),JSON.stringify(result,null,2));console.log(JSON.stringify(result));if(result.failures.length||errors.length||!result.staticRestored)process.exitCode=1;
}finally{await browser.close();server.close();}})().catch(e=>{console.error(e);server.close();process.exitCode=1;});
