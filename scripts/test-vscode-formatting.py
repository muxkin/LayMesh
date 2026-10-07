#!/usr/bin/env python3
"""Verify formatting and file icons in isolated VS Code with its native CDP harness."""
import argparse
import base64
import importlib.util
import json
import os
from pathlib import Path
import signal
import socket
import subprocess
import tempfile
import time
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('native_cdp', ROOT / 'scripts/smoke-browser.py')
native = importlib.util.module_from_spec(spec)
spec.loader.exec_module(native)
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--code', default='/usr/share/code/code')
parser.add_argument('--output', type=Path, default=ROOT / 'release/verification/formatting')
args = parser.parse_args()
args.output = args.output.resolve()
args.output.mkdir(parents=True, exist_ok=True)
with tempfile.TemporaryDirectory(prefix='laymesh-format-host-') as temp:
    base = Path(temp)
    project = base / 'workspace'
    project.mkdir()
    (project / 'main.lay').write_text('value=0', encoding='utf-8')
    (project / 'paper.lcss').write_text('canvas{color:#123456;}', encoding='utf-8')
    user = base / 'profile/User'
    user.mkdir(parents=True)
    (user / 'settings.json').write_text(json.dumps({
        'window.autoDetectColorScheme': False, 'workbench.iconTheme': 'vs-seti',
        'editor.formatOnSave': False, 'editor.minimap.enabled': False,
    }))
    gate, evidence = base / 'gate.json', base / 'evidence.json'
    empty = base / 'empty-path'
    empty.mkdir()
    env = {**os.environ, 'PATH': str(empty), 'LAYMESH_FORMAT_ROOT': str(project),
           'LAYMESH_FORMAT_GATE': str(gate), 'LAYMESH_FORMAT_EVIDENCE': str(evidence)}
    env.pop('ELECTRON_RUN_AS_NODE', None)
    with socket.socket() as sock:
        sock.bind(('127.0.0.1', 0))
        port = sock.getsockname()[1]
    command = [args.code, '--no-sandbox', '--disable-gpu', '--disable-updates',
               '--disable-workspace-trust', '--ozone-platform=headless',
               '--remote-debugging-port=' + str(port), '--remote-allow-origins=*',
               '--user-data-dir=' + str(base / 'profile'), '--extensions-dir=' + str(base / 'extensions'),
               '--extensionDevelopmentPath=' + str(ROOT / 'extensions/vscode'),
               '--extensionTestsPath=' + str(ROOT / 'scripts/vscode-formatting.cjs'),
               '--skip-welcome', '--skip-release-notes', '--new-window', str(project)]
    log = (base / 'host.log').open('w')
    host = subprocess.Popen(command, env=env, stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
    icons = {}
    try:
        opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
        deadline = time.monotonic() + 100
        cdp = None
        while host.poll() is None and time.monotonic() < deadline:
            if cdp is None:
                try:
                    targets = json.load(opener.open(f'http://127.0.0.1:{port}/json/list'))
                    target = next(t for t in targets if t['type'] == 'page')
                    cdp = native.CDP(target['webSocketDebuggerUrl'])
                    cdp.call('Runtime.enable')
                    cdp.call('Page.enable')
                    cdp.call('Emulation.setDeviceMetricsOverride', {'width': 1200, 'height': 800, 'deviceScaleFactor': 1, 'mobile': False})
                except (OSError, StopIteration):
                    pass
            elif gate.exists():
                phase = json.loads(gate.read_text())['phase']
                if phase in ('light', 'dark') and phase not in icons:
                    state = cdp.evaluate("""(()=>[...document.querySelectorAll('.laymesh-lang-file-icon,.lcss-lang-file-icon')].map(n=>({
                        classes:n.className, text:n.innerText, image:getComputedStyle(n,'::before').backgroundImage,
                        explorer:n.classList.contains('explorer-item'), tab:!!n.closest('.tab')
                    })))()""")
                    for language in ('laymesh', 'lcss'):
                        matches = [s for s in state if language + '-' + phase + '.svg' in s['image']]
                        if not any(s['explorer'] for s in matches) or not any(s['tab'] for s in matches):
                            break
                    else:
                        icons[phase] = state
                        shot = cdp.call('Page.captureScreenshot', {'format': 'png', 'captureBeyondViewport': False})
                        (args.output / (phase + '.png')).write_bytes(base64.b64decode(shot['data']))
                        gate.write_text(json.dumps({'phase': phase + '-captured'}))
            time.sleep(.1)
        if host.poll() is None:
            raise RuntimeError('VS Code formatting verification timed out; icon state: ' + json.dumps(icons))
        if host.returncode or not evidence.exists():
            raise RuntimeError('VS Code verification failed:\n' + (base / 'host.log').read_text()[-5000:])
        result = json.loads(evidence.read_text())
        assert len(result['tests']) == 9 and set(icons) == {'light', 'dark'}
        # Keep inspectable evidence without retaining temporary profile paths.
        result['icons'] = {theme: [{**s, 'image': s['image'].split('/icons/')[-1]} for s in state] for theme, state in icons.items()}
        (args.output / 'evidence.json').write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n')
        print(json.dumps({'tests': result['tests'], 'output': str(args.output)}, ensure_ascii=False))
    finally:
        if host.poll() is None:
            os.killpg(host.pid, signal.SIGTERM)
        host.wait(timeout=15)
        log.close()
