"""Observable export behavior through Python, CLI and the editor stdio transport."""
import json
import subprocess
import tempfile
import unittest
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
