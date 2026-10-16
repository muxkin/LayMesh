// Current page/directory latency and the full-document synchronization barrier.
// Usage: PLAYWRIGHT_MODULE=/path/to/playwright node scripts/test-web-document-performance.cjs HTML OUTPUT [SAMPLES=50]
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),crypto=require('node:crypto'),{pathToFileURL}=require('node:url');
const pw=require(process.env.PLAYWRIGHT_MODULE||'playwright'),cases=require('./semantic-document-cases.cjs');
const html=path.resolve(process.argv[2]),out=path.resolve(process.argv[3]),samples=Number(process.argv[4]||50);assert(samples>=(process.env.LAYMESH_SMOKE?2:50));fs.mkdirSync(out,{recursive:true});
const quantile=(a,q)=>{const s=[...a].sort((x,y)=>x-y);return q===.5?(s[Math.floor((s.length-1)/2)]+s[Math.floor(s.length/2)])/2:s[Math.ceil(s.length*q)-1];};
(async()=>{for(const engine of (process.env.LAYMESH_TEST_ENGINES||'chromium,firefox,webkit').split(',')){
 const browser=await pw[engine].launch({headless:true}),rows=[],summaries=[];
 try{
  const p=await browser.newPage({viewport:{width:1600,height:1000}}),errors=[],network=[];p.setDefaultTimeout(240000);p.on('pageerror',e=>errors.push(e.message));await p.route(/^https?:\/\//,r=>{network.push(r.request().url());return r.abort();});
  await p.goto(pathToFileURL(html).href);await p.waitForFunction(()=>window.__LAYMESH_WEB?.editor&&document.querySelector('#app').dataset.painted==='true');
  await p.evaluate(()=>__LAYMESH_WEB.previewCompiler.invalidate());await p.waitForFunction(()=>document.querySelector('#app').dataset.documentState==='ready');
  await p.locator('#page').selectOption('5');await p.waitForFunction(()=>__LAYMESH_WEB.metrics.pageId===__LAYMESH_WEB.pages[5].id&&document.querySelector('#app').dataset.painted==='true');
  const original=await p.evaluate(()=>({css:__LAYMESH_WEB.editor.files['/project/theme.lcss'],doc:__LAYMESH_WEB.editor.files['/project/storyboard.laydoc']}));
  await p.evaluate(()=>{
   const c=__LAYMESH_WEB.previewCompiler,state=c.state,update=c.update;
   c.state=value=>{state(value);const m=window.__docMeasure;if(m&&value.version===m.version&&value.complete&&!value.running){m.documentMs=performance.now()-m.start;m.currentMs=__LAYMESH_WEB.metrics.editToPreviewMs;m.presentationConfirmed=__LAYMESH_WEB.metrics.presentationConfirmed;m.done=true;}};
   c.update=async(...args)=>{await update(...args);const m=window.__docMeasure;if(m&&args[2]===m.version)for(const p of args[0].pages)m.pages.push({id:p.id,...p.compileMetrics});};
   new MutationObserver(()=>{const m=window.__docMeasure;if(m&&m.directoryMs==null)m.directoryMs=performance.now()-m.start;}).observe(document.querySelector('#page'),{childList:true});
  });
  const apply=async(change,measured)=>{
   await p.evaluate(async({change,measured})=>{const w=__LAYMESH_WEB,name='/project/'+change.file;await w.workspace.open(name);const v=w.editor.views.get(name).view;
    window.__docMeasure=measured?{start:performance.now(),version:w.previewCompiler.version+1,installsBefore:w.metrics.pageInstalls||0,pages:[]}:null;
    v.dispatch({changes:{from:0,to:v.state.doc.length,insert:change.source}});
   },{change,measured});
   if(measured)await p.waitForFunction(()=>window.__docMeasure?.done);else await p.waitForFunction(()=>document.querySelector('#app').dataset.documentState==='ready');
  };
  for(const kind of (process.env.LAYMESH_DOCUMENT_CASES||cases.names.join(',')).split(',')){
   await apply({file:'theme.lcss',source:original.css},false);await apply({file:'storyboard.laydoc',source:original.doc},false);
   const count=kind==='laydoc-add-remove'?samples*2:samples;
   for(let i=0;i<count+2;i++){
    const change=cases.change(kind,i,original.css,original.doc);await apply(change,true);
    const row=await p.evaluate(change=>{const w=__LAYMESH_WEB,current=w.pages.find(p=>p.id===w.metrics.pageId),record=w.previewCompiler.current;
     const equal=a=>JSON.stringify(a)===JSON.stringify(change.rgba);let colorMatches=0;
     const visit=n=>{if(n.kind.kind==='glyphs'&&equal(n.kind.color))colorMatches++;if(n.kind.kind==='path'&&(equal(n.kind.stroke?.rgba)||equal(n.kind.fill?.rgba)))colorMatches++;if(n.kind.kind==='group')n.kind.children.forEach(visit);};if(change.rgba)current.list.nodes.forEach(visit);
     return{...window.__docMeasure,selected:record.key,installsAfter:w.metrics.pageInstalls||0,ids:w.previewCompiler.records.map(r=>r.key),title:document.title,colorMatches,wasmRetainedBytes:w.metrics.compilerMemoryBytes};
    },change);
    if(i>=2)assert.equal(row.installsAfter,row.installsBefore,'Background compilation must retain the active drawing baseline');
    assert.equal(row.selected,'editable');assert.equal(row.presentationConfirmed,true);if(change.ids)assert.deepEqual(row.ids,change.ids);if(change.name)assert.equal(row.title,change.name);if(change.rgba)assert(row.colorMatches>0,'Style edit must alter the displayed drawing');
    if(i>=2)rows.push({engine,kind,operation:change.operation||kind,sample:i-2,...row});
    if(i>=2&&(i-1)%10===0)console.log(`${engine} ${kind}: ${i-1}/${count} accepted samples`);
   }
   const selected=rows.filter(r=>r.kind===kind),summary={engine,kind,samples:selected.length};
   for(const key of ['directoryMs','currentMs','documentMs'])summary[key]={median:quantile(selected.map(r=>r[key]),.5),p95:quantile(selected.map(r=>r[key]),.95)};
   summaries.push(summary);console.log(JSON.stringify(summary));
   fs.writeFileSync(path.join(out,engine+'.json'),JSON.stringify({engine,smoke:!!process.env.LAYMESH_SMOKE,version:browser.version(),html,htmlSha256:crypto.createHash('sha256').update(fs.readFileSync(html)).digest('hex'),definition:'Edit end through current-page presentation and all page compilation; 30 ms input window included; 2 warmups excluded; active page is editable; directory timestamp observes DOM page option replacement',summaries,rows,errors,network},null,2)+'\n');
  }
  assert.deepEqual(errors,[]);assert.deepEqual(network,[]);
 }finally{await browser.close();}
}})().catch(e=>{console.error(e);process.exitCode=1});
