// Exercise actual downloads of every page in an editable/playback gradient deck.
const fs=require('node:fs'),path=require('node:path'),cp=require('node:child_process'),crypto=require('node:crypto'),assert=require('node:assert/strict');
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const out=path.resolve(process.argv[2]);
const matrixIndex=process.argv.indexOf('--matrix');
if(matrixIndex>=0){
 const matrix=path.resolve(process.argv[matrixIndex+1]),manifest=JSON.parse(fs.readFileSync(path.join(matrix,'manifest.json'),'utf8'));
 fs.mkdirSync(out,{recursive:true});
 const deck='d=document()\n'+manifest.cases.map(c=>`d.add(page(src=${JSON.stringify(path.relative(out,path.join(matrix,c.name+'.lay')))},id=${JSON.stringify(c.name)}))\n`).join('');
 fs.writeFileSync(path.join(out,'deck.laydoc'),deck);
 const engine=path.resolve(process.env.LAYMESH_TEST_BINARY||'target/release/laymesh');
 for(const mode of ['playback','editable'])cp.execFileSync(engine,['render',path.join(out,'deck.laydoc'),'-o',path.join(out,mode+'.zip.html'),...(mode==='editable'?['--html-editable']:[])],{stdio:'pipe'});
 fs.writeFileSync(path.join(out,'build.json'),JSON.stringify({matrix,engine,sha256:crypto.createHash('sha256').update(fs.readFileSync(engine)).digest('hex'),pages:manifest.cases.length},null,2)+'\n');
}
(async()=>{const browser=await chromium.launch({headless:true,args:['--force-color-profile=srgb']});const results=[];try{
 for(const mode of ['playback','editable']){
  const context=await browser.newContext({acceptDownloads:true}),errors=[],requests=[];
  await context.addInitScript(()=>{Object.defineProperty(window,'showSaveFilePicker',{value:undefined});window.gradientWarnings=[];const Original=Worker;window.Worker=class extends Original{constructor(...a){super(...a);this.addEventListener('message',({data})=>{if(data.warnings)gradientWarnings.push(...data.warnings);});}};});
  await context.route(/^https?:\/\//,r=>{requests.push(r.request().url());return r.abort();});
  const page=await context.newPage();page.setDefaultTimeout(180000);page.on('pageerror',e=>errors.push(e.message));
  await page.goto('file://'+path.join(out,mode+'.zip.html'));await page.waitForFunction(()=>document.querySelector('#app')?.dataset.painted==='true');
  await page.locator('#export').click();await page.locator('#export-settings').click();await page.locator('#export-dpi').fill('96');await page.locator('#export-settings-panel button[type=submit]').click();await page.locator('#export-panel .dialog-close').click();
  for(const format of ['svg','pdf']){
   await page.locator('#export').click();await page.locator('#export-format').selectOption(format);await page.locator('#export-scope').selectOption('all');
   const download=page.waitForEvent('download');await page.locator('#export-file').click();
   const file=await download;await file.saveAs(path.join(out,mode+(format==='svg'?'.svg.zip':'.pdf')));await page.waitForFunction(()=>!document.querySelector('#export-file').disabled);console.log(mode,format);
  }
  const warnings=await page.evaluate(()=>gradientWarnings);assert.deepEqual(errors,[]);assert.deepEqual(requests,[]);assert.deepEqual(warnings,[]);
  results.push({mode,browser:browser.version(),dpi:96,errors,requests,warnings});await context.close();
 }
 fs.writeFileSync(path.join(out,'results.json'),JSON.stringify(results,null,2)+'\n');
}finally{await browser.close();}})().catch(e=>{console.error(e);process.exitCode=1;});
