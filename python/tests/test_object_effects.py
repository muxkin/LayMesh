"""Object effects retain their source, diagnostics and editable siblings through Python."""
import tempfile,unittest,zipfile
from pathlib import Path
from laymesh import render_source,render_file,LayMeshBridgeError

SOURCE='''p=canvas(size=(40mm,35mm),stylesheet="@laymesh/effects.css")
p.add(text("Editable",font_size=8pt),offset=(3mm,2mm))
p.add(rect(size=(12mm,8mm),class="fx-bevel-button",three_d=three_d(depth=2mm,rotation=(20deg,-30deg,0deg)),effects=[soft_edge(radius=0.15mm),reflection(length=0.6)]),offset=(10mm,12mm))
'''
class ObjectEffects(unittest.TestCase):
    def test_source_roundtrip_and_local_pptx_fallback(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp)
            result=render_source(SOURCE,base_dir=root,output=root/'effects.pptx',dpi=100,save_source='effects.lay',show_warnings=False)
            self.assertIn('data:image/png;base64,',result.preview_svg)
            self.assertIn('three_d(',result.saved_source.read_text())
            with zipfile.ZipFile(result.output) as deck:
                xml=deck.read('ppt/slides/slide1.xml').decode();self.assertIn('Editable',xml);self.assertGreaterEqual(xml.count('<p:pic>'),2)
            replay=render_file(result.saved_source,output=root/'effects.png',dpi=100,show_warnings=False)
            self.assertTrue(replay.output.is_file())
    def test_invalid_camera_preserves_existing_file(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);dest=root/'existing.png';dest.write_bytes(b'preserved')
            with self.assertRaises(LayMeshBridgeError):
                render_source(SOURCE.replace('depth=2mm','depth=2mm,projection="perspective",distance=0.1mm'),base_dir=root,output=dest,dpi=100,show_warnings=False)
            self.assertEqual(dest.read_bytes(),b'preserved')
