#!/usr/bin/env python3
"""Execute Python contracts and record the loaded file for each unittest identity."""
import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def cases(suite):
    for item in suite:
        if isinstance(item, unittest.TestSuite):
            yield from cases(item)
        else:
            yield item


def main():
    suite = unittest.defaultTestLoader.discover(str(ROOT / 'python/tests'))
    sources = {case.id(): str(Path(sys.modules[type(case).__module__].__file__).resolve()) for case in cases(suite)}
    print('LAYMESH_PYTHON_TEST_SOURCES ' + json.dumps(sources, sort_keys=True), flush=True)
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    if result.skipped:
        print("Contract run requires every Python test, including Notebook dependencies; skipped tests are not acceptance.", file=sys.stderr)
    raise SystemExit(0 if result.wasSuccessful() and not result.skipped else 1)


if __name__ == '__main__':
    main()
