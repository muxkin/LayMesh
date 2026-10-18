"""Exact-text deduplication of the licenses for the shipped browser dependency graph.

Native distributions keep their full existing inventory. Never abbreviate legal
texts or combine different copyright holders merely because the SPDX ID agrees.
"""
import hashlib
import json
import re
import subprocess
import tomllib


def collect(root):
    packages, texts = [], {}
    profiles = {}
    for package in ('laymesh-playback', 'laymesh-web-export', 'laymesh-tiff-export', 'laymesh-wasm'):
        tree = subprocess.check_output([
            'cargo', 'tree', '--locked', '--target', 'wasm32-unknown-unknown',
            '--no-default-features', '-p', package, '--edges', 'normal',
            '--prefix', 'none', '--format', '{p}',
        ], cwd=root, text=True)
        profiles[package] = set(re.findall(r'^([^\s]+) v([^\s]+)', tree, re.M))
    playback = profiles['laymesh-playback'] | profiles['laymesh-tiff-export']
    media = playback | profiles['laymesh-web-export']
    editable = playback | profiles['laymesh-wasm'] | profiles['laymesh-web-export']

    def add(name, version, license, source, refs, modes):
        record = dict(name=name, version=version, license=license, source=source,
                      profiles=modes, texts=[])
        for filename, data, digest in refs:
            actual = hashlib.sha256(data).hexdigest()
            assert digest is None or actual == digest, f'Modified license: {filename}'
            # JSON escapes preserve the original UTF-8 byte sequence, including
            # CRLF, when decoded. The original hash remains independently checkable.
            text = data.decode('utf-8')
            assert text.encode('utf-8') == data
            texts[actual] = text
            record['texts'].append(dict(name=filename, sha256=actual))
        record['playbackExportOnly'] = (name,version) not in profiles['laymesh-playback'] if (name,version) in editable else False
        assert record['texts'], f'Missing license: {name}'
        packages.append(record)

    inventory = json.loads((root/'release/licenses/manifest.json').read_text())
    found = set()
    for record in inventory:
        key = record['name'], record['version']
        if key not in editable:
            continue
        found.add(key)
        modes = ['editable'] + (['playback'] if key in playback else []) + (['media'] if key in media else [])
        add(*key, record['license'], record.get('repository'), [
            (r['file'], (root/'release/licenses'/r['file']).read_bytes(), r['sha256'])
            for r in record['texts']
        ], modes)
    assert not {p for p in editable-found if not p[0].startswith('laymesh-')}, editable-found

    def local(name, version, license, source, path, modes=('editable','playback','media')):
        add(name, version, license, source, [(path.name, path.read_bytes(), None)], list(modes))

    local('LayMesh', tomllib.loads((root/'Cargo.toml').read_text())['workspace']['package']['version'], 'MIT', 'https://github.com/muxkin/LayMesh', root/'LICENSE')
    for folder, name, license in [('fflate','fflate','MIT'),('canvaskit','CanvasKit','BSD-3-Clause')]:
        path = root/'vendor'/folder
        info = json.loads((path/'manifest.json').read_text())
        local(name, info['version'], license, info['source'], path/'LICENSE')
    # Colormap LUTs, formula fonts, CSS and transition sources carry their own
    # notices. Retain these conservatively in both modes for prepared content.
    for record in inventory:
        if record['name']=='Matplotlib colormap data' or record['name'].endswith(' CSS'):
            add(record['name'], record['version'], record['license'], record.get('repository'), [
                (r['file'], (root/'release/licenses'/r['file']).read_bytes(),r['sha256']) for r in record['texts']
            ], ['editable','playback','media'])
    for name, info in json.loads((root/'vendor/workspace-ui/manifest.json').read_text()).items():
        file=next(p for p in info['files'] if 'LICENSE' in p)
        local(name, info['version'], info['license'], info['source'], root/'vendor/workspace-ui'/file)
    info=json.loads((root/'vendor/headless-tree/manifest.json').read_text())
    local('@headless-tree/core',info['version'],info['license'],info['source'],root/'vendor/headless-tree/LICENSE',('editable',))
    for name, info in json.loads((root/'site/vendor/manifest.json').read_text()).items():
        # The prebundled workspace UI has its complete per-component inventory above.
        if name in ('workspace-ui','@headless-tree/core','pdf-export'): continue
        local(name,info['version'],'See original text',None,root/'site/vendor'/info['license'],('editable',))
    for name, info in json.loads((root/'vendor/pdf-export/manifest.json').read_text()).items():
        refs=[(p,(root/'vendor/pdf-export'/p).read_bytes(),digest) for p,digest in info['files'].items() if any(n in p.upper() for n in ('LICENSE','COPYING','NOTICE'))]
        add(name,info['version'],info['license'],info['source'],refs,['editable','playback','media'])
        packages[-1]['playbackExportOnly']=True
    return dict(schemaVersion=1,
                description='Original license and attribution texts. Each package references the SHA-256 of its complete UTF-8 text; identical texts are stored once. Native-only and build-only dependencies are not part of this web runtime.',
                packages=sorted(packages,key=lambda p:(p['name'],p['version'])),
                texts=dict(sorted(texts.items())))


def select(inventory, editable):
    mode='editable' if editable else 'playback'
    packages=[p for p in inventory['packages'] if mode in p['profiles']]
    used={r['sha256'] for p in packages for r in p['texts']}
    return {**inventory,'packages':packages,'texts':{k:v for k,v in inventory['texts'].items() if k in used}}
