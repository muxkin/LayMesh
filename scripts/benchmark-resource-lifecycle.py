#!/usr/bin/env python3
"""Reproduce document-owned caching checks and 50-sample preview/navigation timings.

Run validation before timing; keep timing sequential with no competing builds.
The validation site must be served on the user's Tailscale address.
"""
import argparse,json,os,subprocess,time,sys,hashlib
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser(description=__doc__)
for key in ['html','baseline','binary','extension','baseline-extension','fixture','output']:p.add_argument('--'+key,required=True,type=Path)
p.add_argument('--phase',choices=['validation','timing'],required=True)
p.add_argument('--site-url',help='Tailscale site/dist server, required for rendering validation')
p.add_argument('--resume',action='store_true',help='Reuse passed stages from this output directory')
a=p.parse_args();out=a.output.resolve();out.mkdir(parents=True,exist_ok=True);logs=out/'logs';logs.mkdir(exist_ok=True)
# Resume is valid only for these exact artifacts and fixtures, not merely the
# same filenames. Keep source/compiler changes out of mixed timing summaries.
def fingerprint(path):
 path=path.resolve();digest=hashlib.sha256()
 for f in sorted(path.rglob('*')) if path.is_dir() else [path]:
  if f.is_file():
   digest.update(str(f.relative_to(path) if path.is_dir() else f.name).encode())
   digest.update(hashlib.sha256(f.read_bytes()).digest())
 return digest.hexdigest()
inputs={key:fingerprint(getattr(a,key)) for key in ['html','baseline','binary','extension','baseline_extension','fixture']}
record=out/'run-inputs.json'
if a.resume and record.exists() and json.loads(record.read_text())!=inputs:p.error('Artifacts changed; use a new output directory for this build')
record.write_text(json.dumps(inputs,indent=2)+'\n')
status=out/('status-'+a.phase+'.json');old=json.loads(status.read_text()) if a.resume and status.exists() else [];rows=[]
env={**os.environ,'SAMPLES':'50','LAYMESH_TEST_EXTENSION_DIR':str(a.extension.resolve()),'LAYMESH_TEST_BINARY':str(a.binary.resolve())}
def run(name,args):
 args=list(map(str,args));previous=next((r for r in old if r['name']==name and r['state']=='passed' and r['command']==args),None)
 if previous:rows.append(previous);status.write_text(json.dumps(rows,indent=2)+'\n');print('REUSE',name,flush=True);return
 row={'name':name,'command':args,'started':time.time(),'state':'running'};rows.append(row)
 def save():status.write_text(json.dumps(rows,indent=2)+'\n')
 save();print('START',name,flush=True)
 with (logs/(name+'.log')).open('w') as log:r=subprocess.run(args,cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT)
 row.update(state='passed' if not r.returncode else 'failed',seconds=time.time()-row['started']);save();print('END',name,row['state'],flush=True)
 if r.returncode:raise SystemExit('Failed '+name+'; see '+str(logs/(name+'.log')))
if a.phase=='validation':
 if not a.site_url:p.error('--site-url is required for validation')
 run('core',['cargo','test','--offline','-p','laymesh-core','--lib','--test','compile_session'])
 run('text-capacity',['cargo','test','--release','--offline','-p','laymesh-core','--test','cache_lifetime','--','--nocapture'])
 run('render',['cargo','test','--offline','-p','laymesh-render','--test','image_policy','--test','incremental_lowering','--test','display_updates','--test','transform_planning','--test','preview_surface'])
 for f in ['presentation-navigation','presentation-frame-delivery','presentation-resources','presentation-clock','display-update','retained-presentation-store','presentation-lifetime','cache-ownership','skia-update-recovery']:run(f,['node','scripts/test-'+f+'.mjs'])
 run('browser-input-validation',['node','scripts/test-web-incremental-preview.cjs',out/'incremental-input'])
 run('native-deck',['node','scripts/test-presentation-incremental.cjs'])
 run('cold',['node','scripts/test-web-cold-edit.cjs',a.html,out/'cold'])
 run('reuse',['node','scripts/test-web-semantic-reuse.cjs',a.html,out/'reuse'])
 run('style-oracle',['node','scripts/test-web-semantic-edit-oracle.cjs',a.html,out/'style-oracle','styles'])
 run('roundtrip',['node','scripts/test-web-semantic-roundtrip.cjs',a.baseline,a.html,out/'roundtrip'])
 run('vscode-ui',[sys.executable,'scripts/test-vscode-preview.py','--extension-dir',a.extension,'--output',out/'vscode-ui','--block-worker-resource-fetch'])
 run('native-host',['node','scripts/test-presentation-host.cjs',out/'served'])
 run('clipping',['node','scripts/test-presentation-clipping.cjs',a.site_url,out/'clipping.json'])
 run('layer-cache',['node','scripts/test-skia-layer-cache.cjs',a.site_url,out/'layer-cache.json'])
 run('static-cache',['node','scripts/test-skia-static-cache.cjs',a.site_url,out/'static-cache.json','--lifecycle-only'])
else:
 baseline=out/'instrumented-baseline.html';stage=out/'instrumented-baseline-extension'
 if not baseline.exists():run('instrument-baseline',['node','scripts/instrument-presentation-baseline.mjs',a.baseline,baseline])
 if not stage.exists():run('instrument-native-baseline',['node','scripts/instrument-presentation-baseline.mjs',a.baseline_extension,stage])
 run('keyboard-before',['node','scripts/test-presentation-keyboard.cjs',baseline,out/'keyboard-before'])
 run('keyboard-after',['node','scripts/test-presentation-keyboard.cjs',a.html,out/'keyboard-after'])
 for label,extension,binary in [('before',stage,stage/'bin/laymesh'),('after',a.extension,a.binary)]:run('native-keyboard-'+label,[sys.executable,'scripts/test-vscode-keyboard.py','--output',out/'native-keyboard'/(label+'.json'),'--source',a.fixture/'storyboard.laydoc','--binary',binary,'--extension-dir',extension])
 run('browser-lay',['node','scripts/test-web-semantic-performance.cjs',a.html,out/'browser','50'])
 run('browser-document',['node','scripts/test-web-document-performance.cjs',a.html,out/'browser-document','50'])
 run('native-lay',[sys.executable,'scripts/test-vscode-semantic-performance.py','--fixture',a.fixture,'--output',out/'vscode','--binary',a.binary,'--extension-dir',a.extension])
 run('native-document',[sys.executable,'scripts/test-vscode-document-performance.py','--fixture',a.fixture,'--output',out/'vscode-document','--binary',a.binary,'--extension-dir',a.extension])
 run('navigation-summary',[sys.executable,'scripts/summarize-resource-lifecycle.py',out])
 run('summary',[sys.executable,'scripts/summarize-semantic-preview-performance.py','--input',out,'--output',out])
