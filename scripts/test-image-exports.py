#!/usr/bin/env python3
"""Check actual native raster codecs, CLI options and editor export transport, optionally via Wine."""
import argparse, hashlib, json, os, subprocess, tempfile
from pathlib import Path
from PIL import Image

parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--binary',type=Path,required=True)
parser.add_argument('--wine',type=Path)
parser.add_argument('--output',type=Path,required=True)
args=parser.parse_args();binary=args.binary.resolve();command=([str(args.wine)] if args.wine else [])+[str(binary)]
def host(path):return ('Z:' if args.wine else '')+str(path.resolve()).replace('\\','/')
checks=[]
with tempfile.TemporaryDirectory(prefix='laymesh-export-codecs-') as temporary:
 root=Path(temporary)/'中文 # 图形';root.mkdir();file=root/'figure.lay'
 source='page=canvas(size=(25.4mm,12.7mm),background="none")\npage.add(rect(size=(10mm,10mm),fill="#ff000080",border_width=0),offset=(1mm,1mm))'
 file.write_text(source,encoding='utf-8')
 env={**os.environ,'PATH':str(root/'empty-path'),'LAYMESH_NO_SYSTEM_FONTS':'1'}
 def run(arguments,input=None,code=0):
  result=subprocess.run(command+arguments,input=input,capture_output=True,env=env,timeout=60)
  assert result.returncode==code,result.stderr.decode(errors='replace')[-2000:]
  return result.stdout
 version=run(['--version']).decode().strip()
 formats=[('png',['--compression','best']),('jpg',['--quality','95','--background','#0000ff']),('jpeg',[]),('tif',['--compression','deflate']),('tiff',['--compression','packbits']),('webp',['--webp-lossless','true','--quality','100','--webp-method','6']),('bmp',[]),('gif',[]),('ico',[]),('pnm',[]),('pam',[]),('ppm',[]),('pgm',[]),('pbm',[]),('tga',[])]
 for extension,options in formats:
  output=root/('输出.'+extension)
  run(['render',host(file),'-o',host(output),'--dpi','144',*options])
  if extension in ('pam','pnm'):
   assert output.read_bytes().startswith(b'P7\n');continue
  with Image.open(output) as image:
   assert image.size==(144,72),(extension,image.size)
   rgba=image.convert('RGBA');assert rgba.getpixel((140,70))[3]==(255 if extension in ('jpg','jpeg','ppm','pgm','pbm') else 0),extension
   if extension in ('png','tif','tiff','webp','bmp','ico','tga'):assert rgba.getpixel((25,25))==(255,0,0,128),extension
   if extension in ('png','jpg','jpeg','tif','tiff','bmp'):assert abs(image.info['dpi'][0]-144)<0.02,(extension,image.info)
  checks.append('CLI '+extension+' dimensions, alpha and density')
 for compression,tag in [('none',1),('lzw',5),('deflate',8),('packbits',32773)]:
  output=root/'compression.tif';run(['render',host(file),'-o',host(output),'--dpi','144','--compression',compression])
  with Image.open(output) as image:assert image.tag_v2[259]==tag
 checks.append('Four lossless TIFF compression tags')
 for lossless,extra in [('true',['--webp-near-lossless','60']),('false',['--webp-alpha-quality','100'])]:
  output=root/'options.webp';run(['render',host(file),'-o',host(output),'--dpi','144','--webp-lossless',lossless,'--quality','87','--webp-method','5',*extra])
  with Image.open(output) as image:assert image.size==(144,72) and image.getpixel((25,25))[3]==128
 checks.append('Lossy and near-lossless WebP options')
 output=root/'protected.png';output.write_bytes(b'existing')
 for options in [['--quality','95'],['--dpi','NaN'],['--compression','lzw'],['--webp-method','4']]:
  run(['render',host(file),'-o',host(output),*options],code=2);assert output.read_bytes()==b'existing'
 checks.append('Invalid parameters preserve an existing file')
 module=root/'colors.lay';module.write_text('export shade="#000000"');output=root/'unsaved.png'
 request=dict(id=1,type='export',file=host(file),source='import {shade} from "./colors.lay"\npage=canvas(size=(25.4mm,12.7mm),background=shade)',overlays={host(module):'export shade="#12ab34"'},output=host(output),options=dict(dpi=144))
 results=[json.loads(line) for line in run(['preview','--stdio'],(json.dumps(request,ensure_ascii=False)+'\n').encode()).decode().splitlines()]
 assert results[1]['exported']==host(output),results[1]
 with Image.open(output) as image:assert image.getpixel((70,30))==(18,171,52,255)
 assert module.read_text()=='export shade="#000000"'
 assert not list(root.glob('*.tmp'))
 checks.append('Editor exports current unsaved entry and imports')
 evidence=dict(version=version,binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),wine=bool(args.wine),empty_path=True,formats=[name for name,_ in formats],checks=checks,windows_vscode_host_tested=False if args.wine else None)
 args.output.parent.mkdir(parents=True,exist_ok=True);args.output.write_text(json.dumps(evidence,ensure_ascii=False,indent=2)+'\n');print(json.dumps(evidence,ensure_ascii=False))
