#!/usr/bin/env python3
"""Reproduce property preview verification and the painted 50-edit benchmarks."""
import argparse,json,os,subprocess,sys,time
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser(description=__doc__)
for key in ['html','original','binary','extension','fixture','output']:p.add_argument('--'+key,required=True,type=Path)
p.add_argument('--phase',choices=['validation','timing'],required=True)
p.add_argument('--baseline',type=Path,default=ROOT/'docs/benchmarks/semantic-preview-2026-10-15')
a=p.parse_args();out=a.output.resolve();out.mkdir(parents=True,exist_ok=True);logs=out/'logs';logs.mkdir(exist_ok=True);rows=[]
def run(name,command):
 row={'name':name,'command':list(map(str,command)),'started':time.time(),'state':'running'};rows.append(row)
 def save():(out/('status-'+a.phase+'.json')).write_text(json.dumps(rows,indent=2)+'\n')
 save();print('Running '+name,flush=True)
 with (logs/(name+'.log')).open('w') as log:r=subprocess.run(list(map(str,command)),cwd=ROOT,stdout=log,stderr=subprocess.STDOUT)
 row.update(state='passed' if r.returncode==0 else 'failed',seconds=time.time()-row['started'],returncode=r.returncode);save()
 if r.returncode:raise SystemExit('Failed '+name+'; see '+str(logs/(name+'.log')))
 print('Passed '+name+' ('+str(round(row['seconds'],1))+' s)',flush=True)
if a.phase=='validation':
 run('frame-delivery',['node','scripts/test-presentation-frame-delivery.mjs'])
 run('cold',['node','scripts/test-web-cold-edit.cjs',a.html,out/'cold'])
 run('reuse',['node','scripts/test-web-semantic-reuse.cjs',a.html,out/'reuse'])
 run('edit-oracle',['node','scripts/test-web-semantic-edit-oracle.cjs',a.html,out/'edit-oracle'])
 run('style-oracle',['node','scripts/test-web-semantic-edit-oracle.cjs',a.html,out/'style-oracle','styles'])
 run('roundtrip',['node','scripts/test-web-semantic-roundtrip.cjs',a.original,a.html,out/'roundtrip'])
 run('native-deck',['node','scripts/test-presentation-incremental.cjs'])
 run('retained-deck',['node','scripts/test-retained-presentation-store.mjs'])
else:
 # Keep all timing stages sequential, without builds or validation in parallel.
 run('browser',['node','scripts/test-web-semantic-performance.cjs',a.html,out/'browser','50'])
 run('browser-document',['node','scripts/test-web-document-performance.cjs',a.html,out/'browser-document','50'])
 run('vscode',[sys.executable,'scripts/test-vscode-semantic-performance.py','--fixture',a.fixture,'--output',out/'vscode','--binary',a.binary,'--extension-dir',a.extension])
 run('vscode-document',[sys.executable,'scripts/test-vscode-document-performance.py','--fixture',a.fixture,'--output',out/'vscode-document','--binary',a.binary,'--extension-dir',a.extension])
 run('summary',[sys.executable,'scripts/summarize-semantic-preview-performance.py','--input',out,'--output',out])
 run('comparison',[sys.executable,'scripts/summarize-property-preview.py','--baseline',a.baseline,'--input',out])
