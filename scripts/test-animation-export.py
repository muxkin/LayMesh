#!/usr/bin/env python3
"""Verify all 97 effects through streamed MP4/GIF, snapshots and cancellation."""
import argparse,hashlib,json,os,re,signal,subprocess,tempfile,time
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',type=Path,default=ROOT/'target/release/laymesh');a=p.parse_args();binary=str(a.binary.resolve())
lib=ROOT/'tests/animation/vendor/animate.css'
assert hashlib.sha256(lib.read_bytes()).hexdigest()=='c1b6f9ed1effff87233740ce612ed3cd3fbd3cb34c0863373d820fde1b2c8d8f'
names=re.findall(r'^@keyframes (\w+)\s*\{',lib.read_text(),re.M);assert len(names)==97
with tempfile.TemporaryDirectory(prefix='laymesh-animation-export-') as temp:
 d=Path(temp);(d/'animate.css').write_bytes(lib.read_bytes());doc=['d=document()']
 for i,name in enumerate(names):
  source=f'p=canvas(size=(40,30),stylesheet="animate.css")\na=p.add(rect(size=(15,10),fill="#3978d4"),offset=(12,10))\ns=p.step(hold=0s)\ns.animate(a,class="animate__animated animate__{name}",duration=100ms)\n'
  file=d/f'{i}.lay';file.write_text(source);doc.append(f'd.add(page(src="{i}.lay"))')
 (d/'all.laydoc').write_text('\n'.join(doc))
 for ext in ['mp4','gif']:
  output=d/f'all.{ext}';result=json.loads(subprocess.check_output([binary,'animate',str(d/'all.laydoc'),'-o',str(output),'--width','160','--fps','20']))
  info=json.loads(subprocess.check_output(['ffprobe','-v','error','-show_entries','stream=width,height,nb_frames,duration','-of','json',str(output)]))['streams'][0]
  assert result['layoutCompilations']==97 and result['pageCount']==97
  assert info['width']==160 and info['height']==120 and int(info['nb_frames'])==result['frames']
  assert abs(float(info['duration'])-9.7)<=0.051,(ext,info)
  subprocess.run(['ffmpeg','-v','error','-i',str(output),'-f','null','-'],check=True)
  print(ext,result,flush=True)
 subprocess.run([binary,'render',str(d/'0.lay'),'-o',str(d/'snapshot.png'),'--time','0.05','--width','160'],check=True)
 # Cancellation must preserve an existing destination and clean staging files.
 (d/'loop.lay').write_text('p=canvas(size=(40,30),stylesheet="animate.css")\na=p.add(rect(size=(15,10)))\ns=p.step(duration=600s)\ns.animate(a,class="animate__animated animate__pulse",iterations="infinite")')
 target=d/'cancel.mp4';target.write_bytes(b'original destination')
 proc=subprocess.Popen([binary,'animate',str(d/'loop.lay'),'-o',str(target),'--width','640'],stdout=subprocess.PIPE,stderr=subprocess.PIPE)
 for _ in range(200):
  if list(d.glob('.laymesh-animation-*')):break
  if proc.poll() is not None:raise AssertionError(proc.communicate())
  time.sleep(.05)
 proc.send_signal(signal.SIGINT);stdout,stderr=proc.communicate(timeout=30)
 assert proc.returncode!=0 and b'cancel' in stderr.lower(),(stdout,stderr)
 assert target.read_bytes()==b'original destination' and not list(d.glob('.laymesh-animation-*'))
 # An unavailable encoder must also preserve the destination.
 env={**os.environ,'PATH':str(d/'empty')};r=subprocess.run([binary,'animate',str(d/'0.lay'),'-o',str(target)],env=env,capture_output=True)
 assert r.returncode!=0 and target.read_bytes()==b'original destination' and not list(d.glob('.laymesh-animation-*'))
 print('97 effects: MP4/GIF decode, metadata, snapshot, cancellation and failure cleanup passed')
