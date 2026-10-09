#!/usr/bin/env python3
"""Measure a persistent native preview worker using a real, unmodified layout."""
import argparse, hashlib, json, statistics, subprocess, tempfile, time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('source', type=Path)
    p.add_argument('--binary', type=Path, default=ROOT/'target/release/laymesh')
    p.add_argument('--protocol', type=int, choices=[2, 3], default=3)
    p.add_argument('--iterations', type=int, default=7)
    p.add_argument('--output', type=Path, required=True)
    a = p.parse_args()
    if a.iterations < 3: p.error('Use at least three iterations')
    source, binary = a.source.resolve(), a.binary.resolve()
    text = source.read_text()
    rows, known = [], []
    with tempfile.TemporaryDirectory(prefix='laymesh-profile-resources-') as directory:
        engine = subprocess.Popen([str(binary), 'preview', '--stdio'], stdin=subprocess.PIPE,
                                  stdout=subprocess.PIPE, text=True)
        try:
            ready = json.loads(engine.stdout.readline())
            for i in range(a.iterations):
                request = {'id':i, 'generation':i, 'protocol':a.protocol, 'file':str(source),
                           'source':text+f'\n# performance edit {i}', 'resource_dir':directory,
                           'known_resources':known}
                start = time.perf_counter()
                engine.stdin.write(json.dumps(request)+'\n'); engine.stdin.flush()
                line = engine.stdout.readline()
                elapsed = (time.perf_counter()-start)*1000
                result = json.loads(line)
                if 'error' in result: raise RuntimeError(result['error'])
                if a.protocol == 3: assert 'displayList' in result and 'svg' not in result
                known.extend(r['id'] for r in result['resources'])
                status = Path(f'/proc/{engine.pid}/status')
                rss = next((int(v.split()[1]) for v in status.read_text().splitlines()
                            if v.startswith('VmHWM:')), None) if status.exists() else None
                rows.append({'ms':round(elapsed,3), 'bytes':len(line.encode()),
                             'new_resources':len(result['resources']), 'peak_rss_kib':rss})
        finally:
            engine.stdin.close(); engine.wait(timeout=15)
    report = {'source':str(source), 'source_sha256':hashlib.sha256(text.encode()).hexdigest(),
              'binary':str(binary), 'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),
              'protocol':a.protocol, 'capabilities':ready.get('capabilities'), 'rows':rows,
              'hot_median_ms':statistics.median(r['ms'] for r in rows[1:]),
              'definition':'write request through full JSON line received; excludes JSON parsing, edit debounce and browser drawing; fresh process and resource directory'}
    a.output.parent.mkdir(parents=True, exist_ok=True)
    a.output.write_text(json.dumps(report, ensure_ascii=False, indent=2)+'\n')
    print(json.dumps({'cold_ms':rows[0]['ms'], 'hot_median_ms':report['hot_median_ms'],
                      'peak_rss_kib':rows[-1]['peak_rss_kib']}))

if __name__ == '__main__': main()
