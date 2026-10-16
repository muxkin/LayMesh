const fs=require('node:fs'),crypto=require('node:crypto'),assert=require('node:assert/strict');
// Usage: PLAYWRIGHT_MODULE=/path/to/playwright node scripts/test-web-semantic-roundtrip.cjs ORIGINAL OPTIMIZED OUTPUT
const path=require('node:path'),{pathToFileURL}=require('node:url');
const pw=require(process.env.PLAYWRIGHT_MODULE||'playwright');
if(process.argv.length!==5)throw Error('Expected ORIGINAL OPTIMIZED OUTPUT');
const out=path.resolve(process.argv[4]);fs.mkdirSync(out,{recursive:true});
const files={original:path.resolve(process.argv[2]),optimized:path.resolve(process.argv[3])};
const hash=b=>crypto.createHash('sha256').update(b).digest('hex');
(async()=>{const results=[];for(const engine of (process.env.ENGINES||'chromium,firefox,webkit').split(',')){let reference;
for(const [kind,file]of Object.entries(files)){const browser=await pw[engine].launch({headless:true});try{
const ctx=await browser.newContext({viewport:{width:1280,height:800},acceptDownloads:true});const errors=[],requests=[];await ctx.route(/^https?:\/\//,r=>{requests.push(r.request().url());return r.abort();});const p=await ctx.newPage();p.setDefaultTimeout(120000);p.on('pageerror',e=>errors.push(e.message));await p.goto(pathToFileURL(file).href);await p.waitForFunction(()=>window.__LAYMESH_WEB?.editor&&document.querySelector('#app').dataset.painted==='true');
await p.evaluate(()=>{const w=__LAYMESH_WEB;w.editor.files['/project/theme.lcss']+='\n/* visual verification */';w.editor.schedule(true);});
await p.waitForFunction(()=>{const a=document.querySelector('#app');return a.dataset.documentState?a.dataset.documentState==='ready':a.dataset.compileState==='ready';});
const lists=await p.evaluate(()=>__LAYMESH_WEB.pages.map(p=>JSON.stringify(p.list,(key,value)=>key==='_identity'?undefined:value)));fs.writeFileSync(out+'/'+engine+'-'+kind+'-lists.json',JSON.stringify(lists.map(JSON.parse)));const canonical=v=>Array.isArray(v)?v.map(canonical):v&&typeof v==='object'?Object.fromEntries(Object.keys(v).sort().map(k=>[k,canonical(v[k])])):v;const listHashes=lists.map(s=>hash(JSON.stringify(canonical(JSON.parse(s)))));await p.locator('#page').selectOption('3');await p.waitForFunction(()=>document.querySelector('#app').dataset.painted==='true'&&__LAYMESH_WEB.metrics.pageId===__LAYMESH_WEB.pages[3].id);
await p.evaluate(()=>__LAYMESH_WEB.presentation.open({fullscreen:false}));await p.waitForFunction(()=>__LAYMESH_WEB.presentation.ready&&!__LAYMESH_WEB.presentation.switching);
const animation=[];
for(let index=0;index<lists.length;index++){
 await p.evaluate(async index=>{const pr=__LAYMESH_WEB.presentation;pr.clock.running=false;await pr.go(index,{time:0});pr.clock.running=false;},index);
 for(const time of [0,3,6.5]){
  const bytes=await p.evaluate(async({index,time})=>{const pr=__LAYMESH_WEB.presentation;pr.clock.running=false;pr.clock.seek(time);await pr.call({type:'paint',serial:pr.serial,pageId:pr.deck.pages[index].id,time,reduced:false,running:false,...pr.dimensions()});const r=await pr.call({type:'capture'});return Array.from(new Uint8Array(await crypto.subtle.digest("SHA-256",r.rgba))).map(b=>b.toString(16).padStart(2,"0")).join("");},{index,time});
  animation.push({page:index+1,time,sha256:bytes});
 }
}
if(kind==='original')reference={listHashes,animation};else{assert.deepEqual(listHashes,reference.listHashes);assert.deepEqual(animation,reference.animation);await p.screenshot({path:out+'/'+engine+'-animation.png'});}
await p.evaluate(()=>__LAYMESH_WEB.presentation.close());await p.waitForFunction(()=>!__LAYMESH_WEB.presentation.opened);
if(kind==='optimized'){
await p.evaluate(()=>{const w=__LAYMESH_WEB;w.editor.files['/project/theme.lcss']+='\n/* round trip during pending compilation */';w.editor.schedule();});
await p.locator('#export').click();const wait=p.waitForEvent('download');await p.locator('#save').click();const saved=out+'/'+engine+'-real-saved.zip.html';await(await wait).saveAs(saved);
await p.goto(pathToFileURL(saved).href);await p.waitForFunction(()=>window.__LAYMESH_WEB?.editor&&document.querySelector('#app').dataset.painted==='true');assert.equal(await p.evaluate(()=>__LAYMESH_WEB.pages.length),6);assert.equal(await p.evaluate(()=>__LAYMESH_WEB.previewCompiler.records.filter(r=>r.dependencies).length),6,'Saved archive must restore all dependency indexes');assert(await p.evaluate(()=>__LAYMESH_WEB.editor.files['/project/theme.lcss'].includes('round trip during pending compilation')));
await p.locator('#export').click();const playWait=p.waitForEvent('download');await p.locator('#download').click();const play=out+'/'+engine+'-real-playback.zip.html';await(await playWait).saveAs(play);await p.goto(pathToFileURL(play).href);await p.waitForFunction(()=>window.__LAYMESH_WEB&&document.querySelector('#app').dataset.painted==='true');assert.equal(await p.evaluate(()=>__LAYMESH_WEB.pages.length),6);assert.equal(await p.evaluate(()=>!!__LAYMESH_WEB.editor),false);
}
assert.deepEqual(errors,[]);assert.deepEqual(requests,[]);results.push({engine,version:browser.version(),kind,htmlSha256:hash(fs.readFileSync(file)),listHashes,animationPages:lists.length,animationSamplesPerPage:3,animation,errors,requests});console.log(engine,kind,'visual and animation checks passed');fs.writeFileSync(out+'/visual-verification.json',JSON.stringify(results,null,2));
}finally{await browser.close();}}}})().catch(e=>{console.error(e);process.exitCode=1});
