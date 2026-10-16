#!/usr/bin/env python3
"""Summarize final-archive first edits without treating parallel CPU sums as wall time."""
import argparse,json,math,statistics
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__);p.add_argument('raw',type=Path);p.add_argument('output',type=Path);a=p.parse_args();data=json.loads(a.raw.read_text())
def stats(values):
 values=sorted(values)
 return {'samples':len(values),'median':statistics.median(values),'p95':values[math.ceil(.95*len(values))-1],'min':values[0],'max':values[-1]}
rows=[]
for row in data['rows']:
 events=row['trace']['events'];attempts=[e['attempt'] for e in events if e['type']=='worker-progress' and e['phase']=='attempt-end']
 replies=[e['diagnostics'] for e in events if e['type']=='received']
 pages=[p for d in replies for p in d.get('pages',[])]
 commits={};commit_ms=0
 for e in events:
  if e['type']=='commit-start':commits[e['page']]=e['at']
  if e['type']=='committed':commit_ms+=e['at']-commits[e['page']]
 rows.append({**{k:row[k] for k in ['kind','temperature','sample','currentMs','playableMs','documentMs','compiles','wasmMemoryBytes']},
  'workers':len(row['workers']),'overlaps':len(row['overlaps']),'attempts':len(attempts),'yields':sum(x['status']=='E_YIELD' for x in attempts),
  'workerInitializationSumMs':sum(d['initMs'] for d in {x['worker']:x for x in replies}.values()) if row['temperature']=='cold' else 0,
  'workerQueueSumMs':sum(max(0,d['startedAt']-d['enqueuedAt']) for d in replies),
  'directoryComputeSumMs':sum(x['computeMs'] for x in attempts if x['phase']=='directory'),
  'evaluationComputeSumMs':sum(x['computeMs'] for x in attempts if x['phase'].startswith('evaluate:')),
  'drawingSumMs':sum(x['drawMs'] for x in pages),'serializationSumMs':sum(x['serializeMs'] for x in pages),'commitSumMs':commit_ms})
summary=[]
for kind in dict.fromkeys(r['kind'] for r in rows):
 for temperature in ['cold','warm']:
  group=[r for r in rows if r['kind']==kind and r['temperature']==temperature]
  summary.append({'kind':kind,'temperature':temperature,'samples':len(group),'stats':{k:stats([r[k] for r in group]) for k in group[0] if k not in ['kind','temperature','sample']}})
a.output.write_text(json.dumps({'engine':data['engine'],'version':data['version'],'htmlSha256':data['htmlSha256'],'smoke':data['smoke'],
 'definition':'current/playable/document are wall latency from the editor transaction; phase sums include all attempts and may overlap across workers. Cold is a fresh document context, not a flushed machine cache.', 'summary':summary,'rows':rows},indent=2)+'\n')
for r in summary:print(r['kind'],r['temperature'],r['samples'],*[f'{k}: {r["stats"][k]["median"]:.1f}/{r["stats"][k]["p95"]:.1f}' for k in ['currentMs','playableMs','documentMs']])
