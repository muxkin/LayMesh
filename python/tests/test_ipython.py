import os
import io
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch


class MagicTest(unittest.TestCase):
    def test_preview_settings_are_independent_and_invalid_options_recover(self):
        from PIL import Image, JpegImagePlugin
        from laymesh.bridge import _render_notebook, LayMeshBridgeError
        from IPython.core.interactiveshell import InteractiveShell
        source = 'page=canvas(size=(25.4mm,12.7mm),background="none")\npage.add(rect(size=(8mm,8mm),fill="#ff000080",border_width=0),offset=(1mm,1mm))'
        previous = Path.cwd()
        with tempfile.TemporaryDirectory(prefix="laymesh-preview-settings-") as directory:
            try:
                os.chdir(directory)
                Path("config.json").write_text('{"export":{"dpi":72,"jpeg":{"quality":20,"background":"#000000"}}}')
                shell = InteractiveShell.instance()
                shell.run_line_magic("load_ext", "laymesh.ipython")
                with patch("laymesh.ipython.display") as shown:
                    shell.run_cell_magic("laymesh", '-o export.jpg --config config.json --preview-dpi 144 --preview-quality 97 --preview-background "#0000ff"', source)
                    with Image.open(io.BytesIO(shown.call_args.args[0].data)) as image:
                        self.assertEqual(image.size, (144,72))
                        self.assertEqual(image.info["dpi"], (144,144))
                        self.assertEqual(JpegImagePlugin.get_sampling(image), 0)
                        self.assertTrue(all(abs(a-b) <= 2 for a,b in zip(image.getpixel((15,15)), (128,0,127))))
                        self.assertTrue(all(abs(a-b) <= 2 for a,b in zip(image.getpixel((130,60)), (0,0,255))))
                    with Image.open("export.jpg") as image:
                        self.assertEqual(image.size, (72,36))
                for options in ({"preview_dpi":float("nan")}, {"preview_dpi":0}, {"preview_quality":0}, {"preview_quality":95.5}, {"preview_background":"none"}):
                    with self.assertRaises(LayMeshBridgeError):
                        _render_notebook(source=source, **options)
                result = _render_notebook(source='page=canvas(size=(25.4mm,12.7mm),background="#123456")')
                with Image.open(io.BytesIO(result.preview_jpeg)) as image:
                    self.assertTrue(all(abs(a-b) <= 2 for a,b in zip(image.getpixel((20,20)), (18,52,86))))
            finally:
                os.chdir(previous)

    def test_cli_preview_guards_preserve_export_and_detect_aliases(self):
        import subprocess
        from laymesh.bridge import _command
        with tempfile.TemporaryDirectory(prefix="laymesh-preview-guards-") as directory:
            root = Path(directory)
            layout = root / "wide.lay"
            layout.write_text('page=canvas(size=(65536mm,1mm))')
            output = root / "export.svg"
            output.write_bytes(b"existing export")
            result = subprocess.run([*_command(), "render", str(layout), "-o", str(output), "--preview-output", str(root/"preview.jpg"), "--preview-dpi", "25.4"], capture_output=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn(b"65535", result.stderr)
            self.assertEqual(output.read_bytes(), b"existing export")
            layout.write_text('page=canvas(size=(25.4mm,12.7mm))')
            for args in (["--preview-output", "preview.png"], ["--preview-output", "preview.jpg", "--dpi", "96"], ["-o", "same.jpg", "--preview-output", "./same.jpg", "--dpi", "96"]):
                result = subprocess.run([*_command(), "render", str(layout), *args], cwd=root, capture_output=True)
                self.assertNotEqual(result.returncode, 0)
                self.assertFalse((root/"same.jpg").exists())
            result = subprocess.run([*_command(), "render", str(layout), "-o", "same.jpg", "--preview-output", "./same.jpg", "--dpi", "192", "--quality", "95"], cwd=root, capture_output=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertFalse((root/"same.jpg").exists())
            if hasattr(os, "symlink"):
                (root/"same.jpg").write_bytes(b"existing")
                (root/"alias.jpg").symlink_to("same.jpg")
                result = subprocess.run([*_command(), "render", str(layout), "-o", "same.jpg", "--preview-output", "alias.jpg", "--dpi", "192", "--quality", "95"], cwd=root, capture_output=True)
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual((root/"same.jpg").read_bytes(), b"existing")

    def test_cpu_jpeg_preview_alpha_export_dpi_and_reused_jpeg(self):
        from IPython.core.interactiveshell import InteractiveShell
        from PIL import Image, JpegImagePlugin
        from laymesh import bridge, render_file

        shell = InteractiveShell.instance()
        shell.run_line_magic("load_ext", "laymesh.ipython")
        source = ('page=canvas(size=(25.4mm,12.7mm),background="none")\n'
                  'page.add(rect(size=(8mm,8mm),fill="#ff000080",border_width=0),offset=(1mm,1mm))\n'
                  'page.add(text("$\\frac{a}{b}$",font_size=8pt),offset=(12mm,1mm))\n')
        previous = Path.cwd()
        with tempfile.TemporaryDirectory(prefix="laymesh-notebook-cpu-") as directory:
            try:
                os.chdir(directory)
                with patch("laymesh.ipython.display") as shown, patch.object(bridge, "_run", wraps=bridge._run) as run:
                    shell.run_cell_magic("laymesh", "-o export.png --dpi 96 --save-source figure.lay", source)
                    preview = shown.call_args.args[0]
                    with Image.open(io.BytesIO(preview.data)) as image:
                        self.assertEqual(image.size, (192, 96))
                        self.assertEqual(image.format, "JPEG")
                        self.assertEqual(image.info["dpi"], (192, 192))
                        self.assertEqual(JpegImagePlugin.get_sampling(image), 0)
                        self.assertTrue(all(abs(a-b) <= 2 for a,b in zip(image.getpixel((20,20)), (255,127,127))))
                        self.assertEqual(image.getpixel((180,80)), (255,255,255))
                    with Image.open("export.png") as image:
                        self.assertEqual(image.size, (96, 48))
                        self.assertEqual(image.convert("RGBA").getpixel((10,10)), (255,0,0,128))
                    self.assertEqual(len(run.call_args_list), 1)
                    self.assertTrue(all(not any(str(arg).endswith(".svg") for arg in call.args[1]) for call in run.call_args_list))
                    shell.run_cell_magic("laymesh", '-o retina.jpg --dpi 192 --quality 95 --background "#FFFFFF"', source)
                    self.assertEqual(len(run.call_args_list), 2)
                    self.assertEqual(shown.call_args.args[0].data, Path("retina.jpg").read_bytes())
                # The compatibility API still returns its SVG and the same CPU export.
                result = render_file("figure.lay", output="reference.png", dpi=192)
                self.assertIn("<svg", result.preview_svg)
                with Image.open("reference.png") as image:
                    self.assertEqual(image.convert("RGBA").getpixel((20,20)), (255,0,0,128))
            finally:
                os.chdir(previous)

    def test_line_and_cell_magics_display_previews(self):
        from IPython.core.interactiveshell import InteractiveShell

        shell = InteractiveShell.instance()
        shell.run_line_magic("load_ext", "laymesh.ipython")
        self.assertIsNotNone(shell.find_line_magic("laymesh"))
        self.assertIsNotNone(shell.find_cell_magic("laymesh"))
        previous = Path.cwd()
        with tempfile.TemporaryDirectory(prefix="laymesh-magic-test-") as directory:
            try:
                os.chdir(directory)
                shell.user_ns["title"] = "Notebook title"
                with patch("laymesh.ipython.display") as display:
                    shell.run_cell_magic(
                        "laymesh", "-o figure.svg",
                        'page=canvas(size=(10 mm,10 mm))\ncaption=text(content={{title}},font_size=8 pt)\npage.add(caption,target=page.top_left)\n',
                    )
                    self.assertTrue(Path("figure.svg").is_file())
                    Path("existing.lay").write_text("page=canvas(size=(10 mm,10 mm))\n")
                    shell.run_line_magic("laymesh", "existing.lay")
                    self.assertEqual(display.call_count, 2)
                    self.assertTrue(all(call.args[0].data.startswith(b"\xff\xd8") for call in display.call_args_list))
                    self.assertTrue(all(call.args[0].retina for call in display.call_args_list))
                    shell.user_ns["native_x"] = [0, 1, 2]
                    shell.user_ns["native_y"] = [1, 3, 2]
                    shell.run_cell_magic(
                        "laymesh", "-o native.svg --save-source native.lay",
                        "page=canvas(size=(100 mm,75 mm))\n"
                        "p=plot(size=(90 mm,65 mm))\n"
                        "p.line(x={{native_x}},y={{native_y}})\n"
                        "page.add(p)\n",
                    )
                    self.assertTrue(Path("native.svg").is_file())
                    self.assertIn("array(src=", Path("native.lay").read_text())
                    shell.run_line_magic("laymesh", "native.lay")
                    self.assertEqual(display.call_count, 4)
                    shell.run_cell_magic('laymesh', '-o quality.jpg --dpi 144 --quality 91 --background "#ffffff"', 'page=canvas(size=(25.4mm,12.7mm))')
                    shell.run_line_magic('laymesh', 'existing.lay -o compressed.tif --dpi 144 --compression lzw')
                    shell.run_line_magic('laymesh', 'existing.lay -o compressed.pdf --dpi 1200 --pdf-image-compression jpeg --pdf-preserve-16bit false --pdf-preserve-alpha false --pdf-alpha-background \"#ffffff\"')
                    shell.run_cell_magic('laymesh', '-o compressed.webp --dpi 144 --webp-lossless false --quality 83 --webp-method 5 --webp-alpha-quality 100', 'page=canvas(size=(25.4mm,12.7mm))')
                    from PIL import Image
                    for filename in ['quality.jpg', 'compressed.webp']:
                        with Image.open(filename) as image:self.assertEqual(image.size,(144,72))
                    with Image.open('compressed.tif') as image:self.assertEqual(image.tag_v2[259],5)
                    self.assertEqual(display.call_count, 8)
                    shell.run_cell_magic('laymesh', '-o editable.pptx --dpi 144 --warnings hide', 'page=canvas(size=(64mm,36mm))\npage.add(text("Notebook PPTX"))')
                    shell.run_line_magic('laymesh', 'native.lay -o replay.pptx --warnings hide')
                    import zipfile
                    from xml.etree import ElementTree as ET
                    with zipfile.ZipFile('editable.pptx') as deck:
                        xml=ET.fromstring(deck.read('ppt/slides/slide1.xml'))
                        self.assertEqual([n.text for n in xml.findall('.//{http://schemas.openxmlformats.org/drawingml/2006/main}t')],['Notebook PPTX'])
                    self.assertTrue(zipfile.is_zipfile('replay.pptx'))
                    self.assertEqual(display.call_count, 10)
            finally:
                os.chdir(previous)


if __name__ == "__main__":
    unittest.main()
