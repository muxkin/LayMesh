#!/usr/bin/env python3
"""Verify local effect exports, editable PPTX text, LibreOffice round trips and atomic failures."""
import argparse,importlib.util,json,subprocess,tempfile,zipfile
import xml.etree.ElementTree as ET
from pathlib import Path
from PIL import Image,ImageChops,ImageStat
ROOT=Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',type=Path,default=ROOT/'target/debug/laymesh');p.add_argument('--cases',type=Path,default=ROOT/'examples/output/effects3d/browser');p.add_argument('--output',type=Path,default=ROOT/'examples/output/effects3d/exports');a=p.parse_args();out=a.output.resolve();out.mkdir(parents=True,exist_ok=True);binary=a.binary.resolve()
spec=importlib.util.spec_from_file_location('pptx_check',ROOT/'scripts/test-pptx-export.py');check=importlib.util.module_from_spec(spec);spec.loader.exec_module(check)
def run(args):
 r=subprocess.run([str(x) for x in args],capture_output=True,text=True,timeout=180);assert r.returncode==0,r.stderr[-2000:];return r
sources=sorted(a.cases.glob('*.lay'))+[ROOT/'examples/effects/object-3d.lay']
assert len(sources)>=6
result={'status':'running','cases':[],'powerpoint':'not tested: no PowerPoint installation','libreoffice':run(['libreoffice','--version']).stdout.strip()}
try:
 for source in sources:
  name=source.stem
  for ext in ['png','svg','pdf','pptx']:
   dest=out/f'{name}.{ext}';run([binary,'render',source.resolve(),'-o',dest,'--dpi','152.4','--warnings','hide']);assert dest.stat().st_size>100
  info=check.inspect(out/f'{name}.pptx');assert info['pictures']>0
  with zipfile.ZipFile(out/f'{name}.pptx') as deck:
   tree=ET.fromstring(deck.read('ppt/slides/slide1.xml'));sizes=[(int(e.attrib['cx']),int(e.attrib['cy'])) for e in tree.findall('.//p:pic/p:spPr/a:xfrm/a:ext',check.NS)]
   assert sizes and all(w*h<info['size_emu'][0]*info['size_emu'][1]*0.8 for w,h in sizes),(name,sizes)
   info['local_picture_sizes_emu']=sizes
  if name=='object-3d':assert any('Shape, type & image' in t for t in info['text_box_contents']);assert info['text_boxes']>=7
  result['cases'].append({'name':name,'pptx':info})
 pdfs=out/'libreoffice';pdfs.mkdir(exist_ok=True);rt=out/'roundtrip';rt.mkdir(exist_ok=True)
 decks=[out/f'{c["name"]}.pptx' for c in result['cases']]
 with tempfile.TemporaryDirectory(prefix='laymesh-effects-lo-') as profile:
  lo=['libreoffice',f'-env:UserInstallation={Path(profile).as_uri()}','--headless']
  for d in decks:(pdfs/(d.stem+'.pdf')).unlink(missing_ok=True);(rt/d.name).unlink(missing_ok=True)
  run([*lo,'--convert-to','pdf','--outdir',pdfs,*decks]);run([*lo,'--convert-to','pptx:Impress MS PowerPoint 2007 XML','--outdir',rt,*decks])
  for case in result['cases']:
   name=case['name'];assert (pdfs/f'{name}.pdf').is_file() and (rt/f'{name}.pptx').is_file()
   saved=check.inspect(rt/f'{name}.pptx',roundtrip=True);assert saved['text_box_contents']==case['pptx']['text_box_contents'];assert saved['pictures']==case['pptx']['pictures'];case['roundtrip']={'pictures':saved['pictures'],'text_boxes':saved['text_boxes']}
   with Image.open(out/f'{name}.png') as ref:
    size=ref.size;ref=Image.alpha_composite(Image.new('RGBA',size,'white'),ref.convert('RGBA')).convert('RGB')
    for label,pdf in [('pdf',out/f'{name}.pdf'),('pptx',pdfs/f'{name}.pdf')]:
     prefix=out/f'{name}-{label}';run(['pdftoppm','-scale-to-x',size[0],'-scale-to-y',size[1],'-singlefile','-png',pdf,prefix])
     with Image.open(prefix.with_suffix('.png')) as image:
      diff=ImageChops.difference(ref,image.convert('RGB'));mean=sum(ImageStat.Stat(diff).mean)/3;case[label+'_mean_rgb_delta']=round(mean,5);assert mean<5,(name,label,mean)
 # Oversized 3D exports and impossible geometry must leave existing files intact.
 with tempfile.TemporaryDirectory(prefix='laymesh-effects-atomic-') as folder:
  folder=Path(folder);src=folder/'bad.lay';dest=folder/'existing.pptx'
  for config,dpi in [('depth=3mm,bevel_top=(30mm,1mm)','150'),('depth=3mm,projection="perspective",distance=1mm','150'),('depth=3mm','25400')]:
   src.write_text(f'p=canvas(size=(80mm,60mm))\np.add(rect(size=(50mm,30mm),fill="#48a",three_d=three_d({config})),offset=(10mm,10mm))');dest.write_bytes(b'keep existing destination')
   r=subprocess.run([str(binary),'render',str(src),'-o',str(dest),'--dpi',dpi],capture_output=True);assert r.returncode!=0;assert dest.read_bytes()==b'keep existing destination'
 result['atomic_failure_preserves_destination']=True;result['status']='passed'
except Exception as e:result['status']='failed';result['error']=str(e);raise
finally:(out/'verification.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n')
print(json.dumps({'status':result['status'],'cases':len(result['cases']),'powerpoint':result['powerpoint']},ensure_ascii=False))
