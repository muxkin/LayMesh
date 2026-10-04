#!/usr/bin/env python3
"""Verify workspace refactoring and BMP rendering in the actual VS Code host."""
import argparse,json,os,subprocess,tempfile
from pathlib import Path
from PIL import Image
ROOT=Path(__file__).resolve().parents[1]
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--code',default='/usr/share/code/code')
parser.add_argument('--output',type=Path,default=Path('/tmp/laymesh-update-verification/vscode-bindings.json'))
args=parser.parse_args();args.output=args.output.resolve();args.output.parent.mkdir(parents=True,exist_ok=True)
args.output.unlink(missing_ok=True)
with tempfile.TemporaryDirectory(prefix='laymesh-vscode-bindings-') as temporary:
 base=Path(temporary);project=base/'workspace';project.mkdir();assets=project/'中文 # assets';assets.mkdir()
 Image.new('RGB',(20,10),(30,150,220)).save(assets/'图片.bmp')
 (project/'main.lay').write_text('page=canvas(size=(60mm,30mm))\nrec=rect(size=(10mm,8mm))\nimg1=image(src="./中文 # assets/图片.bmp")\ncopy=rec\npage.add(img1,size=(20mm,20mm))\npage.add(copy,offset=(30mm,0))',encoding='utf-8')
 (project/'lib.lay').write_text('export amount=10',encoding='utf-8')
 (project/'alias.lay').write_text('import {amount as w} from "./lib.lay"\nlabel=f"中文 😀 {w}"',encoding='utf-8')
 (project/'caller.lay').write_text('import {amount} from "./lib.lay"\nresult=amount',encoding='utf-8')
 (project/'target').mkdir();(project/'target/ignored.lay').write_text('import {amount} from "../lib.lay"\nx=amount',encoding='utf-8')
 empty=base/'empty-path';empty.mkdir()
 env={**os.environ,'PATH':str(empty),'LAYMESH_BINDINGS_ROOT':str(project),'LAYMESH_BINDINGS_EVIDENCE':str(args.output)}
 env.pop('ELECTRON_RUN_AS_NODE',None)
 command=[args.code,'--no-sandbox','--disable-gpu','--disable-updates','--disable-workspace-trust','--ozone-platform=headless','--user-data-dir='+str(base/'profile'),'--extensions-dir='+str(base/'extensions'),'--extensionDevelopmentPath='+str(ROOT/'extensions/vscode'),'--extensionTestsPath='+str(ROOT/'scripts/vscode-bindings.cjs'),'--skip-welcome','--skip-release-notes','--new-window',str(project)]
 completed=subprocess.run(command,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True,timeout=120)
 if completed.returncode or not args.output.is_file():raise RuntimeError('VS Code binding verification failed:\n'+completed.stdout[-6000:])
 result=json.loads(args.output.read_text());assert len(result['tests'])==12
 for exported in result['exports']:
  if exported['extension'] in ('svg','pdf','pam'):continue
  with Image.open(exported['file']) as raster:
   assert raster.size==(144,72),(exported,raster.size)
   if exported['extension'] not in ('jpg','pgm','pbm','webp'):
    assert raster.convert('RGBA').getpixel((70,30))==(18,171,52,255),exported
   if exported['extension'] in ('jpg','webp'):
    assert max(abs(a-b) for a,b in zip(raster.convert('RGB').getpixel((70,30)),(18,171,52)))<6,exported
 result['export_pixels_verified']=True
 # The temporary project is removed after validation; retain the evidence, not stale file links.
 for exported in result['exports']:exported['file']=Path(exported['file']).name
 args.output.write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n')
 print(json.dumps(result,ensure_ascii=False))
