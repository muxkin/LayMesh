#!/usr/bin/env python3
"""Collect license texts for locked Rust dependencies without exposing local paths."""
import hashlib,json,subprocess
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def main():
 external=json.loads((ROOT/'release/license-sources.json').read_text());data=json.loads(subprocess.check_output(['cargo','metadata','--locked','--format-version','1'],cwd=ROOT,text=True));dest=ROOT/'release/licenses';dest.mkdir(exist_ok=True);inventory=[]
 for package in data['packages']:
  if package['source'] is None:continue
  base=Path(package['manifest_path']).parent;files=[]
  for pattern in ['LICENSE*','COPYING*','NOTICE*','fonts/OFL.txt','fonts/FONT_NOTICE.txt']:
   files.extend(p for p in base.glob(pattern) if p.is_file())
  if package.get('license_file'):
   p=base/package['license_file']
   if p.is_file():files.append(p)
  refs=[]
  for p in sorted(set(files)):
   name=f'{package["name"]}-{package["version"]}-{p.name}';(dest/name).write_bytes(p.read_bytes());refs.append({'file':name,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()})
  for item in external:
   if item['name']==package['name'] and item['version']==package['version']:
    p=dest/item['file'];assert p.is_file(),f'Missing pinned upstream license: {p.name}';digest=hashlib.sha256(p.read_bytes()).hexdigest();assert digest==item['sha256'];refs.append({'file':item['file'],'sha256':digest,'source':item['url']})
  assert refs,f'No license text for {package["name"]}'
  inventory.append({'name':package['name'],'version':package['version'],'license':package['license'],'repository':package['repository'],'texts':refs})
 inventory.append({'name':'Matplotlib colormap data','version':'3.11.2','license':'Matplotlib license','repository':'https://github.com/matplotlib/matplotlib','texts':[{'file':'Matplotlib-3.11.2-LICENSE','sha256':hashlib.sha256((dest/'Matplotlib-3.11.2-LICENSE').read_bytes()).hexdigest()}]})
 (dest/'manifest.json').write_text(json.dumps(inventory,indent=2)+'\n')
 (ROOT/'release/THIRD_PARTY_NOTICES.md').write_text('# Third-party notices\n\nThe native executable and WebAssembly module contain the following locked Rust dependencies. Original license texts are in `licenses/`. KaTeX formula fonts are under SIL OFL 1.1; their FONT_NOTICE and OFL text are included. Colormap LUTs are derived from Matplotlib 3.11.2, quantized to 8-bit RGB with native reversed LUTs and category metadata. Matplotlib copyright and license are retained in Matplotlib-3.11.2-LICENSE. No Matplotlib runtime is bundled. No body fonts are bundled.\n\n| Package | Version | License |\n| --- | --- | --- |\n'+''.join(f'| {p["name"]} | {p["version"]} | {p["license"] or "See source"} |\n' for p in inventory))
 print(f'Collected {len(inventory)} dependency records')
if __name__=='__main__':main()
