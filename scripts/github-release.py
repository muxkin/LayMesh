#!/usr/bin/env python3
"""Validate a stable release tag, then publish exactly the reviewed GitHub assets."""
from __future__ import annotations
import argparse
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]

def run(*args: str, **kwargs):
    return subprocess.run(args, check=True, text=True, **kwargs)

def validate_tag(tag: str) -> str:
    if not re.fullmatch(r'v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)', tag):
        raise ValueError('Release requires a stable canonical vMAJOR.MINOR.PATCH tag')
    version = tag[1:]
    sys.path.insert(0, str(ROOT / 'scripts'))
    import importlib.util
    spec = importlib.util.spec_from_file_location('release_check', ROOT / 'scripts/check-release.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    if module.check(ROOT) != version:
        raise ValueError('Tag must match Rust, Python, runtime and extension versions')
    run('git', 'fetch', 'origin', 'main', '--no-tags', cwd=ROOT)
    run('git', 'merge-base', '--is-ancestor', f'{tag}^{{commit}}', 'origin/main', cwd=ROOT)
    head = run('git', 'rev-parse', 'HEAD', cwd=ROOT, capture_output=True).stdout.strip()
    sha = run('git', 'rev-parse', f'{tag}^{{commit}}', cwd=ROOT, capture_output=True).stdout.strip()
    if head != sha:
        raise ValueError('Checkout must be the exact tagged commit')
    if not (ROOT / f'release/notes/{version}.md').is_file():
        raise ValueError('Missing bilingual release notes')
    return version

def checksums(directory: Path) -> dict[str, str]:
    result = {}
    for line in (directory / 'SHA256SUMS').read_text().splitlines():
        digest, name = line.split(maxsplit=1)
        name = name.lstrip('*')
        if not re.fullmatch('[0-9a-f]{64}', digest) or Path(name).name != name or name in result:
            raise ValueError('Invalid or duplicate checksum entry')
        if hashlib.sha256((directory / name).read_bytes()).hexdigest() != digest:
            raise ValueError(f'Asset checksum mismatch: {name}')
        result[name] = digest
    files = {p.name for p in directory.iterdir() if p.is_file() and p.name != 'SHA256SUMS'}
    if set(result) != files or len([n for n in files if n.endswith('.whl')]) != 5 or len([n for n in files if n.endswith('.vsix')]) != 5:
        raise ValueError('Expected exactly five reviewed wheels and five reviewed VSIX packages')
    return result

def publish(tag: str, directory: Path, repo: str) -> None:
    version = validate_tag(tag)
    expected = checksums(directory)
    probe = subprocess.run(['gh', 'api', f'repos/{repo}/releases/tags/{tag}'], capture_output=True, text=True)
    if probe.returncode:
        # A transient/API/auth failure must not be treated as a missing release.
        listing = json.loads(run('gh', 'api', f'repos/{repo}/releases?per_page=100', capture_output=True).stdout)
        if any(r['tag_name'] == tag for r in listing):
            raise ValueError('Existing release could not be inspected')
        run('gh', 'release', 'create', tag, '--repo', repo, '--verify-tag', '--draft', '--title', f'LayMesh {version}', '--notes-file', str(ROOT / f'release/notes/{version}.md'))
        info = {'assets': [], 'draft': True}
    else:
        info = json.loads(probe.stdout)
    existing = {a['name']: a for a in info['assets']}
    wanted = {**expected, 'SHA256SUMS': hashlib.sha256((directory / 'SHA256SUMS').read_bytes()).hexdigest()}
    if set(existing) - set(wanted):
        raise ValueError('Existing release contains unexpected assets; refusing to replace it')
    for name, metadata in existing.items():
        # GitHub provides a content digest for release assets; never overwrite a different package.
        asset = json.loads(run('gh', 'api', f'repos/{repo}/releases/assets/{metadata["id"]}', capture_output=True).stdout)
        if asset.get('digest') != f'sha256:{wanted[name]}':
            raise ValueError(f'Existing asset has an absent or conflicting digest: {name}')
    for name in sorted(set(wanted) - set(existing)):
        run('gh', 'release', 'upload', tag, str(directory / name), '--repo', repo)
    final = json.loads(run('gh', 'api', f'repos/{repo}/releases/tags/{tag}', capture_output=True).stdout)
    if {a['name'] for a in final['assets']} != set(wanted):
        raise ValueError('Release asset verification failed')
    for item in final['assets']:
        asset = json.loads(run('gh', 'api', f'repos/{repo}/releases/assets/{item["id"]}', capture_output=True).stdout)
        if asset.get('digest') != f'sha256:{wanted[item["name"]]}':
            raise ValueError(f'Release asset digest mismatch: {item["name"]}')
    if info['draft']:
        run('gh', 'release', 'edit', tag, '--repo', repo, '--draft=false')

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command', choices=['check-tag', 'publish'])
    parser.add_argument('--tag', required=True)
    parser.add_argument('--assets', type=Path)
    parser.add_argument('--repo', default='muxkin/LayMesh')
    args = parser.parse_args()
    if args.command == 'check-tag':
        print(validate_tag(args.tag))
    else:
        if not args.assets:
            parser.error('publish requires --assets')
        publish(args.tag, args.assets, args.repo)
