"""Locate the native Rust engine; never download code or fonts at runtime."""
from __future__ import annotations
import os
import subprocess
import sys
from pathlib import Path


def bundled_command() -> list[str] | None:
    root = Path(__file__).resolve().parent / "_vendor"
    if not root.is_dir():
        return None
    cli = root / "bin" / ("laymesh.exe" if sys.platform == "win32" else "laymesh")
    if not cli.is_file():
        raise RuntimeError("Incomplete LayMesh installation. Reinstall with python -m pip install --force-reinstall laymesh")
    return [str(cli)]


def development_command() -> list[str] | None:
    root = Path(__file__).resolve().parents[2]
    executable = "laymesh.exe" if sys.platform == "win32" else "laymesh"
    for profile in ("release", "debug"):
        cli = root / "target" / profile / executable
        if cli.is_file():
            return [str(cli)]
    return None


def override_command() -> list[str] | None:
    configured = os.environ.get("LAYMESH_CLI")
    if not configured:
        return None
    cli = Path(configured).expanduser().resolve()
    if not cli.is_file():
        raise RuntimeError(f"LAYMESH_CLI 指向的文件不存在：{cli}")
    if cli.suffix.lower() in (".js", ".mjs", ".cjs", ".ts"):
        raise RuntimeError("LAYMESH_CLI must point to the native Rust executable; JavaScript runtimes are unsupported")
    return [str(cli)]


def main() -> int:
    """Expose the native engine without a system runtime or recursive PATH lookup."""
    try:
        command = override_command() or bundled_command() or development_command()
        if command is None:
            raise RuntimeError("No native engine. Install a supported LayMesh wheel, or run cargo build --release -p laymesh-cli.")
        return subprocess.call([*command, *sys.argv[1:]])
    except (OSError, RuntimeError) as error:
        print(f"LayMesh: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
