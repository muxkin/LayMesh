#!/usr/bin/env python3
"""Stage the host-only VS Code client plus a native Rust language server."""
import argparse,json,re,shutil,subprocess,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def build(binary=None,extension_dir=None):
 if binary is None:
  subprocess.run(['cargo','build','--release','--locked','-p','laymesh-cli'],cwd=ROOT,check=True)
  binary=ROOT/'target/release'/('laymesh.exe' if sys.platform=='win32' else 'laymesh')
 if not binary.is_file():raise RuntimeError(f'Missing Rust binary: {binary}')
 if binary.name not in ('laymesh','laymesh.exe'):raise RuntimeError('Native binary must be named laymesh or laymesh.exe')
 source=ROOT/'extensions/vscode';extension=extension_dir.resolve() if extension_dir else source
 extension.mkdir(parents=True,exist_ok=True)
 if extension!=source:
  # Keep platform staging separate so a Windows build cannot replace the local host engine.
  for item in source.iterdir():
   if item.is_file() and item.suffix.lower() not in ('.vsix','.pyc'):shutil.copy2(item,extension/item.name)
  shutil.copytree(source/'syntaxes',extension/'syntaxes',dirs_exist_ok=True)
  shutil.copytree(source/'icons',extension/'icons',dirs_exist_ok=True)
 (extension/'dist').mkdir(exist_ok=True);(extension/'bin').mkdir(exist_ok=True)
 # Function spellings come from the same registry as language services.
 grammar=extension/'syntaxes/laymesh.tmLanguage.json';syntax=json.loads(grammar.read_text())
 names=sorted({e['name'].rsplit('.',1)[-1] for e in json.loads((ROOT/'crates/laymesh-core/api.json').read_text())['api']})
 for pattern in syntax['patterns']:
  if pattern.get('name')=='support.function':pattern['match']=r'\b(?:'+'|'.join(re.escape(n) for n in names)+r')(?=\s*\()'
 grammar.write_text(json.dumps(syntax,ensure_ascii=False,indent=2)+'\n')
 shutil.copy2(ROOT/'site/live/color-panel.mjs',extension/'dist/color-panel.mjs');shutil.copy2(ROOT/'site/live/color-math.mjs',extension/'dist/color-math.mjs');shutil.copy2(source/'src/color-webview.mjs',extension/'dist/color-webview.mjs')
 for name in ['preview-host.cjs','preview-math.mjs','preview-webview.mjs','preview.css']:shutil.copy2(source/'src'/name,extension/'dist'/name)
 # A platform package contains exactly one engine, even when reusing a staging directory.
 stale=extension/'bin'/('laymesh' if binary.name=='laymesh.exe' else 'laymesh.exe')
 if stale.exists():stale.unlink()
 shutil.copy2(source/'src/client.cjs',extension/'dist/client.cjs')
 if binary.resolve()!=(extension/'bin'/binary.name).resolve():shutil.copy2(binary,extension/'bin'/binary.name)
 shutil.copy2(ROOT/'LICENSE',extension/'LICENSE')
 stale=extension/'dist/server.cjs'
 if stale.exists():stale.unlink()
 if (ROOT/'release/licenses').is_dir():shutil.copytree(ROOT/'release/licenses',extension/'licenses',dirs_exist_ok=True)
 if (ROOT/'release/THIRD_PARTY_NOTICES.md').is_file():shutil.copy2(ROOT/'release/THIRD_PARTY_NOTICES.md',extension/'THIRD_PARTY_NOTICES.md')
 print(f'Built thin VS Code client + native Rust services at {extension} (no npm packages)')
if __name__=='__main__':
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',type=Path);p.add_argument('--extension-dir',type=Path,help='Stage a platform package separately from the local extension');a=p.parse_args();build(a.binary,a.extension_dir)
