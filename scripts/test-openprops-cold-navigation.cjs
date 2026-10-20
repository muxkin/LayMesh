// Reproduce a user targeting P16 while its background preparation is already
// submitted. This is a separate input test, not the natural-playback benchmark.
// Use the normal numeric navigation shortcut; never seek or alter the artwork.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto'),{pathToFileURL}=require('node:url');
const pw=require(process.env.PLAYWRIGHT_MODULE||'playwright'),[file,out]=process.argv.slice(2);
const width=Number(process.env.WIDTH||1920),height=Number(process.env.HEIGHT||920),samples=Number(process.env.SAMPLES||20),engine=process.env.ENGINE||'chromium';
const args=engine==='chromium'&&process.env.HARDWARE_GPU==='1'?['--enable-gpu','--use-gl=angle','--use-angle=vulkan','--enable-features=Vulkan']:[];
fs.mkdirSync(out,{recursive:true});
(async()=>{const rows=[];let version;
 for(let iteration=0;iteration<samples;iteration++){
  const browser=await pw[engine].launch({headless:true,args});version=browser.version();let page,row={iteration};
  try{
   row.phase='bootstrap';page=await browser.newPage({viewport:{width,height}});page.setDefaultTimeout(60000);await page.route(/^https?:\/\//,r=>r.abort());await page.goto(pathToFileURL(path.resolve(file)).href);
   await page.waitForFunction(()=>window.__LAYMESH_WEB?.presentation&&document.querySelector('#app').dataset.painted==='true');
   await page.evaluate(()=>{
    const v=__LAYMESH_WEB.presentation,m=window.coldNavigation={sent:null,completed:null},call=v.call.bind(v);
    v.call=async(d,...a)=>{const value=await call(d,...a);if(d.type==='init'){
     const pool=v.worker,request=pool.request.bind(pool);
     pool.request=(r,data,...a)=>{const result=request(r,data,...a);if(!data.priority&&['load','prepare'].includes(data.type)&&(data.page?.id||data.pageId)==='P16'&&m.sent===null){m.sent=performance.now();m.renderer=pool.renderers.indexOf(r);result.then(value=>{m.completed=performance.now();m.preparation=value.preparation;},error=>{m.backgroundError=error.message;});}return result;};
    }return value;};
    v.open({fullscreen:false,startPageId:'P15'});
   });
   row.phase='background';await page.waitForFunction(()=>__LAYMESH_WEB.presentation.ready&&!__LAYMESH_WEB.presentation.switching&&coldNavigation.sent!==null);
   // Give the submitted background command a repeatable opportunity to begin.
   // The optimized path is allowed to finish useful work during this interval.
   row.phase='navigation';await page.waitForTimeout(150);await page.keyboard.type('16');await page.keyboard.press('Enter');
   await page.waitForFunction(()=>{const v=__LAYMESH_WEB.presentation;return !v.switching&&(v.index===15||v.retry?.hidden===false);},null,{timeout:45000});
   Object.assign(row,await page.evaluate(()=>{const v=__LAYMESH_WEB.presentation,t=v.metrics.transitions.at(-1);return {...coldNavigation,status:v.index===15?'visible':'failed',error:v.index===15?null:v.message.textContent,firstVisibleMs:t?.page==='P16'?t.firstVisibleMs:null,completedMs:t?.page==='P16'?t.completedMs:null,backend:v.metrics.lastFrame?.backend,device:v.metrics.lastFrame?.device};}));
  }catch(error){row.status='failed';row.error=String(error);if(page)await page.screenshot({path:path.join(out,`failed-${iteration}.png`)}).catch(()=>{});}
  finally{await browser.close();}
  rows.push(row);fs.writeFileSync(path.join(out,'progress.json'),JSON.stringify(rows,null,2));console.log(iteration,row.status,row.firstVisibleMs??row.error);
 }
 const visible=rows.filter(r=>r.status==='visible'&&Number.isFinite(r.firstVisibleMs)),sorted=visible.map(r=>r.firstVisibleMs).sort((a,b)=>a-b);
 const result={engine,version,width,height,samples,args,sha256:crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex'),status:visible.length===samples?'passed':'failed',visible:visible.length,firstVisibleP95Ms:visible.length===samples?sorted[Math.ceil(sorted.length*.95)-1]:null,rows,definition:'Fresh browser/document per sample. Start at P15, observe actual background P16 request submission, wait 150 ms, then use the trusted 16 + Enter shortcut. This input stress test intentionally navigates early; full authored playback is measured separately. Failures are censored, never treated as zero latency.'};
 fs.writeFileSync(path.join(out,'result.json'),JSON.stringify(result,null,2));
 if(result.status!=='passed'&&process.env.ALLOW_FAILURE!=='1')process.exitCode=1;
})().catch(error=>{console.error(error);process.exitCode=1;});
