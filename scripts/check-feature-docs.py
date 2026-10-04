#!/usr/bin/env python3
"""Fail on undocumented public surfaces; optionally verify drawing examples in all formats."""
import argparse,json,re,subprocess,tempfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--render',action='store_true');p.add_argument('--binary',type=Path,default=ROOT/'target/release/laymesh');a=p.parse_args()
r=json.loads((ROOT/'crates/laymesh-core/api.json').read_text());features=json.loads((ROOT/'docs/feature-coverage.json').read_text())['features']
expected={'api:'+e['name'] for e in r['api']}|{'lcss:'+n for n in r['styleProperties']}
for typ,info in r['geometryTypes'].items():
 expected.update('geometry:'+typ+'.'+n for k in ['members','methods'] for n in info.get(k,{}))
 if info.get('index'):expected.add('geometry:'+typ+'[i]')
assert expected<=features.keys(),'Missing features: '+str(sorted(expected-features.keys()))
assert {k for k in features if k.startswith(('api:','lcss:','geometry:'))}==expected,'Stale feature entries'
for name,f in features.items():
 for lang in ['en','zh-CN']:
  doc=ROOT/f'docs/topics/{f["topic"]}.{lang}.md'
  assert doc.is_file(),f'{name}: missing {doc}'
  assert re.search(r'^#{2,3}\s+'+re.escape(f['anchor'])+r'\s*$',doc.read_text(),re.M),f'{name}: missing heading {f["anchor"]} in {doc}'
 for use in ['minimal','composition']:
  source=ROOT/f[use];assert source.is_file(),f'{name}: missing {source}'
 if name.startswith('api:'):
  surface=name[4:];leaf=surface.rsplit('.',1)[-1]
  pattern=r'\.'+re.escape(leaf)+r'\s*\(' if '.' in surface or surface in ['add','fuse'] else r'(?<![\w.])'+re.escape(surface)+r'\s*\('
  for use in ['minimal','composition']:
   assert re.search(pattern,(ROOT/f[use]).read_text()),f'{name}: {use} never exercises the function'
 if name.startswith('geometry:'):
  assert f.get('exercise'),f'{name}: missing executable example condition'
  for use in ['minimal','composition']:
   assert re.search(f['exercise'],(ROOT/f[use]).read_text()),f'{name}: {use} never exercises the member or index'
for entry in r['api']:
 assert entry.get('returns') and len(entry['descriptionEn'])>20,entry['name']
 for param in entry['parameters']:
  assert param.get('type') and param.get('description') and param.get('descriptionEn'),(entry['name'],param['name'])
  for v in param.get('values',[]):
   assert all(param.get('valueDescriptions',{}).get(v,{}).get(lang) for lang in ['zh','en']),(entry['name'],param['name'],v)
names={v['name'] for v in r['predefinedVariables']};assert len(names)==len(r['predefinedVariables'])
for entry in r['api']:
 for p in entry['parameters']:
  for v in p.get('values',[]):assert any(q['value']==v for q in r['predefinedVariables']),v
if a.render:
 sources=sorted({f['minimal'] for k,f in features.items() if k.startswith(('api:','geometry:'))})
 with tempfile.TemporaryDirectory(prefix='laymesh-doc-examples-') as tmp:
  for i,source in enumerate(sources):
   subprocess.run([str(a.binary),'validate',str(ROOT/source),'--warnings','hide'],check=True,stdout=subprocess.DEVNULL)
   for ext in ['svg','pdf','png']:
    output=Path(tmp)/f'{i}.{ext}'
    subprocess.run([str(a.binary),'render',str(ROOT/source),'-o',str(output),'--warnings','hide'],check=True,stdout=subprocess.DEVNULL)
    assert output.stat().st_size>100,(source,ext)
  print(f'Validated {len(sources)} minimal sources and {len(sources)*3} SVG/PDF/PNG exports')
print(f'Documentation coverage: {len(features)} features, {len(r["api"])} APIs, {len(r["predefinedVariables"])} predefined variables; no omissions')
