#!/usr/bin/env python3
"""Build the portable web-export runtime before shipping native binaries. No npm."""
import argparse,hashlib,json,re,subprocess,zipfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def build(output=None,skip_wasm=False):
 subprocess.run(['python3',str(ROOT/'scripts/vendor-workspace-ui.py')],cwd=ROOT,check=True)
 for folder in ['fflate','canvaskit','gl-transitions','headless-tree']:
  vendor=ROOT/'vendor'/folder;meta=json.loads((vendor/'manifest.json').read_text())
  for name,digest in meta['files'].items():assert hashlib.sha256((vendor/name).read_bytes()).hexdigest()==digest,f'Modified vendored asset: {folder}/{name}'
 output=Path(output or ROOT/'target/web-export/runtime.zip');work=output.parent;work.mkdir(parents=True,exist_ok=True)
 for package in ['laymesh-wasm','laymesh-playback','laymesh-web-export']:
  if not skip_wasm: subprocess.run(['cargo','build','--release','--locked','--target','wasm32-unknown-unknown','-p',package,'--no-default-features'],cwd=ROOT,check=True)
  dest=work/package;dest.mkdir(exist_ok=True)
  subprocess.run(['wasm-bindgen','--target','web','--out-dir',str(dest),str(ROOT/f'target/wasm32-unknown-unknown/release/{package.replace("-","_")}.wasm')],cwd=ROOT,check=True)
 entries={}
 for folder,package in [('wasm','laymesh-playback'),('compiler','laymesh-wasm'),('exporter','laymesh-web-export')]:
  dest=work/package
  entries[f'runtime/{folder}/laymesh.wasm']=(dest/(package.replace('-','_')+'_bg.wasm')).read_bytes()
  s=(dest/(package.replace('-','_')+'.js')).read_text();s=re.sub(r'(?:laymesh_wasm|laymesh_playback|laymesh_web_export)_bg\.wasm','laymesh.wasm',s)
  entries[f'runtime/{folder}/laymesh_wasm.js']=s.encode()
 entries['runtime/skia/canvaskit.wasm']=(ROOT/'vendor/canvaskit/canvaskit.wasm').read_bytes()
 entries['runtime/skia/canvaskit.mjs']=(ROOT/'vendor/canvaskit/canvaskit.js').read_bytes()+b'\nexport default CanvasKitInit;\n'
 for p in (ROOT/'site/live').glob('*.mjs'):entries['runtime/'+p.name]=p.read_bytes()
 entries['runtime/web-player.css']=(ROOT/'site/live/workspace-ui.css').read_bytes()+b'\n'+(ROOT/'site/live/web-player.css').read_bytes()
 entries['runtime/presentation.css']=(ROOT/'site/live/presentation.css').read_bytes()
 for p in (ROOT/'site/vendor').rglob('*'):
  if p.is_file() and p.name in ['index.js','LICENSE']:entries['runtime/vendor/'+p.relative_to(ROOT/'site/vendor').as_posix()]=p.read_bytes()
 fflate=(ROOT/'vendor/fflate/fflate.mjs').read_text();archive=(ROOT/'site/live/web-archive.mjs').read_text();archive=re.sub(r'^import\b[\s\S]*?;\s*','',archive,flags=re.M)
 # Export declarations are valid in the standalone bootstrap's inline module.
 entries['runtime/bootstrap.js']=(fflate+'\n'+archive).encode()
 entries['runtime/web-archive.mjs']=(ROOT/'site/live/web-archive.mjs').read_bytes().replace(b"../../vendor/fflate/fflate.mjs",b"./fflate.mjs")
 entries['runtime/fflate.mjs']=(ROOT/'vendor/fflate/fflate.mjs').read_bytes()
 for path in [ROOT/'LICENSE',ROOT/'vendor/fflate/LICENSE',ROOT/'vendor/canvaskit/LICENSE']:
  entries['licenses/'+path.parent.name+'-LICENSE']=path.read_bytes()
 entries['licenses/DEPENDENCIES.txt']=b'\n'.join(b'\n===== '+p.name.encode()+b' =====\n'+p.read_bytes() for p in sorted((ROOT/'release/licenses').glob('*')) if p.is_file())
 entries['licenses/THIRD_PARTY_NOTICES.md']=(ROOT/'release/THIRD_PARTY_NOTICES.md').read_bytes()
 entries['licenses/workspace-ui-manifest.json']=(ROOT/'vendor/workspace-ui/manifest.json').read_bytes()
 entries['licenses/headless-tree-manifest.json']=(ROOT/'vendor/headless-tree/manifest.json').read_bytes()
 entries['licenses/WEB-RUNTIME.json']=json.dumps({'container':{'implementation':'LayMesh Universal plaintext ZIP-HTML','license':'MIT'},'fflate':json.loads((ROOT/'vendor/fflate/manifest.json').read_text()),'CanvasKit':json.loads((ROOT/'vendor/canvaskit/manifest.json').read_text()),'GLTransitions':json.loads((ROOT/'vendor/gl-transitions/manifest.json').read_text()),'editor_modules':json.loads((ROOT/'site/vendor/manifest.json').read_text())}).encode()
 def closure(editable):
  roots=['runtime/web-player.mjs','runtime/presentation-worker.mjs','runtime/web-archive.mjs','runtime/web-static-export-worker.mjs']
  if editable: roots+=['runtime/web-workspace.mjs','runtime/source-editor.mjs','runtime/web-language-worker.mjs','runtime/web-compiler-worker.mjs']
  selected=set()
  def visit(name):
   if name in selected:return
   if name not in entries:raise RuntimeError('Missing runtime module: '+name)
   selected.add(name)
   if name.endswith(('.mjs','.js')):
    for relative in re.findall(r"(?:^|[\n;])\s*(?:import|export)\s*(?:[^'\";]*?\s+from\s*)?['\"]([^'\"]+)['\"]",entries[name].decode()):
     import posixpath
     target=posixpath.normpath(posixpath.join(posixpath.dirname(name),relative)) if relative.startswith('.') else 'runtime/vendor/'+relative+'/index.js'
     visit(target)
  for name in roots:visit(name)
  selected.update(['runtime/web-player.css','runtime/presentation.css','runtime/bootstrap.js','runtime/wasm/laymesh.wasm','runtime/exporter/laymesh.wasm','runtime/skia/canvaskit.wasm'])
  if editable:selected.add('runtime/compiler/laymesh.wasm')
  return sorted(selected)
 entries['runtime-files.json']=json.dumps({'playback':closure(False),'editable':closure(True)}).encode()
 temporary=output.with_suffix('.zip.tmp')
 with zipfile.ZipFile(temporary,'w',zipfile.ZIP_DEFLATED,compresslevel=9) as z:
  for name,data in sorted(entries.items()):z.writestr(name,data)
 temporary.replace(output)
 print(json.dumps({'runtime':str(output),'compressed_MiB':round(output.stat().st_size/1048576,3),'playback_WASM_bytes':len(entries['runtime/wasm/laymesh.wasm']),'compiler_WASM_bytes':len(entries['runtime/compiler/laymesh.wasm'])}))
 return output
if __name__=='__main__':
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--output',type=Path);p.add_argument('--skip-wasm',action='store_true');a=p.parse_args();build(a.output,a.skip_wasm)
