import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch


class MagicTest(unittest.TestCase):
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
                    self.assertTrue(all("<svg" in call.args[0].data for call in display.call_args_list))
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
