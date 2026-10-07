"""Observable export behavior through Python, CLI and the editor stdio transport."""
import json
import subprocess
import tempfile
import unittest
import zipfile
from xml.etree import ElementTree as ET
from pathlib import Path
from PIL import Image
from laymesh import LayMeshBridgeError, render_source, render_file
from laymesh.bridge import _command

SOURCE = 'page=canvas(size=(25.4mm,12.7mm),background="none")\npage.add(rect(size=(10mm,10mm),fill="#ff000080",border_width=0),offset=(1mm,1mm))'

class Exports(unittest.TestCase):
    def test_python_formats_options_and_saved_source_replay(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / '中文 # 图片'; root.mkdir()
            for suffix, options in [('png', dict(compression='best')), ('jpeg', dict(quality=95, background='#0000ff')),
                                    ('tif', dict(compression='deflate')), ('webp', dict(webp_lossless=False,quality=95,webp_method=6,webp_alpha_quality=100)),
                                    ('bmp', {}), ('gif', {}), ('ico', {}), ('ppm', {}), ('pgm', {}), ('pbm', {}), ('tga', {})]:
                with self.subTest(suffix=suffix):
                    result = render_source(SOURCE, base_dir=root, output=root / ('图形.' + suffix), dpi=144, **options)
                    self.assertIn('<svg', result.preview_svg)
                    with Image.open(result.output) as image:
                        self.assertEqual(image.size, (144,72))
                        alpha = image.convert('RGBA').getpixel((140,70))[3]
                        self.assertEqual(alpha, 255 if suffix in ('jpeg','ppm','pgm','pbm') else 0)
                        if suffix in ('png','jpeg','tif','bmp'):
                            self.assertAlmostEqual(image.info['dpi'][0], 144, delta=0.02)
            first = render_source(SOURCE, base_dir=root, output=root/'saved.webp', dpi=144, webp_lossless=True, save_source='saved.lay')
            second = render_file(first.saved_source, output=root/'replay.tiff', dpi=144, compression='packbits')
            with Image.open(first.output) as a, Image.open(second.output) as b:
                self.assertEqual(a.convert('RGBA').tobytes(), b.convert('RGBA').tobytes())

    def test_default_dpi_pdf_overrides_and_full_resolution_resources(self):
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary);file=root/'figure.lay';asset=root/'image.png';resources=root/'resources'
            Image.new('RGB',(400,200),(30,150,220)).save(asset)
            source='page=canvas(size=(25.4mm,12.7mm))\npage.add(image(src="image.png"),size=(25.4mm,12.7mm))'
            file.write_text(source)
            result=render_file(file,output=root/'default.png')
            with Image.open(result.output) as image:self.assertEqual(image.size,(1200,600))
            result=render_file(file,output=root/'forced.pdf',dpi=100,pdf_image_compression='jpeg',pdf_preserve_16bit=False,pdf_preserve_alpha=False,pdf_alpha_background='#ffffff')
            self.assertIn(b'/DCTDecode',result.output.read_bytes())
            original_jpeg=root/'photo.jpg';Image.new('RGB',(400,200),(30,150,220)).save(original_jpeg,quality=77)
            file.write_text(source.replace('image.png','photo.jpg'))
            direct=render_file(file,output=root/'direct.pdf',pdf_downsample=False)
            self.assertIn(original_jpeg.read_bytes(),direct.output.read_bytes())
            recompressed=render_file(file,output=root/'recompressed.pdf',pdf_recompress_jpeg=True,pdf_image_compression='jpeg')
            self.assertNotIn(original_jpeg.read_bytes(),recompressed.output.read_bytes())
            file.write_text(source)
            child=subprocess.Popen([*_command(),'preview','--stdio'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,text=True)
            try:
                ready=json.loads(child.stdout.readline());self.assertIn('resources-v2',ready['capabilities'])
                def send(source,known=()):
                    child.stdin.write(json.dumps(dict(id=1,protocol=2,generation=1,file=str(file),source=source,resource_dir=str(resources),known_resources=list(known),preview=dict(cache_mb=1,processing_memory_mb=1,image_threads=2)))+'\n');child.stdin.flush();return json.loads(child.stdout.readline())
                first=send(source);self.assertNotIn('error',first)
                images=[v for v in first['resources'] if v['mime'].startswith('image/')];self.assertEqual(len(images),1)
                self.assertEqual(images[0]['mime'],'image/jpeg')
                with Image.open(images[0]['path']) as image:self.assertEqual(image.size,(400,200))
                self.assertNotIn('data:image/png',first['svg']);self.assertNotIn('proxy',first['svg'])
                known=[v['id'] for v in first['resources']]
                hot=send(source+'\npage.add(text("changed"))',known);self.assertFalse([v for v in hot['resources'] if v['mime'].startswith('image/')])
                Image.new('RGBA',(400,200),(30,150,220,128)).save(asset)
                changed=send(source,known);images=[v for v in changed['resources'] if v['mime'].startswith('image/')];self.assertEqual(images[0]['mime'],'image/webp');self.assertNotEqual(images[0]['rasterKey'],first['resources'][0].get('rasterKey'))
                asset.unlink();failed=send(source,known);self.assertEqual(failed['error']['code'],'E_ASSET')
                Image.new('RGB',(400,200),(1,2,3)).save(asset);self.assertNotIn('error',send(source,known))
                Image.new('RGB',(1024,512),(30,150,220)).save(asset);Image.new('RGB',(1024,512),(220,150,30)).save(root/'second.png')
                budgeted=send(source+'\npage.add(image(src="second.png"),size=(12.7mm,6.35mm))',known);self.assertNotIn('error',budgeted)
                images=[v for v in budgeted['resources'] if v['mime'].startswith('image/')];self.assertEqual(len(images),2)
                for resource in images:
                    with Image.open(resource['path']) as image:self.assertEqual(image.size,(1024,512))
            finally:
                child.stdin.close();child.wait(timeout=10);child.stdout.close()

    def test_cli_global_project_explicit_config_and_one_shot_priority(self):
        import os
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary);project=root/'project';project.mkdir();file=project/'figure.lay';file.write_text(SOURCE)
            # Exercise each platform's real configuration location in an isolated
            # subprocess environment; XDG_CONFIG_HOME is Linux-specific.
            import sys
            env=dict(os.environ)
            if sys.platform == 'win32':
                global_dir=root/'appdata';env['APPDATA']=str(global_dir)
            elif sys.platform == 'darwin':
                home=root/'home';env['HOME']=str(home)
                global_dir=home/'Library'/'Application Support'
            else:
                global_dir=root/'global';env['XDG_CONFIG_HOME']=str(global_dir)
            (global_dir/'laymesh').mkdir(parents=True)
            (global_dir/'laymesh/config.json').write_text('{"export":{"dpi":600}}')
            def export(extra,expected):
                output=project/'out.png';result=subprocess.run([*_command(),'render',str(file),'-o',str(output),*extra],env=env,capture_output=True,text=True);self.assertEqual(result.returncode,0,result.stderr)
                with Image.open(output) as image:self.assertEqual(image.size,(expected,expected//2))
            export([],600)
            (project/'.laymesh.json').write_text('{"export":{"dpi":800}}');export([],800)
            explicit=root/'explicit.json';explicit.write_text('{"export":{"dpi":900}}');export(['--config',str(explicit)],900)
            export(['--config',str(explicit),'--dpi','1000'],1000)

    def test_invalid_options_preserve_outputs_without_asset_staging(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for suffix, options in [('png', dict(quality=95)), ('jpg', dict(quality=0)), ('tif', dict(compression='jpeg')),
                                    ('webp',dict(webp_method=7)), ('webp',dict(webp_lossless=False,webp_near_lossless=80)),
                                    ('svg',dict(dpi=144)), ('jpg',dict(background='transparent'))]:
                dest=root/('existing.'+suffix); dest.write_bytes(b'existing')
                with self.subTest(options=options), self.assertRaises(LayMeshBridgeError):
                    render_source(SOURCE, base_dir=root, output=dest, save_source='staged.lay', **options)
                self.assertEqual(dest.read_bytes(),b'existing')
                self.assertFalse((root/'staged.lay').exists())
                self.assertFalse(list(root.glob('*.tmp')))
            file=root/'source.lay';file.write_text(SOURCE)
            for flag,value in [('--quality','101'),('--compression','jpeg'),('--webp-method','7')]:
                dest=root/'existing.png';dest.write_bytes(b'existing')
                result=subprocess.run([*_command(),'render',str(file),'-o',str(dest),flag,value],capture_output=True,text=True)
                self.assertEqual(result.returncode,2,result.stderr)
                self.assertEqual(dest.read_bytes(),b'existing')

    def test_editor_export_uses_unsaved_imports_and_recovers_after_errors(self):
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary)/'中文 # 导出';root.mkdir();main=root/'main.lay';module=root/'colors.lay';dest=root/'图形.png'
            main.write_text('page=canvas(size=(1,1))');module.write_text('export shade="#000000"')
            source='import {shade} from "./colors.lay"\npage=canvas(size=(25.4mm,12.7mm),background=shade)'
            request=dict(type='export',file=main.as_posix(),source=source,overlays={module.as_posix():'export shade="#12ab34"'},output=str(dest),options=dict(dpi=144))
            invalid={**request,'id':1,'options':dict(quality=90)}
            good={**request,'id':2}
            broken={**request,'id':3,'source':'page=unknown()'}
            child=subprocess.run([*_command(),'preview','--stdio'],input='\n'.join(map(json.dumps,[invalid,good,broken]))+'\n',capture_output=True,text=True)
            self.assertEqual(child.returncode,0,child.stderr)
            ready,invalid,good,broken=map(json.loads,child.stdout.splitlines())
            self.assertEqual(ready['protocol'],1);self.assertEqual(invalid['error']['code'],'E_EXPORT')
            self.assertEqual(good['exported'],str(dest));self.assertEqual(good['bytes'],dest.stat().st_size)
            self.assertIn(str(module).replace('\\','/'),good['dependencies'])
            self.assertEqual(broken['error']['file'],str(main).replace('\\','/'));self.assertEqual(broken['error']['loc']['line'],1)
            with Image.open(dest) as image:
                self.assertEqual(image.size,(144,72));self.assertEqual(image.getpixel((70,30)),(18,171,52,255))
            self.assertFalse(list(root.glob('*.tmp')))

    def test_pptx_python_export_and_replay_preserve_editable_objects(self):
        ns={'a':'http://schemas.openxmlformats.org/drawingml/2006/main',
            'p':'http://schemas.openxmlformats.org/presentationml/2006/main'}
        source='page=canvas(size=(64mm,36mm))\npage.add(text("Editable 文本"),offset=(2mm,2mm))\npage.add(rect(size=(12mm,8mm),fill=image_fill(src="asset.png",fit=cover),border_width=0,border_radius=2mm),offset=(2mm,16mm))'
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary)/'中文 # PPTX';root.mkdir()
            Image.new('RGBA',(24,16),(30,120,220,128)).save(root/'asset.png')
            first=render_source(source,base_dir=root,output=root/'figure.PPTX',dpi=144,save_source='saved.lay',show_warnings=False)
            second=render_file(first.saved_source,output=root/'replay.pptx',show_warnings=False)
            for result in [first,second]:
                self.assertIn('<svg',result.preview_svg)
                with zipfile.ZipFile(result.output) as deck:
                    self.assertIsNone(deck.testzip())
                    xml=ET.fromstring(deck.read('ppt/slides/slide1.xml'))
                    self.assertEqual(''.join(n.text or '' for n in xml.findall('.//a:t',ns)),'Editable 文本')
                    self.assertEqual(len(xml.findall('.//p:pic',ns)),1)
                    self.assertEqual(xml.find('.//p:pic/p:spPr/a:prstGeom',ns).get('prst'),'roundRect')
                    size=ET.fromstring(deck.read('ppt/presentation.xml')).find('p:sldSz',ns)
                    self.assertEqual((int(size.get('cx')),int(size.get('cy'))),(64*36000,36*36000))
            dest=root/'protected.pptx';dest.write_bytes(b'existing')
            for options in [dict(quality=90),dict(compression='best'),dict(background='#ffffff'),dict(pdf_downsample=False),dict(webp_lossless=True)]:
                with self.subTest(options=options),self.assertRaises(LayMeshBridgeError):
                    render_source(source,base_dir=root,output=dest,save_source='invalid.lay',**options)
                self.assertEqual(dest.read_bytes(),b'existing')
                self.assertFalse((root/'invalid.lay').exists())
            with self.assertRaises(LayMeshBridgeError):
                render_source('page=unknown()',base_dir=root,output=dest)
            self.assertEqual(dest.read_bytes(),b'existing')

    def test_pptx_editor_transport_uses_unsaved_imports_and_defaults(self):
        ns={'a':'http://schemas.openxmlformats.org/drawingml/2006/main'}
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary);main=root/'main.lay';module=root/'label.lay';dest=root/'editor.pptx'
            main.write_text('page=canvas(size=(64mm,36mm))',encoding='utf-8')
            module.write_text('export label="Disk"',encoding='utf-8')
            source='import {label} from "./label.lay"\npage=canvas(size=(64mm,36mm))\npage.add(text(label))'
            request=dict(type='export',file=main.as_posix(),source=source,overlays={module.as_posix():'export label="Unsaved"'},output=str(dest),options={})
            requests=[{**request,'id':1,'options':dict(quality=90)},{**request,'id':2},{**request,'id':3,'source':'page=unknown()'}]
            child=subprocess.run([*_command(),'preview','--stdio'],input='\n'.join(map(json.dumps,requests))+'\n',capture_output=True,text=True)
            self.assertEqual(child.returncode,0,child.stderr)
            ready,invalid,good,broken=map(json.loads,child.stdout.splitlines())
            self.assertEqual(invalid['error']['code'],'E_EXPORT')
            self.assertEqual(good['exported'],str(dest));self.assertEqual(good['bytes'],dest.stat().st_size)
            self.assertIn(module.as_posix(),good['dependencies'])
            self.assertIn('error',broken)
            with zipfile.ZipFile(dest) as deck:
                xml=ET.fromstring(deck.read('ppt/slides/slide1.xml'))
                self.assertEqual([n.text for n in xml.findall('.//a:t',ns)],['Unsaved'])
            self.assertFalse(list(root.glob('*.tmp')))
