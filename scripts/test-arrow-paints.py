#!/usr/bin/env python3
"""Render the six arrow templates with shared paints, outlines, and effects.

Keeps editable .lay sources, SVG/PNG/PDF/PPTX exports, a compact overview, and
measured evidence in --output. Optional WASM and LibreOffice checks exercise
the real asset loader and the exported presentation, respectively.
"""
import argparse
import hashlib
import importlib.util
import io
import json
import tempfile
import xml.etree.ElementTree as ET
import zipfile
from pathlib import Path

from PIL import Image, ImageChops, ImageStat

ROOT = Path(__file__).resolve().parents[1]
FONT = ROOT / 'tests/fonts/DejaVuSans.ttf'
ASSET = ROOT / 'examples/gallery/assets/sample.png'


def module(name, filename):
    spec = importlib.util.spec_from_file_location(name, ROOT / 'scripts' / filename)
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


exports = module('arrow_exports', 'test-arrow-exports.py')
pptx = module('pptx_checks', 'test-pptx-export.py')
run = exports.run

TEMPLATES = [
    ('arrow()', 'arrow(length=54,shaft_width=6,head_size=(11,14)'),
    ('arrow.arc()', 'arrow.arc(radius=14,start_angle=160deg,sweep_angle=250deg,shaft_width=5,head_size=(8,11)'),
    ('arrow.bent()', 'arrow.bent(span=(43,24),corner_radius=8,shaft_width=5,head_size=(9,12)'),
    ('arrow.uturn()', 'arrow.uturn(span=(38,23),corner_radius=9,shaft_width=4,heads=both,head_size=(8,10)'),
    ('arrow.chevron()', 'arrow.chevron(length=54,shaft_width=(4,7),head_size=(12,15),notch_depth=4'),
    ('arrow.path()', 'arrow.path(path=curve,shaft_width=[(0,2mm),(0.5,6mm),(1,3mm)],head_size=(8,10)'),
]
LINEAR = 'linear_gradient(start=(0,0),end=(1,0),stops=[(0,"#28c9b5"),(0.5,"#499bec"),(1,"#7655c6")])'
STYLES = [
    ('white-border', 'White border', 'fill="#087f8c",border_color="#ffffff",border_width=0.8mm'),
    ('dashed-border', 'Dashed border', 'fill="#b7e8e7",border_color="#087f8c",border_width=0.6mm,border_style=dashed'),
    ('dotted-border', 'Dotted border', 'fill="#b7e8e7",border_color="#087f8c",border_width=0.6mm,border_style=dotted'),
    ('double-border', 'Double border', 'fill="#b7e8e7",border_color="#087f8c",border_width=1.4mm,border_style=double'),
    ('triple-border', 'Triple border', 'fill="#b7e8e7",border_color="#087f8c",border_width=1.6mm,border_style=triple'),
    ('linear-gradient', 'Linear gradient', f'fill={LINEAR},border_color="#ffffff",border_width=0.4mm'),
    ('radial-gradient', 'Radial gradient', 'fill=radial_gradient(center=(0.35,0.3),radius=0.8,stops=[(0,"#fff8b9"),(0.45,"#ef9862"),(1,"#94436e")])'),
    ('hatch-slash', 'Slash pattern', 'fill=hatch(pattern=slash,color="#167383",background="#e0f6ed",spacing=2mm,line_width=0.6pt),border_color="#167383",border_width=0.3mm'),
    ('hatch-cross', 'Cross pattern', 'fill=hatch(pattern=cross,color="#7752a1",background="#ede2f7",spacing=2.3mm,line_width=0.5pt),border_color="#7752a1",border_width=0.3mm'),
    ('hatch-dots', 'Dot pattern', 'fill=hatch(pattern=dots,color="#176f8c",background="#def0fa",spacing=2mm,line_width=1.4pt),border_color="#176f8c",border_width=0.3mm'),
    *[(f'image-{fit}', f'Image / {fit}', f'fill=image_fill(src={json.dumps(str(ASSET))},fit={fit}),border_color="#486a84",border_width=0.4mm') for fit in ('cover', 'contain', 'stretch')],
    ('outer-shadow', 'Outer shadow', 'fill="#4daaa9",border_color="#ffffff",border_width=0.4mm,effects=[shadow(color="#153957",blur=1mm,offset=(1.2mm,1.5mm),opacity=0.65)]'),
    ('inner-shadow', 'Inner shadow', 'fill="#4daaa9",effects=[shadow(mode=inner,color="#123047",blur=0.8mm,offset=(1mm,1mm),opacity=0.85)]'),
    ('glow', 'Glow', 'fill="#32699c",border_color="#ffffff",border_width=0.4mm,effects=[glow(color="#2495fc",blur=1.5mm,opacity=0.9)]'),
    ('opacity', '50% opacity', 'fill="#087f8c",border_color="#ffffff",border_width=0.6mm'),
    ('combined', 'Gradient + shadow + alpha', f'fill={LINEAR},border_color="#ffffff",border_width=0.5mm,effects=[shadow(blur=1mm,offset=(1mm,1.5mm),opacity=0.7)]'),
]


def header(title, width=240, height=155):
    return f'''page=canvas(size=({width}mm,{height}mm),background="#f2f5fa")
style {{ text {{font-family:{json.dumps(str(FONT))};font-size:10pt;color:#24324b;}} }}
page.add(text({json.dumps(title)},font_size=20pt),offset=(9,7))
curve=path(commands=[move_to(0,20),cubic_to(18,20,10,0,28,0),cubic_to(44,0,36,20,54,20)])
'''


def card(template, style, label, x, y, *, opacity=1):
    # Fixed card boxes make per-arrow visual comparisons independent of titles.
    return f'''page.add(rect(size=(72,54),fill="#e3eaf3",border_radius=3mm),offset=({x},{y}))
page.add(text({json.dumps(label)},font_size=9pt),offset=({x+3},{y+3}))
page.add({template},{style}),offset=({x+5},{y+15}),opacity={opacity})
'''


def difference(a, b, box=None):
    with Image.open(a) as left, Image.open(b) as right:
        assert left.size == right.size
        if box:
            left, right = left.crop(box), right.crop(box)
        return sum(ImageStat.Stat(ImageChops.difference(left.convert('RGB'), right.convert('RGB'))).mean)/3


def compare_previews(a, b):
    with Image.open(a) as image:
        sx, sy = image.width/240, image.height/155
    return [difference(a, b, tuple(round(v) for v in (
        (8+i%3*76)*sx, (29+i//3*59+12)*sy,
        (8+i%3*76+72)*sx, (29+i//3*59+54)*sy))) for i in range(6)]


def pdf_preview(pdf, png):
    with Image.open(png) as im:
        width, height = im.size
    out = pdf.with_name(pdf.stem+'-pdf.png')
    run(['pdftoppm', '-png', '-singlefile', '-scale-to-x', width, '-scale-to-y', height, pdf, out.with_suffix('')])
    return out


def check_physical_samples(name, png):
    """Check important interiors directly: a page average can hide fit errors."""
    with Image.open(png) as image:
        image = image.convert('RGB')
        def at(x, y):
            return image.getpixel((round(x/240*image.width), round(y/155*image.height)))
        if name == 'image-contain':
            # The 640:400 image fits inside a 54 x 14 mm straight arrow. Both
            # ends remain transparent, while the center contains the image.
            background = (227, 234, 243)
            for x in (20, 60):
                assert max(abs(a-b) for a, b in zip(at(x, 51), background)) <= 3, (png, x, at(x, 51))
            assert max(abs(a-b) for a, b in zip(at(40, 51), background)) > 20, png
        elif name == 'opacity':
            # No internal border or double alpha at the neck x=56 mm.
            colors = [at(x, 51) for x in (50, 55.5, 56, 56.5, 60)]
            assert all(max(abs(a-b) for a, b in zip(color, colors[0])) <= 2 for color in colors), (png, colors)
            expected = (118, 181, 192)
            assert max(abs(a-b) for a, b in zip(colors[0], expected)) <= 3, (png, colors)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, default=ROOT/'target/release/laymesh')
    parser.add_argument('--output', type=Path, default=ROOT/'examples/output/arrow-module/paints')
    parser.add_argument('--wasm', type=Path)
    parser.add_argument('--libreoffice', action='store_true')
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=True)
    evidence_file = out/'evidence.json'
    evidence_file.unlink(missing_ok=True)
    evidence = dict(binary_sha256=hashlib.sha256(args.binary.read_bytes()).hexdigest(),
                    powerpoint_native_tested=False, export_dpi=300, preview_dpi=150,
                    templates=[t[0] for t in TEMPLATES], cases=[])
    for name, title, style in STYLES:
        source = out/f'{name}.lay'
        alpha = 0.5 if name == 'opacity' else 0.7 if name == 'combined' else 1
        source.write_text(header(title)+''.join(card(template, style, label, 8+i%3*76, 29+i//3*59, opacity=alpha)
                                                for i, (label, template) in enumerate(TEMPLATES)))
        case = dict(name=name, exports={})
        for ext in ('svg', 'png', 'pdf', 'pptx'):
            output = source.with_suffix('.'+ext)
            cmd = [args.binary.resolve(), 'render', source, '-o', output]
            if ext != 'svg':
                cmd += ['--dpi', 150 if ext == 'png' else 300]
            result = run(cmd)
            case['exports'][ext] = dict(bytes=output.stat().st_size, warnings=result.stderr.strip().splitlines())
        case['pptx'] = pptx.inspect(source.with_suffix('.pptx'))
        with zipfile.ZipFile(source.with_suffix('.pptx')) as archive:
            slide = ET.fromstring(archive.read('ppt/slides/slide1.xml'))
            case['pptx_custom_geometries'] = len(slide.findall('.//a:custGeom', pptx.NS))
            if name.startswith('image-'):
                assert case['pptx_custom_geometries'] >= 6
                # Asset normalization can re-encode PNG losslessly. Check the
                # decoded original pixels, rather than compressed file bytes.
                with Image.open(ASSET) as original:
                    pixels = original.convert('RGBA')
                preserved = False
                for path in case['pptx']['media']:
                    with Image.open(io.BytesIO(archive.read(path))) as embedded:
                        preserved |= embedded.size == pixels.size and not ImageChops.difference(
                            embedded.convert('RGBA'), pixels).getbbox(alpha_only=False)
                assert preserved, 'image fill was flattened or changed source pixels'
        png = source.with_suffix('.png')
        preview = pdf_preview(source.with_suffix('.pdf'), png)
        for image in (png, preview):
            check_physical_samples(name, image)
        case['pdf_png_arrow_errors'] = compare_previews(png, preview)
        assert max(case['pdf_png_arrow_errors']) < 5, case
        if args.wasm:
            wasm_svg = source.with_name(name+'-wasm.svg')
            run(['node', '-e', '''const fs=require('fs'), m=require(process.argv[1]);
const [file,font,asset,out]=process.argv.slice(2);
m.register_preview_font(font,fs.readFileSync(font));
const files=JSON.stringify({[asset]:Array.from(fs.readFileSync(asset))});
const r=JSON.parse(m.render(fs.readFileSync(file,'utf8'),file,files));
if(!r.svg)throw Error(JSON.stringify(r)); fs.writeFileSync(out,r.svg);
''', args.wasm.resolve(), source, FONT, ASSET, wasm_svg])
            case['wasm_max_numeric_difference'] = exports.compare_svg(source.with_suffix('.svg').read_text(), wasm_svg.read_text())
        evidence['cases'].append(case)
        evidence_file.write_text(json.dumps(evidence, indent=2)+'\n')
        print(name, 'native / PDF'+(' / WASM' if args.wasm else '')+' passed', flush=True)
    # An independently editable overview, drawn by LayMesh itself.
    picks = ['white-border', 'double-border', 'linear-gradient', 'radial-gradient',
             'hatch-slash', 'hatch-cross', 'hatch-dots', 'image-cover', 'image-contain',
             'image-stretch', 'inner-shadow', 'combined']
    summary = header('Arrow paints and effects', height=273)
    for i, name in enumerate(picks):
        _, title, style = next(s for s in STYLES if s[0] == name)
        template = TEMPLATES[1 if i%3 == 0 else 5][1]
        summary += card(template, style, title, 8+i%3*76, 29+i//3*59, opacity=0.7 if name == 'combined' else 1)
    overview = out/'overview.lay'
    overview.write_text(summary)
    for ext in ('svg', 'png', 'pdf'):
        cmd = [args.binary.resolve(), 'render', overview, '-o', overview.with_suffix('.'+ext)]
        if ext != 'svg': cmd += ['--dpi', 150 if ext == 'png' else 300]
        run(cmd)
    if args.libreoffice:
        lo, rt = out/'libreoffice', out/'roundtrip'
        lo.mkdir(exist_ok=True); rt.mkdir(exist_ok=True)
        decks = [out/(case['name']+'.pptx') for case in evidence['cases']]
        for deck in decks:
            (lo/(deck.stem+'.pdf')).unlink(missing_ok=True)
            (rt/deck.name).unlink(missing_ok=True)
            (rt/(deck.stem+'.pdf')).unlink(missing_ok=True)
        with tempfile.TemporaryDirectory(prefix='arrow-paints-lo-') as profile:
            prefix = ['libreoffice', '-env:UserInstallation='+Path(profile).as_uri(), '--headless']
            run(prefix+['--convert-to', 'pdf', '--outdir', lo, *decks])
            run(prefix+['--convert-to', 'pptx:Impress MS PowerPoint 2007 XML', '--outdir', rt, *decks])
            run(prefix+['--convert-to', 'pdf', '--outdir', rt, *[rt/d.name for d in decks]])
        for case in evidence['cases']:
            name = case['name']
            saved = pptx.inspect(rt/(name+'.pptx'), roundtrip=True)
            assert saved['text_box_contents'] == case['pptx']['text_box_contents']
            assert saved['pictures']+saved['shapes'] == case['pptx']['pictures']+case['pptx']['shapes']
            png = out/(name+'.png')
            lo_png = pdf_preview(lo/(name+'.pdf'), png)
            rt_png = pdf_preview(rt/(name+'.pdf'), png)
            for image in (lo_png, rt_png):
                check_physical_samples(name, image)
            case['libreoffice_arrow_errors'] = compare_previews(out/(name+'-pdf.png'), lo_png)
            case['roundtrip_arrow_errors'] = compare_previews(lo_png, rt_png)
            case['roundtrip'] = saved
            evidence_file.write_text(json.dumps(evidence, indent=2)+'\n')
            assert max(case['libreoffice_arrow_errors']) < 12, case
            assert max(case['roundtrip_arrow_errors']) < 4, case
            print(name, 'LibreOffice round-trip passed', flush=True)
    evidence['status'] = 'passed'
    evidence_file.write_text(json.dumps(evidence, indent=2)+'\n')
    print(evidence_file)


if __name__ == '__main__':
    main()
