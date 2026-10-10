import io
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from laymesh import DocumentResult, LayMeshBridgeError, render_document


class DocumentTest(unittest.TestCase):
    def test_export_preview_and_notebook_use_one_native_process(self):
        from PIL import Image, JpegImagePlugin
        from laymesh.bridge import _run
        from IPython.core.interactiveshell import InteractiveShell
        with tempfile.TemporaryDirectory(prefix="laydoc-python-") as directory:
            root = Path(directory)
            (root / "first.lay").write_text('p=canvas(size=(25.4,12.7),background="#fff")\np.add(rect(size=(5,5),fill="#f00"))')
            (root / "second.lay").write_text('p=canvas(size=(12.7,25.4),background="#fff")\np.add(rect(size=(5,5),fill="#00f"))')
            file = root / "report.laydoc"
            file.write_text('d=document(name="Report")\nd.add(page(src="first.lay",id="first"))\nd.add(page(src="second.lay",id="second"))')
            with patch("laymesh.bridge._run", wraps=_run) as run:
                result = render_document(file, output=root / "report.pdf", preview_page=2)
                self.assertEqual(run.call_count, 1)
            self.assertIsInstance(result, DocumentResult)
            self.assertEqual(result.page, 2)
            self.assertIn(b"/Count 2", result.output.read_bytes())
            self.assertNotIn("preview_jpeg=", repr(result))
            with Image.open(io.BytesIO(result.preview_jpeg)) as image:
                self.assertEqual(image.size, (96, 192))
                self.assertEqual(JpegImagePlugin.get_sampling(image), 0)
            bundle = result._repr_mimebundle_()
            self.assertIn("image/jpeg", bundle[0])
            self.assertEqual(bundle[1]["image/jpeg"]["width"], 48)
            shell = InteractiveShell.instance()
            shell.run_line_magic("load_ext", "laymesh.ipython")
            with patch("laymesh.ipython.display") as shown:
                shell.run_line_magic("laymesh", f'"{file}" --page 2')
                self.assertEqual(shown.call_args.args[0].data, result.preview_jpeg)
            with patch("laymesh.bridge._run", wraps=_run) as run:
                page = render_document(file, output=root / "second.png", page=2, dpi=96)
                self.assertEqual(run.call_count, 1)
            with Image.open(page.output) as image:
                self.assertEqual(image.size, (48, 96))

    def test_invalid_pages_and_options_keep_previous_exports(self):
        with tempfile.TemporaryDirectory(prefix="laydoc-errors-") as directory:
            root = Path(directory)
            file = root / "report.laydoc"
            file.write_text('d=document()\nd.add(page(src="page.lay"))')
            (root / "page.lay").write_text('p=canvas(size=(10,10))')
            output = root / "keep.pdf"
            output.write_bytes(b"existing")
            for kwargs in (dict(page=0), dict(page=True), dict(page=2), dict(preview_page=2),
                           dict(preview_dpi=float("nan")), dict(preview_quality=101), dict(quality=80)):
                with self.assertRaises(LayMeshBridgeError):
                    render_document(file, output=output, **kwargs)
                self.assertEqual(output.read_bytes(), b"existing")
            with self.assertRaises(LayMeshBridgeError):
                render_document(file, output=root / "implicit.png")


if __name__ == "__main__":
    unittest.main()
