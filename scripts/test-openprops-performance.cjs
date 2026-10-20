// Natural first/repeat traversal, resource accounting and trusted input checks.
// No seeking, duration changes, preflight rendering or all-page readiness wait.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),crypto=require('node:crypto'),{pathToFileURL}=require('node:url');
const pw=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const [file,out]=process.argv.slice(2);assert(file&&out);const sourceSha256=crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');fs.mkdirSync(out,{recursive:true});
const width=Number(process.env.WIDTH||1920),height=Number(process.env.HEIGHT||920),rounds=Number(process.env.ROUNDS||2);
const first=Number(process.env.FIRST||1)-1,count=Number(process.env.PAGES||26);
const browserArgs=process.env.HARDWARE_GPU==='1'?['--enable-gpu','--use-gl=angle','--use-angle=vulkan','--enable-features=Vulkan']:[];
const memory=require('./browser-process-memory.cjs');
(async()=>{for(const engine of (process.env.ENGINES||'chromium,firefox,webkit').split(',')){
 const server=await pw[engine].launchServer({headless:true,args:engine==='chromium'?browserArgs:[]}),browser=await pw[engine].connect(server.wsEndpoint()),samples=[],errors=[],network=[],checks=[],records=[],visits=[];let timer;
 const result={memoryProtocol:memory.protocol,forcedCPU:process.env.FORCE_CPU==='1',streamedDiagnostics:true,streamedVisitDiagnostics:true,phaseDiagnostics:process.env.PHASE_DIAGNOSTICS==='1',memorySampleMs:2000,engine,version:browser.version(),browserArgs:engine==='chromium'?browserArgs:[],width,height,rounds,first:first+1,pages:count,sha256:sourceSha256,started:new Date().toISOString()};
 result.browserDiagnostics={stderr:'',exit:null,crashes:[]};server.process().stderr?.on('data',chunk=>{result.browserDiagnostics.stderr+=String(chunk);});server.process().on('exit',(code,signal)=>{result.browserDiagnostics.exit={code,signal};});
 let page;try{
  timer=setInterval(()=>samples.push(memory(server.process().pid)),2000);
  page=await browser.newPage({viewport:{width,height}});if(process.env.FORCE_CPU==='1')await page.addInitScript(()=>{const Base=Worker;window.Worker=class extends Base{postMessage(data,transfer){super.postMessage(data?.type==='init'&&data.preserveDrawingBuffer?{...data,software:true}:data,transfer);}};});if(process.env.PHASE_DIAGNOSTICS==='1')await page.addInitScript(require('./openprops-phase-probe.cjs'));await page.exposeBinding('__recordPlayback',(_,batch)=>{records.push(...batch);});await page.exposeBinding('__recordPlaybackVisit',(_,visit)=>{visits.push(visit);});page.on('crash',()=>result.browserDiagnostics.crashes.push(new Date().toISOString()));page.setDefaultTimeout(45000);page.on('pageerror',e=>errors.push(e.message));
  await page.route(/^https?:\/\//,r=>{network.push(r.request().url());return r.abort();});await page.goto(pathToFileURL(path.resolve(file)).href);
  await page.waitForFunction(()=>window.__LAYMESH_WEB?.presentation&&document.querySelector('#app').dataset.painted==='true');
  await page.evaluate(()=>{
   const v=__LAYMESH_WEB.presentation,m=window.playbackProbe={calls:[],visits:[],inputs:[],heartbeats:[],started:performance.now(),notifications:[]},call=v.call.bind(v),go=v.go.bind(v);const visible=v.navigationVisible.bind(v);v.navigationVisible=data=>{m.notifications.push({page:data.pageId,at:performance.now(),deliveryMs:performance.timeOrigin+performance.now()-data.at,initialEntry:!!data.initialEntry});return visible(data);};
   m.flushing=Promise.resolve();m.flush=()=>{const batch=m.calls.splice(0);if(batch.length)m.flushing=m.flushing.then(()=>window.__recordPlayback(batch));return m.flushing;};m.flushTimer=setInterval(()=>m.flush(),250);
   let last=performance.now();m.timer=setInterval(()=>{const now=performance.now();m.heartbeats.push(now-last);last=now;},50);
   v.call=async(d,...a)=>{const start=performance.now();try{const r=await call(d,...a);const fields=['softwareGpuBypass','backendProbeMs','initialEntryRasterMs','renderer','transitionTimings','transitionSamples','transitionOpaque','diagnosticCompositionMs','diagnosticSurfaceFlushMs','handoffMs','handoffBytes','handoffFormat','transitionComposition','transitionMs','transitionFrames','frameCount','gpuSamples','backend','device','skipped','unchanged','sampleMs','rasterMs','queueWaitMs','gpuMs','resourceWaitMs','presentationWaitMs','cacheBytes','staticTextureBytes','layerTextureBytes','residentCacheBytes','residentSnapshotBytes','resourceStats','wasmMemoryBytes','wasmAllocation','skiaMemoryBytes','installMs','decodeMs','staticCacheHits','staticCacheBuilds','layerCacheHits','layerCacheBuilds','cpuLayerBlits','cpuTileConversions','effectDraws','effectPictureReplays','transitionError','preparation'];m.calls.push({type:d.type,stage:d.stage,page:d.pageId||d.page?.id,serial:d.serial,time:d.time,running:d.running,priority:d.priority,start,end:performance.now(),result:Object.fromEntries(fields.filter(k=>r[k]!==undefined).map(k=>[k,r[k]]))});return r;}catch(e){m.calls.push({type:d.type,page:d.pageId||d.page?.id,start,end:performance.now(),error:e.message});throw e;}};
   v.go=async(...a)=>{const start=performance.now(),ok=await go(...a);if(ok){const visit={index:v.index,page:v.deck.pages[v.index].id,start,end:performance.now(),transition:v.metrics.transitions.at(-1)};m.visits.push(visit);m.flushing=m.flushing.then(()=>window.__recordPlaybackVisit(visit));}return ok;};
   document.addEventListener('keydown',e=>m.inputs.push({key:e.key,at:performance.now(),switching:v.switching}),true);
  });
  await page.evaluate(first=>{const v=__LAYMESH_WEB.presentation;v.open({fullscreen:false,startPageId:__LAYMESH_WEB.pages?.[first]?.id||`P${String(first+1).padStart(2,'0')}`});},first);
  for(let round=0;round<rounds;round++)for(let index=first;index<first+count;index++){
   if(round||index!==first){if(index===first){await page.keyboard.type(String(index+1));await page.keyboard.press('Enter');}else await page.keyboard.press(index%2?'ArrowDown':'ArrowRight');}
   await page.waitForFunction(index=>{const v=__LAYMESH_WEB.presentation;if(!v.ready&&!v.switching&&v.retry?.hidden===false)throw Error(v.message.textContent);return v.ready&&!v.switching&&v.index===index;},index);
   await page.waitForFunction(()=>{const v=__LAYMESH_WEB.presentation;if(!v.ready)throw Error(v.message.textContent);return !v.clock.running&&!v.painting&&!v.switching;},null,{timeout:90000});
   const state=await page.evaluate(()=>{const v=__LAYMESH_WEB.presentation;return {index:v.index,time:v.clock.time,duration:v.clock.duration,frame:v.metrics.lastFrame,resources:v.metrics.resources,ready:[...v.readyResident||[]]};});
   checks.push({round,index,time:state.time,duration:state.duration,memory:memory(server.process().pid),frame:state.frame});
   fs.writeFileSync(path.join(out,engine+'-progress.json'),JSON.stringify({result,checks,errors,samples},null,2));
   console.log(engine,width,round+1,index+1,'complete',state.frame?.backend);
  }
  // Check actual editable-workspace context menu as well as the presenter menu.
  const menuStart=Date.now();await page.mouse.click(width*.7,height*.45,{button:'right'});
  await page.waitForFunction(()=>[...document.querySelectorAll('.context-menu-list,.presentation-menu')].some(n=>n.getClientRects().length&&!n.hidden&&getComputedStyle(n).visibility!=='hidden'));
  result.menuMs=Date.now()-menuStart;await page.keyboard.press('Escape');
  const closeStart=Date.now();await page.keyboard.press('Escape');await page.waitForFunction(()=>!__LAYMESH_WEB.presentation.opened&&!document.querySelector('.presentation'));result.closeMs=Date.now()-closeStart;
  await page.evaluate(()=>playbackProbe.flush());const failures=records.filter(c=>c.error&&!/closed|cancel|terminated/i.test(c.error));
  result.taskErrors=failures;result.status=errors.length||failures.length?'failed':'passed';
 }catch(e){result.status='failed';result.error=String(e);console.error(engine,'FAILED',String(e));}
 finally{
  clearInterval(timer);if(page)try{result.probe=await page.evaluate(async()=>{const m=window.playbackProbe;if(!m)return null;clearInterval(m.timer);clearInterval(m.flushTimer);await m.flush();return {visits:m.visits,inputs:m.inputs,heartbeats:m.heartbeats,notifications:m.notifications,patchedWorkers:window.__phasePatchedWorkers||0,started:m.started};});if(result.probe)result.probe.calls=records;await page.screenshot({path:path.join(out,engine+'.png')});}catch(e){result.captureError=String(e);}
  result.probe??={calls:records,visits,inputs:[],heartbeats:[],notifications:[],incomplete:true};result.probe.calls=records;
  Object.assign(result,{checks,errors,network,memory:samples,ended:new Date().toISOString(),definition:'Natural authored duration; trusted navigation; first traversal starts immediately. Trace batches stream to the external test process every 250 ms and do not accumulate in browser memory. PSS includes browser descendants; GPU process PSS is not VRAM. Render replies and GPU fence completion are not physical scanout.'});
  fs.writeFileSync(path.join(out,engine+'.json'),JSON.stringify(result,null,2));await browser.close();await server.close();
 }
 if(result.status!=='passed'&&process.env.ALLOW_FAILURE!=='1')process.exitCode=1;
}})().catch(e=>{console.error(e);process.exitCode=1;});
