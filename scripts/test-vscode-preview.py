#!/usr/bin/env python3
"""Test the real VS Code preview DOM and extension bridge with trusted input.

Uses the repository's native Electron CDP harness; no npm packages or standalone
web server. Evidence and screenshots are written only to the requested directory.
"""
import argparse, base64, importlib.util, json, os, shutil, signal, socket, subprocess, tempfile, time, urllib.request
from pathlib import Path
from PIL import Image
ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('native_cdp',ROOT/'scripts/smoke-browser.py')
native=importlib.util.module_from_spec(spec);spec.loader.exec_module(native)
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--output',type=Path,required=True);p.add_argument('--code',default='/usr/share/code/code');p.add_argument('--extension-dir',type=Path,default=ROOT/'extensions/vscode');p.add_argument('--normal-host',action='store_true',help='Use a fixture extension in a normal host so native confirmation dialogs are allowed');p.add_argument('--block-worker-resource-fetch',action='store_true',help='Return HTTP 408 for extension resource requests inside blob workers');a=p.parse_args();a.output.mkdir(parents=True,exist_ok=True)
with tempfile.TemporaryDirectory(prefix='laymesh-native-preview-',ignore_cleanup_errors=True) as temp:
 profile=Path(temp);gate=profile/'gate.json';evidence=profile/'evidence.json'
 extension=a.extension_dir.resolve()
 if a.block_worker_resource_fetch:
  extension=profile/'fixture/extensions/vscode'
  shutil.copytree(a.extension_dir,extension,ignore=shutil.ignore_patterns('*.vsix','src','__pycache__'))
  (profile/'fixture/tests').symlink_to(ROOT/'tests',target_is_directory=True)
  # Exercise the real packaged renderer while reproducing the resource proxy
  # failure. In-memory CanvasKit blobs are allowed; extension URLs return 408.
  guard="const windowResourceFetch=self.fetch.bind(self);self.fetch=(input,...args)=>String(input?.url||input).startsWith('blob:')?windowResourceFetch(input,...args):Promise.resolve(new Response('Request Timeout',{status:408}));\n"
  for name in ['preview-worker.mjs','presentation-worker.mjs']:
   worker=extension/'dist'/name;worker.write_text(guard+worker.read_text())
 (profile/'profile/User').mkdir(parents=True)
 (profile/'profile/User/settings.json').write_text(json.dumps({'workbench.colorTheme':'VS Code Dark','window.autoDetectColorScheme':False,'files.simpleDialog.enable':True,'window.dialogStyle':'custom'}))
 with socket.socket() as sock:sock.bind(('127.0.0.1',0));port=sock.getsockname()[1]
 bmp=profile/'中文 # 图片.bmp';Image.new('RGB',(20,10),(30,150,220)).save(bmp)
 transparent=profile/'透明.png';Image.new('RGBA',(20,10),(255,0,0,128)).save(transparent)
 high=profile/'16位.png';Image.new('I;16',(20,10),35000).save(high)
 env={**os.environ,'LAYMESH_PREVIEW_GATE':str(gate),'LAYMESH_PREVIEW_EVIDENCE':str(evidence),'LAYMESH_PREVIEW_BMP':str(bmp),'LAYMESH_PREVIEW_TRANSPARENT':str(transparent),'LAYMESH_PREVIEW_16BIT':str(high),'LAYMESH_PREVIEW_NORMAL_HOST':'1' if a.normal_host else ''};env.pop('ELECTRON_RUN_AS_NODE',None)
 command=[a.code,'--no-sandbox','--disable-gpu','--disable-updates','--ozone-platform=headless','--force-device-scale-factor=2','--remote-debugging-port='+str(port),'--remote-allow-origins=*','--user-data-dir='+str(profile/'profile'),'--extensions-dir='+str(profile/'extensions'),'--extensionDevelopmentPath='+str(extension),'--extensionTestsPath='+str(ROOT/'scripts/vscode-preview.cjs'),'--skip-welcome','--skip-release-notes','--new-window']
 if a.normal_host:
  fixture=profile/'extensions/laymesh-preview-qa-0.0.1';fixture.mkdir(parents=True)
  (fixture/'package.json').write_text(json.dumps({'name':'laymesh-preview-qa','publisher':'local','version':'0.0.1','engines':{'vscode':'^1.85.0'},'main':'./index.cjs','activationEvents':['onStartupFinished']}))
  (fixture/'index.cjs').write_text("exports.activate=()=>{require("+json.dumps(str(ROOT/'scripts/vscode-preview.cjs'))+").run().then(()=>{require('vscode').commands.executeCommand('workbench.action.quit').catch(()=>{});},error=>{console.error(error);require('fs').writeFileSync("+json.dumps(str(gate))+",JSON.stringify({phase:'failed',error:String(error)}));});};")
  command=[arg for arg in command if not arg.startswith('--extensionTestsPath=')]
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
  def wait_phase(value):
   def check():
    status=json.loads(gate.read_text())
    if status.get('phase')=='failed':raise RuntimeError(status.get('error','Fixture failed'))
    return status.get('phase')==value
   return until(check,'Gate '+value)
  def toolbar_layout():
   layout=evaluate("(()=>{const r=e=>e.getBoundingClientRect().toJSON(),t=d.querySelector('.toolbar'),g=d.querySelector('.present-group');return {toolbar:r(t),present:r(g),buttons:[...g.querySelectorAll('button,summary')].slice(0,2).map(r),visible:[...t.children].filter(e=>e.getBoundingClientRect().width).map(r)}})()")
   t=layout['toolbar'];assert all(b['top']>=t['top'] and b['bottom']<=t['bottom'] and b['right']<=t['right']+1 for b in layout['visible']),layout
   a,b=layout['buttons'];assert abs(a['top']-b['top'])<1 and abs(a['right']-b['left'])<2,layout
   assert evaluate("d.querySelector('[data-action=refresh]').getAttribute('aria-label')==='Refresh'||d.querySelector('[data-action=refresh]').textContent.includes('刷新')")
   assert not evaluate("!!d.querySelector('.toolbar-label,[data-action=toggle-playback]')")
   return layout
  evaluate("(()=>{d.defaultView.previewEvents=[];for(const type of ['pointerdown','pointerup','pointerout','pointerleave','mouseout','mouseleave'])d.addEventListener(type,e=>{const es=d.defaultView.previewEvents;es.push({type:e.type,target:e.target.className,related:e.relatedTarget?.className});if(es.length>12)es.shift()},true);return true})()")
  wait("d.querySelector('.preview').dataset.state==='success'&&(d.querySelector('.preview').dataset.painted==='true'||d.querySelector('.figure').complete)")
  wait("d.body.classList.contains('vscode-dark')")
  toolbar_layout()
  root.call('Page.bringToFront')
  # Follow the real webview -> host -> format picker and save dialog, without replacing VS Code APIs.
  assert evaluate("d.querySelector('[data-action=export]').textContent==='导出'")
  evaluate("(()=>{d.querySelector('[data-action=export]').click();return true})()")
  def quick_text():return root.evaluate("document.querySelector('.quick-input-widget')?.textContent||''")
  until(lambda:all(name in quick_text() for name in ['SVG','PDF','JPEG','TIFF','WebP']),'Export format picker')
  def key(name,code):
   for kind in ['keyDown','keyUp']:root.call('Input.dispatchKeyEvent',{'type':kind,'key':name,'code':name,'windowsVirtualKeyCode':code})
  root.call('Input.insertText',{'text':'TIFF'})
  until(lambda:'TIFF' in quick_text() and 'SVG' not in quick_text(),'Filtered TIFF format')
  key('Enter',13)
  until(lambda:root.evaluate("!!document.querySelector('.quick-input-widget input')?.value.endsWith('.tif')"),'Direct TIFF save dialog')
  assert 'DPI' not in quick_text() and 'PackBits' not in quick_text(),quick_text()
  key('Escape',27)
  last_layout=None;stable_frames=0
  def stable_layout():
   global last_layout,stable_frames
   geometry=root.evaluate("document.querySelector('iframe').getBoundingClientRect().toJSON()")
   geometry['figure']=evaluate("d.querySelector('.figure').getBoundingClientRect().toJSON()")
   stable_frames=stable_frames+1 if geometry==last_layout else 0;last_layout=geometry
   return stable_frames>=5
  until(stable_layout,'Stable native layout')
  # Sample the rendered page screenshot; external full-resolution resources cannot
  # be drawn into a canvas without changing webview origin security.
  bounds=root.evaluate("document.querySelector('iframe').getBoundingClientRect().toJSON()")
  rect=evaluate("d.querySelector('.figure').getBoundingClientRect().toJSON()")
  shot=root.call('Page.captureScreenshot',{'format':'png','captureBeyondViewport':False})
  import io
  rendered=Image.open(io.BytesIO(base64.b64decode(shot['data']))).convert('RGBA')
  ratio=rendered.width/1440
  bitmap=list(rendered.getpixel((round((bounds['x']+rect['x']+rect['width']*11/12)*ratio),round((bounds['y']+rect['y']+rect['height']*.55/9)*ratio))))
  assert all(abs(a-b)<=3 for a,b in zip(bitmap,[30,150,220,255])),bitmap
  def sample(fx,fy):return list(rendered.getpixel((round((bounds['x']+rect['x']+rect['width']*fx)*ratio),round((bounds['y']+rect['y']+rect['height']*fy)*ratio))))
  transparent_pixel=sample(11.75/12,.55/9);high_pixel=sample(11/12,1.25/9)
  assert all(abs(a-b)<=4 for a,b in zip(transparent_pixel,[255,127,127,255])),transparent_pixel
  assert all(abs(a-b)<=2 for a,b in zip(high_pixel,[136,136,136,255])),high_pixel
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
  state=evaluate("({readout:d.querySelector('.coordinates').textContent,blank:d.querySelector('.figure').getBoundingClientRect().width===0,errors:d.querySelector('.error-location').hidden,unit:d.querySelector('.corner').textContent})");assert not state['blank'] and state['errors']
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
  assert evaluate("(()=>{const b=d.querySelector('.render-details').getBoundingClientRect(),f=d.querySelector('.preview-footer').getBoundingClientRect();return b.right<=f.right&&b.left>=f.left})()"), "Narrow footer details must stay visible beside long coordinates"
  screenshot=root.call('Page.captureScreenshot',{'format':'png','captureBeyondViewport':False});(a.output/'light-narrow.png').write_bytes(base64.b64decode(screenshot['data']))
  toolbar_layout()
  console_errors=[event for event in c.events if event.get('method')=='Runtime.exceptionThrown' or event.get('method')=='Runtime.consoleAPICalled' and event['params'].get('type')=='error']
  assert not console_errors,console_errors
  stable_before_error=evaluate("d.querySelector('.viewport').getBoundingClientRect().toJSON()")
  phase('light-verified');wait_phase('english-error')
  wait("d.querySelector('.preview').dataset.state==='error'&&d.querySelector('.status').textContent==='Source error · last successful preview'")
  assert evaluate("d.querySelector('.viewport').getBoundingClientRect().toJSON()")==stable_before_error
  evaluate("(()=>{d.querySelector('.diagnostic-menu').open=true;return true})()")
  assert evaluate("d.querySelector('.viewport').getBoundingClientRect().toJSON()")==stable_before_error
  assert evaluate("!d.querySelector('.error-location').hidden&&d.querySelector('.error-location').title==='Go to source'&&d.querySelector('.figure').getBoundingClientRect().width>0")
  screenshot=root.call('Page.captureScreenshot',{'format':'png','captureBeyondViewport':False});(a.output/'diagnostics-narrow.png').write_bytes(base64.b64decode(screenshot['data']))
  phase('error-verified');wait_phase('document-ready')
  c=until(attach,'Document preview iframe');c.call('Log.enable')
  wait("d.querySelector('.preview').dataset.state==='success'&&d.querySelector('.preview').dataset.painted==='true'&&d.querySelector('.page-select').value==='cover'")
  assert evaluate("!d.querySelector('.page-select').hidden&&d.querySelector('.page-select').options.length===3")
  assert evaluate("d.querySelector('[data-action=previous-page]').disabled&&!d.querySelector('[data-action=next-page]').disabled")
  evaluate("(()=>{d.querySelector('[data-action=next-page]').click();return true})()")
  wait("d.querySelector('.page-select').selectedIndex===1&&d.querySelector('.preview').dataset.painted==='true'")
  evaluate("(()=>{d.querySelector('[data-action=previous-page]').click();return true})()")
  wait("d.querySelector('.page-select').value==='cover'&&d.querySelector('.preview').dataset.painted==='true'")
  evaluate("(()=>{const s=d.querySelector('.page-select');s.value='results';s.dispatchEvent(new d.defaultView.Event('change'));return true})()")
  wait("d.querySelector('.page-select').value==='results'&&d.querySelector('.preview').dataset.state==='success'&&d.querySelector('.corner').textContent==='cm'&&d.querySelector('.preview').dataset.painted==='true'")
  def drawing_hash(name=None, visible_card=False):
   import hashlib
   outer=root.evaluate("document.querySelector('iframe').getBoundingClientRect().toJSON()")
   bounds=evaluate("d.querySelector('.figure-pixels').getBoundingClientRect().toJSON()")
   shot=root.call('Page.captureScreenshot',{'format':'png','captureBeyondViewport':False})
   pixels=Image.open(io.BytesIO(base64.b64decode(shot['data']))).convert('RGBA')
   scale=pixels.width/root.evaluate('innerWidth')
   x=(outer['x']+bounds['x'])*scale;y=(outer['y']+bounds['y'])*scale
   cropped=pixels.crop((round(x),round(y),round(x+bounds['width']*scale),round(y+bounds['height']*scale)))
   if visible_card:
    return cropped.getpixel((round(cropped.width*32.5/80),round(cropped.height*17.5/40)))[:3]==(13,110,253)
   if name:
    cropped.save(a.output/(name+'.png'))
    (a.output/(name+'.json')).write_text(json.dumps({'outer':outer,'bounds':bounds,'scale':scale}))
   return hashlib.sha256(cropped.tobytes()).hexdigest()

  formula_before=drawing_hash();phase('document-results');wait_phase('document-dirty-ready')
  until(lambda:drawing_hash()!=formula_before,'Unsaved formula visible')
  wait("d.querySelector('.preview').dataset.painted==='true'")
  phase('document-dirty-verified');wait_phase('document-reordered')
  wait("d.querySelector('.page-select').value==='results'&&d.querySelector('.page-select').selectedIndex===2&&d.querySelector('.preview').dataset.painted==='true'")
  root.call('Emulation.setDeviceMetricsOverride',{'width':1440,'height':1000,'deviceScaleFactor':2,'mobile':False})
  evaluate("(()=>{d.querySelector('[data-action=fit]').click();return true})()");until(stable_layout,'Stable document layout')
  screenshot=root.call('Page.captureScreenshot',{'format':'png','captureBeyondViewport':False});(a.output/'document-desktop.png').write_bytes(base64.b64decode(screenshot['data']))
  evaluate("(()=>{d.querySelector('[data-action=export-document]').click();return true})()")
  until(lambda:root.evaluate("!!document.querySelector('.quick-input-widget input')?.value.endsWith('.pdf')"),'Whole document PDF save dialog');key('Escape',27)
  evaluate("(()=>{d.querySelector('[data-action=open-page]').click();return true})()")
  document_errors=[event for event in c.events if event.get('method')=='Runtime.exceptionThrown' or event.get('method')=='Runtime.consoleAPICalled' and event['params'].get('type')=='error'];assert not document_errors,document_errors
  phase('document-ui-verified');wait_phase('animation-ready')
  c=until(attach,'Animation preview iframe');c.call('Log.enable')
  wait("d.querySelector('.preview').dataset.painted==='true'&&d.querySelector('.animation-controls').style.display==='flex'")
  # Painted acknowledges GPU submission. Wait for the expected card to reach
  # the compositor before recording the static screenshot as our reference.
  until(lambda:drawing_hash(visible_card=True),'CSS-styled card presented')
  toolbar_layout()
  assert evaluate("(()=>{const p=d.querySelector('.playback-buttons'),f=d.querySelector('.preview-footer'),r=p.getBoundingClientRect(),b=f.getBoundingClientRect();return !p.hidden&&p.querySelectorAll('button').length===3&&p.parentElement===f&&r.top>=b.top&&r.bottom<=b.bottom&&!d.querySelector('.animation-controls').contains(p)})()"), 'Three small step buttons belong in the footer'
  full_hash=drawing_hash('animation-static-before')
  evaluate("(()=>{d.querySelector('[data-action=animation-play]').click();return true})()")
  wait("d.querySelector('[data-action=animation-play]').getAttribute('aria-label')==='Pause'")
  evaluate("(()=>{d.querySelector('[data-action=animation-play]').click();const r=d.querySelector('[aria-label=\"Animation time\"]');r.value='0';r.dispatchEvent(new d.defaultView.Event('input'));return true})()")
  wait("d.querySelector('.preview').dataset.painted==='true'");until(lambda:drawing_hash()!=full_hash,'Animation state rendered')
  evaluate("(()=>{d.querySelector('[data-action=animation-next]').click();d.querySelector('[data-action=animation-next]').click();d.querySelector('[data-action=animation-play]').click();return true})()")
  wait("d.querySelector('[aria-label=\"Current step\"]').textContent.includes('Pulse')")
  evaluate("(()=>{d.querySelector('[data-action=animation-static]').click();return true})()")
  wait("d.querySelector('.preview').dataset.painted==='true'")
  try:until(lambda:drawing_hash()==full_hash,'Static layout restored')
  finally:drawing_hash('animation-static-after')
  screenshot=root.call('Page.captureScreenshot',{'format':'png','captureBeyondViewport':False});(a.output/'animation-controls.png').write_bytes(base64.b64decode(screenshot['data']))
  # Full presentation uses an independent prepared snapshot and real webview controls.
  wait("d.defaultView.laymeshPresentation.deck?.pages.length>0")
  evaluate("(()=>{d.defaultView.laymeshPresentation.open();return true})()")
  wait("d.defaultView.laymeshPresentation.ready&&d.querySelector('.presentation').dataset.painted==='true'")
  evaluate("(()=>{d.querySelector('.presentation').dispatchEvent(new d.defaultView.KeyboardEvent('keydown',{key:' ',bubbles:true}));return true})()")
  wait("Number(d.querySelector('.presentation').dataset.time)>0.05")
  evaluate("(()=>{d.querySelector('.presentation').dispatchEvent(new d.defaultView.KeyboardEvent('keydown',{key:' ',bubbles:true}));return true})()")
  wait("d.querySelector('.presentation').dataset.running==='false'")
  screenshot=root.call('Page.captureScreenshot',{'format':'png','captureBeyondViewport':False});(a.output/'presentation.png').write_bytes(base64.b64decode(screenshot['data']))
  evaluate("(()=>{d.defaultView.laymeshPresentation.close();return true})()")
  wait("!d.querySelector('.presentation')")
  phase('animation-verified')
  if a.normal_host:
   wait_phase('ffmpeg-dialog-ready')
   until(lambda:'No usable FFmpeg' in root.evaluate("document.querySelector('.monaco-dialog-box')?.innerText||''"),'Missing FFmpeg confirmation')
   assert 'China mirror' in root.evaluate("document.querySelector('.monaco-dialog-box').innerText")
   screenshot=root.call('Page.captureScreenshot',{'format':'png','captureBeyondViewport':False});(a.output/'ffmpeg-confirmation.png').write_bytes(base64.b64decode(screenshot['data']))
   root.evaluate("(()=>{const b=[...document.querySelectorAll('.monaco-dialog-box [role=button],.monaco-dialog-box .monaco-button')].find(b=>b.textContent.trim()==='Cancel');if(!b)throw Error('Cancel button missing');b.click();})()")
   until(lambda:not root.evaluate("!!document.querySelector('.monaco-dialog-box')"),'Download declined')
   phase('ffmpeg-dialog-cancelled')
  if os.environ.get('LAYMESH_ACADEMIC_DOC'):
   wait_phase('academic-ready');c=until(attach,'Academic preview iframe');c.call('Log.enable')
   wait("d.defaultView.laymeshPresentation.deck?.pages.length===32")
   evaluate("(()=>{d.defaultView.laymeshPresentation.open();return true})()")
   wait("d.defaultView.laymeshPresentation.ready")
   assert evaluate("d.querySelector('.presentation-bar').inert"), 'Embedded controls are hidden by default'
   evaluate("(()=>{d.querySelector('.presentation').dispatchEvent(new d.defaultView.KeyboardEvent('keydown',{key:'Tab',bubbles:true,cancelable:true}));return true})()")
   assert evaluate("!d.querySelector('.presentation-bar').inert && d.activeElement.dataset.action==='present-previous'"), 'Tab reveals and focuses embedded controls'
   evaluate("(()=>{const v=d.defaultView.laymeshPresentation;v.showMenu();v.showPanel('toolbar');d.querySelector('[data-toolbar-mode=always]').click();return true})()")
   assert evaluate("!d.querySelector('.presentation-bar').inert"), 'Embedded always-visible choice'
   evaluate("(()=>{const v=d.defaultView.laymeshPresentation;v.showMenu();v.showPanel('toolbar');d.querySelector('[data-toolbar-mode=hidden]').click();return true})()")
   assert evaluate("d.querySelector('.presentation-bar').inert"), 'Embedded hidden choice'
   evaluate("(()=>{d.querySelector('.presentation-canvas').dispatchEvent(new d.defaultView.WheelEvent('wheel',{deltaY:120,bubbles:true,cancelable:true}));return true})()")
   wait("d.defaultView.laymeshPresentation.clock.time>0.05")
   time.sleep(.22)
   evaluate("(()=>{d.querySelector('.presentation-canvas').dispatchEvent(new d.defaultView.WheelEvent('wheel',{deltaY:-120,bubbles:true,cancelable:true}));return true})()")
   wait("d.defaultView.laymeshPresentation.clock.time===0")
   for page_index in [17,24,28,31,0]:
    evaluate(f"(()=>{{d.defaultView.laymeshPresentation.go({page_index},{{end:true}});return true}})()")
    wait(f"!d.defaultView.laymeshPresentation.switching&&d.querySelector('.presentation').dataset.page==='p{page_index+1:02d}'")
    if page_index==17:
     screenshot=root.call('Page.captureScreenshot',{'format':'png','captureBeyondViewport':False});(a.output/'academic-dense.png').write_bytes(base64.b64decode(screenshot['data']))
   evaluate("(()=>{d.defaultView.laymeshPresentation.close();return true})()");wait("!d.querySelector('.presentation')")
   phase('academic-verified')
  wait_phase('complete')
  host.wait(timeout=20)
  assert host.returncode==0
  result=json.loads(evidence.read_text());result['ui']={'bmp_pixel':bitmap,'transparent_webp_pixel':transparent_pixel,'opaque_16bit_jpeg_pixel':high_pixel,'direct_export_save_dialog_verified':True,'no_dpi_or_compression_prompts':True,'initial_readout':readout,'english_readout':english_readout,'state':state,'narrow':layout,'console_errors':console_errors,'viewports':['1440x1000 dark DPR 2','760x720 light DPR 2'],'checks':['BMP on a Chinese/space/hash path','canvas/data hover','ruler markers','wheel anchor','Space drag','100%','ruler toggle','fit','continuous motion','pointer exit','narrow resize','DPR ruler pixels','explicit English and auto UI','view preserved on language change','English stale error and source action']}
  result['ui']['documents']={'page_selector':True,'formula_edit_visible':True,'reordering_preserves_page_id':True,'whole_pdf_direct_save_dialog':True,'console_errors':document_errors}
  result['ui']['worker_resource_fetch_blocked']=a.block_worker_resource_fetch
  result['ui']['native_ffmpeg_confirmation']=a.normal_host
  if a.normal_host:result['ui']['normal_host_shutdown']={'command':'workbench.action.quit','exit_code':host.returncode}
  result['ui']['toolbar']={'single_line':True,'joined_present_button':True,'refresh':True,'page_arrows':True,'footer_step_buttons':3,'playback_visibility_toggle':False}
  (a.output/'evidence.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n');print(json.dumps(result,ensure_ascii=False))
 except Exception:
  shutil.copy2(profile/'host.log',a.output/'failure-host.log')
  if gate.exists():shutil.copy2(gate,a.output/'failure-gate.json')
  try:
   debug=evaluate("({text:d.body.innerText,error:d.querySelector('.error-location')?.textContent,figure:d.querySelector('.figure')?.getBoundingClientRect().toJSON(),viewport:d.querySelector('.viewport')?.getBoundingClientRect().toJSON(),events:d.defaultView.previewEvents,panning:d.querySelector('.viewport').className,active:d.activeElement.className})")
   (a.output/'failure.json').write_text(json.dumps(debug,ensure_ascii=False,indent=2)+'\n')
   print(json.dumps(debug,ensure_ascii=False))
   shot=root.call('Page.captureScreenshot',{'format':'png','captureBeyondViewport':False});(a.output/'failure.png').write_bytes(base64.b64decode(shot['data']))
  except Exception:pass
  raise
 finally:
  if host.poll() is None:os.killpg(host.pid,signal.SIGTERM);host.wait(timeout=10)
  log.close()
