// Export actual offline packages through their user-visible dialog.
const assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path');
const {pathToFileURL}=require('node:url');const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
(async()=>{const dir=path.resolve(process.argv[2]||'target/media-mainline/browser'),out=path.resolve(process.argv[3]||'target/media-mainline/pptx-web');fs.mkdirSync(out,{recursive:true});const b=await chromium.launch({headless:true,args:['--no-sandbox']});try{const page=await b.newPage({acceptDownloads:true});await page.addInitScript(()=>{window.showSaveFilePicker=undefined;});const errors=[];page.on('pageerror',e=>errors.push(e.message));
 for(const [input,name] of [['powerpoint.zip.html','playback'],['powerpoint-editable.zip.html','editable']]){
  await page.goto(pathToFileURL(path.join(dir,input)).href);await page.waitForFunction(()=>window.__LAYMESH_WEB?.pages.length===2&&window.__LAYMESH_WEB.preview.presented);
  if(name==='editable'){
   await page.waitForFunction(()=>window.__LAYMESH_WEB?.editor&&window.__LAYMESH_WEB.workspace);
   const before=await page.evaluate(async()=>{const w=window.__LAYMESH_WEB,id=w.pages[0].id,file=Object.keys(w.editor.files).find(p=>p.endsWith('/powerpoint.lay'));await w.workspace.open(file);const view=w.editor.views.get(file).view;view.dispatch({changes:{from:0,to:view.state.doc.length,insert:view.state.doc.toString().replace('#eef1f5','#eef2f5')}});return id;});
   await page.waitForFunction(id=>window.__LAYMESH_WEB.pages[0].id!==id&&document.querySelector('#app').dataset.documentState==='ready',before);
  }
  await page.locator('#export').click();await page.selectOption('#export-format','pptx');assert.equal(await page.locator('#export-scope').inputValue(),'all');assert.equal(await page.locator('#export-extension').textContent(),'.pptx');
  const download=page.waitForEvent('download',{timeout:60000});await page.locator('#export-file').click();await(await download).saveAs(path.join(out,name+'.pptx'));await page.waitForFunction(()=>!document.querySelector('#export-cancel').offsetParent);
 }
 assert.deepEqual(errors,[]);fs.writeFileSync(path.join(out,'results.json'),JSON.stringify({passed:true,checks:['playback HTML native media PPTX','editable HTML native media PPTX','one deck for all pages']}));console.log('Offline playback/editable HTML exported one multi-page media PPTX each');
 }finally{await b.close();}})().catch(e=>{console.error(e);process.exitCode=1;});
