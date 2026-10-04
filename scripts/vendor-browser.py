#!/usr/bin/env python3
"""Verify the checked-in, locked browser ESM sources and their license files.

Normal builds use these files directly. Updating a dependency is a reviewed source
change: replace its ESM source, preserve its upstream license, and update version
and SHA-256 in site/vendor/manifest.json. No package manager is used at build time.
"""
import hashlib,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]/'site/vendor'
manifest=json.loads((ROOT/'manifest.json').read_text())
for name,info in manifest.items():
 assert hashlib.sha256((ROOT/info['entry']).read_bytes()).hexdigest()==info['sha256'],f'Modified dependency: {name}'
 assert (ROOT/info['license']).is_file(),f'Missing license: {name}'
print(f'Checked {len(manifest)} locked static ESM packages')
