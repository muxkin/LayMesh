import json
import tempfile
import unittest
import warnings
from pathlib import Path

import numpy as np
import pandas as pd

from laymesh import LayMeshBridgeError, render_source, render_file


class NativePlotTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="laymesh-native-")
        self.addCleanup(self.temp.cleanup)
        self.dir = Path(self.temp.name)

    def test_dataframe_array_and_matrix_are_portable_hashed_assets(self):
        frame = pd.DataFrame({"x": [0, 1, 2], "y": [1, 2, 3], "sample": ["a", "b", "c"]})
        source = '''page=canvas(size=(180 mm,80 mm),background="#ffffff")
        d={{df}}
        xs={{xs}}
        z={{z}}
        p=plot(size=(80 mm,65 mm),x=axis(label="x"),y=axis(label="y"))
        p.line(x=xs,y=d["y"])
        a=page.add(p)
        h=plot(size=(80 mm,65 mm))
        heat=h.heatmap(z=z)
        h.colorbar(heat,label="Value")
        page.add(h,target=a.top_right,offset=(5 mm,0 mm))'''
        font = Path(__file__).resolve().parents[2] / "tests/fonts/DejaVuSans.ttf"
        source = source.replace("plot(size=", f'plot(font_family="{font}",size=')
        result = render_source(source, namespace={"df": frame, "xs": np.arange(3), "z": np.arange(9).reshape(3, 3)},
                               base_dir=self.dir, output=self.dir / "native.pdf", save_source="native.lay")
        self.assertIn("<text", result.preview_svg)
        saved = result.saved_source.read_text()
        self.assertNotIn("{{", saved)
        self.assertIn('table(src="native.assets/df-', saved)
        self.assertIn('array(src="native.assets/z-', saved)
        assets = list((self.dir / "native.assets").glob("*.json"))
        self.assertEqual(len(assets), 3)
        for asset in assets:
            self.assertNotIn("NaN", asset.read_text())
            json.loads(asset.read_text())
        repeat = render_file(result.saved_source, output=self.dir / "repeat.svg")
        self.assertEqual(result.preview_svg, repeat.preview_svg)
        render_source(source, namespace={"df": frame, "xs": np.arange(3), "z": np.arange(9).reshape(3, 3)},
                      base_dir=self.dir, save_source="native.lay")
        self.assertEqual(len(list((self.dir / "native.assets").glob("*.json"))), 3)

    def test_missing_values_are_null_and_reported_by_native_layers(self):
        source = '''page=canvas(size=(100 mm,75 mm))
        x={{x}}
        y={{y}}
        p=plot(size=(90 mm,65 mm))
        p.line(x=x,y=y)
        page.add(p)'''
        with warnings.catch_warnings(record=True) as caught:
            warnings.simplefilter("always")
            result = render_source(source, namespace={"x": [0, 1, 2], "y": pd.Series([1, pd.NA, 3], dtype="Float64")},
                                   base_dir=self.dir, save_source="missing.lay")
        self.assertTrue(any("W_PLOT_MISSING" in str(w.message) for w in caught))
        data = [json.loads(p.read_text()) for p in (self.dir / "missing.assets").glob("*.json")]
        self.assertIn([1.0, None, 3.0], data)
        self.assertIn("<svg", result.preview_svg)

    def test_scientific_labels_markers_and_horizontal_colorbar_roundtrip(self):
        source = 'page=canvas(size=(150 mm,115 mm),background="#ffffff")\n        p=plot(size=(145 mm,110 mm),plot_area=box(offset=(30 mm, 20 mm), size=(80 mm, 50 mm)),\n               x=axis(label=formula(source=r"t\\;(\\mathrm{s})",font_size=9 pt),notation="offset",exponent=-6,\n                      exponent_offset=(0 mm,-0.5 mm),label_offset=(0 mm,0.5 mm)),\n               y=axis(label="Signal",notation="offset",exponent=3))\n        h=p.heatmap(z={{z}},extent=(0,0.000002,0,2000),vmin=0,vmax=3)\n        p.line(x={{x}},y={{y}},marker="diamond",marker_fill="none",marker_border_color="#000000",\n               label=formula(source=r"I-I_0",font_size=8 pt))\n        p.legend(position="top_left",background="none")\n        p.colorbar(h,position="bottom",length=60 mm,ticks=[0,1,3],notation="scientific",label="Value")\n        page.add(p)'
        result = render_source(source, namespace={"z": np.arange(4).reshape(2, 2), "x": [0, 0.000001, 0.000002], "y": [100, 600, 1200]},
                               base_dir=self.dir, output=self.dir / "scientific.pdf", save_source="scientific.lay")
        self.assertIn('data-latex-source="I-I_0"', result.preview_svg)
        self.assertIn(r'data-latex-source="\times 10^{-6}"', result.preview_svg)
        self.assertIn('label_offset=(0 mm,0.5 mm)', result.saved_source.read_text())
        self.assertIn('stroke-linejoin="round"', result.preview_svg)
        repeat = render_file(result.saved_source, output=self.dir / "scientific.png", dpi=150)
        self.assertEqual(result.preview_svg, repeat.preview_svg)
        self.assertTrue((self.dir / "scientific.pdf").read_bytes().startswith(b"%PDF"))
        self.assertTrue((self.dir / "scientific.png").read_bytes().startswith(b"\x89PNG"))

    def test_statistics_named_axes_contours_and_shared_colorbar_roundtrip(self):
        source = 'page=canvas(size=(185 mm,100 mm),background="#ffffff")\n        colors=color_scale(norm="centered",vmin=-1,vmax=1,center=0,cmap="rdbu")\n        p=plot(size=(95 mm,85 mm),plot_area=box(offset=(20 mm, 12 mm), size=(55 mm, 50 mm)),\n               x=axis(range=(0,4)),y=axis(range=(0,1)))\n        p.add_axis(name="sum",side="right",offset=3 mm,axis=axis(range=(0,1)))\n        h=p.hist(values={{values}},weights={{weights}},bins=[0,1,2,4],stat="density",hatch="slash",label="Density")\n        p.ecdf(values={{values}},y_axis="sum")\n        a=page.add(p)\n        page.add(legend(layers=[h]),target=a.plot_bottom_left,offset=(0 mm,17 mm))\n        q=plot(size=(85 mm,85 mm),plot_area=box(offset=(15 mm, 12 mm), size=(48 mm, 50 mm)))\n        q.contourf(z={{z}},x={{x}},y={{y}},levels=[-1,0,0.5],color_scale=colors)\n        b=page.add(q,offset=(98 mm,0 mm))\n        page.add(colorbar(scale=colors,orientation="horizontal",length=45 mm,ticks=[-1,0,1]),target=b.plot_bottom_left,offset=(0 mm,15 mm))'
        values = np.array([0, 0.5, 1, 2, 3, np.nan])
        original = values.copy()
        with warnings.catch_warnings(record=True) as caught:
            warnings.simplefilter("always")
            result = render_source(source, namespace={"values": values, "weights": np.ones(6),
                "x": [0, 1, 2], "y": [0, 1, 2], "z": np.array([[-1, 0, 1], [0, 1, 0], [1, 0, -1]])},
                base_dir=self.dir, output=self.dir / "complete.pdf", save_source="complete.lay")
        np.testing.assert_equal(values, original)
        self.assertTrue(any("W_PLOT_MISSING" in str(w.message) for w in caught))
        again = render_file(result.saved_source, output=self.dir / "complete.png", dpi=120)
        self.assertEqual(result.preview_svg, again.preview_svg)
        self.assertIn("clipPath", result.preview_svg)
        self.assertTrue((self.dir / "complete.pdf").read_bytes().startswith(b"%PDF"))

    def test_invalid_array_and_dataframe_shapes_fail_before_render(self):
        bad = [np.zeros((2, 2, 2)), np.array([]), np.array([1, np.inf]), np.array([1+2j]),
               [[1, 2], [3]], [True, False], pd.DataFrame([[1, 2]], columns=["x", "x"]),
               pd.DataFrame({0: [1]}), [2**60]]
        for value in bad:
            with self.subTest(value=repr(value)):
                with self.assertRaises(LayMeshBridgeError):
                    render_source("page=canvas(size=(10 mm,10 mm))\nd={{data}}", namespace={"data": value}, base_dir=self.dir)
        self.assertFalse(list(self.dir.glob(".laymesh-*.lay")))


if __name__ == "__main__":
    unittest.main()
