#!/usr/bin/env python3
"""Native CLI/OOXML/LibreOffice QA of this particular recreation."""
from pathlib import Path
import argparse, hashlib, importlib.util, json, subprocess, tempfile, zipfile
from PIL import Image, ImageChops, ImageStat, ImageDraw
ROOT=Path(__file__).resolve().parents[3]
SRC=Path(__file__).resolve().parent
parser=argparse.ArgumentParser();parser.add_argument('--output',type=Path,default=ROOT/'examples/output/research-cycle');args=parser.parse_args()
OUT=args.output.resolve();OUT.mkdir(parents=True,exist_ok=True)
BINARY=ROOT/'target/release/laymesh'
spec=importlib.util.spec_from_file_location('check',ROOT/'scripts/test-pptx-export.py');check=importlib.util.module_from_spec(spec);spec.loader.exec_module(check);run=check.run
for fmt in ['pdf','pptx','png']:
 cmd=[BINARY,'render',SRC/'research-cycle.lay','-o',OUT/f'research-cycle.{fmt}']
 if fmt=='png':cmd+=['--dpi','142.875']
 result=run(cmd)
 if fmt=='pptx':warnings=result.stderr.strip().splitlines()
assert not any('W_PPTX_RASTER' in w for w in warnings),warnings
actual=check.inspect(OUT/'research-cycle.pptx')
labels=json.loads((SRC/'labels.json').read_text())
assert actual['text_box_contents']==labels,(len(labels),actual['text_box_contents'])
assert actual['pictures']==3 and len(actual['media'])==3
assert actual['groups']==6
assert all(abs(a-b)<2 for a,b in zip(actual['size_emu'],[int(1998*200/1125*36000),7200000]))
with zipfile.ZipFile(OUT/'research-cycle.pptx') as z:
 xml=check.ET.fromstring(z.read('ppt/slides/slide1.xml'))
 assert all(p.find('p:spPr/a:prstGeom',check.NS).attrib['prst']=='roundRect' for p in xml.findall('.//p:pic',check.NS))
 presets=[p.attrib['prst'] for p in xml.findall('.//a:prstGeom',check.NS)]
 assert {'rect','roundRect','ellipse','line'}<=set(presets)
 assert xml.findall('.//a:gradFill',check.NS)
for f in ['verification','roundtrip']:(OUT/f).mkdir(exist_ok=True)
with tempfile.TemporaryDirectory(prefix='laymesh-cycle-lo-') as profile:
 lo=['libreoffice',f'-env:UserInstallation={Path(profile).as_uri()}','--headless']
 for folder,fmt,ext in [('verification','pdf','pdf'),('roundtrip','pptx:Impress MS PowerPoint 2007 XML','pptx')]:
  destination=OUT/folder/f'research-cycle.{ext}';destination.unlink(missing_ok=True)
  run([*lo,'--convert-to',fmt,'--outdir',OUT/folder,OUT/'research-cycle.pptx']);assert destination.is_file()
saved=check.inspect(OUT/'roundtrip/research-cycle.pptx',roundtrip=True)
assert saved['text_box_contents']==actual['text_box_contents']
assert saved['shapes']+saved['pictures']==actual['shapes']+actual['pictures']
assert saved['groups']==actual['groups']
with tempfile.TemporaryDirectory(prefix='laymesh-cycle-save-lo-') as profile:
 dest=OUT/'roundtrip/research-cycle.pdf';dest.unlink(missing_ok=True)
 run(['libreoffice',f'-env:UserInstallation={Path(profile).as_uri()}','--headless','--convert-to','pdf','--outdir',OUT/'roundtrip',OUT/'roundtrip/research-cycle.pptx']);assert dest.exists()
for pdf,png in [(OUT/'research-cycle.pdf','native'),(OUT/'verification/research-cycle.pdf','pptx-preview'),(OUT/'roundtrip/research-cycle.pdf','roundtrip-preview')]:
 run(['pdftoppm','-scale-to-x','1998','-scale-to-y','1125','-singlefile','-png',pdf,OUT/png])
def error(a,b):return sum(ImageStat.Stat(ImageChops.difference(a,b)).mean)/3
native=Image.open(OUT/'native.png').convert('RGB');pptx=Image.open(OUT/'pptx-preview.png').convert('RGB');saved_png=Image.open(OUT/'roundtrip-preview.png').convert('RGB')
mean=error(native,pptx);roundtrip_mean=error(pptx,saved_png)
# Font handling is deliberately unchanged. LibreOffice inserts script-dependent
# spacing during layout. Compare non-text appearance separately; all label contents
# and all object counts above are checked without masking or tolerance.
mask=Image.new('L',native.size,255);draw_mask=ImageDraw.Draw(mask)
for group in xml.findall('.//p:grpSp',check.NS):
 xf=group.find('p:grpSpPr/a:xfrm',check.NS)
 assert xf.find('a:off',check.NS).attrib==xf.find('a:chOff',check.NS).attrib
 assert xf.find('a:ext',check.NS).attrib==xf.find('a:chExt',check.NS).attrib
scale=1125/actual['size_emu'][1]
for shape in xml.findall('.//p:sp',check.NS):
 if not shape.findall('.//a:t',check.NS):continue
 xf=shape.find('p:spPr/a:xfrm',check.NS)
 off=xf.find('a:off',check.NS).attrib;ext=xf.find('a:ext',check.NS).attrib
 font_px=max(int(n.attrib.get('sz',0)) for n in shape.findall('.//a:rPr',check.NS))/100*25.4/72/(200/1125)
 x=int(off['x'])*scale;y=int(off['y'])*scale;w=int(ext['cx'])*scale;h=int(ext['cy'])*scale
 draw_mask.rectangle((x-font_px,y-5,x+w+font_px,y+h+5),fill=0)
nontext_mean=sum(ImageStat.Stat(ImageChops.difference(native,pptx),mask).mean)/3
nontext_roundtrip_mean=sum(ImageStat.Stat(ImageChops.difference(pptx,saved_png),mask).mean)/3
assert nontext_mean<5 and nontext_roundtrip_mean<1,(nontext_mean,nontext_roundtrip_mean)
mask.save(OUT/'nontext-mask.png')
regions={name:error(native.crop(box),pptx.crop(box)) for name,box in [('header',(0,0,1998,360)),('traditional',(33,369,991,798)),('agent',(1007,369,1965,798)),('continuation',(29,806,1969,979)),('footer',(0,984,1998,1125))]}
reference=SRC/'reference.png'
canvas=Image.new('RGB',(1998,1193),'#f3f7fb');draw=ImageDraw.Draw(canvas)
for i,(title,path) in enumerate([('Reference',reference),('LayMesh native PDF',OUT/'native.png'),('CLI PPTX / LibreOffice',OUT/'pptx-preview.png'),('PPTX after LibreOffice save',OUT/'roundtrip-preview.png')]):
 x=i%2*999;y=i//2*596;draw.text((x+14,y+12),title,fill='#172e50');canvas.paste(Image.open(path).convert('RGB').resize((999,562)),(x,y+32))
canvas.save(OUT/'comparison.png')
evidence=dict(branch='codex/pptx-export',binary_version=run([BINARY,'--version']).stdout.strip(),binary_sha256=hashlib.sha256(BINARY.read_bytes()).hexdigest(),source_labels=len(labels),export=actual,roundtrip=saved,warnings=warnings,raster_fallbacks=0,mean_absolute_rgb_error_0_255=mean,region_rgb_error_0_255=regions,roundtrip_rgb_error_0_255=roundtrip_mean,nontext_rgb_error_0_255=nontext_mean,nontext_roundtrip_rgb_error_0_255=nontext_roundtrip_mean,font_layout_limit="LibreOffice adds spacing between script runs; whole-image pixel difference includes this unchanged font behavior",libreoffice_version=run(['libreoffice','--version']).stdout.strip(),powerpoint_native_tested=False,manual_visual_review='pending',reference_semantics='Scenario estimates supplied by the reference, not benchmark measurements',checks=['ZIP/XML validity and all relationship targets','Unique drawing IDs and canvas dimensions','Every source label preserved in order','Three native rounded picture fills and independent PNG resources','Native geometry and linear gradients','All text, drawings and groups preserved in LibreOffice save','PDF appearance compared before and after save'])
(OUT/'evidence.json').write_text(json.dumps(evidence,ensure_ascii=False,indent=2)+'\n')
print(json.dumps(dict(text_boxes=actual['text_boxes'],vector_shapes=actual['shapes']-actual['text_boxes'],groups=actual['groups'],pictures=actual['pictures'],mean=mean,regions=regions,roundtrip_mean=roundtrip_mean,nontext_mean=nontext_mean,nontext_roundtrip_mean=nontext_roundtrip_mean),indent=2))
