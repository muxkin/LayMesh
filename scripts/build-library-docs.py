#!/usr/bin/env python3
"""Maintain pinned library inventory, validated recipes and bilingual catalog coverage."""
import argparse,json,subprocess,hashlib
from pathlib import Path
from site_support import engine_files, fingerprint
ROOT=Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser();p.add_argument('--check',action='store_true');p.add_argument('--data',type=Path);a=p.parse_args()
path=ROOT/'site/style-libraries.json'
if a.check:
 data=json.loads(path.read_text())
else:
 data=json.loads(a.data.read_text()) if a.data else json.loads(subprocess.check_output(['cargo','run','--release','--locked','-p','laymesh-language','--example','css_catalog','--','--verify'],cwd=ROOT,text=True))
 # Page transitions have a two-page shader catalog and compatibility suite.
 # The single-object appearance browser cannot preview them meaningfully.
 data['libraries']=[lib for lib in data['libraries'] if lib['name']!='gl-transitions']
 for library in data['libraries']:
  for entry in library['entries']:
   if entry['status']=='unsupported':continue
   entry['reason']=entry['reason'].replace('preferred typography needs matching project font faces; browser fonts may fall back / 首选文字外观需要匹配的项目字体文件；网页字体可能回退','preferred typography needs matching project font faces, browser fonts may fall back / 首选文字外观需要匹配的项目字体文件，网页字体可能回退')
   font_resource=entry['consumer']=='font-family' or (entry['consumer']=='font-weight' and entry['value'] not in ['400','normal']) or any(d['property'] in ['font-weight','font-style','font-family'] and d['value'] not in ['400','normal','inherit'] for d in entry['declarations'])
   if font_resource:
    if entry['status']=='supported':entry['status']='resources'
    if 'matching project font' not in entry['reason']:entry['reason']+='; preferred typography needs matching project font faces, browser fonts may fall back / 首选文字外观需要匹配的项目字体文件，网页字体可能回退'
 data['engine']=fingerprint(engine_files())
 data['generator']=fingerprint(['scripts/build-library-docs.py','crates/laymesh-language/examples/css_catalog.rs'])
 path.write_text(json.dumps(data,ensure_ascii=False,separators=(',',':'))+'\n')
assert data['verified']
assert data['generator']==fingerprint(['scripts/build-library-docs.py','crates/laymesh-language/examples/css_catalog.rs']), 'Catalog generator changed; regenerate verified recipes'
assert data['engine']==fingerprint(engine_files()), 'Engine changed; regenerate verified library recipes'
inventory=json.loads(subprocess.check_output(['cargo','run','--release','--locked','-p','laymesh-language','--example','css_catalog','--','--inventory'],cwd=ROOT,text=True))
expected={l['name']:{(e['kind'],e['name']) for e in l['entries']} for l in inventory['libraries']}
manifest=json.loads((ROOT/'crates/laymesh-core/assets/styles/manifest.json').read_text())
transitions=next(lib for lib in manifest if lib['name']=='gl-transitions')
raw=(ROOT/'crates/laymesh-core/assets/styles/gl-transitions.css').read_bytes()
assert hashlib.sha256(raw).hexdigest()==transitions['sha256']
effects=json.loads((ROOT/'crates/laymesh-core/assets/transitions.json').read_text())
assert expected['gl-transitions']=={('class','gl-transition-'+e['name']) for e in effects}
for lang in ['en','zh-CN']:
 catalog=(ROOT/f'docs/topics/page-transitions.{lang}.md').read_text()
 assert all('| `'+e['name']+'` |' in catalog for e in effects), 'Incomplete page transition catalog'
manifest=[lib for lib in manifest if lib['name']!='gl-transitions']
assert [lib['name'] for lib in data['libraries']]==[lib['name'] for lib in manifest]
seen=set();usable=0
for lib,pinned in zip(data['libraries'],manifest):
 assert (lib['name'],lib['version'],lib['sha256'])==(pinned['name'],pinned['version'],pinned['sha256'])
 assert hashlib.sha256((ROOT/'crates/laymesh-core/assets/styles'/f'{lib["name"]}.css').read_bytes()).hexdigest()==lib['sha256']
 assert {(e['kind'],e['name']) for e in lib['entries']}==expected[lib['name']], f'Incomplete inventory: {lib["name"]}'
 raw=(ROOT/'crates/laymesh-core/assets/styles'/f'{lib["name"]}.css').read_bytes()
 assert lib['entries']
 for entry in lib['entries']:
  assert entry['id'] not in seen;seen.add(entry['id'])
  location=entry['location'];assert raw[location['from']:location['to']].decode()==entry['name'],f'Incorrect source location: {entry["id"]}'
  assert entry['name'] and entry['kind'] in ['class','variable','keyframe'] and entry['reason']
  assert entry['status'] in ['supported','partial','resources','unsupported']
  if entry['status']!='unsupported':assert entry['sample'] and entry['verified'],entry['id'];usable+=1
 default=next(e for e in lib['entries'] if e['name']=={'bootstrap':'bg-primary','open-props':'--gradient-7','animate':'animate__fadeInUp'}[lib['name']] and e['kind']==('variable' if lib['name']=='open-props' else 'class'))
 source=ROOT/'examples/libraries'/f'catalog-{lib["name"]}.lay'
 if a.check:assert source.read_text()==default['sample'],f'Stale {source}'
 else:source.write_text(default['sample'])
 for language in ['zh-CN','en']:
  topic={'bootstrap':'style-bootstrap','open-props':'style-open-props','animate':'style-animate-library'}[lib['name']]
  assert f'<!-- style-catalog:{lib["name"]} -->' in (ROOT/'docs/topics'/f'{topic}.{language}.md').read_text()
# Index uses the compiler parser: rebuilding verifies the full inventory, including
# unsupported rows. CI checks the pinned source hashes and every usable recipe link.
print(f'{"Checked" if a.check else "Generated"} {len(seen)} library entries; {usable} individually verified recipes')
