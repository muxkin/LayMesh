#!/usr/bin/env python3
"""Compare complete CLI exports with alternating runs and per-process peak RSS."""
import argparse
import hashlib
import json
import os
import statistics
import subprocess
import tempfile
import time
from pathlib import Path

from site_support import ROOT, dependencies, fingerprint

SOURCES = [
    'examples/typography.lay', 'examples/vector.lay', 'examples/effects/art-text.lay',
    'examples/effects/shadow-glow.lay', 'examples/effects/images-groups.lay',
    'examples/gallery/images/svg.lay', 'examples/plot/scientific.lay',
]

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--baseline', required=True, type=Path)
    parser.add_argument('--binary', type=Path, default=ROOT / 'target/release/laymesh')
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--repetitions', type=int, default=5)
    args = parser.parse_args()
    if not hasattr(os, 'wait4'):
        parser.error('This peak-RSS measurement requires POSIX wait4')
    if args.repetitions < 1:
        parser.error('Repetitions must be positive')
    args.output.parent.mkdir(parents=True, exist_ok=True)
    binaries = {'baseline': args.baseline.resolve(), 'current': args.binary.resolve()}
    result = {'method': 'One unrecorded warmup, then alternating fresh CLI processes; wall time includes compilation, rendering and encoding. RSS is wait4 ru_maxrss in KiB on Linux.',
              'dpi': 144, 'repetitions': args.repetitions,
              'binary_sha256': {k: hashlib.sha256(p.read_bytes()).hexdigest() for k, p in binaries.items()},
              'input_fingerprints': {s: fingerprint(dependencies(s)) for s in SOURCES}, 'cases': []}
    with tempfile.TemporaryDirectory(prefix='laymesh-renderer-benchmark-') as directory:
        for source in SOURCES:
            for fmt in ['png', 'pdf', 'pptx', 'svg']:
                values = {k: [] for k in binaries}
                dest = Path(directory) / f'figure.{fmt}'
                for iteration in range(args.repetitions + 1):
                    order = ['baseline', 'current'] if iteration % 2 == 0 else ['current', 'baseline']
                    for backend in order:
                        command = [str(binaries[backend]), 'render', source, '-o', str(dest), '--warnings', 'hide']
                        if fmt != 'svg': command += ['--dpi', '144']
                        start = time.perf_counter()
                        with tempfile.TemporaryFile() as log:
                            process = subprocess.Popen(command, cwd=ROOT, stdout=log, stderr=log)
                            _, status, usage = os.wait4(process.pid, 0)
                            process.returncode = os.waitstatus_to_exitcode(status)
                            elapsed = (time.perf_counter() - start) * 1000
                            if process.returncode:
                                log.seek(0)
                                raise RuntimeError(log.read().decode(errors='replace'))
                        if iteration:
                            values[backend].append({'elapsed_ms': elapsed, 'peak_rss_kib': usage.ru_maxrss, 'bytes': dest.stat().st_size})
                case = {'source': source, 'format': fmt}
                for backend, runs in values.items():
                    case[backend] = {'median_ms': statistics.median(v['elapsed_ms'] for v in runs),
                                     'peak_rss_kib': max(v['peak_rss_kib'] for v in runs),
                                     'bytes': runs[-1]['bytes'], 'runs': runs}
                result['cases'].append(case)
                args.output.write_text(json.dumps(result, indent=2) + '\n')
                print(source, fmt, round(case['baseline']['median_ms'], 1), round(case['current']['median_ms'], 1), flush=True)

if __name__ == '__main__':
    main()
