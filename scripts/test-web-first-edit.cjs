// Fresh offline HTML for every cold sample; its immediate second edit is warm.
// No compiler preparation during document opening. UI edits use CodeMirror.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),crypto=require('node:crypto'),{pathToFileURL}=require('node:url');
const pw=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const [html,output,count='50']=process.argv.slice(2),samples=+count;
assert(samples>=(process.env.SMOKE?1:50));fs.mkdirSync(output,{recursive:true});
const quantile=(a,q)=>{a=[...a].sort((a,b)=>a-b);return q===.5?(a[Math.floor((a.length-1)/2)]+a[Math.floor(a.length/2)])/2:a[Math.ceil(a.length*q)-1];};
(async()=>{
 for(const engine of (process.env.ENGINES||'chromium').split(',')) {
  const browser=await pw[engine].launch({headless:true}),rows=[];
  try {
   for(const kind of (process.env.CASES||'lay,lcss,laydoc').split(','))for(let sample=0;sample<samples;sample++) {
    const ctx=await browser.newContext({viewport:{width:1600,height:1000}}),p=await ctx.newPage(),errors=[];
    try {
     p.setDefaultTimeout(180000);p.on('pageerror',e=>errors.push(e.message));await p.route(/^https?:\/\//,r=>r.abort());
     await p.goto(pathToFileURL(path.resolve(html)).href);await p.waitForFunction(()=>window.__LAYMESH_WEB?.editor&&document.querySelector('#app').dataset.painted==='true');
     await p.evaluate(async kind=>{
      const w=__LAYMESH_WEB,c=w.previewCompiler;
      if(c.version!==0)throw Error('Cold sample unexpectedly compiled before editing');
      const file='/project/'+({lay:'pages/01-cover.lay',lcss:'theme.lcss',laydoc:'storyboard.laydoc'}[kind]);
      await w.workspace.open(file);const view=w.editor.views.get(file).view,original=view.state.doc.toString();
      window.__firstEdit={file,view,original,initial:c.records.map(r=>r.value),indexes:c.records.filter(r=>r.dependencies).length};
     },kind);
     for(const temperature of ['cold','warm']) {
      const row=await p.evaluate(async({kind,temperature,sample})=>{
       const w=__LAYMESH_WEB,c=w.previewCompiler,{view,original,initial,indexes}=__firstEdit;
       const color=temperature==='cold'?'#c61717':'#1769c6';let source;
       if(kind==='lay')source=original.replace(/navy = "#[a-f0-9]+"/i,'navy = "'+color+'"');
       else if(kind==='lcss')source=original+'\n* { color:'+color+' !important; border-color:'+color+' !important; }\n';
       else {const lines=original.trimEnd().split('\n'),pages=lines.filter(l=>l.startsWith('doc.add('));source=[...lines.filter(l=>!l.startsWith('doc.add(')),pages[0],...pages.slice(1).reverse()].join('\n')+'\n';if(temperature==='warm')source=original;}
       if(source===view.state.doc.toString())throw Error('No actual edit');
       let visibleAt,doneAt;const started=performance.now(),baseCompiles=w.metrics.compiles;
       const unsubscribe=c.subscribe(()=>{if(c.currentReady&&visibleAt==null)visibleAt=performance.now();if(c.complete&&doneAt==null)doneAt=performance.now();});
       view.dispatch({changes:{from:0,to:view.state.doc.length,insert:source}});
       const version=c.version;
       // Preserve the normal input merge window before requesting playback.
       await new Promise(resolve=>setTimeout(resolve,30));
       const startPlayback=performance.now();
       await w.presentation.open({fullscreen:false,startPageId:'cover'});
       if(!w.presentation.ready)throw Error(w.presentation.root?.dataset.error||'Presentation did not start');
       const playableAt=performance.now();c.event('playable',{sourceVersion:version});
       await c.ensure();
       const trace=structuredClone(c.trace),unchanged=c.records.slice(1).every((r,i)=>r.value===initial[i+1]);
       const rgba=color.slice(1).match(/../g).map(n=>parseInt(n,16)/255),matches=w.pages.map(p=>{
        let count=0;const same=v=>v?.slice(0,3).every((x,i)=>Math.abs(x-rgba[i])<1e-8);
        const walk=n=>{if(same(n.kind.color)||same(n.kind.fill?.rgba)||same(n.kind.stroke?.rgba))count++;for(const e of n.effects||[])if(same(e.rgba))count++;if(n.kind.kind==='group')n.kind.children.forEach(walk);};p.list.nodes.forEach(walk);return count;
       });
       const result={kind,temperature,sample,version,indexes,matches,timeOrigin:performance.timeOrigin,currentMs:visibleAt-started,playableMs:playableAt-started,documentMs:doneAt-started,
         playbackClickMs:startPlayback-started,compiles:w.metrics.compiles-baseCompiles,unchanged,ids:c.records.map(r=>r.key),trace,
         hardwareConcurrency:navigator.hardwareConcurrency,wasmMemoryBytes:w.metrics.compilerMemoryBytes};
       unsubscribe();await w.presentation.close();
       return result;
      },{kind,temperature,sample});
      assert.equal(row.indexes,6,'All six dependency indexes must restore in the final HTML');
      if(kind==='lay'){assert.equal(row.compiles,1);assert(row.unchanged);assert(row.matches[0]>0);}
      if(kind==='lcss')assert(row.matches.every(n=>n>0),'The shared style must visibly affect all six pages');
      if(kind==='laydoc')assert.deepEqual(row.ids,temperature==='cold'?['cover','editable','offline','motion','anchors','seed']:['cover','seed','anchors','motion','offline','editable']);
      const events=row.trace.events.filter(e=>e.type==='received').map(e=>e.diagnostics).filter(Boolean);
      row.workers=[...new Set(events.map(e=>e.worker))];
      row.overlaps=events.flatMap((a,i)=>events.slice(i+1).filter(b=>a.worker!==b.worker&&Math.max(a.startedAt,b.startedAt)<Math.min(a.finishedAt,b.finishedAt)).map(b=>[a.worker,b.worker]));
      if(kind==='lcss'&&row.hardwareConcurrency>1)assert(row.overlaps.length>0,'Multiple compiler Workers must actually overlap');
      assert.deepEqual(errors,[]);rows.push(row);
      fs.writeFileSync(path.join(output,engine+'-raw.json'),JSON.stringify({engine,version:browser.version(),htmlSha256:crypto.createHash('sha256').update(fs.readFileSync(html)).digest('hex'),samples,smoke:!!process.env.SMOKE,rows},null,2));
     }
     console.log(engine,kind,sample+1,'/',samples);
    } finally { await ctx.close(); }
   }
   const summaries=[];
   for(const kind of new Set(rows.map(r=>r.kind)))for(const temperature of ['cold','warm']){
    const selected=rows.filter(r=>r.kind===kind&&r.temperature===temperature),summary={kind,temperature,samples:selected.length};
    for(const key of ['currentMs','playableMs','documentMs'])summary[key]={median:quantile(selected.map(r=>r[key]),.5),p95:quantile(selected.map(r=>r[key]),.95)};
    summaries.push(summary);
   }
   fs.writeFileSync(path.join(output,engine+'-summary.json'),JSON.stringify(summaries,null,2));console.log(JSON.stringify(summaries));
  } finally { await browser.close(); }
 }
})().catch(e=>{console.error(e);process.exitCode=1;});
