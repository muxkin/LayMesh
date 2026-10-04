#!/usr/bin/env python3
"""Test the real VS Code preview DOM and extension bridge with trusted input.

Uses the repository's native Electron CDP harness; no npm packages or standalone
web server. Evidence and screenshots are written only to the requested directory.
"""
import argparse, base64, importlib.util, json, os, signal, socket, subprocess, tempfile, time, urllib.request
from pathlib import Path
from PIL import Image
ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('native_cdp',ROOT/'scripts/smoke-browser.py')
native=importlib.util.module_from_spec(spec);spec.loader.exec_module(native)
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--output',type=Path,required=True);p.add_argument('--code',default='/usr/share/code/code');a=p.parse_args();a.output.mkdir(parents=True,exist_ok=True)
with tempfile.TemporaryDirectory(prefix='laymesh-native-preview-',ignore_cleanup_errors=True) as temp:
 profile=Path(temp);gate=profile/'gate.json';evidence=profile/'evidence.json'
 (profile/'profile/User').mkdir(parents=True)
 (profile/'profile/User/settings.json').write_text(json.dumps({'workbench.colorTheme':'VS Code Dark','window.autoDetectColorScheme':False}))
 with socket.socket() as sock:sock.bind(('127.0.0.1',0));port=sock.getsockname()[1]
 bmp=profile/'中文 # 图片.bmp';Image.new('RGB',(20,10),(30,150,220)).save(bmp)
 env={**os.environ,'LAYMESH_PREVIEW_GATE':str(gate),'LAYMESH_PREVIEW_EVIDENCE':str(evidence),'LAYMESH_PREVIEW_BMP':str(bmp)};env.pop('ELECTRON_RUN_AS_NODE',None)
 command=[a.code,'--no-sandbox','--disable-gpu','--disable-updates','--ozone-platform=headless','--force-device-scale-factor=2','--remote-debugging-port='+str(port),'--remote-allow-origins=*','--user-data-dir='+str(profile/'profile'),'--extensions-dir='+str(profile/'extensions'),'--extensionDevelopmentPath='+str(ROOT/'extensions/vscode'),'--extensionTestsPath='+str(ROOT/'scripts/vscode-preview.cjs'),'--skip-welcome','--skip-release-notes','--new-window']
 log=(profile/'host.log').open('w');host=subprocess.Popen(command,env=env,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
 try:
  opener=urllib.request.build_opener(urllib.request.ProxyHandler({}))
  def targets():return json.load(opener.open(f'http://127.0.0.1:{port}/json/list'))
  def until(fn,label):
   deadline=time.monotonic()+35
   while time.monotonic()<deadline:
    try:
     value=fn()
     if value:return value
    except (OSError,ValueError,StopIteration):pass
    if host.poll() is not None:raise RuntimeError('VS Code stopped: '+(profile/'host.log').read_text()[-3500:])
    time.sleep(.1)
   raise RuntimeError(label+'\n'+(profile/'host.log').read_text()[-3500:])
  until(lambda:gate.exists(),'Fixture ready')
  root=native.CDP(next(t['webSocketDebuggerUrl'] for t in targets() if t['type']=='page'));root.call('Runtime.enable');root.call('Page.enable');root.call('Emulation.setDeviceMetricsOverride',{'width':1440,'height':1000,'deviceScaleFactor':2,'mobile':False})
  def attach():
   for t in targets():
    if t['type']=='iframe':
     c=native.CDP(t['webSocketDebuggerUrl']);c.call('Runtime.enable')
     if c.evaluate("!!document.querySelector('iframe')?.contentDocument?.querySelector('.preview')"):return c
  c=until(attach,'Real preview iframe');c.call('Log.enable')
  def evaluate(expression):return c.evaluate("(()=>{const d=document.querySelector('iframe').contentDocument;return ("+expression+");})()")
  def wait(expression):return until(lambda:evaluate(expression),expression)
  def phase(value):gate.write_text(json.dumps({'phase':value}))
  def wait_phase(value):return until(lambda:json.loads(gate.read_text()).get('phase')==value,'Gate '+value)
  evaluate("(()=>{d.defaultView.previewEvents=[];for(const type of ['pointerdown','pointerup','pointerout','pointerleave','mouseout','mouseleave'])d.addEventListener(type,e=>{const es=d.defaultView.previewEvents;es.push({type:e.type,target:e.target.className,related:e.relatedTarget?.className});if(es.length>12)es.shift()},true);return true})()")
  wait("d.querySelector('.preview').dataset.state==='success'&&d.querySelector('.figure').complete")
  # Sample the actual SVG image after the browser rendered its embedded BMP pixels.
  bitmap=evaluate("(()=>{const img=d.querySelector('.figure'),c=d.createElement('canvas');c.width=img.naturalWidth;c.height=img.naturalHeight;const ctx=c.getContext('2d');ctx.drawImage(img,0,0);return Array.from(ctx.getImageData(Math.floor(c.width*11/12),Math.floor(c.height*.55/9),1,1).data)})()")
  assert bitmap==[30,150,220,255],bitmap
  wait("d.body.classList.contains('vscode-dark')")
  root.call('Page.bringToFront')
  # Follow the real webview -> host -> Quick Input export wizard, without replacing VS Code APIs.
  assert evaluate("d.querySelector('[data-action=export]').textContent==='导出'")
  evaluate("(()=>{d.querySelector('[data-action=export]').click();return true})()")
  def quick_text():return root.evaluate("document.querySelector('.quick-input-widget')?.textContent||''")
  until(lambda:all(name in quick_text() for name in ['SVG','PDF','JPEG','TIFF','WebP']),'Export format picker')
  def key(name,code):
   for kind in ['keyDown','keyUp']:root.call('Input.dispatchKeyEvent',{'type':kind,'key':name,'windowsVirtualKeyCode':code})
  root.call('Input.insertText',{'text':'TIFF'});key('Enter',13)
  until(lambda:'DPI' in quick_text(),'Export DPI input')
  root.evaluate("(()=>{const input=document.querySelector('.quick-input-widget input');input.focus();input.select();return true})()")
  root.call('Input.insertText',{'text':'0'})
  until(lambda:'25400' in quick_text(),'Invalid DPI validation')
  root.evaluate("(()=>{document.querySelector('.quick-input-widget input').select();return true})()")
  root.call('Input.insertText',{'text':'144'});key('Enter',13)
  until(lambda:all(name in quick_text() for name in ['LZW','Deflate','PackBits']),'TIFF compression picker')
  key('Escape',27)
  last_layout=None;stable_frames=0
  def stable_layout():
   global last_layout,stable_frames
   geometry=root.evaluate("document.querySelector('iframe').getBoundingClientRect().toJSON()")
   geometry['figure']=evaluate("d.querySelector('.figure').getBoundingClientRect().toJSON()")
   stable_frames=stable_frames+1 if geometry==last_layout else 0;last_layout=geometry
   return stable_frames>=5
  until(stable_layout,'Stable native layout')
  phase('mouse-start');wait_phase('mouse-ready')
  def screen_point(fx,fy):
   outer=root.evaluate("document.querySelector('iframe').getBoundingClientRect().toJSON()")
   figure=evaluate("d.querySelector('.figure').getBoundingClientRect().toJSON()")
   return {'x':outer['x']+figure['x']+figure['width']*fx,'y':outer['y']+figure['y']+figure['height']*fy}
  def move(fx,fy):root.call('Input.dispatchMouseEvent',{'type':'mouseMoved',**screen_point(fx,fy)})
  def wait_readout():
   return wait("(()=>{const t=d.querySelector('.coordinates').textContent,m=/X=([0-9.]+) cm\\s+Y=([0-9.]+) cm/.exec(t),r=d.querySelector('.figure').getBoundingClientRect();return m&&Math.abs(Number(m[1])-5.5)*r.width/12<=0.8&&Math.abs(Number(m[2])-4)*r.height/9<=0.8&&t})()")
  move(5.5/12,4/9)
  readout=wait_readout()
  assert 'x=' in readout and 'y=' in readout,readout
  assert evaluate("d.querySelector('.corner').textContent==='cm'&&!d.querySelector('.crosshair').hidden")
  # Real wheel zoom is anchored at the mouse and keeps its canvas/data position.
  point=screen_point(5.5/12,4/9)
  root.call('Input.dispatchMouseEvent',{'type':'mouseWheel','deltaY':-220,'deltaX':0,'modifiers':2,**point})
  wait_readout()
  assert evaluate("d.querySelector('.zoom').textContent!=='100%'")
  # Space + drag shifts the page while preserving correct readout after release.
  root.call('Input.dispatchMouseEvent',{'type':'mousePressed','button':'left','clickCount':1,**point})
  root.call('Input.dispatchMouseEvent',{'type':'mouseReleased','button':'left','clickCount':1,**point})
  wait("d.activeElement.classList.contains('viewport')")
  time.sleep(.15)
  before_pan=evaluate("d.querySelector('.figure').getBoundingClientRect().toJSON()")
  root.call('Input.dispatchKeyEvent',{'type':'keyDown','key':' ','code':'Space','windowsVirtualKeyCode':32})
  root.call('Input.dispatchMouseEvent',{'type':'mousePressed','button':'left','clickCount':1,**point})
  root.call('Input.dispatchMouseEvent',{'type':'mouseMoved','button':'left','buttons':1,'x':point['x']+35,'y':point['y']+20})
  root.call('Input.dispatchMouseEvent',{'type':'mouseReleased','button':'left','clickCount':1,'x':point['x']+35,'y':point['y']+20})
  root.call('Input.dispatchKeyEvent',{'type':'keyUp','key':' ','code':'Space','windowsVirtualKeyCode':32})
  wait("Math.abs(d.querySelector('.figure').getBoundingClientRect().x-("+str(before_pan['x'])+")-35)<1")
  move(5.5/12,4/9);wait_readout()
  evaluate("(()=>{d.querySelector('[data-action=actual]').click();return true})()");wait("d.querySelector('.zoom').textContent==='100%'")
  evaluate("(()=>{d.querySelector('[data-action=rulers]').click();return true})()");wait("d.querySelector('.workspace').classList.contains('no-rulers')")
  evaluate("(()=>{d.querySelector('[data-action=rulers]').click();d.querySelector('[data-action=fit]').click();return true})()");wait("!d.querySelector('.workspace').classList.contains('no-rulers')")
  move(5.5/12,4/9);wait_readout()
  # Continuous motion proves the last sample is reflected without source writes.
  for i in range(30):move(.3+.3*i/29,.3)
  move(5.5/12,4/9);wait_readout()
  screenshot=root.call('Page.captureScreenshot',{'format':'png','captureBeyondViewport':False});(a.output/'dark-desktop.png').write_bytes(base64.b64decode(screenshot['data']))
  state=evaluate("({readout:d.querySelector('.coordinates').textContent,blank:!d.querySelector('.figure').naturalWidth,errors:d.querySelector('.error-location').hidden,unit:d.querySelector('.corner').textContent})");assert not state['blank'] and state['errors']
  outer=root.evaluate("document.querySelector('iframe').getBoundingClientRect().toJSON()");vp=evaluate("d.querySelector('.viewport').getBoundingClientRect().toJSON()")
  root.call('Input.dispatchMouseEvent',{'type':'mouseMoved','x':outer['x']+vp['x']+3,'y':outer['y']+vp['y']+3});wait("d.querySelector('.coordinates').textContent===''&&d.querySelector('.crosshair').hidden")
  # Manual view survives explicit English and automatic editor-language selection.
  evaluate("(()=>{d.querySelector('[data-action=actual]').click();d.querySelector('[data-action=in]').click();return true})()");wait("d.querySelector('.zoom').textContent==='125%'")
  language_view=evaluate("({zoom:d.querySelector('.zoom').textContent,width:d.querySelector('.figure').getBoundingClientRect().width,src:d.querySelector('.figure').src})")
  phase('ui-verified');wait_phase('english-ready')
  wait("d.documentElement.lang==='en'&&d.querySelector('[data-action=refresh]').textContent==='Refresh'&&d.querySelector('[data-action=export]').textContent==='Export'&&d.querySelector('.status').textContent==='Updated'")
  assert evaluate("({zoom:d.querySelector('.zoom').textContent,width:d.querySelector('.figure').getBoundingClientRect().width,src:d.querySelector('.figure').src})")==language_view
  move(5.5/12,4/9);english_readout=wait_readout();assert english_readout.startswith('Canvas:') and '|  Plot ' in english_readout,english_readout
  assert evaluate("d.querySelector('.viewport').getAttribute('aria-label').startsWith('Figure preview')&&d.querySelector('.ruler').getAttribute('aria-label')==='Horizontal ruler'")
  screenshot=root.call('Page.captureScreenshot',{'format':'png','captureBeyondViewport':False});(a.output/'english-desktop.png').write_bytes(base64.b64decode(screenshot['data']))
  phase('english-verified');wait_phase('auto-ready');wait("d.documentElement.lang==='en'&&d.querySelector('[data-action=rulers]').textContent==='Rulers'")
  assert evaluate("({zoom:d.querySelector('.zoom').textContent,width:d.querySelector('.figure').getBoundingClientRect().width,src:d.querySelector('.figure').src})")==language_view
  evaluate("(()=>{d.querySelector('[data-action=fit]').click();return true})()")
  phase('auto-verified');wait_phase('light-ready')
  wait("d.body.classList.contains('vscode-light')")
  root.call('Emulation.setDeviceMetricsOverride',{'width':760,'height':720,'deviceScaleFactor':2,'mobile':False})
  wait("d.querySelector('.viewport').clientWidth<500")
  until(stable_layout,'Stable narrow layout')
  wait("d.defaultView.devicePixelRatio===2")
  until(stable_layout,'Stable high DPI layout')
  move(5.5/12,4/9);wait_readout()
  layout=evaluate("({body:d.body.scrollWidth,width:d.documentElement.clientWidth,canvas:d.querySelector('.horizontal.ruler').width,viewport:d.querySelector('.viewport').clientWidth,dpr:d.defaultView.devicePixelRatio,theme:d.body.className})")
  assert layout['body']<=layout['width'] and layout['canvas']==round(layout['viewport']*layout['dpr']),layout
  screenshot=root.call('Page.captureScreenshot',{'format':'png','captureBeyondViewport':False});(a.output/'light-narrow.png').write_bytes(base64.b64decode(screenshot['data']))
  console_errors=[event for event in c.events if event.get('method')=='Runtime.exceptionThrown' or event.get('method')=='Runtime.consoleAPICalled' and event['params'].get('type')=='error']
  assert not console_errors,console_errors
  phase('light-verified');wait_phase('english-error')
  wait("d.querySelector('.preview').dataset.state==='error'&&d.querySelector('.status').textContent==='Error · preview stale'")
  assert evaluate("!d.querySelector('.error-location').hidden&&d.querySelector('.error-location').title==='Go to source'&&d.querySelector('.figure').naturalWidth>0")
  phase('error-verified');wait_phase('complete');host.wait(timeout=20);assert host.returncode==0
  result=json.loads(evidence.read_text());result['ui']={'bmp_pixel':bitmap,'export_wizard_verified':True,'invalid_export_dpi_rejected':True,'tiff_compression_choices_verified':True,'initial_readout':readout,'english_readout':english_readout,'state':state,'narrow':layout,'console_errors':console_errors,'viewports':['1440x1000 dark DPR 2','760x720 light DPR 2'],'checks':['BMP on a Chinese/space/hash path','canvas/data hover','ruler markers','wheel anchor','Space drag','100%','ruler toggle','fit','continuous motion','pointer exit','narrow resize','DPR ruler pixels','explicit English and auto UI','view preserved on language change','English stale error and source action']}
  (a.output/'evidence.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n');print(json.dumps(result,ensure_ascii=False))
 except Exception:
  try:
   debug=evaluate("({text:d.body.innerText,figure:d.querySelector('.figure')?.getBoundingClientRect().toJSON(),viewport:d.querySelector('.viewport')?.getBoundingClientRect().toJSON(),events:d.defaultView.previewEvents,panning:d.querySelector('.viewport').className,active:d.activeElement.className})")
   print(json.dumps(debug,ensure_ascii=False))
   shot=root.call('Page.captureScreenshot',{'format':'png','captureBeyondViewport':False});(a.output/'failure.png').write_bytes(base64.b64decode(shot['data']))
  except Exception:pass
  raise
 finally:
  if host.poll() is None:os.killpg(host.pid,signal.SIGTERM);host.wait(timeout=10)
  log.close()
