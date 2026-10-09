#!/usr/bin/env python3
"""Verify filled-arrow exports, optional WASM parity and LibreOffice round-trip.

Build WASM with cargo and wasm-bindgen --target nodejs first; pass its .js file
using --wasm. Artifacts and measured evidence remain under --output.
"""
import argparse
import importlib.util
import json
import re
import subprocess
import tempfile
import xml.etree.ElementTree as ET
from pathlib import Path
from PIL import Image, ImageChops, ImageStat

ROOT = Path(__file__).resolve().parents[1]
SVG = '{http://www.w3.org/2000/svg}'
NUM = re.compile(r'[-+]?(?:\d*\.\d+|\d+\.?\d*)(?:[eE][-+]?\d+)?')


def run(command):
    result = subprocess.run([str(v) for v in command], cwd=ROOT, capture_output=True, text=True, timeout=240)
    assert result.returncode == 0, result.stderr[-4000:]
    return result


def compare_svg(a, b):
    left, right = list(ET.fromstring(a).iter()), list(ET.fromstring(b).iter())
    assert len(left) == len(right), (len(left), len(right))
    maximum = 0.0
    for x, y in zip(left, right):
        assert x.tag == y.tag
        assert x.text == y.text
        assert set(x.attrib) == set(y.attrib), (x.attrib.keys(), y.attrib.keys())
        for key, value in x.attrib.items():
            other = y.attrib[key]
            if value == other:
                continue
            assert NUM.sub('#', value) == NUM.sub('#', other), (key, value[:100], other[:100])
            ns, ms = list(map(float, NUM.findall(value))), list(map(float, NUM.findall(other)))
            assert len(ns) == len(ms)
            for u, v in zip(ns, ms):
                error = abs(u-v)
                assert error <= 1e-8 * max(1, abs(u), abs(v)), (key, u, v)
                maximum = max(maximum, error)
    return maximum


def image_difference(a, b):
    with Image.open(a) as x, Image.open(b) as y:
        assert x.size == y.size, (x.size, y.size)
        return sum(ImageStat.Stat(ImageChops.difference(x.convert('RGB'), y.convert('RGB'))).mean)/3


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--binary', type=Path, default=ROOT/'target/release/laymesh')
    p.add_argument('--output', type=Path, default=ROOT/'examples/output/arrow-module/verification')
    p.add_argument('--wasm', type=Path)
    p.add_argument('--libreoffice', action='store_true')
    args = p.parse_args()
    args.output = args.output.resolve(); args.output.mkdir(parents=True, exist_ok=True)
    binary = args.binary.resolve()
    font = ROOT/'tests/fonts/DejaVuSans.ttf'
    spec = importlib.util.spec_from_file_location('pptx_checks', ROOT/'scripts/test-pptx-export.py')
    pptx = importlib.util.module_from_spec(spec); spec.loader.exec_module(pptx)
    evidence = {'powerpoint_native_tested': False, 'cases': []}
    for name in ['arrows', 'arrow-connections', 'arrow-effects']:
        source = ROOT/f'examples/gallery/shapes/{name}.lay'
        fixed = args.output/f'{name}.lay'
        fixed.write_text(source.read_text().replace('DejaVu Sans', str(font)))
        case = {'name': name, 'exports': {}}
        for ext in ['svg', 'png', 'pdf', 'pptx']:
            output = args.output/f'{name}.{ext}'
            command = [binary, 'render', fixed, '-o', output]
            if ext == 'png': command += ['--dpi', '150']
            result = run(command)
            case['exports'][ext] = {'bytes': output.stat().st_size, 'warnings': result.stderr.strip().splitlines()}
        deck = args.output/f'{name}.pptx'
        info = pptx.inspect(deck)
        case['pptx'] = info
        if name != 'arrow-effects':
            assert info['pictures'] == 0 and not info['media'], info
            assert not any('W_PPTX_RASTER' in w for w in case['exports']['pptx']['warnings'])
        else:
            assert info['pictures'] > 0
            assert any('W_PPTX_RASTER' in w for w in case['exports']['pptx']['warnings'])
        # Poppler rasterizes the actual PDF, independently of native PNG output.
        png = args.output/f'{name}.png'
        with Image.open(png) as im: width, height = im.size
        native_pdf_prefix = args.output/f'{name}-pdf'
        run(['pdftoppm', '-png', '-singlefile', '-scale-to-x', width, '-scale-to-y', height, args.output/f'{name}.pdf', native_pdf_prefix])
        case['pdf_png_mean_rgb_error'] = image_difference(png, native_pdf_prefix.with_suffix('.png'))
        assert case['pdf_png_mean_rgb_error'] < 5, case
        images = run(['pdfimages', '-list', args.output/f'{name}.pdf']).stdout
        image_rows = [line for line in images.splitlines() if re.match(r'^\s*\d+\s+\d+', line)]
        case['pdf_image_layers'] = len(image_rows)
        if name != 'arrow-effects': assert not image_rows, images
        if args.wasm:
            js = r'''
const fs=require('fs');
const m=require(process.argv[1]);
const file=process.argv[2], font=process.argv[3];
m.register_preview_font(font,fs.readFileSync(font));
const source=fs.readFileSync(file,'utf8');
const result=JSON.parse(m.render(source,file,'{}'));
fs.writeFileSync(process.argv[4],result.svg);
const files=JSON.stringify({[file]:source});
const d=JSON.parse(m.language_query(files,file,'diagnostics',0,'en'));
if(d.length)throw Error(JSON.stringify(d));
const partial='a=arrow.';
const c=JSON.parse(m.language_query(JSON.stringify({[file]:partial}),file,'completions',partial.length,'en'));
for(const name of ['arc','bent','uturn','chevron','path'])if(!c.some(x=>x.label===name))throw Error(name);
'''
            wasm_svg = args.output/f'{name}-wasm.svg'
            run(['node', '-e', js, args.wasm.resolve(), fixed, font, wasm_svg])
            case['wasm_max_numeric_difference'] = compare_svg((args.output/f'{name}.svg').read_text(), wasm_svg.read_text())
        if args.libreoffice:
            lo = args.output/'libreoffice'; lo.mkdir(exist_ok=True)
            rt = args.output/'roundtrip'; rt.mkdir(exist_ok=True)
            with tempfile.TemporaryDirectory(prefix='laymesh-arrow-lo-') as profile:
                prefix = ['libreoffice', '-env:UserInstallation='+Path(profile).as_uri(), '--headless']
                run(prefix+['--convert-to','pdf','--outdir',lo,deck])
                run(prefix+['--convert-to','pptx:Impress MS PowerPoint 2007 XML','--outdir',rt,deck])
                run(prefix+['--convert-to','pdf','--outdir',rt,rt/f'{name}.pptx'])
            rt_info = pptx.inspect(rt/f'{name}.pptx', roundtrip=True)
            assert rt_info['text_box_contents'] == info['text_box_contents']
            assert rt_info['shapes'] == info['shapes'], (info,rt_info)
            for folder in [lo, rt]:
                run(['pdftoppm','-png','-singlefile','-scale-to-x',width,'-scale-to-y',height,folder/f'{name}.pdf',folder/name])
            case['libreoffice_mean_rgb_error'] = image_difference(native_pdf_prefix.with_suffix('.png'),lo/f'{name}.png')
            case['roundtrip_mean_rgb_error'] = image_difference(lo/f'{name}.png',rt/f'{name}.png')
            assert case['libreoffice_mean_rgb_error'] < 8,case
            assert case['roundtrip_mean_rgb_error'] < 4,case
            case['roundtrip'] = rt_info
        evidence['cases'].append(case)
        print(name, 'passed', flush=True)
    evidence['status'] = 'passed'
    (args.output/'evidence.json').write_text(json.dumps(evidence,ensure_ascii=False,indent=2)+'\n')
    print(args.output/'evidence.json')


if __name__ == '__main__': main()
