// Synthetic FontData catalogs + real font-manager/picker DOM. No OS timing claim.
const fs=require('fs'),path=require('path'),http=require('http'),assert=require('assert/strict');
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const root=path.resolve(__dirname,'..'),out=process.argv[2]||'/tmp/laymesh-font-performance',host=process.env.LAYMESH_TEST_HOST||'100.64.0.6';fs.mkdirSync(out,{recursive:true});
const server=http.createServer((req,res)=>{const name=path.basename(new URL(req.url,'http://local').pathname);if(name.endsWith('.mjs')||name.endsWith('.css')){res.setHeader('Content-Type',name.endsWith('.mjs')?'text/javascript':'text/css');res.end(fs.readFileSync(root+'/site/live/'+name));}else if(name==='font.ttf'){res.end(fs.readFileSync(root+'/tests/fonts/DejaVuSans.ttf'));}else{res.setHeader('Content-Type','text/html; charset=utf-8');res.end('<!doctype html><link rel="stylesheet" href="/web-player.css"><div id="app"></div>');}});
(async()=>{await new Promise(r=>server.listen(0,host,r));const browser=await chromium.launch({headless:true});try{const page=await browser.newPage(),base=`http://${host}:${server.address().port}`;await page.goto(base);const results=[];
for(const count of [1005,5000])results.push(await page.evaluate(async count=>{
 const {buildShell}=await import('/web-shell.mjs'),{createFontManager}=await import('/web-fonts.mjs');
 const root=document.querySelector('#app');buildShell(root,true);let reads=0,compiles=0,installs=0;
 const family={family:'Microsoft YaHei',postscriptName:'MicrosoftYaHei',style:'Regular'},localFonts=Array.from({length:count-1},(_,i)=>({family:'Font '+String(i).padStart(5,'0'),postscriptName:'Font'+i,style:'Regular',blob:async()=>{reads++;throw Error('Unexpected unrelated font read');}}));
 const bytes=await(await fetch('/font.ttf')).arrayBuffer();localFonts.push({...family,blob:async()=>{reads++;return new Blob([bytes]);}});
 let enumerationMs=0;window.queryLocalFonts=async()=>{const start=performance.now();const f=localFonts.slice();enumerationMs+=performance.now()-start;return f;};
 const manifest={requestedFonts:[{family:'Missing Font'}],fontRequirements:[]};const editor={files:{'/project/a.lay':'p=canvas()'},assets:{}};
 const manager=createFontManager({root,manifest,files:{},editor,compilerCall:async()=>{installs++;return {faces:[{...family,weight:400,italic:false}]};},resetCompiler(){},compileNow:async()=>{compiles++;}});
 root.querySelector('#fonts').click();let start=performance.now();await root.querySelector('#connect-fonts').onclick();const connectionMs=performance.now()-start;
 if(reads)throw Error('Catalog connection read font bytes');
 const input=root.querySelector('.font-row input'),search=[],selection=[];for(let i=0;i<80;i++){
  input.value=i%2?'微软雅黑':'Font '+String(i).padStart(5,'0');start=performance.now();input.dispatchEvent(new Event('input'));search.push(performance.now()-start);
  start=performance.now();input.dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',bubbles:true}));selection.push(performance.now()-start);
 }
 if(!input.value.includes('微软雅黑'))throw Error('Dictionary Chinese lookup failed');
 input.click();const list=document.getElementById(input.getAttribute('aria-controls'));list.scrollTop=1000;list.dispatchEvent(new Event('scroll'));const top=list.scrollTop;input.dispatchEvent(new CompositionEvent('compositionstart'));input.value='字';input.dispatchEvent(new Event('input'));if(list.scrollTop!==top)throw Error('IME moved list');input.dispatchEvent(new CompositionEvent('compositionend'));
 root.querySelector('#font-later').click();root.querySelector('#fonts').click();await root.querySelector('#connect-fonts').onclick();if(reads)throw Error('Reconnect or selection read unrelated font');
 manager.dispose();root.querySelector('#font-panel').close();
 // Same-family matching and read dedup are exercised separately from catalog timings.
 buildShell(root,true);manifest.requestedFonts=[{family:'微软雅黑',weight:400}];const manager2=createFontManager({root,manifest,files:{},editor,compilerCall:async()=>({faces:[{...family,weight:400,italic:false}]}),resetCompiler(){},compileNow:async()=>{compiles++;}});
 start=performance.now();await root.querySelector('#connect-fonts').onclick();const requiredLoadMs=performance.now()-start;await manager2.assertFonts();await root.querySelector('#connect-fonts').onclick();if(reads!==1)throw Error('Needed font should be read once, got '+reads);if(Object.keys(manager2.bindings).length)throw Error('Same-font resolution created a replacement');if(manager2.compilerBindings()['微软雅黑']!=='Microsoft YaHei')throw Error('Compiler alias absent');manager2.dispose();
 const p95=a=>a.sort((a,b)=>a-b)[Math.ceil(a.length*.95)-1];return {count,connectionMs,syntheticEnumerationMs:enumerationMs,requiredLoadMs,unrelatedFontBlobCalls:0,requiredFontBlobCalls:reads,searchP95Ms:p95(search),selectionP95Ms:p95(selection),automaticCompiles:compiles};
},count));
for(const r of results){assert(r.searchP95Ms<50);assert(r.selectionP95Ms<50);console.log(JSON.stringify(r));}fs.writeFileSync(out+'/performance.json',JSON.stringify({environment:'Headless Chromium; synthetic FontData enumeration; real font manager, picker DOM and loaded SFNT parsing. OS enumeration and FontData.blob latency are not measured.',results},null,2));
}finally{await browser.close();server.close();}})().catch(e=>{console.error(e);server.close();process.exitCode=1;});
