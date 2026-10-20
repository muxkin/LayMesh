#!/usr/bin/env python3
"""Package measured records and their reproduction scripts, without build caches."""
import argparse,hashlib,json,os,tempfile,zipfile
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('records',type=Path);p.add_argument('output',type=Path)
a=p.parse_args();files={}
def add(path,name):
    if path.is_file():files[name]=path
for path in a.records.rglob('*'):
    if path.suffix not in ['.json','.png','.patch'] or path.name.endswith('-progress.json') or path.name=='progress.json':continue
    if any(part.startswith('report') for part in path.relative_to(a.records).parts):continue
    add(path,'records/'+path.relative_to(a.records).as_posix())
for name in ['validation-runner.py','resume-validation.py','freeze-snapshot.py','finalize-delivery.py','retry-webkit.py','validate.py','delivery-smoke.cjs']:
    add(a.records/name,'records/'+name)
add(a.records/'diagnostic-profiles/rejected-variants.md','records/diagnostic-profiles/rejected-variants.md')
manifest=json.loads((a.records/'source-manifest.json').read_text())
qa=a.records.parent/'browser-qa'
for path in qa.rglob('*'):
    if path.is_file():add(path,'qa/'+path.relative_to(qa).as_posix())
for name,expected in manifest['files'].items():
    path=a.records/'source'/name
    if not path.is_file():raise FileNotFoundError('Missing frozen source: '+str(path))
    actual=hashlib.sha256(path.read_bytes()).hexdigest()
    digest=expected.get('sha256') if isinstance(expected,dict) else expected
    if actual!=digest:raise ValueError('Frozen source hash mismatch: '+name)
    add(path,'source/'+name)
for name in ['benchmark-openprops.py','collect-openprops-validation.py','summarize-openprops.py','report-openprops.py','package-openprops-verification.py','browser-process-memory.cjs','test-browser-process-memory.cjs','test-static-opaque-performance.cjs','test-openprops-performance.cjs','test-openprops-cold-navigation.cjs','test-openprops-interaction.cjs','test-openprops-quality.cjs','test-animation-cache-quality.cjs','test-shadow-mask.mjs','test-presentation-preparation.cjs','test-presentation-clipping.cjs','test-skia-layer-cache.cjs','test-skia-static-cache.cjs','test-cache-ownership.mjs','test-presentation-workers.mjs','test-presentation-resources.mjs','test-presentation-frame-delivery.mjs','test-presentation-pixels.cjs','test-presentation-pixels.mjs','test-presentation-scheduling.mjs','test-fade-performance.cjs','test-layer-blit-performance.cjs','test-layer-density-performance.cjs','test-cpu-layer-blit-performance.cjs','test-cpu-layer-blit.cjs','openprops-phase-probe.cjs']:
    add(ROOT/'scripts'/name,'source/scripts/'+name)
add(ROOT/'scripts/serve-preview.py','source/scripts/serve-preview.py')
for name in ['profile-presentation-composition.cjs','profile-native-canvas.cjs','diagnose-openprops-backend-aa.cjs','test-native-presentation.cjs','test-openprops-first-entry.cjs','report-openprops-followup.py']:
    add(ROOT/'scripts'/name,'source/scripts/'+name)
for name in ['README.zh-CN.md','results.zh-CN.md','followup.zh-CN.md','followup.html','per-page.csv','backend-aa.html']:
    add(ROOT/'docs/benchmarks/animation-resources-2026-10-19'/name,'documentation/'+name)
for path in (ROOT/'docs/benchmarks/animation-resources-2026-10-19/aa').glob('*.png'):
    add(path,'documentation/aa/'+path.name)
a.output.parent.mkdir(parents=True,exist_ok=True)
handle=tempfile.NamedTemporaryFile(dir=a.output.parent,prefix=a.output.name+'.',suffix='.tmp',delete=False);handle.close();temporary=Path(handle.name)
try:
    entries={}
    with zipfile.ZipFile(temporary,'w',compression=zipfile.ZIP_DEFLATED,compresslevel=6)as archive:
        for name,path in sorted(files.items()):
            data=path.read_bytes();entries[name]={'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest()};archive.writestr(name,data)
        archive.writestr('manifest.json',json.dumps(entries,indent=2,ensure_ascii=False)+'\n')
        archive.writestr('README.txt','Open Props animation verification\n\nThe three original/current/optimized editable HTML files are distributed alongside this archive.\nTheir SHA-256 identities are preserved in records/preservation.json and individual results.\nEach browser run is sequential. Natural playback and early input stress tests are separate.\nUse a fresh output directory for a new experiment; do not replace historical measurements.\nRead documentation/README.zh-CN.md for commands and interpretation boundaries.\nSource files are an implementation snapshot, not a release or a standalone repository.\n')
    with zipfile.ZipFile(temporary)as archive:
        bad=archive.testzip()
        if bad:raise RuntimeError('Invalid verification member: '+bad)
    os.replace(temporary,a.output)
finally:
    temporary.unlink(missing_ok=True)
print(json.dumps({'path':str(a.output),'files':len(files),'bytes':a.output.stat().st_size,'sha256':hashlib.sha256(a.output.read_bytes()).hexdigest()}))
