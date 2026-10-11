"""Stage pinned CanvasKit standard assets and hashed, precompressed WASM."""
import gzip
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def compressed_file(path, data):
    path.write_bytes(data)
    compressed = gzip.compress(data, compresslevel=9, mtime=0)
    path.with_name(path.name + '.gz').write_bytes(compressed)
    sizes = {'raw': len(data), 'gzip': len(compressed)}
    try:
        import brotli
        compressed = brotli.compress(data, quality=11)
        path.with_name(path.name + '.br').write_bytes(compressed)
        sizes['brotli'] = len(compressed)
    except ImportError:
        pass
    return sizes


def stage(output):
    source = ROOT / 'vendor/canvaskit'
    manifest = json.loads((source / 'manifest.json').read_text())
    output = Path(output) / 'skia'
    output.mkdir(parents=True, exist_ok=True)
    for name, digest in manifest['files'].items():
        assert hashlib.sha256((source / name).read_bytes()).hexdigest() == digest, f'CanvasKit checksum mismatch: {name}'
    sizes = {}
    names = {}
    for suffix, data in [('wasm', (source / 'canvaskit.wasm').read_bytes()), ('mjs', (source / 'canvaskit.js').read_bytes() + b'\nexport default CanvasKitInit;\n')]:
        name = 'canvaskit-' + hashlib.sha256(data).hexdigest()[:20] + '.' + suffix
        names[suffix] = name
        sizes[name] = compressed_file(output / name, data)
    (output / 'canvaskit.mjs').write_text(f"export {{default}} from './{names['mjs']}';\n")
    (output / 'LICENSE').write_bytes((source / 'LICENSE').read_bytes())
    (output / 'manifest.json').write_text(json.dumps({'version': manifest['version'], 'wasm': names['wasm'], 'module': names['mjs'], 'sizes': sizes}, indent=2) + '\n')
    # Drop files from earlier builds, including stale precompressed entry modules.
    keep = {'manifest.json', 'LICENSE', 'canvaskit.mjs'} | {n + ext for n in names.values() for ext in ['', '.gz', '.br']}
    for file in output.iterdir():
        if file.is_file() and file.name not in keep:
            file.unlink()
    return names['wasm']


def stage_rust(output):
    output = Path(output)
    original = output / 'laymesh_wasm_bg.wasm'
    if original.exists():
        data = original.read_bytes()
    else:
        data = (output / json.loads((output / 'manifest.json').read_text())['wasm']).read_bytes()
    digest = hashlib.sha256(data).hexdigest()
    name = f'laymesh_wasm_bg-{digest[:20]}.wasm'
    sizes = compressed_file(output / name, data)
    bindings = output / 'laymesh_wasm.js'
    import re
    text = re.sub(r'laymesh_wasm_bg(?:-[a-f0-9]+)?\.wasm', name, bindings.read_text())
    bindings.write_text(text)
    (output / 'manifest.json').write_text(json.dumps({'wasm': name, 'sha256': digest, 'sizes': sizes}, indent=2) + '\n')
    for file in output.glob('laymesh_wasm_bg*.wasm*'):
        if not file.name.startswith(name):
            file.unlink()
    return name
