#!/usr/bin/env python3
"""Repeatable native/legacy CLI measurements; outputs never enter product packages."""
import argparse, hashlib, json, os, pathlib, statistics, subprocess, shutil, tempfile, zipfile
ROOT=pathlib.Path(__file__).resolve().parents[1]
EXAMPLES=['basic.lay','typography.lay','outlines.lay','plot/multi-axes-breaks.lay','plot/statistics.lay','plot/polar-field.lay']
p=argparse.ArgumentParser();p.add_argument('backend',choices=['node','rust']);p.add_argument('--runs',type=int,default=3);p.add_argument('--legacy-wheel',type=pathlib.Path);a=p.parse_args()
out=ROOT/'release/comparison'/a.backend;out.mkdir(parents=True,exist_ok=True)
scratch=tempfile.TemporaryDirectory(prefix='laymesh-benchmark-');scratch_path=pathlib.Path(scratch.name)
profiler=scratch_path/'measure-command'
subprocess.run(['cc','-O2',str(ROOT/'migration/measure-command.c'),'-o',str(profiler)],check=True)
cli=[str(ROOT/'target/release/laymesh')]
if a.backend=='node':
    if not a.legacy_wheel:p.error('--legacy-wheel is required to benchmark the isolated archived Node runtime')
    with zipfile.ZipFile(a.legacy_wheel) as z:
        names=[n for n in z.namelist() if '/laymesh/_vendor/' in n]
        assert all(not pathlib.Path(n).is_absolute() and '..' not in pathlib.Path(n).parts for n in names)
        z.extractall(scratch_path,names)
    node=next(scratch_path.rglob('_vendor/node/node'));node.chmod(0o755)
    entry=next(scratch_path.rglob('_vendor/engine/node_modules/@laymesh/cli/dist/main.js'))
    cli=[str(node),str(entry)]
def measure(args):
    with (out/'stderr.txt').open('w+b') as stderr:
        process=subprocess.run([str(profiler),*cli,*args],cwd=ROOT,stdout=subprocess.PIPE,stderr=stderr)
        if process.returncode:
            stderr.seek(0);raise RuntimeError(stderr.read().decode(errors='replace')[:1000])
    return json.loads(process.stdout)
startup=[measure(['--help']) for _ in range(a.runs)];records=[]
for source in EXAMPLES:
    src=ROOT/'examples'/source;name=source.replace('/','-').replace('.lay','')
    for ext in ['svg','png','pdf']:
        output=out/f'{name}.{ext}';args=['render',str(src),'-o',str(output),'--warnings','hide']
        if ext=='png':args+=['--dpi','144']
        runs=[measure(args) for _ in range(a.runs)]
        records.append({'source':str(src.relative_to(ROOT)),'source_sha256':hashlib.sha256(src.read_bytes()).hexdigest(),'format':ext,'runs':runs,'median_seconds':statistics.median(r['seconds'] for r in runs),'max_peak_rss_kib':max(r['peak_rss_kib'] for r in runs),'output_bytes':output.stat().st_size,'sha256':hashlib.sha256(output.read_bytes()).hexdigest()})
    print(f'{a.backend}: {source}',flush=True)
report={'commit':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),'binary_sha256':hashlib.sha256(pathlib.Path(cli[0]).read_bytes()).hexdigest(),'backend':a.backend,'platform':os.uname().sysname+' '+os.uname().machine,'runs':a.runs,'startup':startup,'startup_median_seconds':statistics.median(r['seconds'] for r in startup),'renders':records}
report['measurement']={'method':'isolated minimal C posix_spawnp/wait4 helper','helper_sha256':hashlib.sha256((ROOT/'migration/measure-command.c').read_bytes()).hexdigest(),'clock':'CLOCK_MONOTONIC from child spawn until wait4 return; helper startup excluded','memory':'child ru_maxrss, including CLI startup; Python report writer high-water mark excluded'}
if a.legacy_wheel:
    report['legacy_wheel_sha256']=hashlib.sha256(a.legacy_wheel.read_bytes()).hexdigest()
    report['node_binary_sha256']=hashlib.sha256(pathlib.Path(cli[0]).read_bytes()).hexdigest()
    report['entrypoint_sha256']=hashlib.sha256(pathlib.Path(cli[-1]).read_bytes()).hexdigest()
(ROOT/'migration'/f'{a.backend}-benchmark.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n')
print(json.dumps({'backend':a.backend,'startup_median_seconds':report['startup_median_seconds'],'outputs':len(records)}))
