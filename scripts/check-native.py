#!/usr/bin/env python3
"""Build and test the native workspace with no Node/npm on PATH.

All Cargo dependencies must already be cached (--offline --locked). The test
font fixtures stay in the checkout and are not copied into release products.
"""
from pathlib import Path
import json, os, shutil, subprocess, tempfile, time
ROOT=Path(__file__).resolve().parents[1]
def main():
    cargo=Path(subprocess.check_output(['rustup','which','cargo'],text=True).strip())
    rustc=cargo.with_name('rustc')
    report={'rustc':subprocess.check_output([str(rustc),'--version'],text=True).strip(),'offline':True,'commands':[]}
    logs=ROOT/'release/comparison/no-node';logs.mkdir(parents=True,exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='laymesh-no-node-') as directory:
        path=Path(directory)
        names=['cc','gcc','c++','g++','ld','ar','as','ranlib','pkg-config','make','cmake','ninja','python3','pdftotext','pdftoppm','mutool','pdfimages']
        for name in names:
            if executable:=shutil.which(name): (path/name).symlink_to(executable)
        for name in ['cargo','rustc','rustdoc']:(path/name).symlink_to(cargo.with_name(name))
        env=dict(os.environ,PATH=str(path),RUSTC=str(rustc),RUSTDOC=str(cargo.with_name('rustdoc')))
        report['path_tools']=sorted(p.name for p in path.iterdir())
        report['node_available']=shutil.which('node',path=str(path))
        report['npm_available']=shutil.which('npm',path=str(path))
        assert report['node_available'] is None and report['npm_available'] is None
        for label,command in [
            ('tests',[str(path/'python3'),'scripts/test-contracts.py','--offline','--log',str(logs/'cargo-tests.log')]),
            ('release',[str(cargo),'build','--offline','--locked','--release','-p','laymesh-cli']),
            ('wasm',[str(cargo),'build','--offline','--locked','--release','--target','wasm32-unknown-unknown','-p','laymesh-wasm','--no-default-features']),
        ]:
            start=time.monotonic()
            with (logs/f'{label}.log').open('wb') as output:
                status=subprocess.run(command,cwd=ROOT,env=env,stdout=output,stderr=subprocess.STDOUT).returncode
            report['commands'].append({'label':label,'command':command[1:],'status':status,'elapsed_s':time.monotonic()-start})
            (logs/'verification.json').write_text(json.dumps(report,indent=2)+'\n')
            print(f'{label}: exit={status}',flush=True)
            if status:raise SystemExit(status)
    print('Native locked/offline build and tests passed with Node/npm absent from PATH.')
if __name__=='__main__':main()
