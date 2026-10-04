#!/usr/bin/env python3
"""Audit fonts inside native/WASM binaries and release archives, without Node.

Run after packaging: python migration/audit-release-fonts.py
Only the Python standard library is needed. The locked font crate must already
be in Cargo's cache (from the build), or supplied with --font-crate. Its archive
checksum is verified against Cargo.lock before its fonts become the allowlist.
No network access, extraction to disk, or product-file changes are performed.
"""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import re
import struct
import subprocess
import tarfile
import zipfile

ROOT = Path(__file__).resolve().parents[1]
FONT_EXTENSIONS = {'.ttf', '.otf', '.ttc', '.otc', '.woff', '.woff2'}


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def label(path):
    try:
        return str(path.resolve().relative_to(ROOT))
    except ValueError:
        return str(path.resolve())


def locked_fonts(crate_path):
    blocks = (ROOT / 'Cargo.lock').read_text().split('[[package]]')
    block = next(b for b in blocks if '\nname = "ratex-katex-fonts"\n' in b)
    version = re.search(r'^version = "([^"]+)"$', block, re.M)[1]
    checksum = re.search(r'^checksum = "([^"]+)"$', block, re.M)[1]
    crate_name = f'ratex-katex-fonts-{version}'
    if crate_path is None:
        cargo_home = Path(os.environ.get('CARGO_HOME', Path.home() / '.cargo'))
        matches = sorted((cargo_home / 'registry/cache').glob(f'*/{crate_name}.crate'))
        require(matches, 'Locked font crate is not cached; build first or pass --font-crate.')
        crate_path = matches[0]
    data = crate_path.read_bytes()
    require(sha(data) == checksum, f'{crate_path}: archive does not match Cargo.lock checksum')
    with tarfile.open(fileobj=io.BytesIO(data), mode='r:gz') as archive:
        prefix = crate_name + '/fonts/'
        assets = {m.name.removeprefix(prefix): archive.extractfile(m).read()
                  for m in archive.getmembers() if m.isfile() and m.name.startswith(prefix)}
    fonts = {name: data for name, data in assets.items() if name.endswith('.ttf')}
    require(fonts and all(name.startswith('KaTeX_') for name in fonts), 'Unexpected font allowlist')
    notices = {name: assets[name] for name in ('OFL.txt', 'FONT_NOTICE.txt')}
    return version, checksum, fonts, notices


def scan(data, allowed, fixtures):
    found = []
    for match in re.finditer(b'\x00\x01\x00\x00|OTTO', data):
        start = match.start()
        try:
            count = struct.unpack_from('>H', data, start + 4)[0]
            if not 1 <= count <= 100:
                continue
            tags, end = {}, 12 + 16 * count
            for i in range(count):
                tag, _, offset, size = struct.unpack_from('>4sIII', data, start + 12 + i * 16)
                if (not re.fullmatch(b'[A-Za-z0-9 /]{4}', tag) or size > 20_000_000
                        or offset < 12 + 16 * count or start + offset + size > len(data)):
                    raise ValueError()
                tags[tag] = (offset, size)
                end = max(end, offset + size)
            if not {b'head', b'maxp', b'name', b'cmap'} <= tags.keys():
                continue
            if struct.unpack_from('>I', data, start + tags[b'head'][0] + 12)[0] != 0x5F0F3CF5:
                continue
            font = data[start:start + end]
            known = next((name for name, value in allowed.items() if value == font), None)
            found.append({'offset': start, 'font': known, 'bytes': end, 'sha256': sha(font)})
        except (ValueError, struct.error):
            continue
    require(len(found) == len(allowed) and all(f['font'] is not None for f in found),
            f'Expected only {len(allowed)} complete allowlisted KaTeX fonts, found {found}')
    require(sorted(f['font'] for f in found) == sorted(allowed), 'Missing or duplicated embedded font')
    # Collection and webfont payloads would bypass standalone sfnt offsets.
    for match in re.finditer(b'ttcf|wOFF|wOF2', data):
        start, magic = match.start(), match.group()
        try:
            flavor, length = struct.unpack_from('>II', data, start + 4)
            if magic == b'ttcf':
                if flavor in (0x10000, 0x20000) and 1 <= length <= 100:
                    offsets = struct.unpack_from(f'>{length}I', data, start + 12)
                    require(not all(start + o + 12 <= len(data) and
                                    data[start + o:start + o + 4] in (b'OTTO', b'\0\1\0\0')
                                    for o in offsets), 'Unexpected embedded TTC/OTC collection')
            elif flavor in (0x10000, 0x4F54544F) and 44 <= length <= len(data) - start:
                count = struct.unpack_from('>H', data, start + 12)[0]
                require(not 1 <= count <= 100, 'Unexpected embedded WOFF/WOFF2 font')
        except struct.error:
            continue
    absent = {name: raw not in data for name, raw in fixtures.items()}
    require(all(absent.values()), f'Body test font bytes leaked into binary: {absent}')
    return {'bytes': len(data), 'sha256': sha(data), 'complete_sfnt_fonts': found,
            'all_font_bytes_match_locked_ratex': True, 'test_font_bytes_absent': absent}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--dist', type=Path, default=ROOT / 'release/dist')
    parser.add_argument('--native', type=Path, help='Native binary; default: DIST/laymesh-linux-x64')
    parser.add_argument('--wasm', type=Path, default=ROOT / 'site/dist/site/live/wasm/laymesh_wasm_bg.wasm')
    parser.add_argument('--font-crate', type=Path)
    parser.add_argument('--output', type=Path,
                        default=ROOT / 'release/verification/render-assets/release-font-policy.json')
    args = parser.parse_args()
    native = args.native or args.dist / 'laymesh-linux-x64'
    version, checksum, allowed, notices = locked_fonts(args.font_crate)
    fixtures = {label(f): f.read_bytes() for directory in ('tests', 'crates/laymesh-render/tests')
                for f in (ROOT / directory).rglob('*') if f.suffix.lower() in FONT_EXTENSIONS}
    require(fixtures, 'Body font fixtures are required for the absence checks')
    binary_assets = [{'path': label(p), **scan(p.read_bytes(), allowed, fixtures)}
                     for p in (native, args.wasm)]
    expected_binaries = {row['sha256'] for row in binary_assets}
    report = {'git_commit_at_audit': subprocess.check_output(
        ['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
        'method': 'Validate sfnt table bounds and head magic, then compare every complete font '
                  'byte to the checksum-verified locked KaTeX crate. Reject other valid sfnt, '
                  'TTC/OTC/WOFF payloads and complete body-font fixture bytes. Inspect archive '
                  'members and decompressed native/WASM binaries; require license bytes.',
        'locked_font_crate': f'ratex-katex-fonts {version}', 'locked_crate_sha256': checksum,
        'binary_assets': binary_assets, 'packages': []}
    archives = sorted(p for p in args.dist.iterdir() if p.suffix in ('.whl', '.vsix', '.zip'))
    require({p.suffix for p in archives} == {'.whl', '.vsix', '.zip'}, 'Expected wheel, VSIX and WASM zip')
    for path in archives:
        with zipfile.ZipFile(path) as archive:
            names = archive.namelist()
            fonts = [n for n in names if Path(n).suffix.lower() in FONT_EXTENSIONS]
            require(not fonts, f'{path}: standalone font files are not expected: {fonts}')
            binaries = []
            for name in names:
                if name.endswith('/'):
                    continue
                data = archive.read(name)
                if data.startswith((b'\x7fELF', b'\0asm', b'MZ', b'\xcf\xfa\xed\xfe', b'\xfe\xed\xfa\xcf')):
                    item = {'member': name, **scan(data, allowed, fixtures)}
                    require(item['sha256'] in expected_binaries, f'{path}: packaged binary differs from audited build')
                    binaries.append(item)
            require(len(binaries) == 1, f'{path}: expected exactly one native/WASM binary')
            license_names = [n for n in names if f'ratex-katex-fonts-{version}-' in n]
            for suffix, contents in notices.items():
                require(any(n.endswith(suffix) and archive.read(n) == contents for n in license_names),
                        f'{path}: missing or changed formula font notice {suffix}')
            report['packages'].append({'path': label(path), 'bytes': path.stat().st_size,
                'sha256': sha(path.read_bytes()), 'uncompressed_bytes': sum(i.file_size for i in archive.infolist()),
                'standalone_font_files': fonts, 'formula_license_notices': license_names, 'binaries': binaries})
    packaging = ROOT / 'release/verification/packaging/verification.json'
    if packaging.exists():
        expected = json.loads(packaging.read_text())['artifacts']
        for item in report['packages']:
            require(expected[Path(item['path']).name] == item['sha256'], 'Packaging verification hash mismatch')
        report['packaging_verification_sha256'] = sha(packaging.read_bytes())
    report['status'] = 'passed'
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n')
    print(args.output)
    for item in report['packages']:
        print(item['path'], item['sha256'], f'{len(allowed)} allowed fonts; no standalone fonts')


if __name__ == '__main__':
    main()
