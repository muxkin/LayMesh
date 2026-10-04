import os
import tempfile
import unittest
import warnings
from pathlib import Path
from unittest.mock import patch

from laymesh import render_source, render_file, LayMeshBridgeError
from laymesh.ipython import _options

FONT = (Path(__file__).resolve().parents[2] / 'tests/fonts/DejaVuSans.ttf').as_posix()
SOURCE = f'''page=canvas(size=(100 mm,90 mm))
p=plot(projection="polar",size=(95 mm,85 mm),plot_area=box(offset=(15 mm, 15 mm), size=(60 mm, 60 mm)),style=plot_style(font_family="{FONT}"),r=axis(range=(0,5)))
p.line(theta={{{{angles}}}},r={{{{radii}}}})
page.add(p)'''


class PolarWarningTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='laymesh-python-polar-')
        self.addCleanup(self.temp.cleanup)
        self.dir = Path(self.temp.name)

    def render(self, **kwargs):
        return render_source(SOURCE, namespace={'angles': [0, 90, 180], 'radii': [-2, 3, 4]}, base_dir=self.dir, **kwargs)

    def test_presets_overrides_and_deduplication(self):
        with patch.dict(os.environ, {'LAYMESH_WARNINGS': 'hide'}):
            with warnings.catch_warnings(record=True) as caught:
                warnings.simplefilter('always')
                hidden = self.render(output=self.dir/'hidden.pdf', save_source='polar.lay')
            self.assertEqual(caught, [])
            with warnings.catch_warnings(record=True) as caught:
                warnings.simplefilter('always')
                shown = self.render(output=self.dir/'shown.pdf', show_warnings=True)
            negative = [w for w in caught if 'W_POLAR_NEGATIVE_RADIUS' in str(w.message)]
            self.assertEqual(len(negative), 1)
            self.assertEqual(hidden.preview_svg, shown.preview_svg)
            repeat = render_file(hidden.saved_source, show_warnings=False)
            self.assertEqual(repeat.preview_svg, hidden.preview_svg)
        with patch.dict(os.environ, {'LAYMESH_WARNINGS': 'show'}):
            with warnings.catch_warnings(record=True) as caught:
                warnings.simplefilter('always')
                self.render(show_warnings=False)
                warnings.warn('unrelated application warning')
            self.assertEqual(len(caught), 1)
            self.assertEqual(str(caught[0].message), 'unrelated application warning')

    def test_hidden_warnings_do_not_hide_errors(self):
        with self.assertRaises(LayMeshBridgeError):
            render_source('invalid()', base_dir=self.dir, show_warnings=False)
        with patch.dict(os.environ, {'LAYMESH_WARNINGS': 'invalid'}):
            with self.assertRaises(LayMeshBridgeError):
                self.render()
            self.render(show_warnings=False)

    def test_notebook_option(self):
        self.assertEqual(_options('--warnings hide', cell=True).warnings, 'hide')
        self.assertEqual(_options('figure.lay --warnings show', cell=False).warnings, 'show')

    def test_radar_and_periodic_fields_python_bindings(self):
        source=f'''page=canvas(size=(180 mm,95 mm))
s=plot_style(font_family="{FONT}")
p=plot(projection="polar",size=(90 mm,90 mm),plot_area=box(offset=(15 mm, 15 mm), size=(60 mm, 60 mm)),style=s,r=axis(range=(0,2)))
p.contourf(z={{{{z}}}},theta=[0,90,180,270],r=[0,1,2],levels=[0,1,2],periodic=true)
a=page.add(p)
r=plot(projection="radar",size=(90 mm,90 mm),plot_area=box(offset=(15 mm, 15 mm), size=(60 mm, 60 mm)),style=s,categories=["A","B","C"],ranges=[(0,10),(-5,5),(0,1)])
r.area(values={{{{values}}}})
page.add(r,target=a.top_right)'''
        result=render_source(source, namespace={'z': [[0]*4,[1]*4,[2]*4], 'values':[5,-2,0.8]}, base_dir=self.dir, output=self.dir/'polar.png', dpi=120, save_source='fields.lay', show_warnings=False)
        self.assertIn('clip-rule="evenodd"', result.preview_svg)
        self.assertTrue(result.output.read_bytes().startswith(b'\x89PNG'))
