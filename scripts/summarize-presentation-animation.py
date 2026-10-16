#!/usr/bin/env python3
"""Summarize natural playback; skipped submissions never reset the frame interval."""
import argparse,json,math,statistics
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__);p.add_argument('input',type=Path);p.add_argument('output',type=Path);a=p.parse_args()
def stats(values):
 values=sorted(values)
 return {'samples':len(values),'median':statistics.median(values),'p95':values[math.ceil(.95*len(values))-1],'max':max(values)} if values else None
def summarize(data):
 rows=[]
 for visit in data['visits']:
  serial=visit['serial'];calls=[c for c in data['calls'] if c.get('serial')==serial];last=next((c['end'] for c in calls if c['type']=='activate'),None);frames=[];skipped=0
  for c in calls:
   if c['type']!='paint':continue
   active=c['running'] and c.get('time') is not None and any(s['duration']>0 and s['start']<=c['time']<s['start']+s['duration'] for s in visit['steps'])
   if c['result'].get('skipped'):
    skipped+=int(active);continue
   if active:
    assert last is not None
    frames.append({'intervalMs':c['end']-last,'requestMs':c['end']-c['start'],**{k:c['result'].get(k,0) for k in ['sampleMs','rasterMs','queueWaitMs','layerCacheBuilds']}})
   last=c['end']
  assert frames,visit
  rows.append({'page':visit['page'],'index':visit['index'],'serial':serial,'frames':len(frames),'skipped':skipped,'firstDynamicIntervalMs':frames[0]['intervalMs'],'layerBuilds':sum(f['layerCacheBuilds'] for f in frames),'backgroundRequestsDuringPlayback':sum(c['running'] and c['type'] in ['load','prepare'] for c in calls),**{k:stats([f[k] for f in frames]) for k in ['intervalMs','requestMs','sampleMs','rasterMs','queueWaitMs']}})
 background=[c for c in data['calls'] if c['type'] in ['load','prepare'] and c.get('running') and not c['result'].get('deferred')]
 overlaps=sum(any(f['type']=='paint' and not f['result'].get('skipped') and f['start']<b['end'] and f['end']>b['start'] and f['result'].get('renderer')!=b['result'].get('renderer') for f in data['calls']) for b in background)
 initial=next((c['start']for c in data['calls']if c['type']=='init'),None);visible=next((c['end']for c in data['calls']if c['type']=='activate'),None)
 result={'backend':data['backend'],'dimensions':data['dimensions'],'usesWorker':data['usesWorker'],'initialRendererToVisibleMs':visible-initial if initial is not None and visible is not None else None,'overlappingBackgroundRequests':overlaps,'rows':rows}
 edit=data.get('initialEdit')
 if edit:
  completed=edit.get('completedAt')
  result['initialEdit']={'version':edit['version'],'startPageVisibleMs':visible-edit['startedAt'] if visible is not None else None,
   'documentCompleteMs':completed-edit['startedAt'] if completed is not None else None,
   'successfulAnimationFramesBeforeDocumentComplete':sum(c['type']=='paint' and c.get('running') and not c['result'].get('skipped') and edit['startedAt']<=c['end']<completed for c in data['calls']) if completed is not None else None}
 return result
files=sorted(a.input.glob('playback-*/*.json'))+sorted(a.input.glob('4k-*/*.json'))+sorted(a.input.glob('native-animation-*.json'))
result={str(f.relative_to(a.input)):summarize(json.loads(f.read_text())) for f in files};a.output.write_text(json.dumps(result,indent=2)+'\n')
for name,data in result.items():
 print(name,data['dimensions'])
 for row in data['rows']:
  print(row['page'],row['frames'],'interval median/p95',*[round(row['intervalMs'][k],1) for k in ['median','p95']],'sample p95',round(row['sampleMs']['p95'],1),'background',row['backgroundRequestsDuringPlayback'])
