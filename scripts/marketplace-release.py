#!/usr/bin/env python3
"""Validate original release artifacts and verify Marketplace publication.

The GitHub workflow owns publishing credentials; this script never reads them.
"""
from __future__ import annotations

import argparse
import base64
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import time
import tomllib
from urllib.request import Request, urlopen
from xml.etree import ElementTree
import zipfile

PUBLISHER = "Hyacine"
EXTENSION = "laymesh-language"
TARGETS = {
    "linux-x64": "linux-x64", "linux-arm64": "linux-arm64",
    "darwin-x64": "darwin-x64", "darwin-arm64": "darwin-arm64",
    "win-x64": "win32-x64",
}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def validate_source(run: dict, jobs: list, artifacts: list, repository: str) -> str:
    require((run.get("head_repository") or {}).get("full_name") == repository,
            "Release must originate from this repository")
    require(run.get("path", "").split("@")[0] == ".github/workflows/publish-pypi.yml",
            "Release must use publish-pypi.yml")
    require(run.get("event") == "workflow_dispatch" and run.get("head_branch") == "main",
            "Only a manually dispatched release on main may publish extensions")
    require(run.get("status") == "completed" and run.get("conclusion") == "success",
            "Release workflow must have completed successfully")
    require(any(job.get("name") == "publish" and job.get("conclusion") == "success"
                for job in jobs), "The PyPI publish job must have succeeded, not been skipped")
    sha = run.get("head_sha", "")
    require(re.fullmatch(r"[0-9a-f]{40}", sha) is not None, "Invalid release commit")
    needed = {"reviewed-python-release", *(f"editor-{target}" for target in TARGETS)}
    available = {artifact["name"] for artifact in artifacts if not artifact.get("expired")}
    require(needed <= available, "All five original VSIX artifacts and reviewed wheels are required")
    return sha


def github_json(path: str) -> dict:
    return json.loads(subprocess.check_output(["gh", "api", path], text=True))


def release_version(files: dict[str, str]) -> str:
    package = json.loads(files["extensions/vscode/package.json"])
    runtime = json.loads(files["release/runtime.json"])
    python = tomllib.loads(files["python/pyproject.toml"])["project"]["version"]
    rust = tomllib.loads(files["Cargo.toml"])["workspace"]["package"]["version"]
    version = package["version"]
    require(version == python == rust == runtime["rust_version"], "Release versions must match")
    require(re.fullmatch(r"(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)", version) is not None,
            "This workflow publishes stable numeric versions")
    require(package["publisher"] == PUBLISHER and package["name"] == EXTENSION,
            "Unexpected Marketplace publisher or extension")
    require(set(runtime["targets"]) == set(TARGETS), "Unexpected release platform set")
    return version


VERSION_FILES = ["extensions/vscode/package.json", "release/runtime.json", "python/pyproject.toml", "Cargo.toml"]


def source(args) -> None:
    require(re.fullmatch(r"[1-9]\d*", args.run_id) is not None, "run_id must be a positive integer")
    repository = os.environ["GITHUB_REPOSITORY"]
    require(re.fullmatch(r"[\w.-]+/[\w.-]+", repository) is not None, "Invalid repository")
    prefix = f"repos/{repository}/actions/runs/{args.run_id}"
    run = github_json(prefix)
    sha = validate_source(run, github_json(prefix + "/jobs?per_page=100")["jobs"],
                          github_json(prefix + "/artifacts?per_page=100")["artifacts"], repository)
    files = {}
    for name in VERSION_FILES:
        data = github_json(f"repos/{repository}/contents/{name}?ref={sha}")
        files[name] = base64.b64decode(data["content"]).decode("utf-8")
    version = release_version(files)
    require(not args.version or args.version == version, "Requested version differs from the original release")
    if output := os.environ.get("GITHUB_OUTPUT"):
        with Path(output).open("a") as stream:
            stream.write(f"run_id={args.run_id}\nsha={sha}\nversion={version}\n")
    print(json.dumps({"run_id": args.run_id, "sha": sha, "version": version}))


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def verify_checksums(directory: Path, checksums: Path) -> None:
    expected = {}
    for line in checksums.read_text().splitlines():
        digest, name = line.split()
        require(re.fullmatch(r"[0-9a-f]{64}", digest) is not None and Path(name).name == name,
                "Invalid reviewed checksum entry")
        require(name not in expected, "Duplicate reviewed checksum entry")
        expected[name] = digest
    wheels = {wheel.name: wheel for wheel in directory.glob("*.whl")}
    require(len(wheels) == 5 and set(wheels) == set(expected), "Reviewed checksums must cover exactly five wheels")
    require(all(sha256(wheel) == expected[name] for name, wheel in wheels.items()),
            "Reviewed wheel checksum mismatch")


def audit(args) -> None:
    version = release_version({name: (args.source / name).read_text() for name in VERSION_FILES})
    require(version == args.version, "Artifacts must match the original release version")
    verify_checksums(args.wheels, args.checksums)
    subprocess.run([sys.executable, str(args.source / "scripts/check-python-wheels.py"),
                    str(args.wheels), "--all-targets"], check=True)
    native_hashes = {}
    for wheel in args.wheels.glob("*.whl"):
        with zipfile.ZipFile(wheel) as archive:
            name = next(name for name in archive.namelist() if name.endswith("laymesh/_vendor/manifest.json"))
            manifest = json.loads(archive.read(name))
            native_hashes[TARGETS[manifest["target"]]] = manifest["binary_sha256"]
    vsixes = sorted(args.vsix.glob("*.vsix"))
    require(len(vsixes) == 5, "Exactly five VSIX packages are required")
    results = []
    for vsix in vsixes:
        with zipfile.ZipFile(vsix) as archive:
            package = json.loads(archive.read("extension/package.json"))
            identity = ElementTree.fromstring(archive.read("extension.vsixmanifest")).find(".//{*}Identity")
            target = identity.get("TargetPlatform")
            require(target in native_hashes, "Duplicate or unexpected VSIX target")
            require(package["version"] == version and package["publisher"] == PUBLISHER
                    and package["name"] == EXTENSION, "VSIX identity differs from the reviewed release")
            native_name = next(name for name in archive.namelist() if name.startswith("extension/bin/"))
            native = archive.read(native_name)
            binary_hash = hashlib.sha256(native).hexdigest()
            require(binary_hash == native_hashes.pop(target), "VSIX and wheel must contain the same native engine")
            with tempfile.TemporaryDirectory() as temp:
                binary = Path(temp) / Path(native_name).name
                binary.write_bytes(native)
                subprocess.run([sys.executable, str(args.source / "scripts/check-editor.py"), str(vsix),
                                "--binary", str(binary), "--target", target, "--marketplace"], check=True)
        results.append({"target": target, "file": vsix.name, "vsix_sha256": sha256(vsix),
                        "binary_sha256": binary_hash})
    require(not native_hashes, "Missing VSIX platforms")
    args.report.write_text(json.dumps({"version": version, "packages": results}, indent=2) + "\n")
    print(f"Verified five stable VSIX packages and reviewed wheel engines for {version}")


def marketplace_status(data: dict, report: dict) -> set[str]:
    expected = {package["target"]: package["vsix_sha256"] for package in report["packages"]}
    missing = set(expected)
    for block in data["results"]:
        for extension in block["extensions"]:
            require(extension["publisher"]["publisherName"] == PUBLISHER and extension["extensionName"] == EXTENSION,
                    "Unexpected Marketplace query result")
            for version in extension["versions"]:
                target = version.get("targetPlatform")
                if version["version"] != report["version"] or target not in expected:
                    continue
                properties = {item["key"]: item["value"] for item in version.get("properties", [])}
                require(properties.get("Microsoft.VisualStudio.Code.PreRelease", "false").lower() != "true",
                        f"Marketplace {target} must be a stable release")
                digest = properties.get("Microsoft.VisualStudio.Services.VsixSha256")
                if digest:
                    require(digest.lower() == expected[target], f"Marketplace {target} contains a different VSIX; use a new version")
                    missing.discard(target)
    return missing


def verify(args) -> None:
    report = json.loads(args.report.read_text())
    payload = {"filters": [{"criteria": [{"filterType": 7, "value": f"{PUBLISHER}.{EXTENSION}"}],
                            "pageNumber": 1, "pageSize": 1}], "assetTypes": [], "flags": 49}
    request = Request("https://marketplace.visualstudio.com/_apis/public/gallery/extensionquery",
                      data=json.dumps(payload).encode(), headers={"Content-Type": "application/json",
                      "Accept": "application/json;api-version=7.1-preview.1"})
    for attempt in range(args.attempts):
        with urlopen(request, timeout=30) as response:
            missing = marketplace_status(json.load(response), report)
        if not missing:
            print(f"Marketplace {report['version']}: all five stable platform packages and SHA-256 hashes verified")
            return
        print("Waiting for Marketplace validation: " + ", ".join(sorted(missing)), flush=True)
        if attempt + 1 < args.attempts:
            time.sleep(args.delay)
    raise ValueError("Marketplace has not validated every expected platform package")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    p = commands.add_parser("source")
    p.add_argument("--run-id", required=True)
    p.add_argument("--version", default="")
    p.set_defaults(function=source)
    p = commands.add_parser("audit")
    for name in ["source", "wheels", "vsix", "checksums", "report"]:
        p.add_argument("--" + name, required=True, type=Path)
    p.add_argument("--version", required=True)
    p.set_defaults(function=audit)
    p = commands.add_parser("verify")
    p.add_argument("--report", required=True, type=Path)
    p.add_argument("--attempts", default=15, type=int)
    p.add_argument("--delay", default=20, type=int)
    p.set_defaults(function=verify)
    args = parser.parse_args()
    args.function(args)


if __name__ == "__main__":
    main()
