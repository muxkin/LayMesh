#!/usr/bin/env python3
"""Regenerate responsive documentation media through the native Rust renderer."""
import argparse,hashlib,json,os,subprocess,tempfile
from pathlib import Path
from PIL import Image
from site_support import ROOT,examples,dependencies,fingerprint,engine_files
RECIPE={'widths':[960,1920,3840],'format':'webp','lossless':True,'version':2,'engine':'rust-ratex'}
def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--check',action='store_true');p.add_argument('--binary',type=Path,default=ROOT/'target/release'/('laymesh.exe' if os.name=='nt' else 'laymesh'));a=p.parse_args();dest=ROOT/'site/media';dest.mkdir(parents=True,exist_ok=True);manifest_path=dest/'manifest.json'
 inputs=examples();seen={e['source'] for e in inputs}
 for doc in [ROOT/'README.md',ROOT/'README.en.md',*(ROOT/'docs').rglob('*.md')]:
  import re,posixpath
  for url in re.findall(r'!\[[^]]*\]\(([^)]+)\)',doc.read_text()):
   if url.startswith(('https:','http:','data:')):continue
   source=posixpath.normpath(posixpath.join(doc.parent.relative_to(ROOT).as_posix(),url))
   layout=re.sub(r'(?:-preview)?\.(?:png|jpg|jpeg|webp)$','.lay',source)
   if (ROOT/layout).is_file():source=layout
   if source in seen or not (ROOT/source).is_file() or source.startswith('site/media/'):continue
   seen.add(source);inputs.append({'source':source,'id':source.replace('/','-').rsplit('.',1)[0],'bitmap':not source.endswith('.lay')})
 manifest={'recipe':RECIPE,'engine':fingerprint(engine_files()),'items':{}}
 if a.check:
  old=json.loads(manifest_path.read_text());assert old['engine']==manifest['engine'],'Renderer changed; rebuild media'
  for e in inputs:
   item=old['items'][e['source']];deps=dependencies(e['source']);assert item['fingerprint']==fingerprint(deps),f'Stale source: {e["source"]}'
   for v in item['variants']:
    file=ROOT/v['file'];assert hashlib.sha256(file.read_bytes()).hexdigest()==v['sha256']
    with Image.open(file) as image:assert image.size==(v['width'],v['height']) and image.format=='WEBP'
  print(f'Checked {len(inputs)} media sources');return
 previous=json.loads(manifest_path.read_text()) if manifest_path.is_file() else {}
 for e in inputs:
  source=e['source'];deps=dependencies(source)
  cached=previous.get('items',{}).get(source)
  if previous.get('engine')==manifest['engine'] and previous.get('recipe')==RECIPE and cached and cached['fingerprint']==fingerprint(deps) and all((ROOT/v['file']).is_file() and hashlib.sha256((ROOT/v['file']).read_bytes()).hexdigest()==v['sha256'] for v in cached['variants']):
   manifest['items'][source]=cached;continue
  with tempfile.TemporaryDirectory(prefix='laymesh-media-') as tmp:
   if e.get('bitmap'):image=Image.open(ROOT/source).convert('RGBA')
   else:
    result=subprocess.run([str(a.binary),'inspect',str(ROOT/source),'--json'],capture_output=True,text=True,check=True);width=json.loads(result.stdout)['page']['width'];png=Path(tmp)/'figure.png';subprocess.run([str(a.binary),'render',str(ROOT/source),'-o',str(png),'--dpi',str(3840*25.4/width),'--warnings','hide'],check=True,stdout=subprocess.DEVNULL);image=Image.open(png).convert('RGBA')
   variants=[]
   for width in sorted({min(w,image.width) for w in RECIPE['widths']}):
    resized=image.resize((width,round(image.height*width/image.width)),Image.Resampling.LANCZOS);file=dest/f'{e["id"]}-{width}.webp';resized.save(file,'WEBP',lossless=True,method=4);variants.append({'file':file.relative_to(ROOT).as_posix(),'width':resized.width,'height':resized.height,'sha256':hashlib.sha256(file.read_bytes()).hexdigest()})
  manifest['items'][source]={'fingerprint':fingerprint(deps),'dependencies':deps,'variants':variants};print(f'WebP {source}')
 manifest_path.write_text(json.dumps(manifest,indent=2,ensure_ascii=False)+'\n')
if __name__=='__main__':main()
