#!/usr/bin/env python3
"""Measure actual gradient exports and write a self-contained comparison gallery.

Input is test-gradient-parity.cjs output. --vectors also renders PDFs with
Poppler and PPTX with an isolated LibreOffice profile. Native pixel coordinates
are never rescaled to hide export alignment differences.
"""
import argparse,json,subprocess,tempfile
from pathlib import Path
import numpy as np
from PIL import Image,ImageFilter
p=argparse.ArgumentParser(description=__doc__);p.add_argument('directory',type=Path);p.add_argument('--vectors',action='store_true');p.add_argument('--check',action='store_true');a=p.parse_args();root=a.directory.resolve();manifest=json.loads((root/'manifest.json').read_text())
if a.vectors:
    office=root/'pptx';office.mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='laymesh-gradient-office-') as profile:
        files=list(root.glob('*.pptx'))
        # The default Impress PDF filter recompresses PNG fills as JPEG. Disable
        # that and downsampling so the reference measures the exported PPTX.
        pdf_filter='pdf:impress_pdf_Export:'+json.dumps({'UseLosslessCompression':{'type':'boolean','value':'true'},'ReduceImageResolution':{'type':'boolean','value':'false'}})
        if files:subprocess.run(['libreoffice','-env:UserInstallation='+Path(profile).as_uri(),'--headless','--convert-to',pdf_filter,'--outdir',str(office),*map(str,files)],check=True,stdout=subprocess.DEVNULL)
    for case in manifest['cases']:
        for kind,path in [('pdf',root/(case['name']+'.pdf')),('pptx',office/(case['name']+'.pdf'))]:
            if path.exists():
                subprocess.run(['pdftocairo','-png','-r','96','-singlefile',str(path),str(root/(case['name']+'-'+kind))],check=True,stderr=subprocess.DEVNULL)
                # Keep the independent Splash renderer too: it interpolates
                # image pixels differently even when embedded PNGs are exact.
                subprocess.run(['pdftoppm','-png','-r','96','-singlefile',str(path),str(root/(case['name']+'-'+kind+'-splash'))],check=True,stderr=subprocess.DEVNULL)

def pixels(path):return np.array(Image.open(path).convert('RGB')).astype(np.int16)
def metric(actual,reference):
    if actual.shape!=reference.shape:return {'error':'dimension mismatch','actual':list(actual.shape),'reference':list(reference.shape)}
    d=np.abs(actual-reference);edge=np.zeros(reference.shape[:2],bool)
    for image in [reference,actual]:
        dx=np.max(np.abs(image[:,1:]-image[:,:-1]),2)>12;dy=np.max(np.abs(image[1:]-image[:-1]),2)>12
        edge[:,1:]|=dx;edge[:,:-1]|=dx;edge[1:]|=dy;edge[:-1]|=dy
    edge=np.array(Image.fromarray((edge*255).astype('uint8')).filter(ImageFilter.MaxFilter(5)))>0
    edge[:2]=True;edge[-2:]=True;edge[:,:2]=True;edge[:,-2:]=True;smooth=~edge
    def stats(mask):
        v=d[mask];return {'pixels':len(v),'meanChannel':round(float(v.mean()),6),'maxChannel':int(v.max()),'pixelsOver3':int((v.max(1)>3).sum())} if len(v) else None
    return {'all':stats(np.ones(edge.shape,bool)),'smooth':stats(smooth),'edges':stats(edge)}
known={
 'hint':'Chromium uses nine inserted hint stops; native power-curve oracle is tested independently.',
 'hint-auto':'Chromium uses nine inserted hint stops; omitted stops keep positioned hints as anchors.',
 'round':'Chromium resolves percentage position against pre-round tile size; object-box geometry uses final size.',
 'zero-repeat':'CSS Images average-color rule; this Chromium renders the last stop.',
 'radial-zero-width':'CSS Images zero-width ellipse becomes a mirrored horizontal gradient; this Chromium renders the last stop.',
 'calc-size':'Inspect computed reference: this Chromium may reject otherwise valid calc radii.',
}
rows=[]
for c in manifest['cases']:
    name=c['name'];rgba=root/(name+'-canvaskit.rgba')
    if rgba.exists():Image.frombytes('RGBA',(c['w'],c['h']),rgba.read_bytes()).save(root/(name+'-canvaskit.png'))
    reference=pixels(root/(name+'-css.png'));native=pixels(root/(name+'-native.png'))
    row={'name':name,'css':c['css'],'computedCSS':c['reference'],'comparisons':{}}
    if name in known:row['referenceDifference']=known[name]
    for kind in ['css','native','canvaskit','svg','pdf','pptx','pdf-splash','pptx-splash']:
        file=root/(name+'-'+kind+'.png')
        if not file.exists():continue
        image=pixels(file)
        if kind!='css':row['comparisons'][kind]={'css':metric(image,reference),'native':metric(image,native)}
    rows.append(row)
report={'conditions':{k:v for k,v in manifest.items() if k!='cases'},'reference':f"Chromium CSS, forced sRGB, 240x160 CSS px, deviceScaleFactor=1; export {manifest.get('exportDpi',96)} DPI, readback 96 DPI. Primary PDF/PPTX: Poppler Cairo. Independent Splash diagnostics retained. LibreOffice lossless images, no downsampling.",'edgeRule':'Exclude 2-pixel page border and radius-2 neighborhoods of >12/255 adjacent-channel changes from smooth metrics; report both sets, without image registration.','cases':rows}
(root/'verification.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n')
from gradient_gallery import write_gallery
write_gallery(root, report)
for kind in ['native','canvaskit','svg','pdf','pptx']:
    m=[c['comparisons'][kind]['native']['smooth'] for c in rows if kind in c['comparisons'] and 'smooth' in c['comparisons'][kind]['native']]
    print(kind,len(m),'cases; smooth pixels over 3 vs native:',sum(s['pixelsOver3'] for s in m if s))

if a.check:
    assert len(rows)==119, 'Expected all 60 original recipes plus 59 semantic cases'
    for row in rows:
        for kind in ['canvaskit','svg','pdf','pptx']:
            m=row['comparisons'][kind]['native']['smooth']
            assert m and m['maxChannel']<=3,(row['name'],kind,m)
        m=row['comparisons']['native']['css']['smooth']
        if row['name'] in known:continue
        if row['name'] in ['open-default-18','open-srgb-18']:
            assert m['maxChannel']<=4 and m['pixelsOver3']<=1,(row['name'],m)
        else:assert m['maxChannel']<=3,(row['name'],m)
    print('CHECKED: 119 cases across all primary backends; explicit browser-reference exceptions retained.')
