#!/usr/bin/env python3
"""Build a platform wheel containing exactly one native executable; never upload."""
from __future__ import annotations
import argparse, hashlib, json, os, platform, re, shutil, subprocess, sys, tempfile, tomllib, zipfile, struct
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
CONFIG = json.loads((ROOT / "release/runtime.json").read_text())
def target():
    system={"Linux":"linux","Darwin":"darwin","Windows":"win"}.get(platform.system())
    arch={"x86_64":"x64","AMD64":"x64","arm64":"arm64","aarch64":"arm64"}.get(platform.machine())
    name=f"{system}-{arch}"
    if name not in CONFIG["targets"]: raise SystemExit(f"Unsupported wheel build host: {name}")
    if system=="linux" and platform.libc_ver()[0]!="glibc": raise SystemExit("Linux GNU wheels require glibc.")
    return name

def run(args,cwd=ROOT): subprocess.run([str(a) for a in args],cwd=cwd,check=True)
def sha(path): return hashlib.sha256(path.read_bytes()).hexdigest()
def build(output,binary=None,target_name=None):
    run([sys.executable,ROOT/"scripts/check-release.py"])
    name=target_name or target();spec=CONFIG["targets"][name]
    executable="laymesh.exe" if name.startswith("win-") else "laymesh"
    if binary is None:
        run(["cargo","build","--release","--locked","-p","laymesh-cli"])
        binary=ROOT/"target/release"/executable
    binary=binary.resolve()
    if not binary.is_file(): raise SystemExit(f"Missing native engine: {binary}")
    data=binary.read_bytes()
    if name.startswith('linux-'):
        assert data[:4]==b'\x7fELF' and data[4:6]==b'\x02\x01','Expected a 64-bit little-endian ELF binary'
        assert struct.unpack_from('<H',data,18)[0]==(62 if name.endswith('x64') else 183),'Native architecture does not match requested wheel'
    elif name.startswith('darwin-'):
        assert data[:4]==b'\xcf\xfa\xed\xfe','Expected a 64-bit Mach-O binary'
        assert struct.unpack_from('<I',data,4)[0]==(0x01000007 if name.endswith('x64') else 0x0100000c),'Native architecture does not match requested wheel'
    else:
        assert data[:2]==b'MZ','Expected a Windows executable'
        pe=struct.unpack_from('<I',data,60)[0];assert data[pe:pe+4]==b'PE\0\0' and struct.unpack_from('<H',data,pe+4)[0]==0x8664,'Expected a Windows x64 executable'
    wheel_tag=spec['wheel_tag']
    glibc_required=None
    if name.startswith('linux-'):
        info=subprocess.check_output(['readelf','--version-info',str(binary)],text=True)
        versions=[tuple(map(int,m)) for m in re.findall(r'Name: GLIBC_(\d+)\.(\d+)',info)]
        if not versions:raise SystemExit('Could not inspect GNU libc requirements; refusing an unverified manylinux tag')
        required=max(versions+[(2,28)]);glibc_required='.'.join(map(str,required))
        wheel_tag=f'manylinux_{required[0]}_{required[1]}_'+('x86_64' if name=='linux-x64' else 'aarch64')
    version=tomllib.loads((ROOT/"python/pyproject.toml").read_text())["project"]["version"]
    rust_version=tomllib.loads((ROOT/"Cargo.toml").read_text())["workspace"]["package"]["version"]
    output.mkdir(parents=True,exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="laymesh-wheel-") as tmp:
        stage=Path(tmp)
        for file in ["pyproject.toml","setup.py","README.md","LICENSE"]: shutil.copy2(ROOT/"python"/file,stage/file)
        shutil.copytree(ROOT/"python/laymesh",stage/"laymesh",ignore=shutil.ignore_patterns("__pycache__","_vendor","*.pyc"))
        vendor=stage/"laymesh/_vendor";(vendor/"bin").mkdir(parents=True)
        shutil.copy2(binary,vendor/"bin"/executable)
        if not name.startswith("win-"): (vendor/"bin"/executable).chmod(0o755)
        # Fonts are compiled only into the math renderer. No text font files or runtime tree.
        licenses=vendor/"licenses";licenses.mkdir();shutil.copy2(ROOT/"LICENSE",licenses/"LayMesh-LICENSE")
        if not (ROOT/"release/licenses/manifest.json").is_file():
            run([sys.executable,ROOT/"scripts/collect-licenses.py"])
        shutil.copytree(ROOT/"release/licenses",licenses,dirs_exist_ok=True)
        shutil.copy2(ROOT/"release/THIRD_PARTY_NOTICES.md",vendor/"THIRD_PARTY_NOTICES.md")
        manifest={"engine":"rust","version":version,"rust_version":rust_version,"target":name,"rust_target":spec["rust_target"],"wheel_tag":wheel_tag,"glibc_required":glibc_required,"binary_sha256":sha(binary),"cargo_lock_sha256":sha(ROOT/"Cargo.lock"),"bundled_text_fonts":False,"runtime_downloads":False}
        (vendor/"manifest.json").write_text(json.dumps(manifest,indent=2)+"\n")
        run([sys.executable,"-m","build","--wheel","--no-isolation","--outdir",output],stage)
    wheel=output/f'laymesh-{version}-py3-none-{wheel_tag}.whl'
    if not wheel.is_file(): raise SystemExit("Missing expected platform wheel")
    with zipfile.ZipFile(wheel) as z: installed=sum(i.file_size for i in z.infolist())
    print(json.dumps({"wheel":wheel.name,"compressed_bytes":wheel.stat().st_size,"installed_bytes":installed,"sha256":sha(wheel)}))
if __name__=="__main__":
    p=argparse.ArgumentParser(description=__doc__);p.add_argument("--output",type=Path,required=True);p.add_argument("--binary",type=Path);p.add_argument("--target",choices=CONFIG["targets"]);p.add_argument("--cache",type=Path,help=argparse.SUPPRESS);a=p.parse_args();build(a.output.resolve(),a.binary,a.target)
