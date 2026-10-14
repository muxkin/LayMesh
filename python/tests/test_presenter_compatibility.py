"""Presentation metadata must not change Python/static/Notebook rendering."""
import io
import json
import tempfile
import unittest
import warnings
import zipfile
from pathlib import Path

from laymesh import render_source
from laymesh.bridge import _render_notebook


PAGE = '''p=canvas(size=(40mm,25mm),background="#ffffff")
a=p.add(line(length=25mm,line_width=1mm,end_head=head(size=(4mm,3mm))),offset=(5mm,8mm))
b=p.add(rect(size=(8mm,6mm),fill="#ff0000",border_color="none"),offset=(5mm,15mm))
'''
ANIMATION = '''s=p.step(name="Draw",hold=1s)
s.animate(a,name="draw",duration=1s,fill="both")
s.animate(b,name="draw",duration=1s,fill="both")
'''
NOTES = '"""Private notes 中文 {{not_a_binding}}"""\n'


class PresenterCompatibility(unittest.TestCase):
    def test_notes_and_ignored_draw_leave_static_and_notebook_pixels_unchanged(self):
        from PIL import Image
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            plain = root / "plain.lay"
            file = root / "animated.lay"
            plain.write_text(PAGE, encoding="utf-8")
            file.write_text(NOTES + PAGE + ANIMATION, encoding="utf-8")
            baseline = _render_notebook(file=plain)
            with warnings.catch_warnings(record=True) as seen:
                warnings.simplefilter("always")
                animated = _render_notebook(file=file, output=root / "figure.svg",
                                           show_warnings=True)
            self.assertEqual(animated.preview_jpeg, baseline.preview_jpeg)
            self.assertNotIn("Private notes", (root / "figure.svg").read_text())
            diagnostics = [str(item.message) for item in seen if "W_ANIMATION" in str(item.message)]
            self.assertEqual(len(diagnostics), 1, diagnostics)
            with Image.open(io.BytesIO(animated.preview_jpeg)) as image:
                self.assertEqual(image.info["dpi"], (192, 192))
            with warnings.catch_warnings(record=True) as hidden:
                warnings.simplefilter("always")
                _render_notebook(file=file, show_warnings=False)
            self.assertEqual(hidden, [])
            # A warning must leave the next ordinary Python render usable.
            self.assertTrue(render_source(PAGE, base_dir=root).preview_svg.startswith("<svg"))

    def test_python_offline_export_keeps_notes_and_valid_animation_metadata(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            output = root / "presentation.zip"
            render_source(NOTES + PAGE + ANIMATION, base_dir=root, output=output,
                          show_warnings=False)
            with zipfile.ZipFile(output) as archive:
                manifest = json.loads(archive.read("manifest.json"))
                page = json.loads(archive.read(manifest["pages"][0]["file"]))
                self.assertEqual(page["inspection"]["speakerNotes"],
                                 "Private notes 中文 {{not_a_binding}}")
                self.assertTrue(page["list"]["animation"])
                self.assertNotIn("Private notes", json.dumps(page["list"]))


if __name__ == "__main__":
    unittest.main()
