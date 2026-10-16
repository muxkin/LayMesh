// Verify ordinary native export and its first browser edit use the same index contract.
import fs from 'node:fs';import path from 'node:path';import assert from 'node:assert/strict';
import {execFileSync} from 'node:child_process';import {createRequire} from 'node:module';import {pathToFileURL} from 'node:url';import {createHash} from 'node:crypto';
import {webPayload,webUnzip} from '../site/live/web-archive.mjs';import {restorePreviewState} from '../site/live/web-preview-state.mjs';
const [binary,source,reference,output]=process.argv.slice(2);assert(output);fs.mkdirSync(output,{recursive:true});
const exported=path.resolve(output,'native-export.zip.html');
execFileSync(binary,['render',source,'-o',exported,'--html-editable','--html-embed-fonts','--warnings','hide'],{stdio:'inherit'});
const files=webUnzip(webPayload(fs.readFileSync(exported))),original=webUnzip(webPayload(fs.readFileSync(reference))),manifest=JSON.parse(new TextDecoder().decode(files['manifest.json']));
assert.deepEqual(files['runtime/compiler/laymesh.wasm'],original['runtime/compiler/laymesh.wasm'],'Native export must embed the tested compiler');
assert.equal(Object.keys(restorePreviewState(manifest,files)).length,6);
const pw=createRequire(import.meta.url)(process.env.PLAYWRIGHT_MODULE||'playwright'),browser=await pw.chromium.launch({headless:true});
try {
 const p=await browser.newPage(),errors=[];p.setDefaultTimeout(120000);p.on('pageerror',e=>errors.push(e.message));await p.route(/^https?:\/\//,r=>r.abort());await p.goto(pathToFileURL(exported).href);
 await p.waitForFunction(()=>window.__LAYMESH_WEB?.editor&&document.querySelector('#app').dataset.painted==='true');
 const result=await p.evaluate(async()=>{const w=__LAYMESH_WEB,c=w.previewCompiler,initial=c.records.map(r=>r.value),targets=[],compile=c.compile;const restored=c.records.filter(r=>r.dependencies).length;
  c.compile=(s,...rest)=>{targets.push(s.targetPageId);return compile(s,...rest);};const key=Object.keys(w.editor.files).find(p=>p.endsWith('/pages/01-cover.lay'));
  const before=w.editor.files[key];w.editor.files[key]=before.replace('navy = "#041c35"','navy = "#c61717"');if(before===w.editor.files[key])throw Error('Fixture did not change');w.editor.schedule(true);await c.ensure();
  return {restored,targets,unchanged:c.records.slice(1).every((r,i)=>r.value===initial[i+1]),ready:c.complete};});
 assert.equal(result.restored,6);assert.deepEqual(result.targets,['cover']);assert(result.unchanged&&result.ready);assert.deepEqual(errors,[]);
 fs.writeFileSync(path.join(output,'verification.json'),JSON.stringify({engine:'chromium',version:browser.version(),htmlSha256:createHash('sha256').update(fs.readFileSync(exported)).digest('hex'),...result,errors},null,2));console.log('Ordinary export restores all indexes and only compiles the first locally edited page');
} finally {await browser.close();}
