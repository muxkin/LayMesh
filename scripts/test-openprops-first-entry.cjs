// Cold first-page visibility and full readiness are separate measurements.
// No waiting for other pages, seeking, preflight draw or authored-time changes.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto'),{pathToFileURL}=require('node:url');
const pw=require(process.env.PLAYWRIGHT_MODULE||'playwright'),[file,out]=process.argv.slice(2),engine=process.env.ENGINE||'chromium',width=Number(process.env.WIDTH||1920),height=Number(process.env.HEIGHT||920),samples=Number(process.env.SAMPLES||10);
const args=engine==='chromium'&&process.env.HARDWARE_GPU==='1'?['--enable-gpu','--use-gl=angle','--use-angle=vulkan','--enable-features=Vulkan']:[];
(async()=>{const rows=[];fs.mkdirSync(out,{recursive:true});let version;
 for(let iteration=0;iteration<samples;iteration++){
  const browser=await pw[engine].launch({headless:true,args});version=browser.version();const row={iteration};
  try{
   const page=await browser.newPage({viewport:{width,height}});page.setDefaultTimeout(60000);await page.route(/^https?:\/\//,r=>r.abort());await page.goto(pathToFileURL(path.resolve(file)).href);
   await page.waitForFunction(()=>window.__LAYMESH_WEB?.presentation&&document.querySelector('#app').dataset.painted==='true');
   Object.assign(row,await page.evaluate(async()=>{
    const v=__LAYMESH_WEB.presentation,notifications=[],presentations=[],calls=[],visible=v.navigationVisible.bind(v),call=v.call.bind(v);v.navigationVisible=d=>{const n={page:d.pageId,initialEntry:!!d.initialEntry,at:performance.now()};notifications.push(n);presentations.push(new Promise(resolve=>requestAnimationFrame(()=>setTimeout(()=>{n.presentedAt=performance.now();resolve();},0))));return visible(d);};v.call=async(d,...a)=>{const at=performance.now(),r=await call(d,...a);calls.push({type:d.type,ms:performance.now()-at,initialEntryRasterMs:r.initialEntryRasterMs,preparation:r.preparation});return r;};
    const at=performance.now();await v.open({fullscreen:false,startPageId:'P01'});if(!v.ready||v.switching)throw Error(v.message?.textContent||'First page not ready');
    const openMs=performance.now()-at,frame=v.metrics.lastFrame;await Promise.all(presentations);const first=notifications.find(n=>n.page==='P01'&&Number.isFinite(n.presentedAt));if(!first)throw Error('No observed entry presentation');return {status:'passed',openMs,observedFirstVisibleMs:first.presentedAt-at,transition:v.metrics.transitions[0],backend:frame.backend,device:frame.device,softwareGpuBypass:frame.softwareGpuBypass,notifications,calls};
   }));
  }catch(error){row.status='failed';row.error=String(error);}finally{await browser.close();}
  rows.push(row);fs.writeFileSync(path.join(out,'progress.json'),JSON.stringify(rows,null,2));console.log(iteration,row.status,row.observedFirstVisibleMs,row.transition?.completedMs);
 }
 const p95=values=>values.sort((a,b)=>a-b)[Math.ceil(values.length*.95)-1],passed=rows.filter(r=>r.status==='passed'),result={engine,version,width,height,samples,args,sha256:crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex'),status:passed.length===samples?'passed':'failed',firstVisibleP95Ms:passed.length===samples?p95(rows.map(r=>r.observedFirstVisibleMs)):null,completeP95Ms:passed.length===samples?p95(rows.map(r=>r.transition.completedMs)):null,visibilityDefinition:'Navigation-visible followed by an animation-frame/post-task checkpoint. Independent of the mutable transition summary; not physical scanout.',rows};
 fs.writeFileSync(path.join(out,'result.json'),JSON.stringify(result,null,2));if(result.status!=='passed')process.exitCode=1;
})().catch(e=>{console.error(e);process.exitCode=1;});
