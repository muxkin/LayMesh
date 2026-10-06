#!/usr/bin/env python3
"""Check existing PyPI wheel hashes before retries and verify all uploaded wheels."""
from __future__ import annotations
import argparse
import hashlib
import json
import time
import tomllib
from pathlib import Path
from urllib.error import HTTPError
from urllib.request import urlopen
ROOT=Path(__file__).resolve().parents[1]

def expected_hashes(path:Path)->dict[str,str]:
    wheels=list(path.glob('*.whl'))
    if len(wheels)!=5:
        raise ValueError('Expected exactly five reviewed platform wheels')
    return {p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in wheels}

def check_payload(expected:dict[str,str],payload:dict|None,complete:bool)->None:
    actual={item['filename']:item['digests']['sha256'] for item in (payload or {}).get('urls',[])}
    if set(actual)-set(expected):
        raise ValueError('PyPI contains unexpected files for this release')
    for name,digest in actual.items():
        if digest!=expected[name]:
            raise ValueError(f'Existing PyPI wheel differs from the reviewed wheel: {name}')
    if complete and set(actual)!=set(expected):
        raise LookupError('PyPI has not exposed every reviewed wheel yet')

def verify(path:Path,version:str,complete:bool)->None:
    expected=expected_hashes(path)
    attempts=10 if complete else 1
    for attempt in range(attempts):
        try:
            with urlopen(f'https://pypi.org/pypi/laymesh/{version}/json',timeout=20) as response:
                payload=json.load(response)
        except HTTPError as e:
            if e.code!=404:raise
            payload=None
        try:
            check_payload(expected,payload,complete)
            print(f'PyPI {version}: '+('all five reviewed wheels verified' if complete else 'existing wheel hashes are compatible'))
            return
        except LookupError:
            if attempt+1==attempts:raise
            time.sleep(5)

if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--wheels',required=True,type=Path);p.add_argument('--complete',action='store_true');args=p.parse_args()
    version=tomllib.loads((ROOT/'python/pyproject.toml').read_text())['project']['version']
    verify(args.wheels,version,args.complete)
