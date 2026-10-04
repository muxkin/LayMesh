#!/usr/bin/env python3
"""Package the shared Rust/WASM API and required notices as a local zip."""
import argparse,zipfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--output',type=Path,default=ROOT/'release/dist/laymesh-wasm.zip');a=p.parse_args();source=ROOT/'site/dist/site/live/wasm';assert (source/'laymesh_wasm_bg.wasm').is_file(),'Build the site with Rust/WASM first';a.output.parent.mkdir(parents=True,exist_ok=True)
with zipfile.ZipFile(a.output,'w',zipfile.ZIP_DEFLATED) as z:
 for file in sorted(source.iterdir()):
  if file.is_file():z.write(file,file.name)
 for file in sorted((ROOT/'release/licenses').iterdir()):
  if file.is_file():z.write(file,'licenses/'+file.name)
 z.write(ROOT/'LICENSE','LICENSE');z.write(ROOT/'release/THIRD_PARTY_NOTICES.md','THIRD_PARTY_NOTICES.md');z.write(ROOT/'docs/topics/node-reference.en.md','README.md')
print(f'Packaged {a.output.name}: {a.output.stat().st_size/1024**2:.2f} MiB')
