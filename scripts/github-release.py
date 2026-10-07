#!/usr/bin/env python3
"""Validate a stable release tag, then publish exactly the reviewed GitHub assets."""
from __future__ import annotations
import argparse
import hashlib
import json
import re
import subprocess
import sys
import tempfile
from pathlib import Path
from urllib.parse import quote
ROOT = Path(__file__).resolve().parents[1]

def run(*args: str, **kwargs):
    return subprocess.run(args, check=True, text=True, **kwargs)

def json_request(endpoint: str, method: str, payload: dict) -> dict:
    with tempfile.TemporaryDirectory(prefix='laymesh-release-') as temp:
        body = Path(temp) / 'body.json'
        body.write_text(json.dumps(payload), encoding='utf-8')
        return json.loads(run('gh', 'api', endpoint, '--method', method,
                              '--input', str(body), capture_output=True).stdout)

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
        # Draft releases may be absent from the tag endpoint. Inspect the
        # authenticated listing before creating or resuming a release.
        listing = json.loads(run('gh', 'api', f'repos/{repo}/releases?per_page=100', capture_output=True).stdout)
        matches = [r for r in listing if r['tag_name'] == tag]
        if not matches:
            # Use the creation response directly: the draft may not yet be
            # visible to an Actions token through list/tag lookup endpoints.
            info = json_request(f'repos/{repo}/releases', 'POST', {
                'tag_name': tag, 'name': f'LayMesh {version}',
                'body': (ROOT / f'release/notes/{version}.md').read_text(encoding='utf-8'),
                'draft': True, 'prerelease': False,
            })
        elif len(matches) == 1:
            info = matches[0]
        else:
            raise ValueError('Release could not be identified uniquely')
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
        endpoint = f'https://uploads.github.com/repos/{repo}/releases/{info["id"]}/assets?name={quote(name, safe="")}'
        run('gh', 'api', endpoint, '--method', 'POST', '--header',
            'Content-Type: application/octet-stream', '--input', str(directory / name),
            capture_output=True)
        print(f'Uploaded reviewed asset: {name}', flush=True)
    # The numeric endpoint can inspect unpublished drafts before publishing.
    final = json.loads(run('gh', 'api', f'repos/{repo}/releases/{info["id"]}', capture_output=True).stdout)
    if {a['name'] for a in final['assets']} != set(wanted):
        raise ValueError('Release asset verification failed')
    for item in final['assets']:
        asset = json.loads(run('gh', 'api', f'repos/{repo}/releases/assets/{item["id"]}', capture_output=True).stdout)
        if asset.get('digest') != f'sha256:{wanted[item["name"]]}':
            raise ValueError(f'Release asset digest mismatch: {item["name"]}')
    if info['draft']:
        json_request(f'repos/{repo}/releases/{info["id"]}', 'PATCH', {'draft': False})

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
