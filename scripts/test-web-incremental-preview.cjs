// Real offline packages: dependency invalidation, latest-input scheduling and
// progressive playback and complete-document save barriers.
const fs = require('node:fs'), path = require('node:path'), os = require('node:os');
const assert = require('node:assert/strict'), cp = require('node:child_process');
const engines = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const root = path.resolve(__dirname, '..');
const out = path.resolve(process.argv[2] || path.join(os.tmpdir(), 'laymesh-incremental-preview'));
fs.mkdirSync(out, { recursive: true });
(async () => {
  const archive = await import(path.join(root, 'site/live/web-archive.mjs'));
  const decode = b => new TextDecoder().decode(b), encode = s => new TextEncoder().encode(s);
  const unpack = file => archive.webUnzip(archive.webPayload(fs.readFileSync(file)));
  fs.writeFileSync(path.join(out, 'shared.lay'), 'function tile() { return rect(size=(12mm,9mm),fill="#0088aa",class="accent") }\nexport tile=tile\n');
  fs.writeFileSync(path.join(out, 'theme.lcss'), '.accent { border-color: #e04020; border-width: 1mm; }\n');
  const image = '<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10" fill="#ff0000"/></svg>';
  fs.writeFileSync(path.join(out, 'pixel.svg'), image);
  for (const id of ['a', 'b', 'c']) fs.writeFileSync(path.join(out, id+'.lay'), 'import { tile } from "shared.lay"\np=canvas(size=(60mm,40mm),background="#ffffff",stylesheet="theme.lcss")\np.add(tile(),offset=(8mm,8mm))\n');
  fs.appendFileSync(path.join(out, 'a.lay'), 'p.add(image(src="pixel.svg"),size=(3mm,3mm),offset=(40mm,30mm))\n');
  const doc = 'd=document(name="Incremental preview")\n'+['a','b','c'].map(id=>'d.add(page(src="'+id+'.lay",id="'+id+'",transition=transition(name="fade",duration=300ms)))').join('\n')+'\n';
  fs.writeFileSync(path.join(out, 'deck.laydoc'), doc);
  const file = path.join(out, 'editable.zip.html');
  cp.execFileSync(process.env.LAYMESH_TEST_BINARY || path.join(root,'target/release/laymesh'), ['render',path.join(out,'deck.laydoc'),'-o',file,'--html-editable']);
  // Test the actual exporter. Replacing its runtime here can silently invalidate
  // the dependency index and hide a first-edit packaging regression.
  const packed = unpack(file), runtime = archive.webUnzip(fs.readFileSync(path.join(root,'target/web-export/runtime.zip')));
  assert(Buffer.from(packed['runtime/compiler/laymesh.wasm']).equals(Buffer.from(runtime['runtime/compiler/laymesh.wasm'])), 'Rebuild the native CLI with the current runtime before testing');
  const results = [];
  for (const engine of process.env.LAYMESH_TEST_ENGINES?.split(',') || ['chromium','firefox','webkit']) {
    const browser = await engines[engine].launch({ headless:true });
    try {
      const context = await browser.newContext({ viewport:{width:1440,height:900},acceptDownloads:true });
      const errors = [], requests = [];
      await context.route(/^https?:\/\//, r => {requests.push(r.request().url());return r.abort();});
      const p = await context.newPage(); p.setDefaultTimeout(90000);
      p.on('pageerror', e => errors.push(e.message));
      await p.goto('file://'+file);
      await p.waitForFunction(() => window.__LAYMESH_WEB?.editor && document.querySelector('#app').dataset.painted==='true');
      assert.equal(await p.title(),'Incremental preview');
      const done = async () => {try{await p.waitForFunction(() => document.querySelector('#app').dataset.documentState==='ready',null,{timeout:30000});}catch(error){error.message+='\n'+JSON.stringify(await p.evaluate(()=>({state:{...document.querySelector('#app').dataset},records:__LAYMESH_WEB.previewCompiler.records.map(r=>({key:r.key,valid:r.valid,error:String(r.error||'')}))})));throw error;}};
      const edit = async (file, source) => p.evaluate(async ({file,source}) => {
        const w=__LAYMESH_WEB; await w.workspace.open('/project/'+file);
        const v=w.editor.views.get('/project/'+file).view;
        v.dispatch({changes:{from:0,to:v.state.doc.length,insert:source}});
      }, {file,source});
      const source = file => p.evaluate(file => __LAYMESH_WEB.editor.files['/project/'+file],file);
      assert.equal(await p.evaluate(()=>__LAYMESH_WEB.previewCompiler.records.filter(r=>r.dependencies).length),3,'The exported archive must restore all dependency indexes before its first edit');
      await p.evaluate(() => {const c=__LAYMESH_WEB.previewCompiler,compile=c.compile.bind(c);window.compileRequests=[];c.compile=async (data,...rest)=>{compileRequests.push({page:data.targetPageId,version:data.version});const r=await compile(data,...rest);if(window.holdResult){window.holdResult=false;await new Promise(resolve=>window.releaseResult=resolve);}return r;};});
      const a = await source('a.lay'), b = await source('b.lay'), shared = await source('shared.lay'), theme = await source('theme.lcss');
      // The completion event must follow the OffscreenCanvas rendering step,
      // not just GPU submission. Read an actual screenshot pixel immediately
      // after readiness, without an extra sleep that would hide stale pixels.
      for(const color of ['#e7c4a2','#a2c4e7','#ffffff']){
        await edit('a.lay',a.replace('#ffffff',color));await done();
        const pixel=await p.evaluate(()=>{const c=document.createElement('canvas');c.width=c.height=1;const x=c.getContext('2d');x.drawImage(__LAYMESH_WEB.preview.canvas,5,5,1,1,0,0,1,1);return [...x.getImageData(0,0,1,1).data];});
        assert.deepEqual(pixel,[...[1,3,5].map(i=>parseInt(color.slice(i,i+2),16)),255],'Ready placeholder bitmap is blank or stale');
        const clip=await p.evaluate(()=>{const r=__LAYMESH_WEB.preview.canvas.getBoundingClientRect();return{x:Math.ceil(r.x)+5,y:Math.ceil(r.y)+5,width:1,height:1};});
        const png=await p.screenshot({clip}),chunks=[];
        assert([2,6].includes(png[25]),'Expected RGB or RGBA screenshot');assert.equal(png[24],8);
        for(let at=8;at<png.length;){const n=png.readUInt32BE(at);if(png.toString('ascii',at+4,at+8)==='IDAT')chunks.push(png.subarray(at+8,at+8+n));at+=n+12;}
        const scan=require('node:zlib').inflateSync(Buffer.concat(chunks));
        assert.deepEqual([...scan.subarray(1,4)],[1,3,5].map(i=>parseInt(color.slice(i,i+2),16)),'Ready preview still shows the previous background');
      }
      assert.deepEqual(await p.evaluate(()=>compileRequests.map(r=>r.page)),['a','a','a'],'Cold and warm local edits must not compile unrelated pages');
      const ids = await p.evaluate(() => __LAYMESH_WEB.pages.map(p=>p.id));
      await edit('b.lay',b.replace('8mm,8mm','10mm,8mm')); await done();
      const ids2=await p.evaluate(()=>__LAYMESH_WEB.pages.map(p=>p.id));
      assert.equal(ids[0],ids2[0]);assert.notEqual(ids[1],ids2[1]);assert.equal(ids[2],ids2[2]);
      await p.locator('#import').setInputFiles({name:'pixel.svg',mimeType:'image/svg+xml',buffer:Buffer.from(image.replace('#ff0000','#00ff00'))});
      await p.getByRole('button',{name:'替换',exact:true}).click();
      await p.evaluate(()=>__LAYMESH_WEB.workspace.whenIdle());await done();
      const imageIDs=await p.evaluate(()=>__LAYMESH_WEB.pages.map(p=>p.id));
      assert.notEqual(imageIDs[0],ids2[0]);assert.equal(imageIDs[1],ids2[1]);assert.equal(imageIDs[2],ids2[2]);
      await p.evaluate(()=>compileRequests.length=0);
      await edit('theme.lcss',theme.replace('#e04020','#20a040'));
      await p.locator('#page').selectOption('2');await done();
      assert.equal(await p.evaluate(()=>compileRequests[0].page),'c');
      assert.equal(await p.evaluate(()=>compileRequests.length),3);
      await p.evaluate(()=>{compileRequests.length=0;window.holdResult=true;});
      await edit('shared.lay',shared.replace('#0088aa','#ff0000'));
      await p.waitForFunction(()=>!!window.releaseResult);
      await edit('shared.lay',shared.replace('#0088aa','#00ff00'));
      await edit('shared.lay',shared.replace('#0088aa','#0000ff'));
      await p.locator('#page').selectOption('0');
      await p.evaluate(()=>{window.releaseResult();window.releaseResult=null;});await done();
      assert.equal(await p.evaluate(()=>compileRequests.length),4);
      assert.equal(await p.evaluate(()=>compileRequests[1].page),'a');
      assert(await p.evaluate(()=>__LAYMESH_WEB.pages.every(p=>JSON.stringify(p.list).includes('[0,0,1,1]'))));
      // Interrupt synchronous WASM at semantic checkpoints. The old page must
      // never commit after the newer source has replaced it.
      const beforeCancel=await p.evaluate(()=>__LAYMESH_WEB.pages[0].id);
      await edit('a.lay',a+'\nfor i in range(10000) { for j in range(3) { z=i*i+i*j+j*j } }\n');
      await p.waitForFunction(()=>__LAYMESH_WEB.previewCompiler.running);
      await p.waitForTimeout(80);
      await edit('a.lay',a+'\n# latest after cooperative cancellation\n');await done();
      assert.notEqual(await p.evaluate(()=>__LAYMESH_WEB.pages[0].id),beforeCancel);
      assert.equal(await p.evaluate(()=>__LAYMESH_WEB.editor.files['/project/a.lay'].includes('range(10000)')),false);
      // Document planning runs arbitrary Lay control flow too, so it shares
      // the same cooperative cancellation boundary as page evaluation.
      await edit('deck.laydoc',doc+'\nfor i in range(10000) { for j in range(3) { z=i*i+i*j+j*j } }\n');
      await p.waitForFunction(()=>__LAYMESH_WEB.previewCompiler.running);
      await p.waitForTimeout(80);
      await edit('deck.laydoc',doc+'\n# latest document after cancellation\n');await done();
      assert.equal(await p.evaluate(()=>__LAYMESH_WEB.editor.files['/project/deck.laydoc'].includes('range(10000)')),false);
      const beforeRename=await p.evaluate(()=>__LAYMESH_WEB.pages.map(p=>p.id));
      await edit('deck.laydoc',doc.replace('Incremental preview','Renamed incremental preview'));await done();
      assert.equal(await p.title(),'Renamed incremental preview');
      assert.equal(await p.locator('.project-name').textContent(),'Renamed incremental preview');
      assert.deepEqual(await p.evaluate(()=>__LAYMESH_WEB.pages.map(p=>p.id)),beforeRename,'document metadata must retain page baselines');
      const reordered=doc.split('\n');[reordered[1],reordered[2]]=[reordered[2],reordered[1]];
      await edit('deck.laydoc',reordered.join('\n'));await done();
      assert.deepEqual(await p.evaluate(()=>__LAYMESH_WEB.pages.map(p=>p.id)),[beforeRename[1],beforeRename[0],beforeRename[2]],'page reordering must retain accepted drawings');
      await edit('deck.laydoc',doc);await done();
      const beforeIME = await p.evaluate(()=>compileRequests.length);
      await p.evaluate(async()=>{const w=__LAYMESH_WEB;await w.workspace.open('/project/a.lay');const v=w.editor.views.get('/project/a.lay').view;v.contentDOM.dispatchEvent(new CompositionEvent('compositionstart',{bubbles:true}));v.dispatch({changes:{from:v.state.doc.length,insert:'\n# 中文输入\n'}});});
      await p.waitForTimeout(400);assert.equal(await p.evaluate(()=>compileRequests.length),beforeIME);
      await p.evaluate(()=>__LAYMESH_WEB.editor.views.get('/project/a.lay').view.contentDOM.dispatchEvent(new CompositionEvent('compositionend',{bubbles:true})));await done();
      const goodID=await p.evaluate(()=>__LAYMESH_WEB.pages[0].id);
      await edit('a.lay',a+'\nx=(\n');
      await p.waitForFunction(()=>document.querySelector('#app').dataset.documentState==='error');
      assert.equal(await p.evaluate(()=>__LAYMESH_WEB.pages[0].id),goodID);
      assert(await p.evaluate(async()=>{try{await __LAYMESH_WEB.previewCompiler.ensure();return false;}catch{return true;}}));
      await edit('a.lay',a+'\nx=E_YIELD\n');
      await p.waitForFunction(()=>document.querySelector('#app').dataset.documentState==='error');
      assert.equal(await p.evaluate(()=>__LAYMESH_WEB.pages[0].id),goodID);
      await edit('a.lay',a);await done();
      await edit('a.lay',a.replace('pixel.svg','missing.svg'));
      await p.waitForFunction(()=>document.querySelector('#app').dataset.documentState==='error');
      await p.locator('#import').setInputFiles({name:'missing.svg',mimeType:'image/svg+xml',buffer:Buffer.from(image)});
      await p.evaluate(()=>__LAYMESH_WEB.workspace.whenIdle());await done();
      assert(await p.evaluate(()=>__LAYMESH_WEB.previewCompiler.records[0].dependencies.includes('/project/missing.svg')));
      await edit('a.lay',a);await done();
      // The selected page can play while an unrelated compilation is held.
      await p.locator('#page').selectOption('2');
      await p.evaluate(()=>{const c=__LAYMESH_WEB.previewCompiler,compile=c.compile;window.originalCompile=compile;const hold=new Promise(resolve=>window.releaseBackground=resolve);c.compile=async(data,...rest)=>{if(data.targetPageId==='a')await hold;return compile(data,...rest);};});
      await edit('shared.lay',shared);
      await p.evaluate(()=>__LAYMESH_WEB.presentation.open({fullscreen:false}));
      await p.waitForFunction(()=>__LAYMESH_WEB.presentation.ready);
      assert.equal(await p.evaluate(()=>__LAYMESH_WEB.presentation.index),2);
      assert.equal(await p.evaluate(()=>__LAYMESH_WEB.previewCompiler.complete),false);
      await p.evaluate(()=>__LAYMESH_WEB.previewCompiler.ensure('c'));
      await p.evaluate(()=>__LAYMESH_WEB.presentation.close());
      await p.waitForFunction(()=>!__LAYMESH_WEB.presentation.opened);
      await p.evaluate(()=>releaseBackground());await done();
      await p.evaluate(()=>{__LAYMESH_WEB.previewCompiler.compile=originalCompile;});
      await edit('deck.laydoc','d=document(name="Incremental preview")\nd.add(page(src="b.lay",id="b"))\nd.add(page(src="a.lay",id="extra"))\n');await done();
      assert.deepEqual(await p.evaluate(()=>__LAYMESH_WEB.pages.map(p=>p.sourceId)),['b','extra']);
      await edit('deck.laydoc',doc.replace('Incremental preview','Saved latest title'));await done();
      await edit('theme.lcss',theme.replace('#e04020','#ff8000'));
      await p.locator('#export').click();
      const wait=p.waitForEvent('download');await p.locator('#save').click();
      const saved=path.join(out,engine+'-saved.zip.html');await (await wait).saveAs(saved);
      const entries=unpack(saved),m=JSON.parse(decode(entries['manifest.json']));
      assert.equal(decode(entries[m.project.files['/project/theme.lcss']]),theme.replace('#e04020','#ff8000'));
      assert.deepEqual(m.pages.map(p=>p.id),['a','b','c']);
      assert.equal(m.name,'Saved latest title');
      assert(decode(entries[m.project.files['/project/deck.laydoc']]).includes('Saved latest title'));
      const expected=await p.evaluate(()=>__LAYMESH_WEB.pages.map(p=>JSON.stringify(p.list)));
      assert.deepEqual(m.pages.map(p=>JSON.stringify(JSON.parse(decode(entries[p.file])).list)),expected);
      await p.goto('file://'+saved);await p.waitForFunction(()=>window.__LAYMESH_WEB?.editor&&document.querySelector('#app').dataset.painted==='true');
      assert.equal(await p.title(),'Saved latest title');
      await p.screenshot({path:path.join(out,engine+'.png')});
      await p.locator('#export').click();const waitPlay=p.waitForEvent('download');await p.locator('#download').click();
      const play=path.join(out,engine+'-play.zip.html');await(await waitPlay).saveAs(play);
      await p.goto('file://'+play);await p.waitForFunction(()=>window.__LAYMESH_WEB&&document.querySelector('#app').dataset.painted==='true');
      assert.equal(await p.evaluate(()=>!!__LAYMESH_WEB.editor),false);
      assert.equal(await p.locator('#page option').count(),3);
      assert.deepEqual(errors,[]);assert.deepEqual(requests,[]);
      results.push({engine,passed:true});console.log(engine,'incremental preview passed');
      fs.writeFileSync(path.join(out,'verification.json'),JSON.stringify(results,null,2));
    } finally { await browser.close(); }
  }
})().catch(e=>{console.error(e.message,e.stack);process.exitCode=1;});
