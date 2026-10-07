#!/usr/bin/env python3
"""Verify CLI PPTX export, optionally render and round-trip with LibreOffice.

Requires Pillow; visual checks also require LibreOffice and Poppler. Generated
sources, decks, PDF comparisons, PNG previews and evidence stay in --output.
"""
import argparse
import hashlib
import json
import os
import posixpath
import shutil
import subprocess
import tempfile
import xml.etree.ElementTree as ET
import zipfile
from pathlib import Path
from PIL import Image, ImageChops, ImageStat

ROOT = Path(__file__).resolve().parents[1]
P = 'http://schemas.openxmlformats.org/presentationml/2006/main'
A = 'http://schemas.openxmlformats.org/drawingml/2006/main'
R = 'http://schemas.openxmlformats.org/officeDocument/2006/relationships'
NS = {'p': P, 'a': A, 'r': R}


def run(command, *, code=0):
    result = subprocess.run([str(v) for v in command], capture_output=True, text=True, timeout=120)
    assert result.returncode == code, result.stderr[-2000:]
    return result


def inspect(deck, *, roundtrip=False):
    with zipfile.ZipFile(deck) as z:
        assert z.testzip() is None
        names = set(z.namelist())
        docs = {name: ET.fromstring(z.read(name)) for name in names if name.endswith(('.xml', '.rels'))}
        types = {n.attrib.get('PartName'): n.attrib['ContentType'] for n in docs['[Content_Types].xml']}
        assert types['/ppt/presentation.xml'].endswith('presentationml.presentation.main+xml')
        for name, doc in docs.items():
            if name.endswith('.rels'):
                parent = name.split('/_rels/')[0] if '/_rels/' in name else ''
                ids = [n.attrib['Id'] for n in doc]
                assert len(ids) == len(set(ids))
                for n in doc:
                    if n.attrib.get('TargetMode') != 'External':
                        assert posixpath.normpath(posixpath.join(parent, n.attrib['Target'])).lstrip('/') in names, name
        presentation = docs['ppt/presentation.xml']
        assert len(presentation.findall('p:sldIdLst/p:sldId', NS)) == 1
        size = presentation.find('p:sldSz', NS).attrib
        slide = docs['ppt/slides/slide1.xml']
        all_ids = [n.attrib['id'] for n in slide.findall('.//p:cNvPr', NS)]
        # LibreOffice 25.2 reuses id=1 for the spTree root and its first shape.
        # Exported packages must be fully unique; external round-trips must keep
        # every actual drawing ID unique (the tree root is not a drawing).
        root_id = slide.find('p:cSld/p:spTree/p:nvGrpSpPr/p:cNvPr', NS)
        ids = all_ids.copy()
        if roundtrip and root_id is not None:
            ids.remove(root_id.attrib['id'])
        assert len(ids) == len(set(ids))
        rels = {n.attrib['Id']: n.attrib['Target'] for n in docs['ppt/slides/_rels/slide1.xml.rels']}
        for blip in slide.findall('.//a:blip', NS):
            assert blip.attrib[f'{{{R}}}embed'] in rels
        return dict(size_emu=[int(size['cx']), int(size['cy'])],
                    texts=[n.text or '' for n in slide.findall('.//a:t', NS)],
                    text_box_contents=[''.join(n.text or '' for n in body.findall('.//a:t', NS)) for body in slide.findall('.//p:txBody', NS) if any(n.text for n in body.findall('.//a:t', NS))],
                    text_boxes=sum(any((n.text or '') for n in body.findall('.//a:t', NS)) for body in slide.findall('.//p:txBody', NS)),
                    shapes=len(slide.findall('.//p:sp', NS)),
                    groups=len(slide.findall('.//p:grpSp', NS)),
                    pictures=len(slide.findall('.//p:pic', NS)),
                    media=sorted(n for n in names if n.startswith('ppt/media/')),
                    sha256=hashlib.sha256(deck.read_bytes()).hexdigest())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, default=ROOT/'target/release'/('laymesh.exe' if os.name == 'nt' else 'laymesh'))
    parser.add_argument('--output', type=Path, default=ROOT/'examples/output/pptx-trial')
    parser.add_argument('--libreoffice', action='store_true')
    args = parser.parse_args()
    binary, output = args.binary.resolve(), args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    (output/'evidence.json').unlink(missing_ok=True)
    cases = [('hello', 'examples/hello.lay'), ('editable', 'examples/export/pptx-editable.lay'),
             ('native', 'examples/export/pptx-native.lay'), ('strokes', 'examples/export/pptx-strokes.lay'), ('vector', 'examples/vector.lay'), ('formula', 'examples/gallery/typography/display-formula.lay'),
             ('nested', 'examples/gallery/containers/group-nested.lay'),
             ('crop', 'examples/gallery/images/crop.lay'), ('effects', 'examples/effects/shadow-glow.lay'),
             ('polar', 'examples/plot/polar-data.lay')]
    evidence = dict(version=run([binary, '--version']).stdout.strip(),
                    binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
                    powerpoint_native_tested=False, cases=[], checks=[])
    for name, source in cases:
        deck = output/f'{name}.pptx'
        result = run([binary, 'render', ROOT/source, '-o', deck, '--dpi', 300])
        actual = inspect(deck)
        actual.update(name=name, source=source, warnings=result.stderr.strip().splitlines())
        assert actual['shapes'] > 0
        if name in ('hello', 'editable', 'vector', 'formula', 'nested'):
            assert actual['text_boxes'] > 0
        if name == 'editable':
            assert '可编辑文字与矢量公式' in ''.join(actual['texts'])
            assert actual['pictures'] == 2  # Original PNG and shadow; gradient is native.
            assert sum('W_PPTX_RASTER' in s for s in actual['warnings']) == 1
        if name == 'strokes':
            assert actual['pictures'] == 0
            assert not any('W_PPTX_RASTER' in s for s in actual['warnings'])
        if name == 'native':
            assert actual['pictures'] == 5  # Four native pictures, one alpha-gradient fallback.
            assert len(actual['media']) == 2  # Source PNG is shared, never cropped/re-encoded.
            # Two explicit groups and one formula group; font fallback can split
            # the title into multiple runs requiring one compound text group.
            assert 3 <= actual['groups'] <= 4, actual
            assert sum('W_PPTX_RASTER' in s for s in actual['warnings']) == 1
        evidence['cases'].append(actual)
    with tempfile.TemporaryDirectory(prefix='laymesh-pptx-cli-') as tmp:
        tmp = Path(tmp)
        protected = tmp/'protected.pptx'
        protected.write_bytes(b'existing destination')
        for flags in [['--quality', '90'], ['--compression', 'lzw'], ['--background', '#ffffff'],
                      ['--pdf-downsample', 'true'], ['--webp-method', '4'], ['--dpi', 'NaN']]:
            run([binary, 'render', ROOT/'examples/hello.lay', '-o', protected, *flags], code=2)
            assert protected.read_bytes() == b'existing destination'
        bad = tmp/'bad.lay'
        bad.write_text('page=canvas(size=(100mm,75mm))\npage.add(image(src="missing.png"))')
        run([binary, 'render', bad, '-o', protected], code=1)
        assert protected.read_bytes() == b'existing destination'
        expensive = tmp/'expensive.lay'
        expensive.write_text('page=canvas(size=(100mm,100mm))\npage.add(rect(size=(100mm,100mm),fill=radial_gradient(stops=[(0,"#fff"),(1,"#000")])))')
        run([binary, 'render', expensive, '-o', protected, '--dpi', '25400'], code=1)
        assert protected.read_bytes() == b'existing destination'
        for size in ['20mm,75mm', '1500mm,75mm']:
            bad.write_text(f'page=canvas(size=({size}))')
            run([binary, 'render', bad, '-o', protected], code=1)
            assert protected.read_bytes() == b'existing destination'
        assert not list(tmp.glob('*.tmp'))
        result = run([binary, 'render', ROOT/'examples/export/pptx-editable.lay', '-o', tmp/'quiet.pptx', '--dpi', '96', '--warnings', 'hide'])
        assert not result.stderr
    evidence['checks'].extend(['Valid XML, package targets, IDs and media links',
                               'Editable text, vector formulas, grouped geometry and local fallback',
                               'Invalid options, resource and encoding failures preserve destinations',
                               '--warnings hide suppresses export diagnostics'])
    if args.libreoffice:
        for program in ('libreoffice', 'pdfinfo', 'pdftoppm'):
            assert shutil.which(program), f'{program} required'
        pdfs, roundtrip = output/'lo', output/'roundtrip'
        pdfs.mkdir(exist_ok=True); roundtrip.mkdir(exist_ok=True)
        # Never attach to or reuse the user's running LibreOffice session.
        with tempfile.TemporaryDirectory(prefix='laymesh-pptx-lo-') as profile:
            lo = ['libreoffice', f'-env:UserInstallation={Path(profile).as_uri()}', '--headless']
            evidence['libreoffice_version'] = run(['libreoffice', '--version']).stdout.strip()
            decks = [output/f'{name}.pptx' for name, _ in cases]
            # LO can return zero even when it failed to load a file. Require new
            # artifacts explicitly instead of accepting an old output as proof.
            for deck in decks:
                (pdfs/f'{deck.stem}.pdf').unlink(missing_ok=True)
                (roundtrip/deck.name).unlink(missing_ok=True)
            run([*lo, '--convert-to', 'pdf', '--outdir', pdfs, *decks])
            run([*lo, '--convert-to', 'pptx:Impress MS PowerPoint 2007 XML', '--outdir', roundtrip, *decks])
            for actual, (name, source) in zip(evidence['cases'], cases):
                pdf, saved = pdfs/f'{name}.pdf', roundtrip/f'{name}.pptx'
                assert pdf.is_file() and saved.is_file(), f'{name}: LibreOffice failed to produce artifacts'
                assert 'Pages:           1' in run(['pdfinfo', pdf]).stdout
                saved_info = inspect(saved, roundtrip=True)
                assert saved_info['text_box_contents'] == actual['text_box_contents'], f'{name}: text lost or reordered after saving'
                assert saved_info['shapes'] >= actual['shapes'], f'{name}: geometry lost after saving'
                assert saved_info['pictures'] + saved_info['shapes'] == actual['pictures'] + actual['shapes'], f'{name}: drawing objects changed'
                assert saved_info['text_boxes'] == actual['text_boxes'], f'{name}: text boxes changed'
                assert all(abs(a-b) <= 900 for a, b in zip(saved_info['size_emu'], actual['size_emu'])), f'{name}: slide dimensions changed'
                reference = output/f'{name}-reference.pdf'
                run([binary, 'render', ROOT/source, '-o', reference, '--dpi', '300'])
                # Canonical pixel dimensions avoid one-pixel differences from
                # LibreOffice's micrometre rounding of custom slide dimensions.
                cx, cy = actual['size_emu']
                width = round(1500*cx/max(cx, cy)); height = round(1500*cy/max(cx, cy))
                for kind, path in [('pptx', pdf), ('reference', reference)]:
                    run(['pdftoppm', '-scale-to-x', width, '-scale-to-y', height, '-singlefile', '-png', path, output/f'{name}-{kind}'])
                with Image.open(output/f'{name}-pptx.png') as a, Image.open(output/f'{name}-reference.png') as b:
                    assert a.size == b.size
                    mean = sum(ImageStat.Stat(ImageChops.difference(a.convert('RGB'), b.convert('RGB'))).mean)/3
                    actual['mean_absolute_rgb_error_0_255'] = round(mean, 4)
                    # Broad visual regression gate, complemented by manual QA of
                    # full-size previews; font hinting and raster filters differ.
                    assert mean < 5, f'{name}: visual difference {mean:.3f}'
                actual['roundtrip'] = {k: saved_info[k] for k in ('shapes', 'text_boxes', 'pictures', 'size_emu')}
        evidence['checks'].append('LibreOffice one-page rendering and editable PPTX round-trip')
    (output/'evidence.json').write_text(json.dumps(evidence, ensure_ascii=False, indent=2)+'\n')
    print(f"Checked {len(cases)} PPTX examples; evidence: {output/'evidence.json'}")


if __name__ == '__main__':
    main()
