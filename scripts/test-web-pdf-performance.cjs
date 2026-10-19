// Actual offline file, default 1200 DPI, cancellation and repeated exports.
// Run separately from builds/other browsers for comparable CPU/RSS measurements.
const fs = require('node:fs'), path = require('node:path'), cp = require('node:child_process');
const assert = require('node:assert/strict');
const engines = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const input = path.resolve(process.argv[2]);
const out = path.resolve(process.argv[3] || '/tmp/laymesh-pdf-performance');
fs.mkdirSync(out, {recursive:true});
const delay = ms => new Promise(resolve=>setTimeout(resolve,ms));
(async()=>{
  const results=[];
  for(const engine of (process.env.LAYMESH_TEST_ENGINES || 'chromium,firefox,webkit').split(',')) {
    const browser=await engines[engine].launch({headless:true,...(engine==='chromium'&&process.env.CHROMIUM_PATH?{executablePath:process.env.CHROMIUM_PATH}:{})});
    try {
      const context=await browser.newContext({acceptDownloads:true}), errors=[];
      await context.addInitScript(()=>{
        Object.defineProperty(window,'showSaveFilePicker',{value:undefined});
        const Original=Worker;window.exportAudit={live:0,messages:[]};
        window.Worker=class extends Original {
          constructor(...args){super(...args);this.live=true;exportAudit.live++;
            this.addEventListener('message',({data:d})=>{
              if(d.progress)console.log('[PDF export] '+d.progress);
              if(d.progress||d.output||d.error)exportAudit.messages.push({at:performance.now(),progress:d.progress,metrics:d.metrics,error:d.error});
            });
          }
          terminate(){if(this.live){exportAudit.live--;this.live=false;}super.terminate();}
        };
      });
      const page=await context.newPage();page.setDefaultTimeout(Number(process.env.LAYMESH_PDF_TIMEOUT_MS || 600000));
      page.on('pageerror',e=>errors.push(e.message));
      page.on('console',m=>{if(m.type()==='error')errors.push(m.text());else if(m.text().startsWith('[PDF export]'))console.log(engine,m.text());});
      await page.goto('file://'+input);
      await page.waitForFunction(()=>document.querySelector('#app')?.dataset.painted==='true');
      // Chromium-only aggregate browser process RSS; other engines have no CDP.
      const cdp=engine==='chromium'?await browser.newBrowserCDPSession():null;
      const sample=async()=>{
        if(!cdp)return null;
        const {processInfo}=await cdp.send('SystemInfo.getProcessInfo');
        const rss=cp.execFileSync('ps',['-o','rss=','-p',processInfo.map(p=>p.id).join(',')],{encoding:'utf8'});
        return {rssMiB:rss.trim().split(/\s+/).map(Number).reduce((a,b)=>a+b,0)/1024,cpuSeconds:processInfo.reduce((n,p)=>n+p.cpuTime,0)};
      };
      const open=async()=>{await page.locator('#export').click();await page.locator('#export-format').selectOption('pdf');};
      // The editable profile lazily starts its persistent compiler/language
      // workers during prepare(). Establish their baseline before testing
      // exporter lifetime, rather than mistaking them for cancelled exporters.
      await open();
      const warmDownload=page.waitForEvent('download');
      await page.locator('#export-file').click();await warmDownload;
      await page.waitForFunction(()=>!document.querySelector('#export-file').disabled);
      await delay(500);
      const live=await page.evaluate(()=>exportAudit.live);
      // Cancel while effects are running, then reuse the same application.
      await open();await page.locator('#export-file').click();await delay(500);
      const cancelStart=Date.now();await page.locator('#export-cancel').click();
      await page.waitForFunction(()=>!document.querySelector('#export-file').disabled);
      const cancelMs=Date.now()-cancelStart;
      assert.equal(await page.evaluate(()=>exportAudit.live),live,'Cancelled exporter worker leaked');
      await page.locator('#export-panel .dialog-close').click();
      const runs=[];
      for(let run=0;run<Number(process.env.LAYMESH_PDF_REPEATS || 3);run++) {
        await open();await delay(1500);
        await page.evaluate(()=>{exportAudit.messages=[];});
        const before=await sample(), samples=[];let sampling=false;
        let sampleError;
        const timer=cdp?setInterval(async()=>{if(sampling)return;sampling=true;try{samples.push(await sample());}catch(e){sampleError=e;}finally{sampling=false;}},250):null;
        try {
        const started=Date.now(), download=page.waitForEvent('download');
        await page.locator('#export-file').click();
        const file=await download, elapsedMs=Date.now()-started;
        const pdf=path.join(out,`${engine}-${run+1}.pdf`);await file.saveAs(pdf);
        if(timer)clearInterval(timer);
        if(sampleError)throw sampleError;
        await page.waitForFunction(()=>!document.querySelector('#export-file').disabled);
        assert.equal(await page.evaluate(()=>exportAudit.live),live,'Completed exporter worker leaked');
        const messages=await page.evaluate(()=>exportAudit.messages);
        const metrics=messages.find(m=>m.metrics)?.metrics;
        if(metrics)assert(metrics.peakTileRGBABytes<=64*1024*1024,'Tile exceeded RGBA budget');
        assert(!messages.some(m=>m.error),JSON.stringify(messages));
        await delay(2000);const after=await sample();
        const peakMiB=before?Math.max(before.rssMiB,...samples.map(s=>s.rssMiB)):null;
        runs.push({elapsedMs,bytes:fs.statSync(pdf).size,before,after,peakMiB,addedPeakMiB:before?peakMiB-before.rssMiB:null,metrics});
        console.log(JSON.stringify({engine,run:run+1,...runs.at(-1)}));
        } finally {if(timer)clearInterval(timer);}
      }
      assert.deepEqual(errors,[]);
      results.push({engine,browser:browser.version(),dpi:1200,warmupExports:1,baselineLiveWorkers:live,cancelMs,runs,errors});
      fs.writeFileSync(path.join(out,'results.json'),JSON.stringify(results,null,2)+'\n');
    } finally {await browser.close();}
  }
})().catch(e=>{console.error(e);process.exitCode=1;});
