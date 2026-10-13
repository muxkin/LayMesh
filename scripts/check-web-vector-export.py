#!/usr/bin/env python3
"""Compare actual browser vector exports against an independent native raster."""
import json,subprocess,sys,zlib,re
from pathlib import Path
from PIL import Image,ImageChops,ImageStat
out=Path(sys.argv[1])
def rgb(image):
 image=image.convert('RGBA');back=Image.new('RGBA',image.size,'white');back.alpha_composite(image);return back.convert('RGB')
native=rgb(Image.open(out/'native.png'));results=[]
for run in json.loads((out/'results.json').read_text()):
 stem=out/(run['engine']+'-'+run['mode']);pdf=stem.with_suffix('.pdf')
 subprocess.run(['pdftoppm','-singlefile','-scale-to-x',str(native.width),'-scale-to-y',str(native.height),'-png',str(pdf),str(stem)+'-pdf'],check=True,stdout=subprocess.DEVNULL,stderr=subprocess.PIPE)
 errors={}
 for kind,suffix in [('png','.png'),('svg','-svg.png'),('pdf','-pdf.png')]:
  image=rgb(Image.open(str(stem)+suffix));assert image.size==native.size
  error=sum(ImageStat.Stat(ImageChops.difference(native,image)).mean)/3;errors[kind]=round(error,4)
  # Native/CanvasKit Skia revisions differ slightly in blur and edge coverage;
  # Poppler has different edge coverage. WebKit composites overlapping
  # luminance-mask SVG children differently (also on the editable exporter);
  # allow up to 3/255 for native browser SVG rasterization, recording each value.
  assert error<{'pdf':4,'svg':3,'png':1}[kind],(str(stem),kind,error)
 images=subprocess.check_output(['pdfimages','-list',str(pdf)],text=True)
 sizes=[]
 for line in images.splitlines()[2:]:
  fields=line.split()
  if len(fields)>5:sizes.append([int(fields[3]),int(fields[4])])
 assert sizes,'Fixture must exercise local effects'
 assert all(w*h<native.width*native.height/2 for w,h in sizes),'No full-page raster fallback'
 # Verify surviving PDF geometry, not just its appearance or filename.
 streams=[]
 for stream in re.findall(rb'stream\r?\n(.*?)\r?\nendstream',pdf.read_bytes(),re.S):
  try:streams.append(zlib.decompress(stream))
  except zlib.error:streams.append(stream)
 assert any(re.search(rb'[-\d.]+ [-\d.]+ m\s',s) and re.search(rb'[-\d.]+ [-\d.]+ l\s',s) for s in streams),'Missing vector path operators'
 results.append({**run,'meanRGBErrorOf255':errors,'localImageSizes':sizes,'vectorPaths':True,'passed':True})
(out/'verification.json').write_text(json.dumps(results,indent=2)+'\n');print(json.dumps(results))
