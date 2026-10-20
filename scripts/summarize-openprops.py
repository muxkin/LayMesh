#!/usr/bin/env python3
"""Summarize raw Open Props runs without treating heap capacity as live memory."""
import argparse,json,math,zipfile
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('html',type=Path);p.add_argument('runs',type=Path);p.add_argument('output',type=Path);a=p.parse_args()
with zipfile.ZipFile(a.html) as z:
 pages=[json.loads(z.read(n)) for n in z.namelist() if n.startswith('pages/') and n.endswith('.json')]
pages.sort(key=lambda page:page['id'])
active={};holds={}
for page in pages:
 intervals=[];stationary=[]
 for step in page['list'].get('animation',{}).get('normal',{}).get('steps',[]):
  end=step['start']+step['duration'];last=step['start']
  for clip in step['clips']:
   if clip['paused'] or not clip['tracks']:continue
   start=step['start']+clip['delay'];stop=min(end,start+clip['duration']*(clip['iterations'] if clip['iterations'] is not None else math.inf))
   if stop>start:intervals.append([max(step['start'],start),stop]);last=max(last,stop)
  if end>last:stationary.append((last,end))
 merged=[]
 for start,end in sorted(intervals):
  if merged and start<=merged[-1][1]:merged[-1][1]=max(merged[-1][1],end)
  else:merged.append([start,end])
 active[page['id']]=merged;holds[page['id']]=stationary

def p95(xs):
 xs=sorted(x for x in xs if x is not None and math.isfinite(x));return xs[max(0,math.ceil(len(xs)*.95)-1)] if xs else None

def gpu_times(calls):
 samples=[x for c in calls for x in c.get('result',{}).get('gpuSamples',[])]
 return samples if samples else [c.get('result',{}).get('gpuMs') for c in calls]

def drm(m,key):return sum(float(c.get(key,'0 KiB').split()[0]) for c in m.get('drm',[]))/1024
rows=[]
for file in sorted(a.runs.glob('**/*.json')):
 if file.name.endswith('-progress.json'):continue
 d=json.loads(file.read_text())
 if not isinstance(d,dict) or not {'engine','rounds','pages','status','sha256','width','height','first'}<=d.keys():continue
 d['probe']=d.get('probe') or {};d['probe'].setdefault('calls',[]);d['probe'].setdefault('visits',[])
 calls=d['probe']['calls'];paints=[c for c in calls if c['type']=='paint' and c.get('running') and not c.get('result',{}).get('skipped') and not c.get('result',{}).get('unchanged') and not c.get('error')]
 measurements=[]
 for page in pages:
  name=page['id'];allCs=sorted([c for c in paints if c.get('page')==name],key=lambda c:c['end']);cs=[c for c in allCs if any(lo<=c.get('time',-1)<hi for lo,hi in active[name])];cadence=[]
  for previous,current in zip(cs,cs[1:]):
   t0,t1=previous.get('time'),current.get('time')
   if t0 is not None and t1 is not None and current.get('serial')==previous.get('serial') and any(lo<=t0<t1<hi for lo,hi in active[name]):cadence.append(current['end']-previous['end'])
  visible=[v['transition'].get('firstVisibleMs') for v in d['probe']['visits'] if v['page']==name]
  if cs or visible:measurements.append({'page':name,'frames':len(cs),'activeReplyIntervalP95Ms':p95(cadence),'rasterP95Ms':p95(c.get('result',{}).get('rasterMs') for c in cs),'gpuP95Ms':p95(gpu_times(cs)),'requestP95Ms':p95(c['end']-c['start'] for c in cs),'firstVisibleP95Ms':p95(visible),'layerBuildsDuringPlayback':sum(c['result']['layerCacheBuilds'] for c in allCs if 'layerCacheBuilds' in c.get('result',{})) if any('layerCacheBuilds' in c.get('result',{}) for c in allCs) else None,'cpuTileConversionsDuringPlayback':sum(c.get('result',{}).get('cpuTileConversions',0) for c in allCs) if any('cpuTileConversions' in c.get('result',{}) for c in allCs) else None,'effectPictureReplaysDuringPlayback':sum(c['result']['effectPictureReplays'] for c in allCs if 'effectPictureReplays' in c.get('result',{})) if any('effectPictureReplays' in c.get('result',{}) for c in allCs) else None})
 stableViolations=[]
 for visit in d['probe']['visits']:
  for lo,hi in holds.get(visit['page'],[]):
   samples=[c for c in paints if c.get('page')==visit['page'] and lo<c.get('time',-1)<hi and c['start']>=visit['end']]
   nextVisit=next((v for v in d['probe']['visits'] if v['start']>visit['end']),None)
   if nextVisit:samples=[c for c in samples if c['start']<nextVisit['start']]
   if len(samples)>1:stableViolations.append({'page':visit['page'],'hold':[lo,hi],'draws':len(samples)})
 allMem=d.get('memory',[]);mem=[m for m in allMem if m.get('protocol')=='proc-ppid-v2'];resources=[c.get('result',{}) for c in calls];preps=[c.get('result',{}).get('preparation',{}) for c in calls]
 errors=[{'type':c['type'],'page':c.get('page'),'error':c['error']} for c in calls if c.get('error') and not any(s in c['error'].lower() for s in ['closed','cancel','terminated'])]
 row={'file':str(file.relative_to(a.runs)),'sha256':d.get('sha256'),'streamedDiagnostics':d.get('streamedDiagnostics',False),'engine':d['engine'],'version':d.get('version'),'dimensions':[d['width'],d['height']],'rounds':d['rounds'],'traversalStatus':d['status'],'failure':d.get('error'),'visited':len(d.get('checks',[])),'taskErrors':errors,'backend':next((v.get('backend') for v in resources if v.get('backend')),next((c['frame'].get('backend') for c in d.get('checks',[]) if c.get('frame',{}).get('backend')),None)),'device':next((v.get('device') for v in resources if v.get('device')),next((c['frame'].get('device') for c in d.get('checks',[]) if c.get('frame',{}).get('device')),None)),'memoryProtocol':d.get('memoryProtocol'),'memorySamples':len(mem),'memorySamplingP95Ms':p95(m.get('sampleMs') for m in mem),'memoryProcessRange':[min((m['processes'] for m in mem),default=None),max((m['processes'] for m in mem),default=None)],'memoryUnreadableSamples':sum(bool(m.get('unreadable')) for m in mem),'peakPssMiB':max((m['pss']/2**20 for m in mem),default=None),'peakDrmVramMiB':max((drm(m,'drm-memory-vram') for m in mem if m.get('drm')),default=None),'peakDrmGttMiB':max((drm(m,'drm-memory-gtt') for m in mem if m.get('drm')),default=None),'roundEnds':[{'round':c['round'],'pssMiB':c['memory']['pss']/2**20,'drmVramMiB':drm(c['memory'],'drm-memory-vram') if c['memory'].get('drm') else None} for c in d.get('checks',[]) if c['index']==d['first']+d['pages']-2 and c['memory'].get('protocol')=='proc-ppid-v2'],'firstVisibleWarmP95Ms':p95(v['transition'].get('firstVisibleMs') for v in d['probe']['visits'] if v['transition'].get('resident')),'menuMs':d.get('menuMs'),'exitMs':d.get('closeMs'),'maxPreparationTaskMs':max((v['maxTaskMs'] for v in preps if 'maxTaskMs' in v),default=None),'maxPreparationGpuWaitMs':max((v['maxGpuWaitMs'] for v in preps if 'maxGpuWaitMs' in v),default=None),'maxSkiaHeapCapacityMiB':max((v['skiaMemoryBytes']/2**20 for v in resources if v.get('skiaMemoryBytes')),default=None),'maxRustHeapCapacityMiB':max((v['wasmMemoryBytes']/2**20 for v in resources if v.get('wasmMemoryBytes')),default=None),'maxRustLiveAllocationMiB':max((v['wasmAllocation'][0]/2**20 for v in resources if v.get('wasmAllocation')),default=None),'maxRustAllocatorPeakMiB':max((v['wasmAllocation'][1]/2**20 for v in resources if v.get('wasmAllocation')),default=None),'holdRedrawViolations':stableViolations,'pages':measurements}
 visits=d['probe']['visits'];secondStart=visits[d['pages']]['start'] if len(visits)>d['pages'] else None
 repeated=[c for c in calls if secondStart is not None and c['start']>=secondStart and c['type'] in ['paint','activate'] and not c.get('error') and not c.get('result',{}).get('skipped') and not c.get('result',{}).get('unchanged')]
 row['secondTraversalCache']={'observedPaints':len(repeated),'complete':len(d.get('checks',[]))>=2*d['pages']}
 for field in ['layerCacheBuilds','staticCacheBuilds','effectDraws','effectPictureReplays','cpuTileConversions']:
  values=[c['result'][field] for c in repeated if field in c.get('result',{})]
  row['secondTraversalCache'][field]=sum(values) if values else None
 rows.append(row)
 stable=[c for c in paints if any(lo<=c.get('time',-1)<hi for lo,hi in active.get(c.get('page'),[])) and c.get('result',{}).get('layerCacheBuilds')==0 and c.get('result',{}).get('staticCacheBuilds')==0 and not c.get('result',{}).get('cpuTileConversions',0)]
 row['activeSkippedPaintRequests']=sum(c['type']=='paint' and c.get('result',{}).get('skipped',False) and any(lo<=c.get('time',-1)<hi for lo,hi in active.get(c.get('page'),[])) for c in calls)
 row['stableAnimation']={'worstPageActiveReplyP95Ms':max((x['activeReplyIntervalP95Ms'] for x in measurements if x['activeReplyIntervalP95Ms'] is not None),default=None),'frames':len(stable),'rasterP95Ms':p95(c['result'].get('rasterMs') for c in stable),'gpuP95Ms':p95(gpu_times(stable)),'framesWithRasterOver60HzBudget':sum((c['result'].get('rasterMs') or 0)>1000/60 for c in stable),'meaning':'Visually active transform/opacity intervals, with no static/object cache rebuild or CPU tile representation conversion. Authored holds excluded. Submission time and asynchronous GPU samples are separate, not physical display cadence.'}
 row['peakDrmSharedVramMiB']=max((drm(m,'drm-shared-vram') for m in mem if m.get('drm')),default=None)
 row['peakDrmPrivateVramMiB']=None
 row['drmDefinition']='On Linux 6.12 amdgpu, drm-memory and drm-shared account backing buffers in each memory domain. Raw per-client fields are retained; no global exclusive physical VRAM estimate is inferred.'
 # Preserve individual phase distributions and last observed ownership per
 # renderer. These counters are not a substitute for process/driver memory.
 row['phases']={}
 for kind in ['load','prepare','activate','paint']:
  subset=[c for c in calls if c['type']==kind and not c.get('error')]
  row['phases'][kind]={'calls':len(subset),'requestP95Ms':p95(c['end']-c['start'] for c in subset)}
  for field in ['decodeMs','installMs','sampleMs','rasterMs','queueWaitMs','gpuMs','resourceWaitMs','presentationWaitMs','handoffMs']:
   row['phases'][kind][field+'P95']=p95(gpu_times(subset)) if field=='gpuMs' else p95(c.get('result',{}).get(field) for c in subset)
 row['gpuMeasurement']='asynchronous query samples' if any(v.get('gpuSamples') for v in resources) else 'legacy last available query value' if any(v.get('gpuMs') is not None for v in resources) else 'unavailable'
 row['transferredCpuFrameBytes']=sum(v['handoffBytes'] for v in resources if v.get('handoffFormat')=='premul-rgba' and v.get('handoffBytes') is not None) if any(v.get('handoffFormat')=='premul-rgba' for v in resources)else None
 row['maxCpuPresentationBufferBytes']=max((v.get('resourceStats',{}).get('presentationBufferBytes',0) for v in resources),default=0) if any('presentationBufferBytes' in v.get('resourceStats',{}) for v in resources)else None
 row['maxCpuTilePixelBytes']=max((v.get('resourceStats',{}).get('cpuTileBytes',0) for v in resources),default=0) if any('cpuTileBytes' in v.get('resourceStats',{}) for v in resources)else None
 row['maxCpuTileConversionWorkingBytes']=max((v.get('resourceStats',{}).get('cpuTileWorkingPeakBytes',0) for v in resources),default=0) if any('cpuTileWorkingPeakBytes' in v.get('resourceStats',{}) for v in resources)else None
 row['lastObservedResourcesByRenderer']={}
 row['lastObservedRuntimeByRenderer']={}
 for c in sorted(calls,key=lambda c:c['end']):
  result=c.get('result',{})
  if 'residentCacheBytes' in result and 'renderer' in result:
   row['lastObservedRuntimeByRenderer'][str(result['renderer'])]={'page':c.get('page'),'atMs':c['end'],**{k:result.get(k) for k in ['residentCacheBytes','residentSnapshotBytes','wasmMemoryBytes','wasmAllocation','skiaMemoryBytes']}}
  if 'resourceStats' in result and 'renderer' in result:
   row['lastObservedResourcesByRenderer'][str(result['renderer'])]={'page':c.get('page'),'atMs':c['end'],'stats':result['resourceStats'],'residentCacheBytes':result.get('residentCacheBytes'),'residentSnapshotBytes':result.get('residentSnapshotBytes')}
 row['preparations']=[{'page':c.get('page'),'stage':c.get('stage'),'priority':c.get('priority'),'requestMs':c['end']-c['start'],**c['result']['preparation']} for c in calls if c.get('result',{}).get('preparation')]
 background=[c for c in calls if c['type'] in ['load','prepare'] and not c.get('priority') and 'renderer' in c.get('result',{}) and not c.get('result',{}).get('deferred')]
 overlapped=[c for c in paints if any(t['start']<c['end'] and c['start']<t['end'] and c.get('result',{}).get('renderer')!=t['result']['renderer'] for t in background)]
 overlappedActive=[c for c in overlapped if any(lo<=c.get('time',-1)<hi for lo,hi in active.get(c.get('page'),[]))]
 row['activeForegroundFramesDuringOtherWorkerPreparation']=len(overlappedActive)
 row['activeForegroundRasterP95DuringOtherWorkerPreparationMs']=p95(c.get('result',{}).get('rasterMs') for c in overlappedActive)
 row['foregroundFramesDuringOtherWorkerPreparation']=len(overlapped)
 row['foregroundRasterP95DuringOtherWorkerPreparationMs']=p95(c.get('result',{}).get('rasterMs') for c in overlapped)
 row['roundResourceEnds']=[{'round':c['round'],'residentCacheBytes':c['frame'].get('residentCacheBytes'),'residentSnapshotBytes':c['frame'].get('residentSnapshotBytes'),'resources':c['frame'].get('resourceStats'),'wasmCapacityBytes':c['frame'].get('wasmMemoryBytes'),'wasmAllocation':c['frame'].get('wasmAllocation'),'skiaCapacityBytes':c['frame'].get('skiaMemoryBytes')} for c in d.get('checks',[]) if c['index']==d['first']+d['pages']-2 and c.get('frame')]
a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(json.dumps({'definition':'Driver DRM resident backing and shared fields are separate from logical RGBA tile bytes and process PSS. Reply intervals are not physical scanout. A hold may draw its first stable frame once; authored durations are unchanged. Missing backend metrics are not zero usage.','runs':rows},indent=2,ensure_ascii=False)+'\n')
