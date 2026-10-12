"""Python bindings retain native diagnostics and offline theme imports."""
import tempfile
import unittest
from pathlib import Path

from laymesh import render_source
from laymesh.bridge import LayMeshBridgeError


class CssUnits(unittest.TestCase):
    def test_inverse_unit_diagnostic_and_valid_suggestions_reach_python(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            with self.assertRaises(LayMeshBridgeError) as caught:
                render_source('page=canvas(size=(20mm,12*7/3cm))', base_dir=root, show_warnings=False)
            message = str(caught.exception)
            for text in ['E_UNIT', '倒数长度', '(12*7)/(3cm)', '12cm*7/3', '(12*7/3)*1cm']:
                self.assertIn(text, message)
            for expression in ['12cm*7/3', '(12*7/3)*1cm']:
                result = render_source(f'page=canvas(size=(20mm,{expression}))', base_dir=root, show_warnings=False)
                self.assertIn('viewBox="0 0 20 280"', result.preview_svg)

    def test_shared_theme_loads_all_bundled_libraries_offline(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            (root / 'theme.lcss').write_text('''@import "@laymesh/bootstrap.css";
@import "@laymesh/open-props.css";
@import "@laymesh/animate.css";
.custom {animation-duration:2s;}
''', encoding='utf-8')
            result = render_source('''page=canvas(size=(40mm,30mm),stylesheet="theme.lcss")
a=page.add(rect(size=(20mm,10mm),class="bg-primary rounded shadow"),offset=(5mm,5mm))
s=page.step()
s.animate(a,class="animate__animated animate__fadeInUp custom")
''', base_dir=root, show_warnings=False)
            self.assertIn('#0d6efd', result.preview_svg)
            self.assertIn('feGaussianBlur', result.preview_svg)
