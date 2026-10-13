#!/usr/bin/env python3
"""Generate synthetic assets and verify media export, snapshots and cancellation."""
import argparse,array,json,math,subprocess,signal,time
from pathlib import Path
from PIL import Image,ImageChops,ImageStat
ROOT=Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser();p.add_argument('--binary',type=Path,default=ROOT/'target/debug/laymesh');p.add_argument('--output',type=Path,default=ROOT/'target/media-trial');a=p.parse_args();binary=str(a.binary.resolve());d=a.output.resolve();d.mkdir(parents=True,exist_ok=True)
def ffmpeg(*args):subprocess.run(['ffmpeg','-hide_banner','-loglevel','error','-y',*map(str,args)],check=True)
def run(*args):return subprocess.run([binary,*map(str,args)],check=True,capture_output=True,text=True)
ffmpeg('-f','lavfi','-i','testsrc2=size=320x180:rate=30','-f','lavfi','-i','sine=frequency=440:sample_rate=48000','-t',3,'-c:v','libx264','-threads',1,'-pix_fmt','yuv420p','-c:a','aac',d/'demo.mp4')
ffmpeg('-f','lavfi','-i','sine=frequency=220:sample_rate=48000','-t',1,'-c:a','pcm_s16le',d/'music.wav')
(d/'first.lay').write_text('p=canvas(size=(100mm,65mm),background="#f4f5f7")\nv=p.add(video(src="demo.mp4"),size=(80mm,45mm),offset=(10mm,5mm))\np.add(text("Video + audio",font_size=4mm),offset=(10mm,55mm))\np.play(v)\np.play(audio(src="music.wav"),loop=true,volume=0.2)\ns=p.step(duration=3s,hold=0s,start="after-previous")\n')
(d/'second.lay').write_text('p=canvas(size=(100mm,65mm),background="#ffffff")\np.add(text("Background music continues",font_size=4mm),offset=(10mm,25mm))\ns=p.step(duration=2s,hold=0s,start="after-previous")\n')
(d/'deck.laydoc').write_text('d=document(name="Media experiment")\nd.play(audio(src="music.wav"),loop=true,volume=0.15)\nd.add(page(src="first.lay"))\nd.add(page(src="second.lay"))\n')
result=json.loads(run('animate',d/'deck.laydoc','-o',d/'deck.mp4','--width',640,'--fps',30).stdout);assert result['frames']==150 and result['duration']==5
info=json.loads(subprocess.check_output(['ffprobe','-v','error','-show_entries','stream=codec_type,codec_name,duration','-of','json',str(d/'deck.mp4')]))['streams'];assert {s['codec_type']for s in info}=={'video','audio'};assert all(abs(float(s['duration'])-5)<1/30 for s in info)
ffmpeg('-i',d/'deck.mp4','-f','null','-')
def amplitude(file,start,frequency):
 raw=subprocess.check_output(['ffmpeg','-v','error','-ss',str(start),'-i',str(file),'-t','0.25','-ac','1','-ar','48000','-f','f32le','pipe:1']);samples=array.array('f',raw)
 real=sum(v*math.cos(2*math.pi*frequency*i/48000)for i,v in enumerate(samples));imag=sum(v*math.sin(2*math.pi*frequency*i/48000)for i,v in enumerate(samples));return 2*math.hypot(real,imag)/len(samples)
assert amplitude(d/'deck.mp4',.6,440)>.02,'Video original audio must be mixed'
assert amplitude(d/'deck.mp4',4,440)<.001,'Page video audio must stop after leaving the page'
assert amplitude(d/'deck.mp4',4,220)>.003,'Document music must continue on the second page'
for extension,codec in [('mp3','libmp3lame'),('m4a','aac')]:
 ffmpeg('-i',d/'music.wav','-c:a',codec,d/f'music.{extension}')
 source=d/f'audio-{extension}.lay';source.write_text(f'p=canvas(size=(40,30))\np.play(audio(src="music.{extension}"),trim_end=500ms)\n')
 run('animate',source,'-o',d/f'audio-{extension}.mp4','--width',160)
 assert amplitude(d/f'audio-{extension}.mp4',.1,220)>.02,f'{extension} source audio must decode and export'
for t in [0,.5]:run('render',d/'first.lay','-o',d/f'frame-{t}.png','--time',t,'--width',640)
delta=ImageStat.Stat(ImageChops.difference(Image.open(d/'frame-0.png').convert('RGB'),Image.open(d/'frame-0.5.png').convert('RGB'))).mean;assert sum(delta)>3,'Video time snapshots must change'
base='p=canvas(size=(100,60))\nv=p.add(video(src="demo.mp4"),size=(80,45),offset=(10,5))\ns=p.step(duration=3s,hold=0s)\n'
(d/'click.lay').write_text(base+'p.play(v,trigger="click",export_start=500ms,trim_start=500ms,trim_end=2s)\n')
run('animate',d/'click.lay','-o',d/'click.mp4','--width',320)
assert amplitude(d/'click.mp4',.1,440)<.001 and amplitude(d/'click.mp4',.8,440)>.02 and amplitude(d/'click.mp4',2.4,440)<.001,'Click export audio must follow its explicit trimmed schedule'
(d/'poster.lay').write_text(base+'p.play(v,trigger="click")\n');r=run('animate',d/'poster.lay','-o',d/'poster.mp4','--width',320);assert 'W_MEDIA_EXPORT' in r.stderr
for t in [0,1]:ffmpeg('-ss',t,'-i',d/'poster.mp4','-frames:v',1,d/f'poster-{t}.png')
delta=ImageStat.Stat(ImageChops.difference(Image.open(d/'poster-0.png').convert('RGB'),Image.open(d/'poster-1.png').convert('RGB'))).mean;assert sum(delta)<2,'Unscheduled click media must remain a poster'
(d/'loop.lay').write_text(base.replace('duration=3s','duration=600s')+'p.play(v,loop=true)\n')
target=d/'cancel.mp4';target.write_bytes(b'original destination');proc=subprocess.Popen([binary,'animate',str(d/'loop.lay'),'-o',str(target),'--width','640'],stdout=subprocess.PIPE,stderr=subprocess.PIPE)
for _ in range(200):
 if list(d.glob('.laymesh-animation-*')):break
 if proc.poll()is not None:raise AssertionError(proc.communicate())
 time.sleep(.025)
proc.send_signal(signal.SIGINT);_,stderr=proc.communicate(timeout=30);assert proc.returncode!=0 and b'cancel' in stderr.lower();assert target.read_bytes()==b'original destination';assert not list(d.glob('.laymesh-animation-*'))
# Native preview protocol exercises the full-resolution poster derivative and AAC sidecar.
resource_dir=d/'preview-resources';resource_dir.mkdir(exist_ok=True)
proc=subprocess.Popen([binary,'preview','--stdio'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True);ready=json.loads(proc.stdout.readline());assert 'media-v1' in ready['capabilities']
proc.stdin.write(json.dumps({'file':str(d/'first.lay'),'source':(d/'first.lay').read_text(),'protocol':3,'generation':1,'resource_dir':str(resource_dir)})+'\n');proc.stdin.flush();frame=json.loads(proc.stdout.readline());proc.terminate();proc.wait(timeout=10)
assert 'error'not in frame,frame.get('error');assert any(r['mime']=='video/mp4'for r in frame['resources']);assert any(r['mime']=='audio/mpeg'for r in frame['resources']);assert all(Path(r['path']).is_file()for r in frame['resources'])
# Static single-page and document exports must respect the configured manager.
config=d/'missing-encoder.json';config.write_text(json.dumps({'export':{'ffmpeg':{'executable':str(d/'missing-ffmpeg')}}}))
for source in ['first.lay','deck.laydoc']:
 failure=subprocess.run([binary,'render',str(d/source),'-o',str(d/'missing.pdf'),'--config',str(config)],capture_output=True,text=True)
 assert failure.returncode!=0 and 'Configured FFmpeg is missing' in failure.stderr,(source,failure.stderr)
print(json.dumps({'passed':True,'checks':['MP4 video/audio mixing','MP3 and M4A source audio','5s/150 frames','page audio stops and document music continues','time snapshots','explicit click audio/video schedule','poster warning','cancellation preserves target','native preview media resources','configured FFmpeg for static exports'],'output':str(d)}))
