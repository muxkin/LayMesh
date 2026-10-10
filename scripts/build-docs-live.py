#!/usr/bin/env python3
"""Build Rust/WASM workers and copy pre-vendored browser ESM; no Node/npm."""
import argparse,hashlib,json,re,shutil,subprocess
from pathlib import Path
from site_support import ROOT,examples,dependencies,engine_files,fingerprint
IMPORT=re.compile(r'''((?:\bfrom\s*|\bimport\s*)['"])([^'"]+)(['"])''')
def runtime_sources():
 return engine_files()+[p.relative_to(ROOT).as_posix() for p in sorted((ROOT/'crates/laymesh-wasm').rglob('*')) if p.is_file() and p.suffix in ('.rs','.toml','.json')]
def browser_module(path,vendor):
 text=path.read_text()
 def replace(m):return m[1]+('./vendor/'+vendor[m[2]]['entry'] if m[2] in vendor else m[2])+m[3]
 target={'worker.mjs':'worker.js','language-worker.mjs':'language-worker.js','editor.mjs':'editor.js'}.get(path.name,path.name)
 return target,IMPORT.sub(replace,text)
def validate_runtime(output):
 vendor=json.loads((ROOT/'site/vendor/manifest.json').read_text())
 for source in (ROOT/'site/live').glob('*.mjs'):
  target,text=browser_module(source,vendor)
  assert (output/target).read_text()==text,f'Stale browser module: {target}; rebuild docs'
 provenance=output/'wasm/laymesh-build.json'
 assert provenance.is_file(),'Missing WASM provenance; rebuild docs without --skip-wasm'
 stored=json.loads(provenance.read_text())
 assert stored['engine_fingerprint']==fingerprint(runtime_sources()),'Stale browser WASM sources; rebuild docs without --skip-wasm'
 assert stored['wasm_sha256']==hashlib.sha256((output/'wasm/laymesh_wasm_bg.wasm').read_bytes()).hexdigest(),'Browser WASM fingerprint mismatch'
def build(output,skip_wasm=False):
 output.mkdir(parents=True,exist_ok=True)
 manifest={'engine':'rust-ratex','resources':{},'fonts':[],'examples':{}}
 (output/'resources').mkdir(exist_ok=True)
 font_dir=output/'fonts';font_dir.mkdir(exist_ok=True)
 for family,filename,license_name in [
  ('DejaVu Sans','DejaVuSans.ttf','DejaVu-LICENSE'),
  ('Noto Sans CJK SC','NotoSansCJK-Regular.ttc','Noto-CJK-LICENSE'),
 ]:
  source=ROOT/'site/page-fonts'/filename
  data=source.read_bytes()
  if len(data)>=20_000_000:raise RuntimeError(f'Page font exceeds jsDelivr 20 MB file limit: {filename}')
  shutil.copy2(source,font_dir/filename)
  shutil.copy2(ROOT/'site/page-fonts'/license_name,font_dir/license_name)
  manifest['fonts'].append({'family':family,'file':'fonts/'+filename,'sha256':hashlib.sha256(data).hexdigest(),
   'cdn':'https://cdn.jsdelivr.net/gh/muxkin/LayMesh@main/site/page-fonts/'+filename})
 for e in examples():
  deps=[f for f in dependencies(e['source']) if Path(f).suffix.lower() not in ('.ttf','.otf','.ttc','.otc','.woff','.woff2')]
  manifest['examples'][e['id']]={'entry':e['source'],'files':[f for f in deps if Path(f).suffix in ('.lay','.laydoc','.lcss','.css','.csv','.json')],'dependencies':deps}
  for f in deps:
   data=(ROOT/f).read_bytes();name='resources/'+hashlib.sha256(data).hexdigest()+Path(f).suffix;(output/name).write_bytes(data);manifest['resources']['/'+f]=name
 vendor=json.loads((ROOT/'site/vendor/manifest.json').read_text())
 for name,info in vendor.items():
  p=ROOT/'site/vendor'/info['entry'];assert hashlib.sha256(p.read_bytes()).hexdigest()==info['sha256'],f'Modified browser dependency: {name}'
  dest=output/'vendor'/name;dest.mkdir(parents=True,exist_ok=True);shutil.copy2(ROOT/'site/vendor'/info['license'],dest/'LICENSE')
  text=p.read_text()
  def replace(m):
   dep=m[2]
   if dep not in vendor:return m[0]
   import os
   relative=os.path.relpath(output/'vendor'/vendor[dep]['entry'],dest).replace('\\','/')
   return m[1]+(relative if relative.startswith('.') else './'+relative)+m[3]
  (dest/'index.js').write_text(IMPORT.sub(replace,text))
 shutil.copy2(ROOT/'site/vendor/manifest.json',output/'vendor/manifest.json')
 for p in (ROOT/'site/live').glob('*.mjs'):
  target,text=browser_module(p,vendor)
  (output/target).write_text(text)
 if not skip_wasm:
  subprocess.run(['cargo','build','--release','--locked','--target','wasm32-unknown-unknown','-p','laymesh-wasm','--no-default-features'],cwd=ROOT,check=True)
  subprocess.run(['wasm-bindgen','--target','web','--out-dir',str(output/'wasm'),str(ROOT/'target/wasm32-unknown-unknown/release/laymesh_wasm.wasm')],check=True)
  (output/'wasm/laymesh-build.json').write_text(json.dumps({'engine_fingerprint':fingerprint(runtime_sources()),'wasm_sha256':hashlib.sha256((output/'wasm/laymesh_wasm_bg.wasm').read_bytes()).hexdigest()}))
 if not (output/'wasm/laymesh_wasm_bg.wasm').is_file():raise RuntimeError('Missing WASM binary. Build without --skip-wasm.')
 validate_runtime(output)
 (output/'manifest.json').write_text(json.dumps(manifest,ensure_ascii=False))
 print(f'Built Rust/WASM preview: {len(manifest["examples"])} examples; {len(vendor)} browser modules; {len(manifest["fonts"])} Page-only body fonts')
if __name__=='__main__':
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--output',type=Path,default=ROOT/'site/dist/site/live');p.add_argument('--skip-wasm',action='store_true');a=p.parse_args();build(a.output.resolve(),a.skip_wasm)
