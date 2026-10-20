#!/usr/bin/env python3
"""Natural first/repeat playback, sequential to avoid benchmark interference."""
import argparse, json, os, subprocess, time, hashlib
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser(description=__doc__)
for name in ['original','baseline','optimized','output']:p.add_argument('--'+name,type=Path,required=True)
p.add_argument('--phase',choices=['baseline','optimized','hardware','full'],required=True)
p.add_argument('--resume',action='store_true')
p.add_argument('--reuse-failed',action='store_true',help='With --resume, preserve completed failure measurements instead of repeating them')
a=p.parse_args();a.output.mkdir(parents=True,exist_ok=True)
files={name:getattr(a,name).resolve() for name in ['original','baseline','optimized']}
cases=[]
if a.phase=='full':
 for name in ['baseline','optimized']:
  for size in [(1920,920),(3840,1840)]:cases.append((name,'chromium',size,True))
 for engine in ['chromium','firefox','webkit']:
  for size in [(1920,920),(3840,1840)]:cases.append(('optimized',engine,size,False))
 for name in ['original','baseline']:
  for engine in ['chromium','firefox','webkit']:
   for size in [(1920,920),(3840,1840)]:cases.append((name,engine,size,False))
elif a.phase=='baseline':
 for name in ['original','baseline']:
  for size in [(1920,920),(3840,1840)]:cases.append((name,'chromium',size,False))
elif a.phase=='optimized':
 for engine in ['chromium','firefox','webkit']:
  for size in [(1920,920),(3840,1840)]:cases.append(('optimized',engine,size,False))
else:
 for name in ['baseline','optimized']:
  for size in [(1920,920),(3840,1840)]:cases.append((name,'chromium',size,True))
rows=[]
for name,engine,(width,height),hardware in cases:
 label=f'{name}-{engine}-{width}-'+('hardware' if hardware else 'default')
 dest=a.output/label;digest=hashlib.sha256(files[name].read_bytes()).hexdigest();record=dest/(engine+'.json')
 if a.resume and record.exists():
  previous=json.loads(record.read_text())
  complete=previous.get('status')=='passed' or a.reuse_failed and previous.get('status')=='failed'
  if previous.get('sha256')==digest and previous.get('rounds')==2 and complete and previous.get('streamedDiagnostics') and previous.get('memoryProtocol')=='proc-ppid-v2' and previous.get('first')==1 and previous.get('pages')==26 and previous.get('width')==width and previous.get('height')==height and previous.get('engine')==engine and not previous.get('forcedCPU') and not previous.get('phaseDiagnostics') and bool(previous.get('browserArgs'))==hardware:
   rows.append({'case':label,'sha256':digest,'status':previous['status'],'reused':True,'record':str(record)})
   (a.output/('status-'+a.phase+'.json')).write_text(json.dumps(rows,indent=2)+'\n')
   print('REUSE',label,flush=True);continue
 env=dict(os.environ,ENGINES=engine,WIDTH=str(width),HEIGHT=str(height),ROUNDS='2',FIRST='1',PAGES='26',ALLOW_FAILURE='1',HARDWARE_GPU=str(int(hardware)),FORCE_CPU='0',PHASE_DIAGNOSTICS='0')
 command=['node','scripts/test-openprops-performance.cjs',str(files[name]),str(dest)]
 row={'case':label,'sha256':digest,'command':command,'started':time.time()};rows.append(row)
 print('START',label,flush=True)
 with (a.output/(label+'.log')).open('w') as log:r=subprocess.run(command,cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT)
 row.update(returncode=r.returncode,ended=time.time(),status=json.loads(record.read_text()).get('status') if record.exists() else 'missing')
 (a.output/('status-'+a.phase+'.json')).write_text(json.dumps(rows,indent=2)+'\n')
 print('DONE',label,row['status'],flush=True)
