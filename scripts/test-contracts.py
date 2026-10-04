#!/usr/bin/env python3
"""Run native contracts and require every legacy assertion's mapped check to pass."""
import argparse
import importlib.util
import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--offline', action='store_true')
    parser.add_argument('--log', type=Path, default=ROOT / 'release/verification/cargo-tests.log')
    args = parser.parse_args()
    # Failed runs must not leave a previous success report looking current.
    args.log.with_name('assertion-coverage.json').unlink(missing_ok=True)
    subprocess.run([sys.executable, '-m', 'unittest', 'discover', '-s', 'migration/tests', '-v'], cwd=ROOT, check=True)
    args.log.parent.mkdir(parents=True, exist_ok=True)
    command = ['cargo', 'test', '--color', 'never', '--locked', '--workspace', '--message-format=json-render-diagnostics']
    if args.offline:
        command.append('--offline')
    spec = importlib.util.spec_from_file_location('assertion_gate', ROOT / 'migration/check-assertions.py')
    gate = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(gate)
    with args.log.open('w') as log:
        log.write('LAYMESH_CONTRACT_SOURCES ' + json.dumps(gate.source_manifest(ROOT), sort_keys=True) + '\n')
        log.flush()
        process = subprocess.Popen(command, cwd=ROOT, stdout=subprocess.PIPE,
                                   stderr=subprocess.STDOUT, text=True)
        for line in process.stdout:
            sys.stdout.write(line)
            log.write(line)
        status = process.wait()
    if status:
        raise SystemExit(status)
    subprocess.run(['cargo', 'build', '--locked', '-p', 'laymesh-cli'] + (['--offline'] if args.offline else []), cwd=ROOT, check=True)
    python_log = args.log.with_name('python-tests.log')
    env = os.environ.copy()
    env['LAYMESH_CLI'] = str(ROOT / 'target/debug' / ('laymesh.exe' if os.name == 'nt' else 'laymesh'))
    env['PYTHONPATH'] = str(ROOT / 'python')
    with python_log.open('w') as log:
        log.write('LAYMESH_CONTRACT_SOURCES ' + json.dumps(gate.source_manifest(ROOT), sort_keys=True) + '\n')
        log.flush()
        process = subprocess.Popen([sys.executable, 'scripts/run-python-contracts.py'],
                                   cwd=ROOT, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        for line in process.stdout:
            sys.stdout.write(line)
            log.write(line)
        status = process.wait()
    if status:
        raise SystemExit(status)
    subprocess.run([sys.executable, 'migration/check-assertions.py', '--python-test-log', str(python_log), '--test-log', str(args.log),
                    '--output', str(args.log.with_name('assertion-coverage.json'))], cwd=ROOT, check=True)

if __name__ == '__main__':
    main()
