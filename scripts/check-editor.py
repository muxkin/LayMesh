#!/usr/bin/env python3
"""Audit a local VSIX for exactly one native engine and the host-only client."""
import argparse, hashlib, json, re, struct, zipfile
from pathlib import Path
from xml.etree import ElementTree
ROOT=Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser(description=__doc__);p.add_argument('vsix',type=Path);p.add_argument('--binary',type=Path);p.add_argument('--target');p.add_argument('--pre-release',action='store_true');p.add_argument('--marketplace',action='store_true');a=p.parse_args()
with zipfile.ZipFile(a.vsix) as z:
 assert z.testzip() is None
 names=z.namelist();assert len(names)==len(set(names))
 assert all('..' not in Path(n).parts and not n.startswith('/') for n in names)
 assert not any('node_modules' in n or Path(n).suffix.lower() in ('.ttf','.otf','.ttc','.otc','.woff','.woff2') for n in names)
 engines=[n for n in names if n.startswith('extension/bin/')];assert len(engines)==1
 if a.binary:assert z.read(engines[0])==a.binary.read_bytes()
 if not engines[0].endswith('.exe'):assert (z.getinfo(engines[0]).external_attr>>16)&0o111
 scripts=[n for n in names if Path(n).suffix in ('.js','.mjs','.cjs')];assert scripts==['extension/dist/client.cjs','extension/dist/color-math.mjs','extension/dist/color-panel.mjs','extension/dist/color-webview.mjs','extension/dist/preview-host.cjs','extension/dist/preview-math.mjs','extension/dist/preview-webview.mjs'],scripts
 assert 'extension/dist/preview.css' in names
 host=z.read('extension/dist/preview-host.cjs').decode()
 assert "['pptx','PowerPoint (PPTX)']" in host, 'PPTX must be available in the export format picker'
 client=z.read(scripts[0]).decode();assert set(re.findall(r"require\(['\"]([^'\"]+)",client))=={'vscode','child_process','path','./preview-host.cjs'}
 package=json.loads(z.read('extension/package.json'));assert not package.get('dependencies') and not package.get('devDependencies');assert package['main']=='./dist/client.cjs'
 languages={v['id']:v for v in package['contributes']['languages']};assert set(languages)=={'laymesh','lcss'}
 for language in languages.values():
  assert set(language['icon'])=={'light','dark'}
  for icon_path in language['icon'].values():
   icon_name='extension/'+icon_path.removeprefix('./')
   assert icon_name in names, f'Missing language icon: {icon_name}'
   svg=ElementTree.fromstring(z.read(icon_name));assert svg.tag=='{http://www.w3.org/2000/svg}svg'
   assert svg.get('viewBox')=='0 0 16 16'
 assert 'registerDocumentFormattingEditProvider' in client and 'registerDocumentRangeFormattingEditProvider' in client
 assert package['contributes']['configuration']['properties']['laymesh.format.lineWidth']['default']==100
 placeholders=set(re.findall(r'"%([^%]+)%"',z.read('extension/package.json').decode()))
 translations=json.loads(z.read('extension/package.nls.json'))
 chinese=json.loads(z.read('extension/package.nls.zh-cn.json'))
 assert placeholders==set(translations)==set(chinese)
 assert all(isinstance(value,str) and value.strip() for table in [translations,chinese] for value in table.values())
 manifest=ElementTree.fromstring(z.read('extension.vsixmanifest'));content=ElementTree.fromstring(z.read('[Content_Types].xml'))
 defaults={v.get('Extension').lstrip('.').lower():v.get('ContentType') for v in content.findall('{*}Default')}
 overrides={v.get('PartName').lstrip('/'):v.get('ContentType') for v in content.findall('{*}Override')}
 def content_type(name):return overrides.get(name) or defaults.get(Path(name).suffix.lstrip('.').lower())
 for language in languages.values():
  for icon_path in language['icon'].values():assert content_type('extension/'+icon_path.removeprefix('./'))=='image/svg+xml'
 assert manifest.find('.//{*}Description').text==translations['extension.description']
 identity=manifest.find('.//{*}Identity')
 assert identity.get('Id')==package['name'] and identity.get('Publisher')==package['publisher'] and identity.get('Version')==package['version']
 properties={v.get('Id'):v.get('Value') for v in manifest.findall('.//{*}Property')}
 if a.target:assert identity.get('TargetPlatform')==a.target
 assert (properties.get('Microsoft.VisualStudio.Code.PreRelease')=='true')==a.pre_release
 assert manifest.find('.//{*}GalleryFlags').text==('Public Preview' if package.get('preview') else 'Public')
 for asset in manifest.findall('.//{*}Asset'):
  assert asset.get('Path') in names
  assert content_type(asset.get('Path')),f'Missing content type for asset: {asset.get("Path")}'
 if a.marketplace:
  if not a.pre_release:assert not package.get('preview'), 'Stable Marketplace packages must not display the Preview badge.'
  assert re.fullmatch(r'(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)',package['version'])
  assert identity.get('TargetPlatform'), 'A native Marketplace package must declare its platform.'
  assert package['extensionKind']==['workspace']
  assert properties['Microsoft.VisualStudio.Code.ExtensionKind']=='workspace'
  assert properties['Microsoft.VisualStudio.Code.ExecutesCode']=='true'
  assert package['repository']['url'].startswith('https://') and package['bugs']['url'].startswith('https://')
  assert 'extension/CHANGELOG.md' in names
  assert manifest.find('.//{*}License').text=='extension/LICENSE.txt'
  assert z.read('extension/LICENSE.txt')==(ROOT/'LICENSE').read_bytes()
  assert content_type('extension/LICENSE.txt')=='text/plain'
  assert all(content_type(n) for n in names if n!='[Content_Types].xml'), 'Missing package part content type'
  icon=z.read('extension/'+package['icon']);assert icon[:8]==b'\x89PNG\r\n\x1a\n'
  assert min(struct.unpack('>II',icon[16:24]))>=128
  assert manifest.find('.//{*}Icon').text=='extension/'+package['icon']
  if identity.get('TargetPlatform') in ('linux-x64','linux-arm64'):
   native=z.read(engines[0]);assert native[:6]==b'\x7fELF\x02\x01' and struct.unpack('<H',native[18:20])[0]==(62 if identity.get('TargetPlatform')=='linux-x64' else 183)
  if identity.get('TargetPlatform') in ('darwin-x64','darwin-arm64'):
   native=z.read(engines[0]);assert native[:4]==b'\xcf\xfa\xed\xfe'
   assert struct.unpack_from('<I',native,4)[0]==(0x01000007 if identity.get('TargetPlatform')=='darwin-x64' else 0x0100000c)
  if identity.get('TargetPlatform') in ('win32-x64','win32-arm64'):
   assert engines[0]=='extension/bin/laymesh.exe'
   native=z.read(engines[0]);assert native[:2]==b'MZ'
   pe=struct.unpack_from('<I',native,0x3c)[0];assert native[pe:pe+4]==b'PE\0\0'
   assert struct.unpack_from('<H',native,pe+4)[0]==(0x8664 if identity.get('TargetPlatform')=='win32-x64' else 0xaa64)
   assert struct.unpack_from('<H',native,pe+24)[0]==0x20b
  for md in ['README.md','CHANGELOG.md']:
   assert all(url.startswith(('https://','#')) for url in re.findall(r'\]\(([^\s)]+)',z.read('extension/'+md).decode())),md
 for package in json.loads(z.read('extension/licenses/manifest.json')):
  assert package['texts']
  for license in package['texts']:assert hashlib.sha256(z.read('extension/licenses/'+license['file'])).hexdigest()==license['sha256']
 print(json.dumps({'file':a.vsix.name,'publisher':identity.get('Publisher'),'version':identity.get('Version'),'target':identity.get('TargetPlatform'),'prerelease':a.pre_release,'marketplace_checks':a.marketplace,'bytes':a.vsix.stat().st_size,'installed_bytes':sum(i.file_size for i in z.infolist()),'native_binary':engines[0],'client_modules':['vscode','child_process','path'],'bundled_text_fonts':False}))
