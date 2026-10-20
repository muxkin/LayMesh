#!/usr/bin/env python3
"""Collect completed correctness and input checks without assuming absent checks pass."""
import argparse,json,math,statistics
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__);p.add_argument('records',type=Path);a=p.parse_args()
def read(name):return json.loads((a.records/name).read_text())
def p95(xs):return sorted(xs)[math.ceil(len(xs)*.95)-1] if xs else None
quality=read('cpu-quality/objects.json')
# Keep the object-level oracle; whole-page means hide small blurred subjects.
objects=[{'page':page['id'],'time':check['time'],**obj} for page in quality for check in page['checks'] for obj in check['objects']]
interaction=[]
for f in sorted((a.records/'interaction').glob('*.json')):
 if f.stem not in ['chromium','firefox','webkit']:continue
 d=json.loads(f.read_text());samples=d['samples'];interaction.append({k:d[k] for k in ['engine','version','status','offlineEditSaved']}|{'menuP95Ms':p95([x['menuMs']for x in samples]),'exitP95Ms':p95([x['exitMs']for x in samples]),'samples':len(samples)})
result={'interaction':interaction,'vscode':read('vscode/evidence.json')['status'],'cpuPages':len(quality),'cpuObjectChecks':len(objects),'cpuWorstMeanObject':max(objects,key=lambda o:o['mean']) if objects else None}
if (a.records/'cpu-blit').exists():
 result['cpuBlit']=[]
 for engine in ['chromium','firefox','webkit']:
  d=read('cpu-blit/'+engine+'.json');checks=[c for p in d['result'] for c in p['checks']]
  result['cpuBlit'].append({'engine':engine,'status':d['status'],'checks':len(checks),'maxChannelDifference':max(c.get('premultiplied',c)['max'] for c in checks),'definition':'Same retained pixel grid. Opaque pages compared as RGBA; transparent overlap compared as premultiplied RGBA. Rotation, nested clipping and one-representation ownership also asserted.'})
if (a.records/'reopen-50/webkit.json').exists():
 d=read('reopen-50/webkit.json');s=d['samples'];last=s[-10:]
 assert all(x['afterClose'].get('protocol')=='proc-ppid-v2' for x in s), 'Incomplete historical process sampler is not valid resource evidence'
 result['reopen']={'engine':d['engine'],'status':d['status'],'cycles':len(s),'firstPssMiB':s[0]['afterClose']['pss']/2**20,'lastTenPssMinMiB':min(x['afterClose']['pss'] for x in last)/2**20,'lastTenPssMaxMiB':max(x['afterClose']['pss']for x in last)/2**20,'firstRuntime':s[0]['runtime'],'lastRuntime':s[-1]['runtime'],'interpretation':'Finite input stress test. Preparation may retain additional useful content; no assertion of indefinite leak freedom.'}
(a.records/'functional-summary.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n')
