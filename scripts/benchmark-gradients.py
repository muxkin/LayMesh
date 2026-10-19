#!/usr/bin/env python3
"""Compare native release binaries in alternating order, on the same fixtures.
Run without concurrent builds/browsers. New Open Props coverage is timed only
on the candidate, since ignored declarations are not equivalent old work.
"""
import argparse, hashlib, json, os, platform, statistics, tempfile, time
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('baseline',type=Path);p.add_argument('candidate',type=Path);p.add_argument('output',type=Path)
p.add_argument('--runs',type=int,default=9);a=p.parse_args();out=a.output.resolve();out.mkdir(parents=True,exist_ok=True)
root=Path(__file__).resolve().parents[1]
binaries={k:v.resolve() for k,v in [('baseline',a.baseline),('candidate',a.candidate)]}
head='p=canvas(size=(160mm,100mm),background="#fff")\n'
loop='for y in range(10) { for x in range(16) { p.add(rect(size=(9mm,9mm),%s),offset=(x*10mm,y*10mm)) } }\n'
fixtures={
 'legacy-constructor':head+loop%'fill=linear_gradient(stops=[(0,"#123456"),(1,"#abcdef")]),border_width=0',
 'legacy-css':head+'style{.a{background:linear-gradient(120deg,#123456,#abcdef);border-width:0}}\n'+loop%'class="a",border_width=0',
 'open-props-30':(root/'examples/libraries/open-props-gradients.lay').read_text(),
}
results={}
for name,source in fixtures.items():
 entry=out/(name+'.lay');entry.write_text(source);runs={k:[] for k in binaries if name!='open-props-30' or k=='candidate'}
 for n in range(a.runs+2):
  keys=list(runs)
  if n%2:keys.reverse()
  for key in keys:
   image=out/(name+'-'+key+'.png')
   with tempfile.TemporaryFile() as log:
    start=time.perf_counter();pid=os.fork()
    if pid==0:
     os.dup2(log.fileno(),1);os.dup2(log.fileno(),2)
     os.execv(str(binaries[key]),[str(binaries[key]),'render',str(entry),'-o',str(image),'--dpi','144'])
    _,status,usage=os.wait4(pid,0);elapsed=(time.perf_counter()-start)*1000
    log.seek(0);diagnostics=log.read().decode()
   if status or 'Ignored background' in diagnostics:raise RuntimeError(diagnostics)
   user,system,rss=usage.ru_utime,usage.ru_stime,usage.ru_maxrss
   if platform.system()=='Darwin':rss/=1024
   if n>=2:runs[key].append({'wallMs':round(elapsed,3),'userSeconds':user,'systemSeconds':system,'peakRSSKiB':int(rss),'outputBytes':image.stat().st_size})
 results[name]={k:{'medianMs':round(statistics.median(r['wallMs'] for r in v),3),'minMs':min(r['wallMs'] for r in v),'maxMs':max(r['wallMs'] for r in v),'peakRSSKiB':max(r['peakRSSKiB'] for r in v),'runs':v} for k,v in runs.items()}
 if len(runs)==2:results[name]['medianChangePercent']=round((results[name]['candidate']['medianMs']/results[name]['baseline']['medianMs']-1)*100,2)
 print(name,{k:v['medianMs'] for k,v in results[name].items() if isinstance(v,dict)},flush=True)
report={'platform':platform.platform(),'method':f'Alternating binaries; two warmups then {a.runs} measured processes; PNG 144 DPI; same inputs and environment; no concurrent task builds/browser tests. POSIX wait4 per-child CPU/RSS. Total CLI compile/render/encode/process time, not a phase profile.','binaries':{k:{'sha256':hashlib.sha256(v.read_bytes()).hexdigest(),'bytes':v.stat().st_size} for k,v in binaries.items()},'results':results}
(out/'performance.json').write_text(json.dumps(report,indent=2)+'\n')
