// Same artwork/resolution, sequential runs. Software GPU results are not hardware GPU claims.
const pw=require(process.env.PLAYWRIGHT_MODULE||'playwright'),fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const pct=(a,q)=>{a=a.filter(Number.isFinite).sort((a,b)=>a-b);return a[Math.min(a.length-1,Math.floor(a.length*q))]||0};
const processMemory=info=>{let pssBytes=0,processes=0;for(const row of info)try{const text=fs.readFileSync(`/proc/${row.id}/smaps_rollup`,'utf8'),pss=text.match(/^Pss:\s+(\d+)/m);if(pss){pssBytes+=Number(pss[1])*1024;processes++;}}catch{}return {pssBytes,processes};};
(async()=>{const [oldFile,newFile,out='/tmp/laymesh-presenter-v004/performance.json']=process.argv.slice(2),results=[];assert(oldFile&&newFile);
 for(const mode of ['slides','previous-presenter','bitmap-presenter']){
  const browser=await pw.chromium.launch({headless:true,ignoreDefaultArgs:['--disable-popup-blocking'],args:['--enable-unsafe-swiftshader']});try{
   const ctx=await browser.newContext({viewport:{width:1280,height:800}}),p=await ctx.newPage(),system=await browser.newBrowserCDPSession();p.setDefaultTimeout(90000);await p.goto('file://'+path.resolve(mode==='previous-presenter'?oldFile:newFile));await p.waitForFunction(()=>window.__LAYMESH_WEB&&document.querySelector('#app').dataset.painted==='true');await p.selectOption('#page','5');
   let audience=p,control;
   if(mode==='slides')await p.evaluate(()=>__LAYMESH_WEB.presentation.open({fullscreen:false,startPageId:'editable'}));
   else{await p.click('#preview-more summary');const popup=ctx.waitForEvent('page');await p.click('#presenter');const other=await popup;await other.setViewportSize({width:1280,height:800});if(mode==='previous-presenter'){audience=other;control=p;await p.waitForFunction(()=>__LAYMESH_WEB.presenter.connected);}else{control=other;await control.waitForFunction(()=>window.laymeshPresenterConsole?.state?.ready);}}
   await audience.waitForFunction(()=>window.laymeshPresentation?.ready||window.__LAYMESH_WEB?.presentation.ready);
   await audience.evaluate(async()=>{window.measuredView=window.laymeshPresentation||__LAYMESH_WEB.presentation;const v=measuredView;v.clock.running=false;v.sync();if(document.fullscreenElement)await document.exitFullscreen();await v.preload;});
   // Allow static previews to finish before measuring active animation.
   await p.waitForTimeout(1200);
   if(mode==='previous-presenter')await control.evaluate(()=>{window.extra={calls:0,ms:0};const r=__LAYMESH_WEB.presenter.currentRenderer,draw=r.draw.bind(r);r.draw=async(...args)=>{const t=performance.now();extra.calls++;try{return await draw(...args);}finally{extra.ms+=performance.now()-t;}}});
   await audience.evaluate(()=>{const v=measuredView,call=v.call.bind(v);window.measureRows=[];v.call=async(data,...args)=>{const t=performance.now(),result=await call(data,...args);if(data.type==='paint')measureRows.push({requestMs:performance.now()-t,sampleMs:result.sampleMs,drawMs:result.rasterMs,skiaMemoryBytes:result.skiaMemoryBytes,wasmMemoryBytes:result.wasmMemoryBytes,backend:result.backend});return result;};});
   const cpu0=(await system.send('SystemInfo.getProcessInfo')).processInfo,memoryBefore=processMemory(cpu0),started=Date.now();
   await audience.evaluate(async()=>{await measuredView.go(5);measuredView.clock.running=true;measuredView.sync();});await audience.waitForFunction(()=>measuredView.clock.status?measuredView.clock.status().animationStatus==='complete':measuredView.clock.completed===measuredView.clock.steps.length&&!measuredView.clock.running,{},{timeout:60000});
   const elapsedMs=Date.now()-started,cpu1=(await system.send('SystemInfo.getProcessInfo')).processInfo;
   const rows=await audience.evaluate(()=>measureRows),last=rows.at(-1)||{},cpuSeconds=cpu1.reduce((n,row)=>n+Math.max(0,row.cpuTime-(cpu0.find(p=>p.id===row.id)?.cpuTime||0)),0);
   const extra=mode==='previous-presenter'?await control.evaluate(()=>extra):{calls:0,ms:0};const mirror=mode==='bitmap-presenter'?await p.evaluate(()=>__LAYMESH_WEB.presenter.mirror.metrics):null;
   results.push({mode,elapsedMs,cpuSeconds,memoryBefore,memoryAfter:processMemory(cpu1),cpuCoreEquivalent:cpuSeconds/(elapsedMs/1000),audienceSamples:rows.length,audienceRequestP95Ms:pct(rows.map(r=>r.requestMs),.95),sampleP95Ms:pct(rows.map(r=>r.sampleMs),.95),drawP95Ms:pct(rows.map(r=>r.drawMs),.95),additionalConsoleSceneDraws:extra.calls,consoleDrawWallMs:extra.ms,mirror,skiaMemoryBytes:last.skiaMemoryBytes,wasmMemoryBytes:last.wasmMemoryBytes,backend:last.backend});console.log(mode,JSON.stringify(results.at(-1)));
  }finally{await browser.close();}
 }
 fs.mkdirSync(path.dirname(out),{recursive:true});fs.writeFileSync(out,JSON.stringify({viewport:'1280x800',page:'editable / actual six-page artwork',environment:'Linux Chromium headless, SwiftShader software GPU; CPU includes all CDP-reported browser processes; PSS sums their Linux smaps_rollup proportional resident memory',results},null,2)+'\n');
})().catch(e=>{console.error(e);process.exitCode=1});
