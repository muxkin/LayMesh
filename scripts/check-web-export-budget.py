#!/usr/bin/env python3
"""Guard the playback exporter increment, including complete applicable licenses.

Conservative budget: all export modules and ZIP entry headers, the complete player
shell and stylesheet (most existed before static export), and incremental notices.
Prepared content/resources and the existing renderer are not an exporter increment.
"""
import argparse,io,json,zipfile
from pathlib import Path
from web_runtime_licenses import select
ROOT=Path(__file__).resolve().parents[1]
def packed(entries):
 out=io.BytesIO()
 with zipfile.ZipFile(out,'w',zipfile.ZIP_DEFLATED,compresslevel=9) as z:
  for name,data in sorted(entries.items()):z.writestr(name,data)
 return len(out.getvalue())
def measure(path):
 with zipfile.ZipFile(path) as z:
  modes=json.loads(z.read('runtime-files.json'))
  playback=modes['playback']
  assert not any('exporter-full/' in name or name.endswith('web-full-export-worker.mjs') for name in playback)
  assert all(name in modes['editable'] for name in playback),'Editable must support saving a self-contained playback copy'
  names=[n for n in playback if any(x in n for x in ('/presenter','/page-previews.mjs','/page-thumbnails.mjs','/presentation-entry.mjs','/presentation.css','/exporter/','/vendor/pdf-export/','/web-export-dialog.mjs','/web-static-export-worker.mjs','/web-pdf-export.mjs','/web-jpeg-export.mjs','/web-vector-export.mjs','/web-image-density.mjs','/web-player.mjs','/web-player.css','/web-licenses.mjs','/web-transition-package.mjs','/runtime-files.json'))]
  data={name:z.read(name) for name in names}
  licenses=select(json.loads(z.read('licenses/manifest.json')),False)
  base={**licenses,'packages':[p for p in licenses['packages'] if not p['playbackExportOnly']]}
  used={t['sha256'] for p in base['packages'] for t in p['texts']}
  base['texts']={k:v for k,v in base['texts'].items() if k in used}
  encode=lambda d:json.dumps(d,ensure_ascii=False,separators=(',',':')).encode()
  incremental=packed({'licenses/manifest.json':encode(licenses)})-packed({'licenses/manifest.json':encode(base)})
  code=packed(data);total=code+incremental
  result={'runtime':str(path),'playbackExporterAndShellZipBytes':code,'incrementalLicenseZipBytes':incremental,'conservativeIncrementBytes':total,'budgetBytes':300*1024,'files':names}
  assert total<300*1024,result
  return result
if __name__=='__main__':
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('runtime',nargs='?',type=Path,default=ROOT/'target/web-export/runtime.zip');p.add_argument('--output',type=Path);a=p.parse_args();result=measure(a.runtime);text=json.dumps(result,indent=2)+'\n';print(text)
 if a.output:a.output.write_text(text)
