import {sha256} from './web-sha256.mjs';
const canonical=value=>JSON.stringify(value,(_,v)=>v&&typeof v==='object'&&!Array.isArray(v)?Object.fromEntries(Object.entries(v).sort(([a],[b])=>a.localeCompare(b))):v);
function inputs(manifest,files){const result={};for(const [path,file]of Object.entries(manifest.project.files))result[path]=sha256(files[file]);for(const font of manifest.fonts||[]){if(!font.file||!files[font.file])return null;result['font:'+font.path]=sha256(files[font.file]);}return result;}
export function makePreviewState(manifest,files,records){
 const reads=inputs(manifest,files);if(!reads||records.some(r=>!r.valid||!Array.isArray(r.dependencies)))return null;
 return {schema:1,compilerHash:sha256(files['runtime/compiler/laymesh.wasm']),entry:manifest.project.entry,page:manifest.project.page??null,inputs:reads,fonts:manifest.fonts||[],bindings:manifest.fontBindings||{},pages:manifest.pages.map(p=>{const r=records.find(r=>r.key===p.id);return {id:p.id,payloadHash:sha256(files[p.file]),dependencies:r.dependencies,fonts:r.fonts,requestedFonts:r.requestedFonts,warnings:r.warnings||[]};})};
}
export function restorePreviewState(manifest,files){
 try {const data=files['preview-state.json'];if(!data)return {};const state=JSON.parse(new TextDecoder().decode(data));
  if(state.schema!==1||state.compilerHash!==sha256(files['runtime/compiler/laymesh.wasm'])||state.entry!==manifest.project.entry||state.page!==(manifest.project.page??null)||canonical(state.inputs)!==canonical(inputs(manifest,files))||canonical(state.fonts)!==canonical(manifest.fonts||[])||canonical(state.bindings)!==canonical(manifest.fontBindings||{}))return {};
  if(state.pages.length!==manifest.pages.length)return {};
  const result={};for(const page of manifest.pages){const r=state.pages.find(r=>r.id===page.id);if(!r||r.payloadHash!==sha256(files[page.file])||!['dependencies','fonts','requestedFonts','warnings'].every(k=>Array.isArray(r[k])))return {};result[r.id]=r;}return result;
 }catch{return {};}
}
