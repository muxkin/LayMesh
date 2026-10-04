import os
import shutil
import tempfile
import unittest
import warnings
from pathlib import Path
from unittest.mock import patch
from xml.etree import ElementTree

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np
import pandas as pd
from PIL import Image

from laymesh import LayMeshBridgeError, render_file, render_source


ROOT = Path(__file__).resolve().parents[2]


class BridgeTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="laymesh-python-test-")
        self.addCleanup(self.temp.cleanup)
        self.dir = Path(self.temp.name)

    def _figure(self, heatmap=False):
        fig, ax = plt.subplots(figsize=(3, 2))
        if heatmap:
            ax.imshow(np.arange(16).reshape(4, 4))
        else:
            frame = pd.DataFrame({"x": np.arange(4), "y": [1, 3, 2, 4]})
            ax.plot(frame["x"], frame["y"])
        self.addCleanup(lambda: plt.close(fig))
        return fig

    def _source(self):
        return (
            'page=canvas(size=(100 mm,70 mm))\nchart=image(src={{fig}})\npage.add(chart,size=(70 mm, auto),target=page.top_left)\nlabel=text(content={{title}},font_size=10 pt)\npage.add(label,target=page.bottom_left)\n'
        )

    def test_vector_figure_saved_source_and_png_rerender(self):
        result = render_source(
            self._source(), namespace={"fig": self._figure(), "title": 'A "quote"'},
            base_dir=self.dir, output=self.dir / "figure.pdf", save_source="figure.lay",
        )
        self.assertTrue(result.output.is_file())
        self.assertIn("<svg", result.preview_svg)
        self.assertNotIn(b"/Subtype/Image", b"".join(result.output.read_bytes().split()))
        saved = result.saved_source.read_text(encoding="utf-8")
        self.assertIn('A \\"quote\\"', saved)
        self.assertIn("figure.assets/fig-", saved)
        self.assertEqual([p.suffix for p in (self.dir / "figure.assets").iterdir()], [".svg"])
        repeated = render_file(result.saved_source, output=self.dir / "repeat.png", dpi=300)
        with Image.open(repeated.output) as image:
            self.assertEqual(image.size, (1181, 827))

    def test_heatmap_raster_fallback_and_separate_plot_dpi(self):
        figure = self._figure(heatmap=True)
        figure.axes[0].set_xlabel("x")
        with warnings.catch_warnings(record=True) as caught:
            warnings.simplefilter("always")
            result = render_source(
                self._source(), namespace={"fig": figure, "title": "heat"},
                base_dir=self.dir, output=self.dir / "figure.png", dpi=150,
                plot_dpi=240, save_source="figure.lay",
            )
        self.assertTrue(any("240 DPI PNG" in str(w.message) for w in caught))
        self.assertTrue(any("W_PLOT_BOUNDS" in str(w.message) for w in caught))
        asset = next((self.dir / "figure.assets").iterdir())
        self.assertEqual(asset.suffix, ".png")
        with Image.open(asset) as image:
            self.assertEqual(image.size, (720, 480))
        with Image.open(result.output) as image:
            self.assertEqual(image.size, (591, 413))

    def test_plot_bounds_warn_once_without_changing_export_size(self):
        fig, ax = plt.subplots(figsize=(4, 2.5))
        self.addCleanup(lambda: plt.close(fig))
        x = np.linspace(0, 2 * np.pi, 100)
        ax.plot(x, np.sin(x))
        ax.set_xlabel("x")
        ax.set_ylabel("sin(x)")
        source = (
            'page=canvas(size=(160 mm,100 mm))\nfirst=image(src={{fig}})\nsecond=image(src={{fig}})\npage.add(first,size=(130 mm, auto),target=page.top_left)\n'
        )
        with warnings.catch_warnings(record=True) as caught:
            warnings.simplefilter("always")
            result = render_source(
                source, namespace={"fig": fig}, base_dir=self.dir,
                output=self.dir / "clipped.pdf", save_source="clipped.lay",
            )
        plot_warnings = [str(w.message) for w in caught if "W_PLOT_BOUNDS" in str(w.message)]
        self.assertEqual(len(plot_warnings), 1)
        self.assertIn("left", plot_warnings[0])
        self.assertIn("bottom", plot_warnings[0])
        self.assertIn("fig.tight_layout()", plot_warnings[0])
        self.assertTrue(plot_warnings[0].isascii())
        asset = next((self.dir / "clipped.assets").glob("*.svg"))
        root = ElementTree.parse(asset).getroot()
        self.assertEqual(root.attrib["viewBox"], "0 0 288 180")
        self.assertIn('viewBox="0 0 160 100"', result.preview_svg)
        self.assertTrue(result.output.is_file())

        fig.tight_layout()
        with warnings.catch_warnings(record=True) as caught:
            warnings.simplefilter("always")
            render_source(source, namespace={"fig": fig}, base_dir=self.dir)
        self.assertFalse(any("W_PLOT_BOUNDS" in str(w.message) for w in caught))

    def test_unavailable_plot_bounds_do_not_prevent_rendering(self):
        fig = self._figure()
        with patch.object(fig, "get_tightbbox", side_effect=RuntimeError("renderer unavailable")):
            result = render_source(
                'page=canvas(size=(100 mm,70 mm))\nchart=image(src={{fig}})\npage.add(chart,size=(60 mm, auto),target=page.top_left)\n',
                namespace={"fig": fig}, base_dir=self.dir,
            )
        self.assertIn("<svg", result.preview_svg)

    def test_file_relative_assets_and_original_source_unchanged(self):
        folder = self.dir / "layouts"
        folder.mkdir()
        shutil.copyfile(ROOT / "examples/assets/photo.png", folder / "photo.png")
        file = folder / "layout.lay"
        source = (
            'page=canvas(size=(100 mm,70 mm))\nphoto=image(src="photo.png")\npage.add(photo,size=(50 mm, auto),target=page.top_left)\n'
        )
        file.write_text(source, encoding="utf-8")
        result = render_file(file, output=self.dir / "out.svg")
        self.assertIn("<image", result.preview_svg)
        self.assertEqual(file.read_text(encoding="utf-8"), source)

    def test_bad_bindings_are_clear_and_do_not_render(self):
        cases = [
            ("{{missing}}", {}, "未定义"),
            ('{{df["x"]}}', {}, "Python 表达式"),
            ("{{value}}", {"value": np.array([1+2j])}, "不支持的数据类型"),
            ("{{value}}", {"value": object()}, "类型不支持"),
            ("{{value}}", {"value": np.nan}, "有限"),
        ]
        for expression, namespace, message in cases:
            with self.subTest(expression=expression, message=message):
                with self.assertRaisesRegex(LayMeshBridgeError, message):
                    render_source(
                        f"page=canvas(size=(10 mm,10 mm))\nvalue={expression}\n",
                        namespace=namespace, base_dir=self.dir,
                    )
        self.assertFalse(list(self.dir.glob(".laymesh-*.lay")))

    def test_binding_tokens_inside_strings_and_comments_remain_literal(self):
        result = render_source(
            'page=canvas(size=(80,40))\n# {{missing}}\n'
            'page.add(text("{{missing}}",font_size=10))\n',
            namespace={}, base_dir=self.dir,
        )
        self.assertIn("{{missing}}", result.preview_svg)

    def test_numpy_scalar_and_generated_file_protection(self):
        result = render_source(
            "page=canvas(size=(10 mm,10 mm))\nvalue={{value}}\n",
            namespace={"value": np.float64(2.5)}, base_dir=self.dir,
            save_source="generated.lay",
        )
        self.assertIn("value=2.5", result.saved_source.read_text(encoding="utf-8"))
        manual = self.dir / "manual.lay"
        manual.write_text("page=canvas(size=(10 mm,10 mm))\n", encoding="utf-8")
        with self.assertRaisesRegex(LayMeshBridgeError, "不会覆盖"):
            render_source("page=canvas(size=(10 mm,10 mm))\n", base_dir=self.dir, save_source="manual.lay")

    def test_ordered_dictionary_binding_saved_source_and_replay(self):
        from collections import OrderedDict
        import json
        source = ('page=canvas(size=(90,60))\n'
                  'series={{series}}\n'
                  'p=plot(size=(70,50),style=plot_style(colors=palette("tab10")))\n'
                  'for name,ys in series.items() {p.line(x=[0,1,2],y=ys,label=name)}\n'
                  'page.add(p)\n')
        series = OrderedDict([("B", np.array([1,2,3])), ("A", (2,1,4))])
        result = render_source(source, namespace={"series": series}, base_dir=self.dir,
                               save_source="dictionary.lay")
        self.assertIn("dict(src=", result.saved_source.read_text())
        data = json.loads(next((self.dir / "dictionary.assets").glob("*.json")).read_text())
        self.assertEqual(list(data), ["B", "A"])
        replay = render_file(result.saved_source)
        self.assertEqual(result.preview_svg, replay.preview_svg)
        self.assertEqual(series["B"].tolist(), [1,2,3])

    def test_nested_empty_dict_and_numpy_values(self):
        result = render_source('page=canvas(size=(10,10))\nd={{data}}\n'
                               'if d["z"]["empty"]!={} or d["a"]!=[true,null,"x"] {bad=missing}\n',
                               namespace={"data": {"z": {"empty": {}}, "a": [True, None, "x"],
                                                   "n": np.float64(3), "missing": float("nan")}},
                               base_dir=self.dir)
        self.assertTrue(result.preview_svg)

    def test_dict_errors_report_the_field_before_rendering(self):
        cycle = {}; cycle["self"] = cycle
        for value, pattern in [({1: 2}, "键"), ({"outer": {"bad": object()}}, "outer.*bad"),
                               (cycle, "self.*循环引用"), ({"large": 2**53}, "large"),
                               ({"inf": float("inf")}, "inf")]:
            with self.subTest(value=pattern):
                with patch("laymesh.bridge.subprocess.run") as run:
                    with self.assertRaisesRegex(LayMeshBridgeError, pattern):
                        render_source('page=canvas(size=(10,10))\nd={{data}}',
                                      namespace={"data": value}, base_dir=self.dir)
                    run.assert_not_called()

    def test_missing_cli_is_reported(self):
        with patch.dict(os.environ, {"LAYMESH_CLI": str(self.dir / "missing")}) :
            with self.assertRaisesRegex(LayMeshBridgeError, "不存在"):
                render_source("page=canvas(size=(10 mm,10 mm))", base_dir=self.dir)


if __name__ == "__main__":
    unittest.main()
