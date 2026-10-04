#!/usr/bin/env python3
"""Verify a packaged native preview and LSP on Windows, Linux, or through Wine.

Only Python's standard library is used. --wine tests the actual Windows EXE;
it does not claim verification of the Windows VS Code extension host.
"""
import argparse
import hashlib
import json
import math
import os
import re
import shutil
import subprocess
import tempfile
from pathlib import Path
from urllib.parse import quote
from xml.etree import ElementTree

ROOT = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--binary', type=Path, required=True)
parser.add_argument('--wine', type=Path, help='Optional Wine executable for a Windows EXE on Linux')
parser.add_argument('--output', type=Path, required=True)
args = parser.parse_args()
binary = args.binary.resolve()
command = ([str(args.wine)] if args.wine else []) + [str(binary)]
env = {**os.environ, 'LAYMESH_NO_SYSTEM_FONTS': '1'}
checks = []

def run(arguments, input=None):
    # Wine services may inherit pipe handles after the EXE exits. Files let us
    # wait on the tested process itself instead of waiting for service pipe EOF.
    with tempfile.TemporaryFile() as stdout, tempfile.TemporaryFile() as stderr:
        result = subprocess.run(command + arguments, input=input, stdout=stdout, stderr=stderr, env=env, timeout=45)
        stderr.seek(0)
        assert result.returncode == 0, stderr.read().decode('utf-8', errors='replace')[-1500:]
        stdout.seek(0)
        return stdout.read()

def host_path(path):
    text = str(path.resolve()).replace('\\', '/')
    return 'Z:' + text if args.wine else text

version = run(['--version']).decode().strip()
checks.append('Packaged engine starts and reports its version')
with tempfile.TemporaryDirectory(prefix='laymesh-native-editor-') as temporary:
    root = Path(temporary) / '中文路径 with spaces'
    root.mkdir()
    file = host_path(root / 'main.lay')
    module = host_path(root / 'values.lay')
    (root / 'values.lay').write_text('export width=1', encoding='utf-8')
    requests = []
    for dpi in [72, 96, 144, 300]:
        for unit in ['mm', 'cm', 'in', 'inch', 'pt', 'px']:
            requests.append({'id': len(requests), 'file': file,
                'source': f'page=canvas(size=(100,80),unit="{unit}",layout_dpi={dpi})\nimport {{width}} from "./values.lay"\npage.add(rect(size=(width,2),fill="#ff0000"))',
                'overlays': {module: 'export width=7'}})
    requests.append({'id': len(requests), 'file': file, 'source': 'page=canvas(size=(10cm,80mm),unit="px",layout_dpi=144)\npage.add(rect(size=(1in,72pt)),offset=(2cm,10mm))'})
    missing = host_path(root / 'missing.lay')
    requests.append({'id': len(requests), 'file': file, 'source': 'page=canvas(size=(10,10))\nimport {x} from "./missing.lay"'})
    requests.append({'id': len(requests), 'file': file, 'source': requests[-1]['source'], 'overlays': {missing: 'export x=1'}})
    (root / 'style.lcss').write_text('rect {fill: #ee2211;}', encoding='utf-8')
    (root / 'data.json').write_text('[1,2]', encoding='utf-8')
    (root / 'data.csv').write_text('x,y\n1,2\n', encoding='utf-8')
    shutil.copy2(ROOT / 'examples/assets/photo.png', root / 'photo.png')
    shutil.copy2(ROOT / 'tests/fonts/DejaVuSans.ttf', root / 'font.ttf')
    requests.append({'id': len(requests), 'file': file,
        'source': 'page=canvas(size=(40,30),stylesheet="style.lcss",font_family="font.ttf")\na=array(src="data.json")\nt=table(src="data.csv")\npage.add(image(src="photo.png"),size=(10,10))\npage.add(rect(size=(a[0],2)),offset=(12,0))\npage.add(text("Preview"),offset=(0,15))',
        'overlays': {host_path(root / 'data.json'): '[7,8]', host_path(root / 'data.csv'): 'x,y\n3,4\n', host_path(root / 'style.lcss'): 'rect {fill: #00ff00;}'}})
    for name in ['multi-axes-breaks', 'polar-data', 'radar', 'inset']:
        example = ROOT / f'examples/plot/{name}.lay'
        requests.append({'id': len(requests), 'file': host_path(example), 'source': example.read_text(encoding='utf-8')})
    if args.wine or os.name == 'nt':
        requests.append({'id': len(requests), 'file': file.replace('/', '\\'),
            'source': requests[0]['source'], 'overlays': {module.replace('/', '\\'): 'export width=8'}})
    # CRLF is normal on Windows; a malformed request must not kill the server.
    payload = '\r\n'.join(['{bad JSON}'] + [json.dumps(r, ensure_ascii=False) for r in requests]) + '\r\n'
    results = [json.loads(line) for line in run(['preview', '--stdio'], payload.encode()).decode().splitlines()]
    assert results[0] == {'type': 'ready', 'protocol': 1}
    assert results[1]['error']['code'] == 'E_PREVIEW_PROTOCOL'
    results = results[2:]
    assert [r['id'] for r in results] == list(range(len(requests)))
    for request, result in zip(requests[:24], results[:24]):
        assert 'svg' in result, result.get('error')
        page = result['inspection']['page']
        unit, dpi = page['unit'], page['layout_dpi']
        factor = {'mm': 1, 'cm': 10, 'in': 25.4, 'inch': 25.4, 'pt': 25.4/72, 'px': 25.4/dpi}[unit]
        assert math.isclose(page['width'], 100*factor) and math.isclose(page['height'], 80*factor)
        assert module in result['dependencies'] and '#ff0000' in result['svg']
        # The unsaved module affects geometry; the saved value remains untouched.
        shape = ElementTree.fromstring(result['svg']).find('.//{*}path[@fill="#ff0000"]')
        x = float(re.search(r'\bL([^, ]+),0', shape.get('d')).group(1))
        assert math.isclose(x, 7*factor, rel_tol=1e-5)
    assert (root / 'values.lay').read_text(encoding='utf-8') == 'export width=1'
    checks.append('Six canvas units at four layout DPI values; unsaved modules and Unicode/space paths')
    assert results[24]['inspection']['page']['width'] == 100
    assert results[24]['inspection']['page']['height'] == 80
    checks.append('Mixed explicit lengths preserve millimeter inspection geometry')
    assert results[25]['error']['code'] == 'E_ASSET' and missing in results[25]['dependencies']
    assert 'svg' in results[26]
    checks.append('Structured missing-dependency error and recovery in the same persistent process')
    resources = results[27]
    assert 'svg' in resources, resources.get('error')
    for name in ['style.lcss', 'data.json', 'data.csv', 'photo.png', 'font.ttf']:
        assert host_path(root / name) in resources['dependencies'], name
    assert '#00ff00' in resources['svg']
    assert (root / 'data.json').read_text(encoding='utf-8') == '[1,2]'
    checks.append('Relative stylesheet/data/image/font paths and unsaved resource overlays')
    for name, result in zip(['multi-axes-breaks', 'polar-data', 'radar', 'inset'], results[28:]):
        assert 'svg' in result and result['inspection']['plots'], (name, result.get('error'))
    checks.append('Native SVG and plot inspection for extra/broken axes, polar, radar and inset examples')
    checks.append('CRLF transport, malformed request recovery and ordered request IDs')
    if args.wine or os.name == 'nt':
        assert 'svg' in results[-1] and module in results[-1]['dependencies']
        checks.append('Windows backslash entry and unsaved module paths')

    source = 'page=canvas(size=(10,10),background="#ff0000")\ncan'
    uri = 'file:///' + quote(file, safe='/:') if args.wine or os.name == 'nt' else Path(file).as_uri()
    messages = [
        {'id': 1, 'method': 'initialize', 'params': {'locale': 'en', 'capabilities': {'textDocument': {'hover': {'contentFormat': ['markdown']}}}}},
        {'method': 'initialized', 'params': {}},
        {'method': 'textDocument/didOpen', 'params': {'textDocument': {'uri': uri, 'languageId': 'laymesh', 'version': 1, 'text': source}}},
        {'id': 2, 'method': 'textDocument/completion', 'params': {'textDocument': {'uri': uri}, 'position': {'line': 1, 'character': 3}}},
        {'id': 3, 'method': 'textDocument/hover', 'params': {'textDocument': {'uri': uri}, 'position': {'line': 0, 'character': 7}}},
        {'method': 'workspace/didChangeConfiguration', 'params': {'settings': {'laymesh': {'language': 'zh-CN'}}}},
        {'id': 4, 'method': 'textDocument/hover', 'params': {'textDocument': {'uri': uri}, 'position': {'line': 0, 'character': 7}}},
        {'id': 5, 'method': 'shutdown', 'params': None},
        {'method': 'exit', 'params': None},
    ]
    payload = b''
    for message in messages:
        body = json.dumps({'jsonrpc': '2.0', **message}, ensure_ascii=False).encode()
        payload += f'Content-Length: {len(body)}\r\n\r\n'.encode() + body
    output = run(['lsp', '--stdio'], payload)
    responses = {}
    while output:
        headers, output = output.split(b'\r\n\r\n', 1)
        length = int(headers.decode().split(':', 1)[1].strip())
        message = json.loads(output[:length])
        output = output[length:]
        if 'id' in message:
            responses[message['id']] = message
    assert responses[1]['result']['capabilities']['positionEncoding'] == 'utf-16'
    items = responses[2]['result']
    if isinstance(items, dict):
        items = items['items']
    assert any(item['label'] == 'canvas' for item in items)
    english = responses[3]['result']['contents']['value']
    chinese = responses[4]['result']['contents']['value']
    assert english != chinese and '/en/docs/' in english and '.zh-CN.html' in chinese
    assert responses[5]['result'] is None
    checks.append('File-path LSP initialization, completion, English/Chinese hover and clean shutdown')

evidence = {'status': 'passed', 'runtime': 'Windows EXE through Wine' if args.wine else os.name,
    'version': version, 'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
    'preview_requests': len(requests), 'checks': checks, 'windows_vscode_host_tested': False}
args.output.parent.mkdir(parents=True, exist_ok=True)
args.output.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
print(json.dumps(evidence, ensure_ascii=False))
