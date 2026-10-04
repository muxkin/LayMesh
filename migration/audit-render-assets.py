#!/usr/bin/env python3
"""Compare actual embedded fonts, inputs and vector exports against the saved Node baseline.

Uses only Python's standard library plus the locally installed Poppler tools.
Writes fresh diagnostic outputs into an ignored directory; never overwrites the
published comparison. Re-run with the final native binary before delivery.
"""
import argparse
import base64
import hashlib
import json
import re
import struct
import subprocess
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SOURCES = ['basic.lay', 'typography.lay', 'outlines.lay', 'plot/multi-axes-breaks.lay',
           'plot/statistics.lay', 'plot/polar-field.lay']


def sha(data):
    return hashlib.sha256(data).hexdigest()


def tables(data):
    result = {}
    for i in range(struct.unpack_from('>H', data, 4)[0]):
        tag, _, offset, size = struct.unpack_from('>4sIII', data, 12 + i * 16)
        result[tag.decode('ascii')] = data[offset:offset + size]
    return result


def names(data):
    table = tables(data)['name']
    _, count, offset = struct.unpack_from('>HHH', table)
    result = set()
    for i in range(count):
        platform, _, _, name_id, length, start = struct.unpack_from('>HHHHHH', table, 6 + i * 12)
        if name_id not in (1, 2, 6):
            continue
        raw = table[offset + start:offset + start + length]
        result.add(raw.decode('utf-16-be' if platform in (0, 3) else 'mac-roman'))
    return sorted(result)


def fonts(svg):
    result = {}
    for mime, encoded in re.findall(r'data:(font/[^;,]+);base64,([A-Za-z0-9+/=]+)', svg):
        data = base64.b64decode(encoded)
        family = names(data)
        result[tuple(family)] = {'data': data, 'names': family, 'mime': mime,
                                 'bytes': len(data), 'sha256': sha(data)}
    return result


def compare_font(old, new):
    record = {b: {k: v for k, v in f.items() if k != 'data'} for b, f in [('node', old), ('rust', new)]}
    a, b = tables(old['data']), tables(new['data'])
    differences = []
    equivalent = True
    for tag in sorted(a.keys() | b.keys()):
        left, right = a.get(tag, b''), b.get(tag, b'')
        if left == right:
            continue
        normalization = None
        if tag == 'DSIG':
            normalization = 'Collection extraction omits the digital-signature table.'
        elif tag == 'head' and left[:8] + left[12:] == right[:8] + right[12:]:
            normalization = 'Only the standalone sfnt checkSumAdjustment differs.'
        elif tag == 'CFF ' and left.replace(b'NotoSansCJKsc', b'NotoSansCJKjp').replace(b'Noto Sans CJK SC', b'Noto Sans CJK JP') == right:
            normalization = 'Only Noto SC/JP family-name strings differ in the shared CFF table; charstrings and every other CFF byte are identical.'
        else:
            equivalent = False
        differences.append({'table': tag, 'node_sha256': sha(left), 'rust_sha256': sha(right),
                            'node_bytes': len(left), 'rust_bytes': len(right),
                            'same_length_differing_bytes': sum(x != y for x, y in zip(left, right)) if len(left) == len(right) else None,
                            'explanation': normalization})
    record.update({'exact_font_bytes_equal': old['data'] == new['data'],
                   'glyph_mapping_outlines_and_metrics_tables_equal': equivalent,
                   'different_tables': differences})
    return record


def vector_record(svg_path, pdf_path):
    tree = ET.fromstring(svg_path.read_text())
    formulae = [e for e in tree.iter() if e.get('data-latex-source') is not None]
    def image(e):
        return e.tag.rsplit('}', 1)[-1] == 'image'
    image_rows = subprocess.check_output(['pdfimages', '-list', str(pdf_path)], text=True)
    count = sum(bool(re.match(r'^\s*\d+\s+\d+\s+image\s', line)) for line in image_rows.splitlines())
    return {'svg_sha256': sha(svg_path.read_bytes()), 'pdf_sha256': sha(pdf_path.read_bytes()),
            'svg_images': sum(image(e) for e in tree.iter()), 'pdf_raster_images': count,
            'svg_formulae': [{'source': e.get('data-latex-source'),
                              'raster_descendants': sum(image(c) for c in e.iter())} for e in formulae],
            'pdf_formula_sources_extract': all(e.get('data-latex-source') in subprocess.check_output(
                ['pdftotext', '-raw', str(pdf_path), '-'], text=True) for e in formulae)}


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--binary', type=Path, default=ROOT / 'target/debug/laymesh')
    p.add_argument('--output', type=Path, default=ROOT / 'release/verification/render-assets')
    args = p.parse_args()
    binary, output = args.binary.resolve(), args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    report = {'baseline_commit': '78db22d', 'rust_commit': subprocess.check_output(
        ['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(), 'binary': str(binary),
        'binary_sha256': sha(binary.read_bytes()), 'examples': [],
        'boundary': 'The six examples explicitly select their body fonts. No differing default system face is used. SVG font bytes prove selected outlines and metrics; package font policy is checked separately. Basic contains intended raster photo assets. Formula-containing examples have zero PDF raster images.'}
    for source in SOURCES:
        rel = 'examples/' + source
        name = source.replace('/', '-').removesuffix('.lay')
        current = (ROOT / rel).read_bytes()
        baseline = subprocess.check_output(['git', 'show', '78db22d:' + rel], cwd=ROOT)
        assert current == baseline, rel
        inputs = [{'path': rel, 'sha256': sha(current), 'same_baseline_bytes': True}]
        for src in re.findall(r'\bsrc\s*=\s*"([^"]+)"', current.decode()):
            asset = (ROOT / rel).parent / src
            rel_asset = asset.relative_to(ROOT).as_posix()
            old = subprocess.check_output(['git', 'show', '78db22d:' + rel_asset], cwd=ROOT)
            assert old == asset.read_bytes(), rel_asset
            inputs.append({'path': rel_asset, 'sha256': sha(old), 'same_baseline_bytes': True})
        for suffix in ('svg', 'pdf'):
            subprocess.run([str(binary), 'render', str(ROOT / rel), '-o', str(output / f'{name}.{suffix}'), '--warnings', 'hide'], check=True, stdout=subprocess.DEVNULL)
        old_base = ROOT / 'release/comparison/node' / name
        old_svg, new_svg = old_base.with_suffix('.svg'), output / f'{name}.svg'
        old_fonts, new_fonts = fonts(old_svg.read_text()), fonts(new_svg.read_text())
        assert old_fonts.keys() == new_fonts.keys(), (rel, old_fonts.keys(), new_fonts.keys())
        records = [compare_font(old_fonts[k], new_fonts[k]) for k in sorted(old_fonts)]
        assert all(r['glyph_mapping_outlines_and_metrics_tables_equal'] for r in records), rel
        record = {'source': rel, 'inputs': inputs, 'fonts': records,
                  'node': vector_record(old_svg, old_base.with_suffix('.pdf')),
                  'rust': vector_record(new_svg, output / f'{name}.pdf')}
        for backend in ('node', 'rust'):
            assert all(f['raster_descendants'] == 0 for f in record[backend]['svg_formulae']), rel
            if record[backend]['svg_formulae']:
                assert record[backend]['pdf_raster_images'] == 0, rel
                assert record[backend]['pdf_formula_sources_extract'], rel
        report['examples'].append(record)
        print(name, 'fonts=', len(records), 'exact=', sum(r['exact_font_bytes_equal'] for r in records), flush=True)
    destination = output / 'font-and-vector-evidence.json'
    destination.write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n')
    print(destination)


if __name__ == '__main__':
    main()
