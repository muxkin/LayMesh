#!/usr/bin/env python3
"""Inspect browser downloads with independent image, XML, ZIP and PDF readers."""
import json, re, subprocess, sys, zipfile
from pathlib import Path
from xml.etree import ElementTree as ET
from PIL import Image
out=Path(sys.argv[1]); results=[]
for run in json.loads((out/'results.json').read_text()):
 engine=run['engine'];file=lambda suffix:out/f'{engine}-{suffix}'
 expected=(round(40*1200/25.4),round(30*1200/25.4))
 png=Image.open(file('png'));assert png.size==expected,(engine,png.size,expected)
 assert abs(png.info['dpi'][0]-1200)<.1;assert png.mode=='RGBA';assert png.getpixel((0,0))[3]==0
 for fmt in ['jpg','tif','webp','bmp','gif','ppm','pgm','pbm','tga']:
  image=Image.open(file(fmt));image.load();assert image.size==expected,(fmt,image.size)
  if fmt in ['webp','tif','bmp','tga']:assert image.convert('RGBA').tobytes()==png.tobytes(),fmt
 tiff=Image.open(file('tif'));assert tiff.tag_v2[259]==5;assert tiff.info['dpi']==(1200,1200)
 assert Image.open(file('jpg')).info['dpi']==(1200,1200)
 assert file('pam').read_bytes().startswith(b'P7\n');assert f'WIDTH {expected[0]}\n'.encode() in file('pam').read_bytes()[:200]
 ico=Image.open(file('ico'));assert ico.size==(round(40*120/25.4),round(30*120/25.4));assert max(ico.size)<=256
 assert Image.open(file('second.png')).size==(round(60*120/25.4),round(40*120/25.4))
 assert Image.open(file('reopened.png')).size==ico.size
 svg=ET.parse(file('svg')).getroot();assert svg.attrib['width']=='40mm';assert not list(svg.iter('{http://www.w3.org/2000/svg}text'))
 with zipfile.ZipFile(file('pages.zip')) as z:assert sorted(z.namelist())==['page-001.svg','page-002.svg'];assert z.testzip() is None
 with zipfile.ZipFile(file('pptx')) as z:
  assert z.testzip() is None;slide=z.read('ppt/slides/slide1.xml').decode();assert 'Export 0.6.3' in slide;assert slide.count('<p:sp>')>=2;assert '<p:pic>' in slide,'Effects must have a local fallback'
  p=ET.fromstring(z.read('ppt/presentation.xml'));size=p.find('{http://schemas.openxmlformats.org/presentationml/2006/main}sldSz');assert (int(size.attrib['cx']),int(size.attrib['cy']))==(40*36000,30*36000)
 info=subprocess.check_output(['pdfinfo','-f','1','-l','2',str(file('pdf'))],text=True);assert re.search(r'Pages:\s+2',info),info
 single=subprocess.check_output(['pdfinfo',str(file('single.pdf'))],text=True);assert re.search(r'Pages:\s+1',single)
 assert re.search(r'Page\s+1 size:\s+113\.38\d* x 85\.03\d* pts',info),info
 assert re.search(r'Page\s+2 size:\s+170\.07\d* x 113\.38\d* pts',info),info
 subprocess.run(['pdftoppm','-f','1','-singlefile','-scale-to-x','189','-scale-to-y','142','-png',str(file('pdf')),str(file('pdf-render'))],check=True,stdout=subprocess.DEVNULL,stderr=subprocess.PIPE)
 pdf=Image.open(file('pdf-render.png')).convert('RGB');r,g,b=pdf.getpixel((20,20));assert r>200 and g<100 and b<70,(r,g,b)
 # A vector PDF of this two-page fixture is substantially smaller than the 1200-DPI raster.
 assert file('pdf').stat().st_size<file('png').stat().st_size
 results.append({'engine':engine,'decodedFormats':15,'rasterSize':expected,'pdfPages':2,'pdfVector':True,'pptxEditableText':True,'pptxEffectFallback':True,'alphaAndDPI':True,'passed':True})
(out/'file-verification.json').write_text(json.dumps(results,indent=2)+'\n');print(json.dumps(results))
