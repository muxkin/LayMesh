#!/usr/bin/env python3
"""Run final-archive checks and timings sequentially, retaining commands and status."""
import argparse,datetime,hashlib,json,os,platform,subprocess,time
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--html',required=True,type=Path);p.add_argument('--baseline',required=True,type=Path);p.add_argument('--output',required=True,type=Path)
p.add_argument('--extension-dir',required=True,type=Path);p.add_argument('--samples',type=int,default=50);p.add_argument('--steps',default='progressive,cold,roundtrip,first-edit,animation,animation-edit,native-animation,native-keyboard,ordinary-export')
a=p.parse_args();assert a.samples>=50;a.output=a.output.resolve();a.output.mkdir(parents=True,exist_ok=True)
html=str(a.html.resolve());baseline=str(a.baseline.resolve());ext=a.extension_dir.resolve();binary=ext/'bin/laymesh'
env={**os.environ,'PLAYWRIGHT_MODULE':os.environ.get('PLAYWRIGHT_MODULE','/home/keqing/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright')}
node=lambda script,*args:['node','scripts/'+script,*map(str,args)]
native=lambda *args:[str(ROOT/'.venv/bin/python'),'scripts/test-vscode-keyboard.py','--extension-dir',str(ext),'--binary',str(binary),'--source',str(ROOT/'target/property-preview-fixture/storyboard.laydoc'),*map(str,args)]
tasks={
 'progressive':node('test-web-progressive-playback.cjs',html,a.output/'progressive'),
 'cold':node('test-web-cold-edit.cjs',html,a.output/'cold'),
 'roundtrip':node('test-web-semantic-roundtrip.cjs',baseline,html,a.output/'roundtrip'),
 'first-edit':node('test-web-first-edit.cjs',html,a.output/'first-edit',a.samples),
 'animation':node('test-presentation-animation.cjs',html,a.output/'playback-v009'),
 'animation-edit':['env','EDIT_LCSS=1',*node('test-presentation-animation.cjs',html,a.output/'playback-edit-v009')],
 'native-animation':native('--animation-rounds',2,'--output',a.output/'native-animation-v009.json'),
 'native-keyboard':native('--samples',a.samples,'--output',a.output/'native-keyboard-v009.json'),
 'ordinary-export':node('test-web-export-index.mjs',binary,ROOT/'target/property-preview-fixture/storyboard.laydoc',html,a.output/'ordinary-export'),
}
status=[];state=a.output/'validation-status.json'
if state.exists():status=json.loads(state.read_text())
def utc():return datetime.datetime.now(datetime.timezone.utc).isoformat()
environment={'startedAt':utc(),'platform':platform.platform(),'cpuCount':os.cpu_count(),'affinityCount':len(os.sched_getaffinity(0)),
 'html':html,'htmlSha256':hashlib.sha256(Path(html).read_bytes()).hexdigest(),'nativeSha256':hashlib.sha256(binary.read_bytes()).hexdigest(),
 'memory':[l for l in Path('/proc/meminfo').read_text().splitlines() if l.split(':')[0] in ['MemTotal','MemAvailable','SwapTotal','SwapFree']]}
environment_path=a.output/'environment.json'
if environment_path.exists():
 previous=json.loads(environment_path.read_text())
 for key in ['htmlSha256','nativeSha256']:assert previous[key]==environment[key], 'Do not mix different artifacts in one result directory'
else:environment_path.write_text(json.dumps(environment,indent=2))
for name in a.steps.split(','):
 command=tasks[name];record={'step':name,'command':command,'startedAt':utc()};status.append(record);state.write_text(json.dumps(status,indent=2))
 print('START',name,record['startedAt'],flush=True);start=time.monotonic()
 with (a.output/(name+'.log')).open('w') as log:
  process=subprocess.Popen(command,cwd=ROOT,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
  for line in process.stdout:log.write(line);log.flush();print(line,end='',flush=True)
  record.update(exitCode=process.wait(),finishedAt=utc(),elapsedSeconds=time.monotonic()-start)
 state.write_text(json.dumps(status,indent=2));print('FINISH',name,record['exitCode'],flush=True)
 if record['exitCode']:raise SystemExit(record['exitCode'])
