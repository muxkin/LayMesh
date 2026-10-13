#!/usr/bin/env python3
"""Collect license texts for locked Rust dependencies without exposing local paths."""
import hashlib,json,subprocess,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def linux_static_licenses(dest):
 if sys.platform!='linux':return []
 flags=subprocess.check_output(['pkg-config','--static','--libs','fontconfig','freetype2'],text=True).split()
 names=sorted({f[2:] for f in flags if f.startswith('-l') and f[2:] not in ('m','pthread','dl','rt')})
 records=[]
 for name in names:
  # Release Linux runners use Ubuntu. Record the exact archive's distribution
  # copyright, including third-party notices, rather than a guessed license.
  owner=subprocess.check_output(['dpkg-query','-S',f'*/lib{name}.a'],text=True).splitlines()[0].split(': ',1)[0]
  package=owner.split(':')[0]
  version=subprocess.check_output(['dpkg-query','-W','-f=${Version}',owner],text=True)
  source=Path('/usr/share/doc')/package/'copyright'
  assert source.is_file(),f'Missing static library copyright: {name}'
  filename=f'linux-static-{name}-{version.replace(":","_")}-copyright'
  data=source.read_bytes();(dest/filename).write_bytes(data)
  records.append({'name':f'Linux static {name}','version':version,'license':'See distribution copyright','texts':[{'file':filename,'sha256':hashlib.sha256(data).hexdigest()}]})
 return records
def main():
 external=json.loads((ROOT/'release/license-sources.json').read_text());data=json.loads(subprocess.check_output(['cargo','metadata','--locked','--format-version','1'],cwd=ROOT,text=True));dest=ROOT/'release/licenses';dest.mkdir(exist_ok=True);inventory=[]
 for package in data['packages']:
  if package['source'] is None:continue
  base=Path(package['manifest_path']).parent;files=[]
  for pattern in ['LICENSE*','COPYING*','NOTICE*','license-*','fonts/OFL.txt','fonts/FONT_NOTICE.txt','vendor/COPYING','vendor/PATENTS']:
   files.extend(p for p in base.glob(pattern) if p.is_file())
  if package.get('license_file'):
   p=base/package['license_file']
   if p.is_file():files.append(p)
  # r-efi publishes its MIT permission notice and copyright in AUTHORS.
  if package['name']=='r-efi' and (base/'AUTHORS').is_file():files.append(base/'AUTHORS')
  refs=[]
  for p in sorted(set(files)):
   name=f'{package["name"]}-{package["version"]}-{p.name}';(dest/name).write_bytes(p.read_bytes());refs.append({'file':name,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()})
  for item in external:
   if item['name']==package['name'] and item['version']==package['version']:
    p=dest/item['file'];assert p.is_file(),f'Missing pinned upstream license: {p.name}';digest=hashlib.sha256(p.read_bytes()).hexdigest();assert digest==item['sha256'];refs.append({'file':item['file'],'sha256':digest,'source':item['url']})
  assert refs,f'No license text for {package["name"]}'
  license=package['license']
  if package['name']=='libwebp-sys':license+=' AND BSD-3-Clause'
  inventory.append({'name':package['name'],'version':package['version'],'license':license,'repository':package['repository'],'texts':refs})
 inventory.extend(linux_static_licenses(dest))
 (dest/'CanvasKit-0.42.0-LICENSE').write_bytes((ROOT/'vendor/canvaskit/LICENSE').read_bytes())
 (dest/'Skia-m153-LICENSE').write_bytes((ROOT/'vendor/canvaskit/LICENSE').read_bytes())
 inventory.append({'name':'Skia','version':'m153-0.101.2','license':'BSD-3-Clause','repository':'https://github.com/rust-skia/skia','texts':[{'file':'Skia-m153-LICENSE','sha256':hashlib.sha256((dest/'Skia-m153-LICENSE').read_bytes()).hexdigest()}]})
 inventory.append({'name':'CanvasKit standard','version':'0.42.0','license':'BSD-3-Clause','repository':'https://github.com/google/skia','texts':[{'file':'CanvasKit-0.42.0-LICENSE','sha256':hashlib.sha256((dest/'CanvasKit-0.42.0-LICENSE').read_bytes()).hexdigest()}]})
 inventory.append({'name':'Matplotlib colormap data','version':'3.11.2','license':'Matplotlib license','repository':'https://github.com/matplotlib/matplotlib','texts':[{'file':'Matplotlib-3.11.2-LICENSE','sha256':hashlib.sha256((dest/'Matplotlib-3.11.2-LICENSE').read_bytes()).hexdigest()}]})
 styles=ROOT/'crates/laymesh-core/assets/styles'
 for item in json.loads((styles/'manifest.json').read_text()):
  asset=styles/f'{item["name"]}.css'
  assert hashlib.sha256(asset.read_bytes()).hexdigest()==item['sha256'],f'Modified pinned CSS library: {asset.name}'
  filename=f'css-{item["name"]}-LICENSE.txt';license_data=(styles/f'{item["name"]}.LICENSE').read_bytes();(dest/filename).write_bytes(license_data)
  inventory.append({'name':f'{item["name"]} CSS','version':item['version'],'license':item['license'],'repository':item['source'],'asset_sha256':item['sha256'],'texts':[{'file':filename,'sha256':hashlib.sha256(license_data).hexdigest(),'source':item['license_source']}]})
 tree=ROOT/'vendor/headless-tree';tree_meta=json.loads((tree/'manifest.json').read_text())
 bundled=[('@headless-tree/core',tree_meta,tree/'LICENSE')]
 ui=ROOT/'vendor/workspace-ui'
 for name,info in json.loads((ui/'manifest.json').read_text()).items():
  bundled.append((name,info,ui/next(p for p in info['files'] if 'LICENSE' in p)))
 pdf=ROOT/'vendor/pdf-export'
 for name,info in json.loads((pdf/'manifest.json').read_text()).items():
  refs=[]
  for file,digest in info['files'].items():
   if 'LICENSE' not in file.upper() and 'COPYING' not in file.upper() and 'NOTICE' not in file.upper():continue
   data=(pdf/file).read_bytes();assert hashlib.sha256(data).hexdigest()==digest
   filename='pdf-'+file;(dest/filename).write_bytes(data);refs.append({'file':filename,'sha256':digest})
  inventory.append({'name':name,'version':info['version'],'license':info['license'],'repository':info['source'],'texts':refs})
 for name,info,source in bundled:
  filename=name.replace('/','-').replace('@','')+'-'+info['version']+'-LICENSE';data=source.read_bytes();(dest/filename).write_bytes(data)
  inventory.append({'name':name,'version':info['version'],'license':info['license'],'repository':info['source'],'texts':[{'file':filename,'sha256':hashlib.sha256(data).hexdigest()}]})
 (dest/'manifest.json').write_text(json.dumps(inventory,indent=2)+'\n')
 (ROOT/'release/THIRD_PARTY_NOTICES.md').write_text('# Third-party notices\n\nThe native executable and WebAssembly module contain the following locked Rust dependencies. Original license texts are in `licenses/`. KaTeX formula fonts are under SIL OFL 1.1; their FONT_NOTICE and OFL text are included. Colormap LUTs are derived from Matplotlib 3.11.2, quantized to 8-bit RGB with native reversed LUTs and category metadata. Matplotlib copyright and license are retained in Matplotlib-3.11.2-LICENSE. No Matplotlib runtime is bundled. No body fonts are bundled. The editable web workbench includes Headless Tree 1.7.0, jQuery-contextMenu 2.10.2 and a private jQuery 3.7.1; all use MIT licenses and pinned local assets with no CDN. Page effects reuse GL Transitions 1.74.0 (MIT, BSD-2-Clause and BSD-3-Clause); upstream shader sources and conversion tools are pinned in vendor/gl-transitions. Per-effect attribution and license texts are in licenses/GL-Transitions.txt.\n\n| Package | Version | License |\n| --- | --- | --- |\n'+''.join(f'| {p["name"]} | {p["version"]} | {p["license"] or "See source"} |\n' for p in inventory))
 with (ROOT/'release/THIRD_PARTY_NOTICES.md').open('a') as notices:
  notices.write('\n## Bundled CSS libraries\n\nUnmodified Bootstrap 5.3.8, Open Props 1.7.23 and Animate.css 4.1.1 are embedded in the core engine under their MIT licenses. Pinned source URLs and SHA-256 values are in `crates/laymesh-core/assets/styles/manifest.json`; license copies are included in `licenses/css-*-LICENSE.txt`.\n')
 print(f'Collected {len(inventory)} dependency records')
if __name__=='__main__':main()
