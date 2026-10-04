#!/usr/bin/env python3
"""Generate/check static presets using Matplotlib 3.11.2 (development only)."""
import argparse
import hashlib
import json
from pathlib import Path
import matplotlib
import numpy as np

VERSION = '3.11.2'
ROOT = Path(__file__).resolve().parents[1]
ALIASES = {'grey': 'gray', 'gist_grey': 'gist_gray', 'gist_yerg': 'gist_yarg', 'Grays': 'Greys', 'rdbu': 'RdBu'}
GROUPS = {
    'sequential': 'viridis plasma inferno magma cividis Greys Purples Blues Greens Oranges Reds YlOrBr YlOrRd OrRd PuRd RdPu BuPu GnBu PuBu YlGnBu PuBuGn BuGn YlGn gray bone pink spring summer autumn winter cool Wistia hot afmhot gist_heat copper'.split(),
    'diverging': 'PiYG PRGn BrBG PuOr RdGy RdBu RdYlBu RdYlGn Spectral coolwarm bwr seismic berlin managua vanimo'.split(),
    'cyclic': 'twilight twilight_shifted hsv'.split(),
    'qualitative': 'Pastel1 Pastel2 Paired Accent okabe_ito Dark2 Set1 Set2 Set3 tab10 tab20 tab20b tab20c'.split(),
}

def registry():
    if matplotlib.__version__ != VERSION:
        raise SystemExit(f'Requires matplotlib=={VERSION}; found {matplotlib.__version__}')
    presets = []
    for name in matplotlib.colormaps:
        if name.endswith('_r') or name in ALIASES:
            continue
        cm = matplotlib.colormaps[name]
        def encode(c):
            # Integer inputs select native LUT entries, including 510-entry twilight.
            rgba = c(np.arange(c.N))
            if not np.all(rgba[:, 3] == 1):
                raise AssertionError(f'Unexpected alpha in {name}')
            rgb = np.floor(rgba[:, :3] * 255 + .5).astype(np.uint8)
            return rgb.tobytes().hex()
        presets.append({'name': name, 'category': next((g for g, names in GROUPS.items() if name in names), 'misc'), 'colors': encode(cm), 'reverse_colors': encode(matplotlib.colormaps[name + '_r'])})
    return {'version': VERSION, 'source': f'https://github.com/matplotlib/matplotlib/tree/v{VERSION}/lib/matplotlib', 'source_sha256': {name: hashlib.sha256((Path(matplotlib.__file__).parent / name).read_bytes()).hexdigest() for name in ['_cm.py', '_cm_listed.py']}, 'aliases': ALIASES, 'presets': presets}

def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--check', action='store_true')
    args = p.parse_args()
    text = json.dumps(registry(), ensure_ascii=False, separators=(',', ':')) + '\n'
    dest = ROOT / 'crates/laymesh-core/cmaps.json'
    if args.check:
        assert dest.read_text() == text, 'Stale Matplotlib preset data'
    else:
        dest.write_text(text)
    print(f'{"Checked" if args.check else "Generated"} {len(json.loads(text)["presets"])} presets against Matplotlib {VERSION}, including native reversed LUTs')

if __name__ == '__main__':
    main()
