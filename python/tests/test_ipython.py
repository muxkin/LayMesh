import importlib.util
import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch


@unittest.skipUnless(importlib.util.find_spec("IPython"), "IPython is optional outside Notebook environments")
class MagicTest(unittest.TestCase):
    def test_line_and_cell_magics_display_previews(self):
        from IPython.core.interactiveshell import InteractiveShell

        shell = InteractiveShell.instance()
        shell.run_line_magic("load_ext", "laymesh.ipython")
        self.assertIsNotNone(shell.find_line_magic("laymesh"))
        self.assertIsNotNone(shell.find_cell_magic("laymesh"))
        previous = Path.cwd()
        try:
            with tempfile.TemporaryDirectory(prefix="laymesh-magic-test-") as directory:
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
        finally:
            os.chdir(previous)


if __name__ == "__main__":
    unittest.main()
