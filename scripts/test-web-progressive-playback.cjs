const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),{pathToFileURL}=require('node:url');
const pw=require(process.env.PLAYWRIGHT_MODULE||'playwright');
(async()=>{const out=process.argv[3];fs.mkdirSync(out,{recursive:true});const results=[];
for(const engine of (process.env.ENGINES||'chromium,firefox,webkit').split(',')){
 const browser=await pw[engine].launch({headless:true});let timer;try{
  const p=await browser.newPage({viewport:{width:1280,height:800}}),errors=[];p.setDefaultTimeout(180000);p.on('pageerror',e=>errors.push(e.message));await p.route(/^https?:\/\//,r=>r.abort());
  await p.goto(pathToFileURL(path.resolve(process.argv[2])).href);await p.waitForFunction(()=>window.__LAYMESH_WEB?.editor&&document.querySelector('#app').dataset.painted==='true');
  console.log(engine,'loaded');
  let polling=false;timer=setInterval(async()=>{if(polling)return;polling=true;try{const d=await p.evaluate(()=>__LAYMESH_WEB.diagnostics());fs.writeFileSync(path.join(out,engine+'-live.json'),JSON.stringify(d,null,2));console.log(engine,'progress',d.currentEdit?.version,d.pages.map(p=>p.ready?'ready':p.error?'error':'pending').join(','));}catch{}finally{polling=false;}},30000);
  const held=await p.evaluate(async()=>{
   const w=__LAYMESH_WEB,c=w.previewCompiler,compile=c.compile;
   window.__progressive={compile,css:w.editor.files['/project/theme.lcss'],seed:w.editor.files['/project/pages/02-seed.lay']};
   const gate=new Promise(r=>__progressive.release=r);
   c.compile=async(s,...rest)=>{if(s.targetPageId==='editable')await gate;return compile(s,...rest);};
   w.editor.files['/project/theme.lcss']+='\n.heading {color:#c61717 !important;}';w.editor.schedule(true);
   await w.presentation.open({fullscreen:false,startPageId:'cover'});
   if(!w.presentation.ready)throw Error('Start page blocked');
   await c.ensure('cover');
   const result={ready:w.presentation.ready,complete:c.complete,statuses:w.presentation.deck.pages.map(p=>p.status),version:w.presentation.deck.sourceVersion};
   __progressive.release();await c.ensure();await w.presentation.close();c.compile=compile;
   return result;
  });assert(held.ready&&!held.complete);assert.equal(held.statuses[5],'pending');console.log(engine,'held neighbor passed');
  const failed=await p.evaluate(async()=>{
   const w=__LAYMESH_WEB,c=w.previewCompiler;
   w.editor.files['/project/pages/02-seed.lay']=__progressive.seed+'\npage.add(!!!)';w.editor.files['/project/theme.lcss']=__progressive.css+'\n.heading {color:#1769c6 !important;}';w.editor.schedule(true);
   await w.presentation.open({fullscreen:false,startPageId:'cover'});
   let exportRejected=false;try{await c.ensure();}catch{exportRejected=true;}
   const ready=w.presentation.ready;await w.presentation.go(1);
   const result={ready,exportRejected,index:w.presentation.index,pageError:w.presentation.deck.pages[1].status,overlay:!w.presentation.overlay.hidden};
   await w.presentation.close();
   w.editor.files['/project/pages/02-seed.lay']=__progressive.seed;w.editor.schedule(true);
   await w.presentation.open({fullscreen:false,startPageId:'seed'});await c.ensure();
   if(!w.presentation.ready||w.presentation.index!==1)throw Error('Recovery failed');
   await w.presentation.close();return result;
  });assert(failed.ready&&failed.exportRejected&&failed.overlay);assert.equal(failed.index,0);assert.equal(failed.pageError,'error');console.log(engine,'error and recovery passed');
  const cancelled=await p.evaluate(async()=>{
   const w=__LAYMESH_WEB,c=w.previewCompiler;let release,entered;
   const gate=new Promise(r=>release=r),started=new Promise(r=>entered=r),compile=c.compile;
   c.compile=async(s,...rest)=>{entered();await gate;return compile(s,...rest);};
   w.editor.files['/project/pages/01-cover.lay']+='\n# obsolete';w.editor.schedule(true);
   await started;const oldVersion=c.version;
   const wait=c.ensure('cover',{version:oldVersion}).then(()=>false,()=>true);
   w.editor.files['/project/pages/01-cover.lay']+='\n# latest';w.editor.schedule(true);release();
   await c.ensure();c.compile=compile;
   return {oldWaitRejected:await wait,version:c.version,baseline:c.records[0].value.listId,trace:c.trace};
  });assert(cancelled.oldWaitRejected);
  await p.evaluate(async()=>{
   const w=__LAYMESH_WEB,c=w.previewCompiler,compile=c.compile;let release,entered;
   const gate=new Promise(r=>release=r),started=new Promise(r=>entered=r);
   c.compile=async(...args)=>{entered();await gate;return compile(...args);};
   w.editor.files['/project/pages/01-cover.lay']+='\n# close while pending';w.editor.schedule(true);
   const opening=w.presentation.open({fullscreen:false});await started;await w.presentation.close();release();await opening;await c.ensure();c.compile=compile;
   await w.presentation.open({fullscreen:false});if(!w.presentation.ready)throw Error('Close/reopen failed');
  });await p.screenshot({path:path.join(out,engine+'.png')});
  clearInterval(timer);assert.deepEqual(errors,[]);results.push({engine,version:browser.version(),held,failed,cancelled,errors});
  fs.writeFileSync(path.join(out,'results.json'),JSON.stringify(results,null,2));console.log(engine,'blocked future page, error, recovery, cancellation and reopen passed');
 }finally{clearInterval(timer);await browser.close();}
}})().catch(e=>{console.error(e);process.exitCode=1;});
