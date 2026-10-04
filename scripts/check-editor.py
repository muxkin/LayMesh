#!/usr/bin/env python3
"""Audit a local VSIX for exactly one native engine and the host-only client."""
import argparse, hashlib, json, re, zipfile
from pathlib import Path
from xml.etree import ElementTree
ROOT=Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser(description=__doc__);p.add_argument('vsix',type=Path);p.add_argument('--binary',type=Path);a=p.parse_args()
with zipfile.ZipFile(a.vsix) as z:
 assert z.testzip() is None
 names=z.namelist();assert len(names)==len(set(names))
 assert all('..' not in Path(n).parts and not n.startswith('/') for n in names)
 assert not any('node_modules' in n or Path(n).suffix.lower() in ('.ttf','.otf','.ttc','.otc','.woff','.woff2') for n in names)
 engines=[n for n in names if n.startswith('extension/bin/')];assert len(engines)==1
 if a.binary:assert z.read(engines[0])==a.binary.read_bytes()
 if not engines[0].endswith('.exe'):assert (z.getinfo(engines[0]).external_attr>>16)&0o111
 scripts=[n for n in names if Path(n).suffix in ('.js','.mjs','.cjs')];assert scripts==['extension/dist/client.cjs','extension/dist/color-math.mjs','extension/dist/color-panel.mjs','extension/dist/color-webview.mjs'],scripts
 client=z.read(scripts[0]).decode();assert set(re.findall(r"require\(['\"]([^'\"]+)",client))=={'vscode','child_process','path'}
 package=json.loads(z.read('extension/package.json'));assert not package.get('dependencies') and not package.get('devDependencies');assert package['main']=='./dist/client.cjs'
 ElementTree.fromstring(z.read('extension.vsixmanifest'));ElementTree.fromstring(z.read('[Content_Types].xml'))
 for package in json.loads(z.read('extension/licenses/manifest.json')):
  assert package['texts']
  for license in package['texts']:assert hashlib.sha256(z.read('extension/licenses/'+license['file'])).hexdigest()==license['sha256']
 print(json.dumps({'file':a.vsix.name,'bytes':a.vsix.stat().st_size,'installed_bytes':sum(i.file_size for i in z.infolist()),'native_binary':engines[0],'client_modules':['vscode','child_process','path'],'bundled_text_fonts':False}))
