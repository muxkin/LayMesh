#!/usr/bin/env python3
"""Guard the playback exporter increment, including complete applicable licenses.

Conservative budget: all export modules and ZIP entry headers, the complete player
shell and stylesheet (most existed before static export), and incremental notices.
Prepared content/resources and the existing renderer are not an exporter increment.
"""
import argparse,io,json,zipfile,struct
from pathlib import Path
from web_runtime_licenses import select
ROOT=Path(__file__).resolve().parents[1]
def packed(entries,level=9):
 out=io.BytesIO()
 with zipfile.ZipFile(out,'w',zipfile.ZIP_DEFLATED,compresslevel=level) as z:
  for name,data in sorted(entries.items()):z.writestr(name,data)
 return len(out.getvalue())
def measure(path):
 with zipfile.ZipFile(path) as z:
  modes=json.loads(z.read('runtime-files.json' if 'runtime-files.json' in z.namelist() else 'runtime/runtime-files.json'))
  playback=modes['playback']
  assert not any('exporter-full/' in name or name.endswith('web-full-export-worker.mjs') for name in playback)
  assert all(name in modes['editable'] for name in playback),'Editable must support saving a self-contained playback copy'
  names=[n for n in playback if any(x in n for x in ('/presenter','/page-previews.mjs','/page-thumbnails.mjs','/presentation-entry.mjs','/presentation.css','/exporter/','/vendor/pdf-export/','/web-export-dialog.mjs','/web-media-export.mjs','/web-static-export-worker.mjs','/web-pdf-export.mjs','/web-pdf-resources.mjs','/web-export-raster.mjs','/web-archive.mjs','/fflate.mjs','/web-jpeg-export.mjs','/web-vector-export.mjs','/web-image-density.mjs','/web-player.mjs','/web-player.css','/web-licenses.mjs','/web-transition-package.mjs','/runtime-files.json'))]
  licenses=select(json.loads(z.read('licenses/manifest.json')),False)
  only={**licenses,'packages':[p for p in licenses['packages'] if p['playbackExportOnly']]}
  used={t['sha256'] for p in only['packages'] for t in p['texts']}
  only['texts']={k:v for k,v in only['texts'].items() if k in used}
  encode=lambda d:json.dumps(d,ensure_ascii=False,separators=(',',':')).encode()
  # Count export-specific notices independently, including their complete texts,
  # even where the original renderer already carried an identical license.
  incremental=max(packed({'licenses/manifest.json':encode(only)},level) for level in [1,6,9])
  # Measure the supplied archive's actual compression, including browser fflate
  # downloads, rather than recompressing every package with Python's zlib.
  code=22
  for name in names:
   info=z.getinfo(name);z.fp.seek(info.header_offset)
   local=struct.unpack('<IHHHHHIIIHH',z.fp.read(30))
   code+=30+local[-2]+local[-1]+info.compress_size+(16 if info.flag_bits&8 else 0)
   code+=46+len(name.encode())+len(info.extra)+len(info.comment)
  # The new compression helper also occurs in the existing inline ZIP loader.
  # Its two extra copies are under 256 bytes; reserve twice that allowance.
  loader=512
  total=code+incremental+loader
  result={'runtime':str(path),'playbackExporterAndShellZipBytes':code,'exportLicenseZipBytes':incremental,'loaderAllowanceBytes':loader,'conservativeIncrementBytes':total,'budgetBytes':300*1024,'files':names}
  assert total<300*1024,result
  return result
if __name__=='__main__':
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('runtime',nargs='?',type=Path,default=ROOT/'target/web-export/runtime.zip');p.add_argument('--output',type=Path);a=p.parse_args();result=measure(a.runtime);text=json.dumps(result,indent=2)+'\n';print(text)
 if a.output:a.output.write_text(text)
