// Separate compiler-only measurements; never substitute these for painted UI timings.
// Usage: node scripts/profile-web-semantic-document.mjs HTML [CACHE_MB=256] [SAMPLES=3]
import fs from 'node:fs';
import {webPayload,webUnzip} from '../site/live/web-archive.mjs';
import init,{CompileSession,register_preview_font,register_preview_asset} from '../target/web-export/laymesh-wasm/laymesh_wasm.js';
import cases from './semantic-document-cases.cjs';
const files=webUnzip(webPayload(fs.readFileSync(process.argv[2]))),decode=b=>new TextDecoder().decode(b),manifest=JSON.parse(decode(files['manifest.json']));
await init({module_or_path:fs.readFileSync(new URL('../target/web-export/laymesh-wasm/laymesh_wasm_bg.wasm',import.meta.url))});
for(const f of manifest.fontRequirements)if(f.path&&f.file)register_preview_font(f.path,files[f.file]);
const source={};for(const [name,key]of Object.entries(manifest.project.files)){if(/\.(lay|laydoc|lcss|css)$/.test(name))source[name]=decode(files[key]);else register_preview_asset(name,files[key]);}
const session=new CompileSession();if(process.env.LAYMESH_PROFILE_DETAILS)session.set_profiling(true);session.set_budget(Number(process.argv[3]||256)*1048576);let version=1;session.apply_changes(version,JSON.stringify(source));const entry=manifest.project.entry;
let pages=JSON.parse(session.metadata(entry)).pages;session.set_active_page('editable');
for(const p of pages){const job=session.evaluate(p.id);job.free();}
for(const kind of (process.env.LAYMESH_DOCUMENT_CASES||'lcss-heading,lcss-curves').split(',')){
 for(let i=0;i<Number(process.argv[4]||3);i++){
  const change=cases.change(kind,i,source['/project/theme.lcss'],source[entry]);session.apply_changes(++version,JSON.stringify({['/project/'+change.file]:change.source}));session.metadata(entry);
  for(const page of [pages.at(-1),...pages.slice(0,-1)].flatMap(p=>process.env.LAYMESH_PROFILE_REPEAT?[p,p]:[p])){const started=performance.now(),job=session.evaluate(page.id),ms=performance.now()-started;job.free();console.log(JSON.stringify({kind,i,page:page.id,evaluateMs:ms,metrics:JSON.parse(session.metrics())}));}
 }
}
session.dispose();session.free();
