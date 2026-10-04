#!/usr/bin/env python3
"""Stage the host-only VS Code client plus a native Rust language server."""
import argparse,json,re,shutil,subprocess,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def build(binary=None):
 if binary is None:
  subprocess.run(['cargo','build','--release','--locked','-p','laymesh-cli'],cwd=ROOT,check=True)
  binary=ROOT/'target/release'/('laymesh.exe' if sys.platform=='win32' else 'laymesh')
 if not binary.is_file():raise RuntimeError(f'Missing Rust binary: {binary}')
 extension=ROOT/'extensions/vscode';(extension/'dist').mkdir(exist_ok=True);(extension/'bin').mkdir(exist_ok=True)
 # Function spellings come from the same registry as language services.
 grammar=extension/'syntaxes/laymesh.tmLanguage.json';syntax=json.loads(grammar.read_text())
 names=sorted({e['name'].rsplit('.',1)[-1] for e in json.loads((ROOT/'crates/laymesh-core/api.json').read_text())['api']})
 for pattern in syntax['patterns']:
  if pattern.get('name')=='support.function':pattern['match']=r'\b(?:'+'|'.join(re.escape(n) for n in names)+r')(?=\s*\()'
 grammar.write_text(json.dumps(syntax,ensure_ascii=False,indent=2)+'\n')
 shutil.copy2(ROOT/'site/live/color-panel.mjs',extension/'dist/color-panel.mjs');shutil.copy2(ROOT/'site/live/color-math.mjs',extension/'dist/color-math.mjs');shutil.copy2(extension/'src/color-webview.mjs',extension/'dist/color-webview.mjs')
 shutil.copy2(extension/'src/client.cjs',extension/'dist/client.cjs');shutil.copy2(binary,extension/'bin'/binary.name);shutil.copy2(ROOT/'LICENSE',extension/'LICENSE')
 stale=extension/'dist/server.cjs'
 if stale.exists():stale.unlink()
 if (ROOT/'release/licenses').is_dir():shutil.copytree(ROOT/'release/licenses',extension/'licenses',dirs_exist_ok=True)
 if (ROOT/'release/THIRD_PARTY_NOTICES.md').is_file():shutil.copy2(ROOT/'release/THIRD_PARTY_NOTICES.md',extension/'THIRD_PARTY_NOTICES.md')
 print('Built thin VS Code client + native Rust LSP (no npm packages)')
if __name__=='__main__':
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',type=Path);a=p.parse_args();build(a.binary)
