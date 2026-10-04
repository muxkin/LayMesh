#!/usr/bin/env python3
"""Check release metadata before building platform wheels; never upload."""
from __future__ import annotations

import json
from pathlib import Path
import tomllib

from packaging.version import Version

ROOT = Path(__file__).resolve().parents[1]


def check(root: Path = ROOT) -> str:
    project = tomllib.loads((root / "python/pyproject.toml").read_text())["project"]
    rust = tomllib.loads((root / "Cargo.toml").read_text())["workspace"]["package"]
    runtime = json.loads((root / "release/runtime.json").read_text())
    version = Version(project["version"])
    if str(version) != project["version"] or version != Version(rust["version"]):
        raise ValueError("Python and Rust versions must describe the same canonical release")
    if runtime["rust_version"] != rust["version"] or runtime["engine"] != "rust":
        raise ValueError("release/runtime.json must match the Rust workspace version and engine")
    if version.pre:
        status = {"a": "3 - Alpha", "b": "4 - Beta", "rc": "4 - Beta"}[version.pre[0]]
        if f"Development Status :: {status}" not in project["classifiers"]:
            raise ValueError("Development Status classifier must match the prerelease stage")
    elif "Development Status :: 5 - Production/Stable" not in project["classifiers"]:
        raise ValueError("Stable releases must use the Production/Stable classifier")
    readme = root / "python" / project["readme"]
    if not readme.is_file() or not readme.read_text(encoding="utf-8").strip():
        raise ValueError("The PyPI README must exist and contain content")
    if (root / "python/LICENSE").read_bytes() != (root / "LICENSE").read_bytes():
        raise ValueError("The Python package and repository licenses must match")
    if project["scripts"].get("laymesh") != "laymesh._runtime:main":
        raise ValueError("The Python package must expose the native CLI entry point")
    return project["version"]


if __name__ == "__main__":
    try:
        print(f"Release metadata verified: {check()}")
    except (KeyError, OSError, ValueError) as error:
        raise SystemExit(f"Release metadata error: {error}") from error
