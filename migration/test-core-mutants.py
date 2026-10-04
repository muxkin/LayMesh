#!/usr/bin/env python3
"""Prove core regressions reject actual implementation mutations in an isolated copy.

Neither the checkout nor its normal Cargo artifacts are mutated. A compile error
does not count as a killed mutant: the named test must run and fail an assertion.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
MUTANTS = [
    ('stale_anchor_snapshot', 'geometry.rs', 'if let Some(reference) = reference {',
     'if let Some(reference) = reference.as_ref().filter(|_| false) {',
     'anchor_identity', 'coincident_boxes_do_not_steal_a_text_anchor_after_inherited_reflow'),
    ('coincident_fuse_identity_accepted', 'geometry.rs',
     '|| reference.as_ref().map(|r| r.instance) != Some(objects[i].borrow().id)',
     '|| false', 'anchor_identity', 'fuse_rejects_a_different_instance_even_when_its_anchor_coincides'),
    ('repeated_names_overwrite_identity', 'engine.rs', 'let name = if *count == 1 {',
     'let name = if true {', 'anchor_identity', 'repeated_named_placements_and_bare_calls_have_stable_unique_scene_ids'),
    ('bare_names_follow_object_allocation', 'engine.rs',
     'let name = json!(format!("@{}", self.unnamed_instances));',
     'let name = json!(format!("@{}", instance.borrow().id));',
     'anchor_identity', 'repeated_named_placements_and_bare_calls_have_stable_unique_scene_ids'),
    ('shape_gets_extra_container_border', 'geometry.rs',
     '"image" | "text" | "formula" | "group" | "plot" | "legend" | "colorbar"',
     '"image" | "text" | "formula" | "group" | "plot" | "legend" | "colorbar" | "rect"',
     'contracts', 'physical_units_and_default_typographic_units'),
    ('fusion_default_black_border', 'geometry.rs',
     '.or_insert_with(|| V::text("none"));', '.or_insert_with(|| V::text("black"));',
     'contracts', 'short_inch_unit_and_fusion_without_border_preserve_semantics'),
    ('inch_abbreviation_rejected', 'parser.rs',
     '"and" | "or" | "else" | "return" | "break" | "continue"',
     '"in" | "and" | "or" | "else" | "return" | "break" | "continue"',
     'contracts', 'short_inch_unit_and_fusion_without_border_preserve_semantics'),
]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, default=ROOT / 'release/verification/core-mutants')
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    report = {'kind': 'production source mutation', 'isolated': True, 'mutants': [],
              'commit': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()}
    with tempfile.TemporaryDirectory(prefix='laymesh-core-mutants-') as directory:
        workspace = Path(directory)
        for file in ('Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml'):
            shutil.copy2(ROOT / file, workspace / file)
        shutil.copytree(ROOT / 'crates', workspace / 'crates')
        for folder in ('tests', 'migration', 'examples'):
            (workspace / folder).symlink_to(ROOT / folder, target_is_directory=True)
        env = dict(os.environ, CARGO_TARGET_DIR=str(ROOT / 'target/core-mutants'))
        def run(test_file, test, label):
            command = ['cargo', 'test', '--offline', '--locked', '-p', 'laymesh-core',
                       '--test', test_file, test, '--', '--exact']
            result = subprocess.run(command, cwd=workspace, env=env, text=True, capture_output=True)
            output = result.stdout + result.stderr
            (args.output / (label + '.log')).write_text(output)
            return result.returncode, output
        checked = set()
        for name, filename, before, after, test_file, test in MUTANTS:
            if (test_file, test) not in checked:
                status, output = run(test_file, test, 'baseline-' + test)
                assert status == 0 and f'test {test} ... ok' in output, output
                checked.add((test_file, test))
            path = workspace / 'crates/laymesh-core/src' / filename
            original = path.read_text()
            assert original.count(before) == 1, (name, original.count(before))
            path.write_text(original.replace(before, after))
            try:
                status, output = run(test_file, test, name)
                killed = status != 0 and f'test {test} ... FAILED' in output
                assert killed and 'error[E' not in output, (name, output)
                report['mutants'].append({'name': name, 'source': filename,
                    'original_sha256': hashlib.sha256(original.encode()).hexdigest(),
                    'test': test_file + '::' + test, 'killed_by_test_failure': True})
                print(f'Killed {name}: {test}', flush=True)
            finally:
                path.write_text(original)
    report['status'] = 'passed'
    (args.output / 'verification.json').write_text(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
