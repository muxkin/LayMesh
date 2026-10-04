#!/usr/bin/env python3
"""Render and check every focused gallery entry with the native Rust engine."""
import argparse,json,os,subprocess,tempfile
from pathlib import Path
from PIL import Image
from site_support import ROOT
p=argparse.ArgumentParser(description=__doc__);mode=p.add_mutually_exclusive_group(required=True);mode.add_argument('--write',action='store_true');mode.add_argument('--check',action='store_true');p.add_argument('--binary',type=Path,default=ROOT/'target/release'/('laymesh.exe' if os.name=='nt' else 'laymesh'));a=p.parse_args();catalog=json.loads((ROOT/'examples/gallery/catalog.json').read_text());count=0
for category in catalog['categories']:
 for item in category['items']:
  source=ROOT/item.get('source',f'examples/gallery/{category["slug"]}/{item["slug"]}.lay');png=source.with_suffix('.png')
  subprocess.run([str(a.binary),'validate',str(source),'--warnings','hide'],check=True,stdout=subprocess.DEVNULL)
  with tempfile.TemporaryDirectory(prefix='laymesh-gallery-') as tmp:
   rendered=Path(tmp)/'preview.png';subprocess.run([str(a.binary),'render',str(source),'-o',str(rendered),'--dpi',str(catalog['dpi']),'--warnings','hide'],check=True,stdout=subprocess.DEVNULL)
   if a.write:png.write_bytes(rendered.read_bytes())
   else:
    with Image.open(png) as old,Image.open(rendered) as current:
     assert old.size==current.size and old.convert('RGBA').tobytes()==current.convert('RGBA').tobytes(),f'Stale gallery image: {png.relative_to(ROOT)}'
  count+=1
print(f'{"Wrote" if a.write else "Checked"} {count} native Rust gallery renders')
