#!/usr/bin/env python3
"""Run 50 measured edits per operation in isolated, actual VS Code windows."""
import argparse, json, re, subprocess, sys
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--fixture', required=True, type=Path)
p.add_argument('--output', required=True, type=Path)
p.add_argument('--extension-dir', required=True, type=Path)
p.add_argument('--binary', required=True, type=Path)
a = p.parse_args(); a.output.mkdir(parents=True, exist_ok=True)
summaries = []
for page in ['02-seed', '06-editable']:
    source = a.fixture / ('benchmark-' + page + '.lay')
    text = source.read_text()
    title = re.search(r'chrome\(page,"[^"\n]*",("[^"\n]*")', text)[1]
    offset = re.search(r'offset=\(([-\d.]+)mm,', text)[1]
    width = re.search(r'rect\(size=\(([-\d.]+)mm,', text)[1]
    changes = {
        'color': ('background="#fbfaf5"', 'background="rgb(250,249,{i})"'),
        'position': (f'offset=({offset}mm,', f'offset=({offset}.{{i:02}}mm,'),
        'text': (title, title[:-1] + ' {i}"'),
        'size': (f'rect(size=({width}mm,', f'rect(size=({int(float(width))-1}.{{i:02}}mm,'),
    }
    for kind, change in changes.items():
        output = a.output / (page + '-' + kind + '.json')
        command = [sys.executable, str(ROOT / 'scripts/test-vscode-preview-performance.py'),
                   '--output', str(output), '--source', str(source), '--edits', '51',
                   '--extension-dir', str(a.extension_dir), '--binary', str(a.binary),
                   '--replace', *change]
        subprocess.run(command, check=True, stdout=subprocess.DEVNULL)
        result = json.loads(output.read_text())
        timing = result['edit_to_visible']
        assert timing['samples'] == 50
        assert all(row['incremental'] for row in result['hot']), 'Warm frames must be drawing patches'
        assert all(row['presentation_confirmed'] for row in result['hot']), 'Measurement requires a visible rendering update'
        if page == '06-editable':
            assert all(row['compile_metrics']['evaluatedBuiltinCalls'].get('path', 0) == 0 for row in result['hot'])
        summary = {'host': 'VS Code', 'page': page, 'kind': kind, **timing,
                   'targetPassed': timing['median_ms'] <= 100 and timing['p95_ms'] <= 200}
        summaries.append(summary)
        (a.output / 'summary.json').write_text(json.dumps(summaries, indent=2) + '\n')
        print(json.dumps(summary), flush=True)
