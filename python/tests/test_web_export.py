"""Project-aware HTML/ZIP exports and failure protection through the public API."""
import json
import io
import tempfile
import unittest
import zipfile
from pathlib import Path
from laymesh import LayMeshBridgeError, render_source, render_document

class WebExports(unittest.TestCase):
    def test_image_policy_preserves_dimensions_alpha_and_original_bytes(self):
        from PIL import Image
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            image = Image.new('RGBA', (97, 71))
            image.putdata([(x * 2, y * 3, (x + y) % 256, (x * 3 + y) % 256) for y in range(71) for x in range(97)])
            image.save(root / 'image.png')
            original = (root / 'image.png').read_bytes()
            source = 'page=canvas(size=(60mm,30mm))\npage.add(image(src="image.png"),size=(20mm,20mm))\npage.add(image(src="image.png"),size=(20mm,20mm),offset=(25mm,0mm))'
            for policy in ['original', 'auto', 'lossless']:
                output = root / (policy + '.zip')
                render_source(source, base_dir=root, output=output, html_image_compression=policy)
                with zipfile.ZipFile(output) as archive:
                    manifest = json.loads(archive.read('manifest.json'))
                    self.assertEqual(len(manifest['resources']), 1)
                    encoded = archive.read(next(iter(manifest['resources'].values())))
                    self.assertLessEqual(len(encoded), len(original))
                    decoded = Image.open(io.BytesIO(encoded)).convert('RGBA')
                    self.assertEqual(decoded.size, image.size)
                    self.assertEqual(decoded.getchannel('A').tobytes(), image.getchannel('A').tobytes())
                    if policy == 'original':
                        self.assertEqual(encoded, original)
                    elif policy == 'lossless':
                        self.assertEqual(decoded.tobytes(), image.tobytes())

    def test_source_dependencies_and_default_font_requirements(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'label.lay').write_text('export label="Offline text"', encoding='utf-8')
            output = root / 'figure.zip.html'
            render_source('import {label} from "label.lay"\npage=canvas(size=(60mm,30mm))\npage.add(text(label))', base_dir=root, output=output, html_editable=True)
            with zipfile.ZipFile(output) as archive:
                self.assertIsNone(archive.testzip())
                manifest = json.loads(archive.read('manifest.json'))
                self.assertTrue(manifest['editable'])
                self.assertTrue(manifest['fontRequirements'])
                self.assertFalse(manifest['embedFonts'])
                self.assertTrue(all('file' not in f for f in manifest['fonts']))
                self.assertIn('/project/label.lay', manifest['project']['files'])
                self.assertNotIn('"kind":"glyphs"', archive.read(manifest['pages'][0]['file']).decode())
                snapshot = json.loads(archive.read(manifest['pages'][0]['file']))['svg']
                self.assertNotIn('data:font', snapshot)
                self.assertNotIn('<text ', snapshot)
                self.assertIn('<path ', snapshot)
            previous = output.read_bytes()
            with self.assertRaises(LayMeshBridgeError):
                render_source('page=canvas(size=(60mm,30mm))', base_dir=root, output=output, html_image_quality=101)
            self.assertEqual(output.read_bytes(), previous)
            with self.assertRaises(LayMeshBridgeError):
                render_source('page=canvas(size=(60mm,30mm))', base_dir=root, output=output, dpi=1200)

    def test_font_policy_keeps_project_fonts_and_explicit_system_embedding(self):
        import shutil
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            shutil.copy(Path(__file__).resolve().parents[2]/'tests/fonts/DejaVuSans.ttf',root/'project.ttf')
            source='page=canvas(size=(60mm,30mm))\npage.add(text("Project",font_family="./project.ttf"))'
            output=root/'project.zip'
            render_source(source,base_dir=root,output=output,html_editable=True)
            with zipfile.ZipFile(output) as archive:
                m=json.loads(archive.read('manifest.json'))
                self.assertIn('/project/project.ttf',m['project']['files'])
                self.assertTrue(m['fonts'])
                self.assertTrue(all(f['source']=='project' and f['file'] in archive.namelist() for f in m['fonts']))
            source='page=canvas(size=(60mm,30mm))\npage.add(text("System"))'
            render_source(source,base_dir=root,output=output,html_editable=True,html_embed_fonts=True)
            with zipfile.ZipFile(output) as archive:
                m=json.loads(archive.read('manifest.json'))
                self.assertTrue(m['embedFonts'])
                self.assertTrue(all(f['file'] in archive.namelist() for f in m['fonts']))
            with self.assertRaises(LayMeshBridgeError):
                render_source(source,base_dir=root,output=output,html_embed_fonts='yes')
            with self.assertRaises(LayMeshBridgeError):
                render_source(source,base_dir=root,output=root/'wrong.png',html_embed_fonts=True)

    def test_document_defaults_to_all_pages_and_supports_selected_page(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root/'page.lay').write_text('page=canvas(size=(60mm,30mm))', encoding='utf-8')
            document = root/'document.laydoc'
            document.write_text('doc=document(name="Offline")\ndoc.add(page(src="page.lay",id="first"))\ndoc.add(page(src="page.lay",id="second"))', encoding='utf-8')
            output=root/'document.zip'
            render_document(document, output=output)
            with zipfile.ZipFile(output) as archive:
                self.assertEqual(len(json.loads(archive.read('manifest.json'))['pages']),2)
            render_document(document, output=output, page=2, html_editable=True)
            with zipfile.ZipFile(output) as archive:
                manifest=json.loads(archive.read('manifest.json'))
                self.assertEqual(len(manifest['pages']),1)
                self.assertEqual(manifest['project']['page'],2)

if __name__ == '__main__':
    unittest.main()
