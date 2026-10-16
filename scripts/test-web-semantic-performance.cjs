// Real file:// editing, measured after warmup through the final painted frame.
// Usage: PLAYWRIGHT_MODULE=/path/to/playwright node scripts/test-web-semantic-performance.cjs HTML OUTPUT [SAMPLES=50]
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {pathToFileURL}=require('node:url');
const pw=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const html=path.resolve(process.argv[2]),out=path.resolve(process.argv[3]),samples=Number(process.argv[4]||50);
const editMode=process.env.LAYMESH_EDIT_MODE||'unique';
assert(['unique','undo'].includes(editMode),'LAYMESH_EDIT_MODE must be unique or undo');
assert(Number.isSafeInteger(samples)&&samples>=50,'Use at least 50 measured edits per case');
fs.mkdirSync(out,{recursive:true});
const quantile=(xs,q)=>{const s=[...xs].sort((a,b)=>a-b);return q===.5?(s[Math.floor((s.length-1)/2)]+s[Math.floor(s.length/2)])/2:s[Math.ceil(s.length*q)-1];};
(async()=>{
 for(const engine of (process.env.LAYMESH_TEST_ENGINES||'chromium,firefox,webkit').split(',')){
  const browser=await pw[engine].launch({headless:true}),rows=[],summaries=[];
  try{
   const context=await browser.newContext({viewport:{width:1600,height:1000}}),errors=[],network=[];
   await context.route(/^https?:\/\//,r=>{network.push(r.request().url());return r.abort();});
   const page=await context.newPage();page.setDefaultTimeout(180000);page.on('pageerror',e=>errors.push(e.message));
   await page.goto(pathToFileURL(html).href);
   await page.waitForFunction(()=>window.__LAYMESH_WEB?.editor&&document.querySelector('#app').dataset.painted==='true');
   await page.evaluate(()=>__LAYMESH_WEB.previewCompiler.invalidate());
   await page.waitForFunction(()=>document.querySelector('#app').dataset.documentState==='ready');
   for(const [index,file] of [[1,'02-seed'],[5,'06-editable']]){
    await page.locator('#page').selectOption(String(index));
    await page.waitForFunction(index=>__LAYMESH_WEB.metrics.pageId===__LAYMESH_WEB.pages[index].id&&document.querySelector('#app').dataset.painted==='true',index);
    const name='/project/pages/'+file+'.lay';
    const original=await page.evaluate(async name=>{await __LAYMESH_WEB.workspace.open(name);return __LAYMESH_WEB.editor.views.get(name).view.state.doc.toString();},name);
    for(const kind of ['color','position','text','size']){
     const edited=kind==='color'?original.replace('#fbfaf5','#faf9f4'):
      kind==='text'?original.replace(/chrome\(page,("[^"\n]*"),("[^"\n]*")/,(_,number,title)=>`chrome(page,${number},${JSON.stringify(JSON.parse(title)+'·')}`):
      kind==='position'?original.replace(/offset=\(([-\d.]+)mm,/,(_,x)=>`offset=(${Number(x)+1}mm,`):
      original.replace(/rect\(size=\(([-\d.]+)mm,/,(_,x)=>`rect(size=(${Number(x)-1}mm,`);
     assert.notEqual(edited,original,'Missing fixture edit: '+file+' '+kind);
     for(let i=0;i<samples+2;i++){
      const unique=kind==='color'?original.replace('#fbfaf5',`#faf9${i.toString(16).padStart(2,'0')}`):
       kind==='text'?original.replace(/chrome\(page,("[^"\n]*"),("[^"\n]*")/,(_,number,title)=>`chrome(page,${number},${JSON.stringify(JSON.parse(title)+' '+i)}`):
       kind==='position'?original.replace(/offset=\(([-\d.]+)mm,/,(_,x)=>`offset=(${(Number(x)+1+i/100).toFixed(2)}mm,`):
       original.replace(/rect\(size=\(([-\d.]+)mm,/,(_,x)=>`rect(size=(${(Number(x)-1+i/100).toFixed(2)}mm,`);
      const before=await page.evaluate(({name,source})=>{const w=__LAYMESH_WEB,v=w.editor.views.get(name).view;const result={id:w.metrics.pageId,installs:w.metrics.pageInstalls||0};window.__semanticEditStarted=performance.now();v.dispatch({changes:{from:0,to:v.state.doc.length,insert:source}});return result;},{name,source:editMode==='unique'?unique:i%2?original:edited});
      await page.waitForFunction(({id,index})=>{const w=__LAYMESH_WEB,a=document.querySelector('#app');return a.dataset.compileState==='ready'&&a.dataset.painted==='true'&&w.metrics.pageId!==id&&w.metrics.pageId===w.pages[index].id;},{id:before.id,index});
      const result=await page.evaluate(index=>({elapsedMs:__LAYMESH_WEB.metrics.editToPreviewMs,presentationConfirmed:__LAYMESH_WEB.metrics.presentationConfirmed,observedMs:performance.now()-__semanticEditStarted,prepareMs:__LAYMESH_WEB.metrics.prepareMs,rasterMs:__LAYMESH_WEB.metrics.rasterMs,compileMs:__LAYMESH_WEB.metrics.previewCompileMs,wasmRetainedBytes:__LAYMESH_WEB.metrics.compilerMemoryBytes,installs:__LAYMESH_WEB.metrics.pageInstalls,metrics:__LAYMESH_WEB.pages[index].compileMetrics}),index);
      if(i>=2)assert.equal(result.installs,before.installs,'Warm edits must apply a drawing patch');
      assert.equal(result.presentationConfirmed,true,'Measurement requires a visible rendering update');
      if(index===5&&i>=2){assert.equal(result.metrics.evaluatedBuiltinCalls.path||0,0,'Unrelated edits must reuse the complex curves');if(kind!=='size')assert.equal(result.metrics.rebuiltGeometry,0,'Local edits must not rebuild curve geometry');}
      if(i>=2)rows.push({engine,page:file,kind,sample:i-2,...result});
     }
     const selected=rows.filter(r=>r.page===file&&r.kind===kind),times=selected.map(r=>r.elapsedMs);
     const summary={engine,page:file,kind,samples:times.length,medianMs:quantile(times,.5),p95Ms:quantile(times,.95),targetPassed:quantile(times,.5)<=100&&quantile(times,.95)<=200};summaries.push(summary);
     fs.writeFileSync(path.join(out,engine+'.json'),JSON.stringify({engine,editMode,version:browser.version(),html,sha256:require('node:crypto').createHash('sha256').update(fs.readFileSync(html)).digest('hex'),definition:'CodeMirror edit end to correct current page painted, including 30 ms input window; two warmups excluded; offline file URL; headless browser (automatically selected renderer); WASM retained capacity is distinct from measured derivation cache bytes',summaries,rows,errors,network},null,2)+'\n');
     console.log(JSON.stringify(summary));
    }
   }
   assert.deepEqual(errors,[]);assert.deepEqual(network,[]);
  }finally{await browser.close();}
 }
})().catch(e=>{console.error(e);process.exitCode=1;});
