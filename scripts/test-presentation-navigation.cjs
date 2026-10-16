const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),crypto=require('node:crypto'),{pathToFileURL}=require('node:url');
const pw=require(process.env.PLAYWRIGHT_MODULE||'playwright'),benchmark=require('./presentation-navigation-benchmark.cjs');
const [before,after,out]=process.argv.slice(2);assert(out,'Usage: BEFORE_HTML AFTER_HTML OUTPUT');
fs.mkdirSync(out,{recursive:true});
(async()=>{for(const engine of (process.env.ENGINES||'chromium,firefox,webkit').split(',')){
 let reference;
 for(const [kind,file]of [['before',before],['after',after]]){
  const existing=path.join(out,engine+'-'+kind+'.json');
  if(kind==='before'&&process.env.REUSE_BASELINE==='1'&&fs.existsSync(existing)){
   const saved=JSON.parse(fs.readFileSync(existing));assert.equal(saved.htmlSha256,crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex'));
   assert.equal(saved.summary.samples,Number(process.env.SAMPLES||50));reference=saved.animation;console.log(engine,'preserved baseline',JSON.stringify(saved.summary));continue;
  }
  const browser=await pw[engine].launch({headless:true});
  try{
   const p=await browser.newPage({viewport:{width:1600,height:1000}}),errors=[],requests=[];
   p.on('pageerror',e=>errors.push(e.message));p.setDefaultTimeout(120000);
   p.on('console',m=>{if(m.text().startsWith('LayMesh navigation samples:'))console.log(engine,kind,m.text());});
   await p.route(/^https?:\/\//,r=>{requests.push(r.request().url());return r.abort();});
   await p.goto(pathToFileURL(path.resolve(file)).href);
   await p.waitForFunction(()=>window.__LAYMESH_WEB?.presentation&&document.querySelector('#app').dataset.painted==='true');
   const result=await p.evaluate('('+benchmark.toString()+')(__LAYMESH_WEB.presentation,'+JSON.stringify({samples:Number(process.env.SAMPLES||50),visual:process.env.VISUAL!=='0'})+')');
   assert.deepEqual(errors,[]);assert.deepEqual(requests,[]);
   result.engine=engine;result.version=browser.version();result.kind=kind;result.errors=errors;result.networkRequests=requests;
   result.htmlSha256=crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
   fs.writeFileSync(path.join(out,engine+'-'+kind+'.json'),JSON.stringify(result,null,2));
   await p.screenshot({path:path.join(out,engine+'-'+kind+'.png')});
   if(kind==='before')reference=result.animation;else assert.deepEqual(result.animation,reference,'Animation pixels must match');
   // Exercise genuine browser keyboard events as well as the timed DOM path.
   await p.keyboard.press('Home');await p.waitForFunction(()=>__LAYMESH_WEB.presentation.index===0&&!__LAYMESH_WEB.presentation.switching);
   for(let k=0;k<12;k++){await p.keyboard.press(k%2?'ArrowRight':'ArrowDown');await p.waitForTimeout(60);}
   await p.waitForFunction(()=>!__LAYMESH_WEB.presentation.switching);
   assert.deepEqual(errors,[]);console.log(engine,kind,JSON.stringify(result.summary));
  }finally{await browser.close();}
 }
}})().catch(e=>{console.error(e);process.exitCode=1;});
