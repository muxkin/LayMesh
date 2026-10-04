#!/usr/bin/env python3
"""Audit native wheel contents and hashes; never upload."""
import argparse, hashlib, json, tomllib, zipfile
from email.parser import BytesParser
from packaging.requirements import Requirement
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('directory',type=Path);p.add_argument('--all-targets',action='store_true');p.add_argument('--checksums',type=Path);a=p.parse_args()
 config=json.loads((ROOT/'release/runtime.json').read_text());project=tomllib.loads((ROOT/'python/pyproject.toml').read_text())['project'];version=project['version'];wheels=sorted(a.directory.glob('*.whl'));assert wheels,'No wheels to review';targets=set();checksums=[]
 for wheel in wheels:
  with zipfile.ZipFile(wheel) as z:
   assert z.testzip() is None;names=z.namelist();assert len(names)==len(set(names))
   assert all(not n.startswith('/') and '..' not in Path(n).parts and not n.endswith('.pyc') for n in names)
   manifests=[n for n in names if n.endswith('laymesh/_vendor/manifest.json')];assert len(manifests)==1
   vendor=manifests[0].removesuffix('manifest.json');m=json.loads(z.read(manifests[0]));target=m['target'];assert target not in targets;targets.add(target);spec=config['targets'][target]
   assert m['engine']=='rust' and m['version']==version and not m['bundled_text_fonts'] and not m['runtime_downloads']
   assert m['rust_version']==config['rust_version']==tomllib.loads((ROOT/'Cargo.toml').read_text())['workspace']['package']['version']
   assert wheel.name==f'laymesh-{version}-py3-none-{m["wheel_tag"]}.whl'
   if target.startswith('linux-'):
    assert m['wheel_tag'].startswith('manylinux_'+m['glibc_required'].replace('.','_')+'_')
    assert tuple(map(int,m['glibc_required'].split('.')))>=(2,28)
   else:assert m['wheel_tag']==spec['wheel_tag']
   assert m['cargo_lock_sha256']==hashlib.sha256((ROOT/'Cargo.lock').read_bytes()).hexdigest()
   native=vendor+'bin/'+('laymesh.exe' if target.startswith('win-') else 'laymesh')
   assert hashlib.sha256(z.read(native)).hexdigest()==m['binary_sha256']
   assert len([n for n in names if n.startswith(vendor+'bin/')])==1
   assert not any('node_modules' in n or Path(n).suffix.lower() in ('.ttf','.otf','.ttc','.otc','.woff','.woff2','.js','.mjs','.cjs') for n in names),'Runtime or text font leaked into wheel'
   if not target.startswith('win-'):assert z.getinfo(native).external_attr>>16&0o111
   meta=BytesParser().parsebytes(z.read(f'laymesh-{version}.dist-info/METADATA'));assert meta['Requires-Python']=='>=3.10';assert meta['Name']=='laymesh'
   assert meta['Version']==version and meta['Summary']==project['description']
   assert meta['Description-Content-Type']=='text/markdown'
   assert not project.get('optional-dependencies') and not meta.get_all('Provides-Extra',[]),'LayMesh installs all features by default'
   assert {Requirement(r) for r in meta.get_all('Requires-Dist',[])}=={Requirement(r) for r in project['dependencies']},'Default dependency metadata must match the project'
   assert z.read(f'laymesh-{version}.dist-info/licenses/LICENSE')==(ROOT/'LICENSE').read_bytes()
   wheel_metadata=BytesParser().parsebytes(z.read(f'laymesh-{version}.dist-info/WHEEL'));assert wheel_metadata['Root-Is-Purelib']=='false';assert wheel_metadata['Tag']=='py3-none-'+m['wheel_tag']
   for dependency in json.loads(z.read(vendor+'licenses/manifest.json')):
    assert dependency['texts'],f'No license text: {dependency["name"]}'
    for license in dependency['texts']:assert hashlib.sha256(z.read(vendor+'licenses/'+license['file'])).hexdigest()==license['sha256']
   for n in names:
    if n.endswith(('.py','.json','.md')):assert str(Path.home()).encode() not in z.read(n),f'Local home path in {n}'
   installed=sum(i.file_size for i in z.infolist())
  checksums.append(f'{hashlib.sha256(wheel.read_bytes()).hexdigest()}  {wheel.name}');print(f'Checked {wheel.name}: wheel {wheel.stat().st_size/1024**2:.2f} MiB, installed {installed/1024**2:.2f} MiB')
 if a.all_targets:assert targets==set(config['targets']),f'Missing platforms: {set(config["targets"])-targets}'
 if a.checksums:a.checksums.parent.mkdir(parents=True,exist_ok=True);a.checksums.write_text('\n'.join(checksums)+'\n')
if __name__=='__main__':main()
