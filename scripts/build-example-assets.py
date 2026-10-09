#!/usr/bin/env python3
"""Render the editable ring source into an antialiased transparent bitmap fixture."""
import argparse
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--binary', type=Path, default=ROOT / 'target/release/laymesh')
args = parser.parse_args()
with tempfile.TemporaryDirectory(prefix='laymesh-example-assets-') as directory:
    source = Path(directory) / 'ring.lay'
    asset = ROOT / 'examples/effects/alpha-ring.svg'
    source.write_text(f'p=canvas(size=(128mm,128mm),background="none")\np.add(image(src="{asset}"),size=(128mm,128mm))\n')
    subprocess.run([str(args.binary.resolve()), 'render', str(source), '-o', str(asset.with_suffix('.png')),
                    '--dpi', '203.2', '--warnings', 'hide'], check=True)
print('Rendered alpha-ring.png at 1024×1024 from its vector source')
