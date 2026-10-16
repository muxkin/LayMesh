#!/usr/bin/env python3
"""Shared LCSS/document edits in actual VS Code, including whole-deck readiness."""
import argparse,json,os,subprocess,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser(description=__doc__)
for key in ['fixture','output','binary','extension-dir']:p.add_argument('--'+key,required=True,type=Path)
a=p.parse_args();a.output.mkdir(parents=True,exist_ok=True);summaries=[]
for kind in os.environ.get('LAYMESH_DOCUMENT_CASES','lcss-heading,lcss-curves,laydoc-name,laydoc-reorder,laydoc-add-remove').split(','):
 output=a.output/(kind+'.json')
 samples=100 if kind=='laydoc-add-remove' else 50
 subprocess.run([sys.executable,str(ROOT/'scripts/test-vscode-preview-performance.py'),'--output',str(output),'--source',str(a.fixture/'storyboard.laydoc'),'--edits',str(samples+1),'--binary',str(a.binary),'--extension-dir',str(a.extension_dir),'--test-script',str(ROOT/'scripts/vscode-document-performance.cjs'),'--timeout','1800'],env={**os.environ,'LAYMESH_DOCUMENT_CASE':kind},check=True,stdout=subprocess.DEVNULL)
 result=json.loads(output.read_text());assert result['summary']['samples']==samples;summaries.append(result['summary']);(a.output/'summary.json').write_text(json.dumps(summaries,indent=2)+'\n');print(json.dumps(result['summary']),flush=True)
