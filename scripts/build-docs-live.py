#!/usr/bin/env python3
"""Build Rust/WASM workers and copy pre-vendored browser ESM; no Node/npm."""
import argparse,hashlib,json,re,shutil,subprocess
from pathlib import Path
from site_support import ROOT,examples,dependencies
IMPORT=re.compile(r'''((?:\bfrom\s*|\bimport\s*)['"])([^'"]+)(['"])''')
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
  manifest['examples'][e['id']]={'entry':e['source'],'files':[f for f in deps if Path(f).suffix in ('.lay','.lcss','.csv','.json')],'dependencies':deps}
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
  text=p.read_text()
  def replace(m):return m[1]+('./vendor/'+vendor[m[2]]['entry'] if m[2] in vendor else m[2])+m[3]
  text=IMPORT.sub(replace,text)
  # Browser entry names are stable; modules retain their source extension.
  target={'worker.mjs':'worker.js','language-worker.mjs':'language-worker.js','editor.mjs':'editor.js'}.get(p.name,p.name)
  (output/target).write_text(text)
 if not skip_wasm:
  subprocess.run(['cargo','build','--release','--locked','--target','wasm32-unknown-unknown','-p','laymesh-wasm','--no-default-features'],cwd=ROOT,check=True)
  subprocess.run(['wasm-bindgen','--target','web','--out-dir',str(output/'wasm'),str(ROOT/'target/wasm32-unknown-unknown/release/laymesh_wasm.wasm')],check=True)
 if not (output/'wasm/laymesh_wasm_bg.wasm').is_file():raise RuntimeError('Missing WASM binary. Build without --skip-wasm.')
 (output/'manifest.json').write_text(json.dumps(manifest,ensure_ascii=False))
 print(f'Built Rust/WASM preview: {len(manifest["examples"])} examples; {len(vendor)} browser modules; {len(manifest["fonts"])} Page-only body fonts')
if __name__=='__main__':
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--output',type=Path,default=ROOT/'site/dist/site/live');p.add_argument('--skip-wasm',action='store_true');a=p.parse_args();build(a.output.resolve(),a.skip_wasm)
