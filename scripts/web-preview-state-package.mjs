// Dependency indexes are derived from the compiler shipped in this archive.
// Never trust a local build or patch a stale index's compiler hash.
import assert from 'node:assert/strict';
import {makePreviewState, restorePreviewState} from '../site/live/web-preview-state.mjs';
export async function rebuildPreviewState(files) {
  const decode = b => new TextDecoder().decode(b), manifest = JSON.parse(decode(files['manifest.json']));
  if (!manifest.project) { delete files['preview-state.json']; return []; }
  const binding = files['runtime/compiler/laymesh_wasm.js'], wasm = files['runtime/compiler/laymesh.wasm'];
  assert(binding && wasm, 'Editable archive is missing its compiler');
  // A distinct module instance prevents resources from a previous archive leaking in.
  const compiler = await import('data:text/javascript;base64,' + Buffer.from(binding).toString('base64') + '#' + crypto.randomUUID());
  await compiler.default({module_or_path: wasm});
  for (const f of manifest.fonts || []) {
    assert(f.file && files[f.file], 'Cannot establish dependencies without embedded font: ' + f.path);
    compiler.register_preview_font(f.path, files[f.file]);
  }
  const source = {};
  for (const [name, key] of Object.entries(manifest.project.files)) {
    assert(files[key], 'Missing project entry: ' + name);
    if (/\.(lay|laydoc|lcss|css|csv|json)$/i.test(name)) source[name] = decode(files[key]);
    else compiler.register_preview_asset(name, files[key]);
  }
  const aliases=await import('data:text/javascript;base64,'+Buffer.from(files['runtime/font-aliases.mjs']).toString('base64'));
  const automatic={};
  for(const font of manifest.fonts || [])for(const name of [font.family,...(font.names || []).filter(n=>[1,16].includes(n.nameID)).map(n=>n.value)])
    for(const alias of aliases.fontAliases(name))if(alias!==font.family)automatic[alias]=font.family;
  const bindings=aliases.compilerFontBindings?.(manifest.fonts || [],manifest.fontBindings || {}) || {...automatic,...manifest.fontBindings};
  compiler.register_preview_asset('/@laymesh/font-bindings.json', new TextEncoder().encode(JSON.stringify(bindings)));
  const session = new compiler.CompileSession(), records = [];
  try {
    session.apply_changes(1, JSON.stringify(source));
    compiler.clear_preview_dependencies();
    const metadata = JSON.parse(session.metadata(manifest.project.entry));
    const planning = JSON.parse(compiler.preview_dependencies());
    const pages = metadata.pages.filter((_, i) => manifest.project.page == null || i + 1 === manifest.project.page);
    assert.deepEqual(pages.map(p => p.id), manifest.pages.map(p => p.id), 'Stored page directory differs from source');
    for (const p of pages) {
      compiler.clear_preview_dependencies();
      const job = session.evaluate(p.id);
      try {
        const inspection = JSON.parse(job.inspection());
        records.push({key:p.id, valid:true,
          dependencies:[...new Set([manifest.project.entry, ...planning, ...JSON.parse(compiler.preview_dependencies())])],
          fonts:manifest.fontRequirements || [], requestedFonts:inspection.requestedFonts || [], warnings:inspection.warnings || []});
      } finally { job.free(); }
    }
  } finally { session.dispose(); session.free(); }
  const state = makePreviewState(manifest, files, records);
  assert(state, 'Cannot establish complete dependency state');
  files['preview-state.json'] = new TextEncoder().encode(JSON.stringify(state));
  assert.equal(Object.keys(restorePreviewState(manifest, files)).length, manifest.pages.length, 'Dependency state failed archive validation');
  return records.map(r => ({id:r.key, dependencies:r.dependencies}));
}
