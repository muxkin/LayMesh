#!/usr/bin/env python3
"""Native media PPTX integration: portable resources, scheduling and atomic outputs.

This checks package semantics, not actual Windows PowerPoint playback.
"""
import argparse,hashlib,io,json,os,posixpath,subprocess,sys,xml.etree.ElementTree as ET,zipfile
from pathlib import Path
from PIL import Image
ROOT=Path(__file__).resolve().parents[1]
NS={'p':'http://schemas.openxmlformats.org/presentationml/2006/main','a':'http://schemas.openxmlformats.org/drawingml/2006/main','r':'http://schemas.openxmlformats.org/officeDocument/2006/relationships','p14':'http://schemas.microsoft.com/office/powerpoint/2010/main'}

def inspect(path,pages):
    with zipfile.ZipFile(path) as z:
        names=set(z.namelist());assert z.testzip() is None
        docs={name:ET.fromstring(z.read(name)) for name in names if name.endswith(('.xml','.rels'))}
        for name,doc in docs.items():
            if name.endswith('.rels'):
                parent=name.split('/_rels/')[0] if '/_rels/' in name else ''
                ids=[n.attrib['Id'] for n in doc];assert len(ids)==len(set(ids))
                for n in doc:
                    assert n.attrib.get('TargetMode')!='External'
                    assert posixpath.normpath(posixpath.join(parent,n.attrib['Target'])).lstrip('/') in names,(name,n.attrib)
        assert len(docs['ppt/presentation.xml'].findall('p:sldIdLst/p:sldId',NS))==pages
        for i in range(1,pages+1):
            slide=docs[f'ppt/slides/slide{i}.xml'];rels=docs[f'ppt/slides/_rels/slide{i}.xml.rels'];ids={r.attrib['Id'] for r in rels}
            shapes=[n.attrib['id'] for n in slide.findall('.//p:cNvPr',NS)];assert len(shapes)==len(set(shapes))
            timing=[n.attrib['id'] for n in slide.findall('.//p:cTn',NS)];assert len(timing)==len(set(timing))
            for n in slide.iter():
                for k,v in n.attrib.items():
                    if k.startswith('{'+NS['r']+'}') and v:assert v in ids
            for n in slide.findall('.//p:spTgt',NS):assert n.attrib['spid'] in shapes
        media=[name for name in names if name.startswith('ppt/media/')]
        hashes=[hashlib.sha256(z.read(n)).digest() for n in media];assert len(hashes)==len(set(hashes)),'Media must be deduplicated across slides'
        for name in media:
            if name.endswith(('.png','.jpeg')):
                with Image.open(io.BytesIO(z.read(name))) as im:im.load()
        return docs,{Path(n).suffix:z.read(n) for n in media if not n.endswith(('.png','.jpeg'))}

def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',type=Path,default=ROOT/'target/debug/laymesh');p.add_argument('--output',type=Path,default=ROOT/'target/media-mainline/pptx');a=p.parse_args();a.output.mkdir(parents=True,exist_ok=True);binary=a.binary.resolve()
    def run(source,out,*args,fail=False):
        r=subprocess.run([str(binary),'render',str(source),'-o',str(out),*args],capture_output=True,text=True,timeout=120)
        assert (r.returncode!=0)==fail,r.stderr
        return r
    source=ROOT/'examples/media/powerpoint.laydoc';deck=a.output/'deck.pptx'
    run(source,deck);docs,media=inspect(deck,2)
    assert media['.mp4']==(ROOT/'examples/media/controls.mp4').read_bytes()
    assert media['.wav']==(ROOT/'examples/media/music.wav').read_bytes()
    slide=docs['ppt/slides/slide1.xml'];second=docs['ppt/slides/slide2.xml']
    assert len(slide.findall('.//p:video',NS))==4 and len(second.findall('.//p:video',NS))==1
    assert len(slide.findall('.//p:audio',NS))==1 and not second.findall('.//p:audio',NS)
    assert slide.find('.//p:audio/p:cMediaNode',NS).get('numSld')=='2'
    assert all(v.get('numSld')=='1' for v in slide.findall('.//p:video/p:cMediaNode',NS))
    assert slide.findall('.//a:t',NS),'Text must remain editable'
    assert any(int(n.get('l','0'))>=25000 for n in slide.findall('.//a:srcRect',NS))
    assert any(int(n.get('rot','0'))==300000 for n in slide.findall('.//a:xfrm',NS))
    trims=[n.attrib for n in slide.findall('.//p14:trim',NS)];assert {'st':'2000','end':'2000'} in trims,trims
    assert any(n.get('repeatCount')=='indefinite' for n in slide.findall('.//p:cTn',NS))
    assert any(n.get('vol')=='40000' for n in slide.findall('.//p:cMediaNode',NS))
    assert any(n.get('mute')=='1' for n in slide.findall('.//p:cMediaNode',NS))
    assert slide.find('.//p:seq',NS) is not None
    assert any(n.get('evt')=='onNext' for n in slide.findall('.//p:seq//p:stCondLst/p:cond',NS))
    assert sum(n.get('evt')=='onClick' for n in slide.findall('.//p:cond',NS))==2
    assert any(n.get('delay')=='500' for n in slide.findall('.//p:cond',NS))
    assert any(n.get('delay')=='200' for n in slide.findall('.//p:seq//p:cond',NS))
    run(source,a.output/'single.pptx','--page','1');single,_=inspect(a.output/'single.pptx',1);assert single['ppt/slides/slide1.xml'].find('.//p:audio/p:cMediaNode',NS).get('numSld')=='1'
    run(source,a.output/'batch.pptx','--all-pages');inspect(a.output/'batch-001.pptx',1);inspect(a.output/'batch-002.pptx',1)
    protected=a.output/'protected.pptx';protected.write_bytes(b'keep existing output');r=run(ROOT/'examples/media/controls.laydoc',protected,fail=True);assert 'page 2' in r.stderr;assert protected.read_bytes()==b'keep existing output'
    # The Python bridge must use the same default multi-page behavior.
    os.environ['LAYMESH_CLI']=str(binary);sys.path.insert(0,str(ROOT/'python'))
    import laymesh
    laymesh.render_document(source,output=a.output/'python.pptx',show_warnings=False);inspect(a.output/'python.pptx',2)
    # A custom cover on the first instance must not become the shared material's
    # default cover for another instance of the same video.
    Image.new('RGB',(32,18),'red').save(a.output/'red.png')
    fixture=a.output/'posters.lay';fixture.write_text('p=canvas(size=(160mm,90mm))\np.add(video(src="'+(ROOT/'examples/media/controls.mp4').as_posix()+'",poster="red.png"),size=(64mm,36mm))\np.add(video(src="'+(ROOT/'examples/media/controls.mp4').as_posix()+'"),size=(64mm,36mm),offset=(80mm,0mm))\n')
    run(fixture,a.output/'posters.pptx');poster_docs,_=inspect(a.output/'posters.pptx',1)
    covers=poster_docs['ppt/slides/slide1.xml'].findall('.//p:pic/p:blipFill/a:blip',NS)
    assert len(covers)==2 and covers[0].get('{'+NS['r']+'}embed')!=covers[1].get('{'+NS['r']+'}embed')
    targets={r.get('Id'):r.get('Target') for r in poster_docs['ppt/slides/_rels/slide1.xml.rels']}
    with zipfile.ZipFile(a.output/'posters.pptx') as z:
        default=Image.open(io.BytesIO(z.read(posixpath.normpath('ppt/slides/'+targets[covers[1].get('{'+NS['r']+'}embed')])))).convert('RGBA')
        assert default.getchannel('A').getextrema()==(255,255) and len(default.resize((16,16)).getcolors(256))>8,'Default cover must contain decoded video pixels'
    result={'passed':True,'checks':['embedded media and posters','shared video across independent instances/pages','independent explicit/default posters','internal relationships and shape targets','editable text','crop/rotation','trim/delay/loop/volume/mute','click and step schedule','first-slide continuous music','CLI default/page/all-pages','Python multi-page','size mismatch preserves output'],'windows_powerpoint':'pending real Windows Microsoft 365 playback and save/reopen'}
    (a.output/'results.json').write_text(json.dumps(result,indent=2));print(json.dumps(result))
if __name__=='__main__':main()
