#!/usr/bin/env python3
"""Render actual 119-page offline SVG/PDF downloads and compare with native."""
import argparse,json,shutil,subprocess,zipfile
from pathlib import Path
import numpy as np
from PIL import Image,ImageFilter
p=argparse.ArgumentParser(description=__doc__);p.add_argument('matrix',type=Path);p.add_argument('offline',type=Path);p.add_argument('--check',action='store_true');a=p.parse_args();m=json.loads((a.matrix/'manifest.json').read_text());rows=[]
def pixels(path):return np.array(Image.open(path).convert('RGB')).astype('int16')
def metrics(a,b):
 assert a.shape==b.shape,(a.shape,b.shape)
 d=abs(a-b);edge=np.zeros(a.shape[:2],bool)
 for image in [a,b]:
  dx=abs(image[:,1:]-image[:,:-1]).max(2)>12;dy=abs(image[1:]-image[:-1]).max(2)>12;edge[:,1:]|=dx;edge[:,:-1]|=dx;edge[1:]|=dy;edge[:-1]|=dy
 edge=np.array(Image.fromarray((edge*255).astype('uint8')).filter(ImageFilter.MaxFilter(5)))>0;edge[:2]=True;edge[-2:]=True;edge[:,:2]=True;edge[:,-2:]=True
 return {'meanChannel':round(float(d.mean()),6),'maxChannel':int(d.max()),'smoothMaxChannel':int(d[~edge].max()),'smoothPixelsOver3':int((d[~edge].max(1)>3).sum())}
for mode in ['playback','editable']:
 out=a.offline/mode;out.mkdir(exist_ok=True);(out/'manifest.json').write_text(json.dumps(m))
 with zipfile.ZipFile(a.offline/(mode+'.svg.zip')) as z:
  files=sorted(n for n in z.namelist() if n.endswith('.svg'));assert len(files)==len(m['cases'])==119
  for c,f in zip(m['cases'],files):(out/(c['name']+'.svg')).write_bytes(z.read(f))
 subprocess.run(['node','scripts/render-gradient-vectors.cjs',str(out)],check=True)
 subprocess.run(['pdftocairo','-png','-r','96',str(a.offline/(mode+'.pdf')),str(out/'pdf-page')],check=True,stderr=subprocess.DEVNULL)
 pages=sorted(out.glob('pdf-page-*.png'));assert len(pages)==119
 for c,pdf in zip(m['cases'],pages):
  ref=pixels(a.matrix/(c['name']+'-native.png'))
  rows.append({'mode':mode,'name':c['name'],'svg':metrics(pixels(out/(c['name']+'-svg.png')),ref),'pdf':metrics(pixels(pdf),ref)})
report={'conditions':'119 pages, 96 DPI; Chromium SVG; Poppler Cairo PDF; sRGB; same smooth/edge rule as primary matrix','cases':rows}
(a.offline/'verification.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n')
for mode in ['playback','editable']:
 for fmt in ['svg','pdf']:
  bad=[(r['name'],r[fmt]['smoothMaxChannel'],r[fmt]['smoothPixelsOver3']) for r in rows if r['mode']==mode and r[fmt]['smoothMaxChannel']>3]
  print(mode,fmt,'over 3:',bad,flush=True)

if a.check:assert all(r[f]["smoothMaxChannel"]<=3 for r in rows for f in ["svg","pdf"]),"Offline export parity failed; see verification.json"
