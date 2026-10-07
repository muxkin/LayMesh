#!/usr/bin/env python3
"""Require complete assertion mappings and passing tests with matching source identities.

Execution mode consumes Cargo JSON artifacts plus libtest output and verbose
unittest output from scripts/test-contracts.py. A test name alone is not proof.
"""
import argparse
import ast
import hashlib
import json
import re
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def source_manifest(root):
    paths = set()
    for directory, patterns in [('crates', ('*.rs', '*.json', 'Cargo.toml')), ('python', ('*.py',)), ('migration/assertion-maps', ('*.json',)), ('migration/tests', ('*.py',)), ('examples', ('*.lay', '*.lcss', '*.csv', '*.json'))]:
        for pattern in patterns:
            paths.update((root / directory).rglob(pattern))
    for name in ('Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml', 'migration/legacy-assertions.json', 'migration/check-assertions.py', 'scripts/test-contracts.py', 'scripts/run-python-contracts.py'):
        if (root / name).is_file():
            paths.add(root / name)
    for directory in ('tests', 'migration/corpus', 'examples/assets'):
        paths.update(path for path in (root / directory).rglob('*') if path.is_file() and '__pycache__' not in path.parts)
    paths = {path for path in paths if not path.is_relative_to(root / "examples/output")}
    for source in list(paths):
        if source.suffix == '.rs':
            for included in re.findall(r'include_(?:str|bytes)!\(\s*"([^"]+)"\s*\)', source.read_text()):
                path = (source.parent / included).resolve()
                if path.is_file() and path.is_relative_to(root.resolve()):
                    paths.add(path)
    return {str(path.relative_to(root)): hashlib.sha256(path.read_bytes()).hexdigest() for path in sorted(paths)}


def rust_execution(log, root, errors):
    artifacts, passed, finished = {}, set(), set()
    active = None
    outcomes = {}
    build_finished = False
    recorded_sources = []
    for line in log.splitlines():
        if line.startswith('LAYMESH_CONTRACT_SOURCES '):
            recorded_sources.append(json.loads(line.removeprefix('LAYMESH_CONTRACT_SOURCES ')))
        if line.startswith('{'):
            try:
                message = json.loads(line)
            except ValueError:
                continue
            if message.get('reason') == 'compiler-artifact' and message.get('executable') and message.get('profile', {}).get('test'):
                artifacts[Path(message['executable']).resolve()] = Path(message['target']['src_path']).resolve()
            if message.get('reason') == 'build-finished':
                build_finished = message.get('success') is True
        running = re.search(r'^\s*Running .+ \((.+)\)$', line)
        if running:
            executable = Path(running[1])
            executable = (root / executable).resolve() if not executable.is_absolute() else executable.resolve()
            if active is not None:
                errors.append('Rust test binary has no terminal result before the next binary')
            active = executable
            outcomes = {}
            if executable not in artifacts:
                errors.append(f'Rust test executable has no Cargo source identity: {executable}')
        if re.match(r'^\s*Doc-tests ', line):
            active = None
        match = re.match(r'^test (\S+)(?: - should panic)? \.\.\. (ok|FAILED|ignored(?:,.*)?)$', line)
        if match and active in artifacts:
            if match[1] in outcomes:
                errors.append('Rust test log repeats a test identity in one binary')
            outcomes[match[1]] = match[2].split(',')[0]
            if match[2] == 'ok':
                passed.add((artifacts[active], match[1]))
        summary = re.match(r'^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;', line)
        if summary:
            if summary[1] != 'ok' or int(summary[3]):
                errors.append('Rust test log contains a failed suite')
            if active:
                counts = Counter(outcomes.values())
                if (counts['ok'], counts['FAILED'], counts['ignored']) != tuple(int(summary[i]) for i in (2,3,4)):
                    errors.append('Rust test summary disagrees with the recorded test outcomes')
                finished.add(active)
                active = None
        if re.search(r'\.\.\. FAILED$|^error:', line):
            errors.append('Rust test log contains a failure')
    if recorded_sources != [source_manifest(root)]:
        errors.append('Rust/Python source or assertion mappings changed since this execution; rerun scripts/test-contracts.py')
    if not artifacts or not build_finished or active is not None or set(artifacts) != finished:
        errors.append('Rust test log is incomplete: require Cargo artifacts, successful build, and a terminal result for every test binary')
    return passed


def python_execution(log, errors, root=None):
    snapshots = [json.loads(line.removeprefix('LAYMESH_CONTRACT_SOURCES ')) for line in log.splitlines() if line.startswith('LAYMESH_CONTRACT_SOURCES ')]
    if root is not None and (len(snapshots) != 1 or snapshots[0] != source_manifest(root)):
        errors.append('Python source or assertion mappings changed since execution; rerun scripts/test-contracts.py')
    # unittest warnings can appear between the identity and its terminal result.
    # Preserve the pending full identity instead of accepting a bare method name.
    identity_maps = [json.loads(line.removeprefix('LAYMESH_PYTHON_TEST_SOURCES ')) for line in log.splitlines() if line.startswith('LAYMESH_PYTHON_TEST_SOURCES ')]
    if len(identity_maps) != 1 or not isinstance(identity_maps[0], dict):
        errors.append('Python execution lacks one loaded-module source identity map')
        sources = {}
    else:
        sources = identity_maps[0]
    outcomes, pending = {}, None
    for line in log.splitlines():
        start = re.match(r'^test_\w+ \(([^)]+)\)', line)
        if start:
            if pending is not None:
                errors.append('Python test identity has no terminal result')
            pending = start[1]
        result = re.search(r'(?:^| \.\.\. )(ok|FAIL|ERROR|skipped(?: .*)?)$', line)
        if result and pending is not None:
            if pending in outcomes:
                errors.append('Python test log repeats a test identity')
            outcomes[pending] = result[1].split(' ')[0]
            pending = None
    summary = re.search(r'^Ran (\d+) tests? in .+\n\s*\nOK(?: \([^\n]*\))?\s*$', log, re.M)
    if set(outcomes) != set(sources):
        errors.append('Python test outcomes disagree with discovered source identities')
    passed = {(Path(sources[name]).resolve(), name) for name,status in outcomes.items() if status == 'ok' and isinstance(sources.get(name), str)}
    if not passed or pending is not None or not summary or len(outcomes) != int(summary[1]) or re.search(r'^FAILED\b|^ERROR:|^FAIL:', log, re.M):
        errors.append('Python test log is incomplete or failed: require all verbose test identities and terminal OK')
    return passed


def rust_identity(target, source, name):
    # Integration tests identify their own source. Unit tests identify lib.rs;
    # module path + test name must match the mapped implementation module.
    if target == source:
        return name
    if target.name in ('lib.rs', 'main.rs'):
        try:
            relative = source.relative_to(target.parent).with_suffix('')
        except ValueError:
            return None
        parts = list(relative.parts)
        if parts[-1] == 'mod':
            parts.pop()
        return '::'.join(parts) + '::' + name if parts else name
    return None


def check(root, rust_log=None, python_log=None):
    errors, covered, targets = [], set(), set()
    inventory = json.loads((root / 'migration/legacy-assertions.json').read_text())
    expected = {a['id']: a for a in inventory['assertions']}
    if len(expected) != len(inventory['assertions']) or len(expected) != inventory['assertion_call_sites']:
        errors.append('Inventory assertion identities/count are inconsistent')
    rust_passed = rust_execution(rust_log, root, errors) if rust_log is not None else set()
    python_passed = python_execution(python_log, errors, root) if python_log is not None else set()
    for file in sorted((root / 'migration/assertion-maps').glob('*.json')):
        data = json.loads(file.read_text())
        if not isinstance(data.get('mappings'), list):
            errors.append(f'{file.name}: mappings must be an array')
            continue
        if data.get('unmapped'):
            errors.append(f'{file.name}: unresolved assertions remain')
        for item in data['mappings']:
            if not isinstance(item, dict) or not isinstance(item.get('ids'), list) or not item['ids'] or not all(isinstance(i, str) for i in item['ids']) or not isinstance(item.get('checks'), list) or not item['checks'] or not isinstance(item.get('note'), str) or not item['note'].strip():
                errors.append(f'{file.name}: mapping needs nonempty ids, executable checks and semantic explanation')
                continue
            valid = True
            for target in item['checks']:
                if not isinstance(target, dict) or not isinstance(target.get('file'), str) or not isinstance(target.get('test'), str) or not target['test']:
                    errors.append(f'{file.name}: check needs file and test fields'); valid = False; continue
                path, name = (root / target['file']).resolve(), target['test']
                if not path.is_relative_to(root.resolve()) or not path.is_file() or path.suffix not in ('.rs', '.py'):
                    errors.append(f'Invalid test source {target["file"]}'); valid = False; continue
                source = path.read_text()
                symbol = name.rsplit('::', 1)[-1]
                targets.add((target['file'], name))
                if path.suffix == '.rs':
                    if not re.search(r'#\s*\[\s*test\s*\]\s*(?:#\[[^]]+\]\s*)*(?:pub\s+)?(?:async\s+)?fn\s+' + re.escape(symbol) + r'\s*\(', source):
                        errors.append(f'Missing Rust test declaration {target["file"]}::{name}'); valid = False
                    if rust_log is not None:
                        matched = False
                        for run_source, run_name in rust_passed:
                            expected_name = rust_identity(run_source, path, name)
                            if expected_name and (run_name == expected_name or run_name.endswith('::' + name) and (run_source == path or run_name.startswith(expected_name.rsplit('::', 1)[0] + '::'))):
                                matched = True
                        if not matched:
                            errors.append(f'No passing Rust execution for source/test {target["file"]}::{name}'); valid = False
                else:
                    declarations = []
                    tree = ast.parse(source)
                    for node in tree.body:
                        if isinstance(node, ast.ClassDef):
                            declarations.extend(path.stem + '.' + node.name + '.' + method.name for method in node.body if isinstance(method, (ast.FunctionDef, ast.AsyncFunctionDef)) and method.name == name)
                    if not declarations:
                        errors.append(f'Missing Python unittest method {target["file"]}::{name}'); valid = False
                    if rust_log is not None or python_log is not None:
                        if not any(run_source == path and (run == declared or run.endswith('.' + declared)) for run_source, run in python_passed for declared in declarations):
                            errors.append(f'No passing Python execution for source/class/test {target["file"]}::{name}'); valid = False
            for assertion in item['ids']:
                if assertion not in expected:
                    errors.append(f'Unknown assertion {assertion}')
                elif valid:
                    covered.add(assertion)
    missing = sorted(set(expected) - covered)
    return {'baseline_commit': inventory['baseline_commit'], 'baseline_tests': inventory['test_count'],
            'assertion_call_sites': len(expected), 'mapped_sites': len(covered), 'unmapped_sites': len(missing),
            'referenced_tests': len(targets), 'execution_verified': rust_log is not None and not errors and not missing,
            'status': 'passed' if not missing and not errors else 'failed',
            'unmapped_by_file': dict(Counter(expected[i]['file'] for i in missing)), 'unmapped': missing,
            'errors': sorted(set(errors)),
            'boundary': 'Source-qualified assertion mapping and passing test execution; baseline parameter loops remain explicit test responsibilities. Compiler replay is separate.'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--test-log', type=Path)
    parser.add_argument('--python-test-log', type=Path)
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    try:
        report = check(ROOT, args.test_log.read_text() if args.test_log else None,
                       args.python_test_log.read_text() if args.python_test_log else None)
    except (OSError, ValueError, KeyError, TypeError, SyntaxError) as error:
        report = {'status': 'failed', 'execution_verified': False, 'errors': [f'Invalid audit inputs: {error}']}
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n')
    print(json.dumps({k:v for k,v in report.items() if k not in ('unmapped','errors')}, ensure_ascii=False, indent=2))
    for error in report['errors']:
        print(error)
    raise SystemExit(0 if report['status'] == 'passed' else 1)

if __name__ == '__main__':
    main()
