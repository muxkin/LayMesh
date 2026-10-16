// Compare retained edits with a fresh compiler worker for each of six pages.
// Usage: PLAYWRIGHT_MODULE=/path/to/playwright node scripts/test-web-semantic-edit-oracle.cjs HTML OUTPUT
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto'),assert=require('node:assert/strict');
const {pathToFileURL}=require('node:url'),pw=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const html=path.resolve(process.argv[2]),out=path.resolve(process.argv[3]);fs.mkdirSync(out,{recursive:true});
const styleEdits=process.argv[4]==='styles',styleCases=require('./semantic-document-cases.cjs');
const hash=b=>crypto.createHash('sha256').update(b).digest('hex');
// Retaining local paths instead of applying a world transform and its inverse
// can change the last few f64 bits. The bitmap comparison below remains exact.
const compareDrawing=(a,b,label)=>{
 const drift={count:0,maxAbsolute:0};
 const visit=(a,b,p)=>{
  if(typeof a==='number'&&typeof b==='number'){
   if(a!==b){const d=Math.abs(a-b);assert(Number.isFinite(d)&&d<=1e-10,`${label} ${p}: ${a} != ${b}`);drift.count++;drift.maxAbsolute=Math.max(drift.maxAbsolute,d);}return;
  }
  if(Array.isArray(a)){assert(Array.isArray(b),label+' '+p);assert.equal(a.length,b.length,label+' '+p);a.forEach((v,i)=>visit(v,b[i],p+'/'+i));return;}
  if(a&&typeof a==='object'){assert(b&&typeof b==='object',label+' '+p);assert.deepEqual(Object.keys(a).sort(),Object.keys(b).sort(),label+' '+p);for(const k of Object.keys(a))visit(a[k],b[k],p+'/'+k);return;}
  assert.equal(a,b,label+' '+p);
 };visit(a,b,'');return drift;
};
const ready=p=>p.waitForFunction(()=>window.__LAYMESH_WEB?.editor&&document.querySelector('#app').dataset.painted==='true');
const select=async(p,index,name)=>{await p.locator('#page').selectOption(String(index));await p.evaluate(name=>__LAYMESH_WEB.workspace.open(name),name);await p.waitForFunction(index=>__LAYMESH_WEB.metrics.pageId===__LAYMESH_WEB.pages[index].id&&document.querySelector('#app').dataset.painted==='true',index);};
const edit=async(p,name,source)=>{
 const previous=await p.evaluate(({name,source})=>{const w=__LAYMESH_WEB,v=w.editor.views.get(name).view,id=w.metrics.pageId;v.dispatch({changes:{from:0,to:v.state.doc.length,insert:source}});return id;},{name,source});
 await p.waitForFunction(id=>{const a=document.querySelector('#app');return a.dataset.compileState==='ready'&&a.dataset.painted==='true'&&__LAYMESH_WEB.metrics.pageId!==id;},previous);
};
// Read the actual visible canvas bitmap at readiness. A separately computed
// screenshot clip can become stale when a ResizeObserver moves the viewport.
const capture=async p=>Buffer.from((await p.evaluate(()=>{const source=__LAYMESH_WEB.preview.canvas,c=document.createElement('canvas');c.width=source.width;c.height=source.height;c.getContext('2d').drawImage(source,0,0);return c.toDataURL('image/png');})).split(',')[1],'base64');
(async()=>{
 const results=[];
 const selectedPages=(process.env.LAYMESH_TEST_PAGES||(styleEdits?'6':'1,2,3,4,5,6')).split(',');
 // Bound oracle-only memory: each comparison holds two independent WASM
 // runtimes plus complete command snapshots. Restart after two source pages.
 for(const engine of (process.env.LAYMESH_TEST_ENGINES||'chromium,firefox,webkit').split(','))for(let batch=0;batch<selectedPages.length;batch+=2){
  const selected=selectedPages.slice(batch,batch+2);
  const browser=await pw[engine].launch({headless:true});
  try{
   const context=await browser.newContext({viewport:{width:1600,height:1000}}),errors=[],network=[];
   await context.route(/^https?:\/\//,r=>{network.push(r.request().url());return r.abort();});
   context.on('page',p=>{p.setDefaultTimeout(180000);p.on('pageerror',e=>errors.push(e.message));});
   const retained=await context.newPage();await retained.goto(pathToFileURL(html).href);await ready(retained);
   await retained.evaluate(()=>__LAYMESH_WEB.previewCompiler.invalidate());await retained.waitForFunction(()=>document.querySelector('#app').dataset.documentState==='ready');
   const names=await retained.evaluate(()=>Object.keys(__LAYMESH_WEB.editor.files).filter(p=>/\/pages\/\d.*\.lay$/.test(p)).sort());assert.equal(names.length,6);
   for(const [index,pageName]of names.entries()){
    if(!selected.includes(String(index+1)))continue;
    const name=styleEdits?'/project/theme.lcss':pageName;
    await select(retained,index,name);const original=await retained.evaluate(name=>__LAYMESH_WEB.editor.files[name],name);
    const afterCanvas=original.indexOf('\n',original.indexOf('page=canvas('));if(!styleEdits)assert(afterCanvas>0);
    const changes=styleEdits?Object.fromEntries(['lcss-heading','lcss-curves'].map(kind=>[kind,styleCases.change(kind,7,original,'').source])):{
     color:original.replace(/background="#[0-9a-fA-F]+"/,'background="#f9f6e8"'),
     text:/chrome\(page,/.test(original)?original.replace(/chrome\(page,("[^"\n]*"),("[^"\n]*")/,(_,n,t)=>`chrome(page,${n},${JSON.stringify(JSON.parse(t)+'·')}`):original.replace(/text\(("[^"\n]*")/,(_,t)=>`text(${JSON.stringify(JSON.parse(t)+'·')}`),
     size:original.slice(0,afterCanvas)+original.slice(afterCanvas).replace(/size=\(([-\d.]+)mm,/,(_,n)=>`size=(${Number(n)+1}mm,`),
    };
    for(const [kind,source]of Object.entries(changes)){
     assert.notEqual(source,original,'Missing edit: '+name+' '+kind);await edit(retained,name,source);
     assert.equal(await retained.evaluate(()=>document.querySelector('#app').dataset.running),'false','Static source preview must not advance the animation clock');
     const a=await capture(retained),cold=await context.newPage();
     try{
      await cold.goto(pathToFileURL(html).href);await ready(cold);await select(cold,index,name);await edit(cold,name,source);
      const b=await capture(cold),label=engine+'-'+path.basename(name,'.lay')+'-'+kind;
      const lists=await Promise.all([retained,cold].map(p=>p.evaluate(index=>JSON.parse(JSON.stringify(__LAYMESH_WEB.pages[index].list,(key,value)=>key==='_identity'?undefined:value)),index)));
      for(const list of lists)list.pageTransition??=null; // Optional serde field versus explicit patch clear.
      let numericDrift;
      try{numericDrift=compareDrawing(lists[0],lists[1],label);}catch(error){
       for(const [i,tag]of ['retained','fresh'].entries())fs.writeFileSync(path.join(out,label+'-'+tag+'-list.json'),JSON.stringify(lists[i]));
       fs.writeFileSync(path.join(out,label+'-metrics.json'),JSON.stringify(await retained.evaluate(index=>({metrics:__LAYMESH_WEB.pages[index].compileMetrics,warnings:__LAYMESH_WEB.pages[index].warnings}),index),null,2));throw error;
      }
      let same=a.equals(b),unchangedRedraw=false;
      if(!same){
       // Skia CPU's first draw can differ from its cached draw even without
       // an edit. Keep the immediate retained screenshot; redraw ONLY the
       // fresh oracle, with exactly the same source and drawing commands.
       await cold.evaluate(()=>document.querySelector('#page').dispatchEvent(new Event('change')));
       await cold.waitForFunction(()=>document.querySelector('#app').dataset.painted==='true');
       const repeated=await capture(cold);unchangedRedraw=a.equals(repeated);same=unchangedRedraw;
       if(unchangedRedraw){fs.writeFileSync(path.join(out,label+'-fresh-first.png'),b);fs.writeFileSync(path.join(out,label+'-fresh-unchanged-redraw.png'),repeated);}
      }
      const presentationConfirmed=await cold.evaluate(()=>__LAYMESH_WEB.metrics.presentationConfirmed);assert.equal(presentationConfirmed,true,'Expected visible presentation completion');
      if(!same){fs.writeFileSync(path.join(out,label+'-fresh-state.json'),JSON.stringify(await cold.evaluate(()=>({state:{...document.querySelector('#app').dataset},metrics:__LAYMESH_WEB.metrics,rect:__LAYMESH_WEB.preview.canvas.getBoundingClientRect().toJSON()})),null,2));for(const [tag,page]of [['retained',retained],['fresh',cold]])fs.writeFileSync(path.join(out,label+'-'+tag+'-list.json'),JSON.stringify(await page.evaluate(index=>__LAYMESH_WEB.pages[index].list,index)));fs.writeFileSync(path.join(out,label+'-retained.png'),a);fs.writeFileSync(path.join(out,label+'-fresh.png'),b);fs.writeFileSync(path.join(out,label+'-state.json'),JSON.stringify(await retained.evaluate(name=>({state:{...document.querySelector('#app').dataset},source:__LAYMESH_WEB.editor.files[name],metrics:__LAYMESH_WEB.metrics,background:__LAYMESH_WEB.pages[0].list.nodes[0]}),name),null,2));fs.writeFileSync(path.join(out,label+'-later.png'),await capture(retained));}
      assert(same,'Retained pixels differ from fresh evaluation: '+label);
      results.push({engine,version:browser.version(),page:path.basename(name),kind,sha256:hash(a),freshCompilerPixelsEqual:true,drawingCommandsEquivalent:true,numericDrift,unchangedRedraw,presentationConfirmed});
      fs.writeFileSync(path.join(out,'verification.json'),JSON.stringify({html,htmlSha256:hash(fs.readFileSync(html)),results},null,2)+'\n');
      console.log(label+' pixels match');
     }finally{await cold.close();}
    }
    await edit(retained,name,original);
   }
   assert.deepEqual(errors,[]);assert.deepEqual(network,[]);
  }finally{await browser.close();}
 }
})().catch(e=>{console.error(e);process.exitCode=1;});
