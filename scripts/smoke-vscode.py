#!/usr/bin/env python3
"""Run editing checks in VS Code's own extension host, without npm or Node tools."""
import argparse,hashlib,json,os,subprocess,tempfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--code',default='/usr/share/code/code',help='Actual VS Code executable, not the shell CLI wrapper');p.add_argument('--headless',action='store_true');p.add_argument('--output',type=Path,default=ROOT/'release/comparison/vscode-verification.json');a=p.parse_args();a.output=a.output.resolve();a.output.parent.mkdir(parents=True,exist_ok=True);a.output.unlink(missing_ok=True)
with tempfile.TemporaryDirectory(prefix='laymesh-vscode-host-') as temp:
 root=Path(temp);env={**os.environ,'LAYMESH_VSCODE_EVIDENCE':str(a.output)};env.pop('ELECTRON_RUN_AS_NODE',None)
 # The host starts its own executable and the bundled Rust server by absolute path.
 empty=root/'empty-path';empty.mkdir();env['PATH']=str(empty)
 command=[a.code,'--no-sandbox','--disable-gpu','--disable-updates','--user-data-dir='+str(root/'profile'),'--extensions-dir='+str(root/'extensions'),'--extensionDevelopmentPath='+str(ROOT/'extensions/vscode'),'--extensionTestsPath='+str(ROOT/'scripts/vscode-smoke.cjs'),'--skip-welcome','--skip-release-notes','--new-window']
 if a.headless:command+=['--ozone-platform=headless']
 completed=subprocess.run(command,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True,timeout=90)
 if completed.returncode or not a.output.is_file():raise RuntimeError('VS Code host verification failed:\n'+completed.stdout[-5000:])
 result=json.loads(a.output.read_text());assert len(result['tests'])==26
 files=[Path(__file__),ROOT/'scripts/vscode-smoke.cjs',ROOT/'extensions/vscode/src/client.cjs',ROOT/'extensions/vscode/bin'/('laymesh.exe' if os.name=='nt' else 'laymesh')]
 result['artifact_sha256']={str(p.resolve().relative_to(ROOT)):hashlib.sha256(p.read_bytes()).hexdigest() for p in files}
 a.output.write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n');print(json.dumps(result,ensure_ascii=False))
