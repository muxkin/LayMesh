#!/usr/bin/env python3
"""Trusted directional keys in an isolated, actual VS Code webview."""
import argparse,json,os,subprocess,tempfile,socket,time,urllib.request,importlib.util
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser(description=__doc__)
for key in ['output','binary','source','extension-dir']:p.add_argument('--'+key,required=True,type=Path)
p.add_argument('--animation-rounds',type=int,default=0,help='Play every authored animation naturally instead of the rapid-key benchmark');p.add_argument('--samples',type=int,default=50);p.add_argument('--timeout',type=float,default=1800);a=p.parse_args();a.output=a.output.resolve();a.output.parent.mkdir(parents=True,exist_ok=True)
with tempfile.TemporaryDirectory(prefix='laymesh-keyboard-') as temp:
 root=Path(temp);user=root/'profile/User';user.mkdir(parents=True);(user/'settings.json').write_text(json.dumps({'files.autoSave':'off','laymesh.executable':str(a.binary.resolve())}))
 gate=root/'viewport-gate';ready=root/'keyboard-ready'
 env={**os.environ,'LAYMESH_PERFORMANCE_OUTPUT':str(a.output),'LAYMESH_PERFORMANCE_BINARY':str(a.binary.resolve()),'LAYMESH_PERFORMANCE_SOURCE':str(a.source.resolve()),'LAYMESH_PERFORMANCE_GATE':str(gate),'LAYMESH_KEYBOARD_READY':str(ready)};env.pop('ELECTRON_RUN_AS_NODE',None)
 if a.animation_rounds:env['LAYMESH_ANIMATION_TEST']='1'
 with socket.socket() as sock:sock.bind(('127.0.0.1',0));port=sock.getsockname()[1]
 args=['/usr/share/code/code',f'--remote-debugging-port={port}','--remote-allow-origins=*','--no-sandbox','--disable-gpu','--disable-updates','--disable-background-timer-throttling','--disable-renderer-backgrounding','--disable-backgrounding-occluded-windows','--ozone-platform=headless','--force-device-scale-factor=2','--user-data-dir='+str(root/'profile'),'--extensions-dir='+str(root/'extensions'),'--extensionDevelopmentPath='+str(a.extension_dir.resolve()),'--extensionTestsPath='+str(ROOT/'scripts/vscode-presentation-keyboard.cjs'),'--skip-welcome','--skip-release-notes','--new-window']
 spec=importlib.util.spec_from_file_location('cdp',ROOT/'scripts/smoke-browser.py');native=importlib.util.module_from_spec(spec);spec.loader.exec_module(native)
 opener=urllib.request.build_opener(urllib.request.ProxyHandler({}));deadline=time.monotonic()+a.timeout;main=web=None;keys=0;w="document.querySelector('iframe').contentWindow"
 log=a.output.with_suffix('.host.log').open('w');process=subprocess.Popen(args,env=env,stdout=log,stderr=subprocess.STDOUT)
 try:
  while process.poll() is None and time.monotonic()<deadline:
   if not main and gate.exists():
    targets=json.load(opener.open(f'http://127.0.0.1:{port}/json/list'));target=next(t for t in targets if t['type']=='page');main=native.CDP(target['webSocketDebuggerUrl']);main.call('Runtime.enable');main.call('Page.enable');main.call('Emulation.setDeviceMetricsOverride',{'width':1440,'height':1000,'deviceScaleFactor':2,'mobile':False});main.call('Page.bringToFront');gate.write_text('start')
   if main and not web and ready.exists():
    for target in json.load(opener.open(f'http://127.0.0.1:{port}/json/list')):
     if target['type']!='iframe':continue
     c=native.CDP(target['webSocketDebuggerUrl']);c.call('Runtime.enable')
     try:
      if c.evaluate(f"!!document.querySelector('iframe')?.contentWindow?.keyboardReady"):
       web=c;web.evaluate(w+'.laymeshPresentation.root.focus()');break
     except RuntimeError:pass
   if web:
    status=web.evaluate(f"(()=>{{const w={w},v=w.laymeshPresentation;return {{count:w.keyboardMeasure?.rows.length||0,ended:v.ended,error:v.root.dataset.error,ready:v.ready,index:v.index,pages:v.deck.pages.length,running:v.clock.running,painting:v.painting,switching:v.switching,visits:w.animationMeasure?.visits.length||0}};}})()")
    finished=(status['visits']>=a.animation_rounds*status['pages'] and not any(status[k] for k in ['running','painting','switching'])) if a.animation_rounds else status['count']>=a.samples
    if finished:
     web.evaluate(f"(()=>{{const w={w},v=w.laymeshPresentation;w.navigationBridge.postMessage({{type:'navigation-test-result',result:{{...(w.animationMeasure||w.keyboardMeasure),backend:v.metrics.backend,resources:v.metrics.resources,resident:[...v.resident],dimensions:v.dimensions(),rendererFallback:v.rendererFallback,usesWorker:!!v.worker,usesMain:!!v.mainRuntime,memory:v.metrics.lastFrame,definition:'CDP trusted ArrowDown/ArrowRight at 80 ms cadence; ordinary animation steps, Home only at end; no seeking or all-page wait. State-only polling does not inspect layout.'}}}});}})()")
     web=None
     process.wait(timeout=90);break
    if status.get('error') or not status['ready']:raise RuntimeError(status)
    if a.animation_rounds and any(status[k] for k in ['running','painting','switching']):time.sleep(.08);continue
    key='Home' if status['ended'] or a.animation_rounds and status['index']==status['pages']-1 else 'ArrowDown' if keys%2 else 'ArrowRight';keys+=1
    code={'Home':36,'ArrowDown':40,'ArrowRight':39}[key]
    main.call('Input.dispatchKeyEvent',{'type':'keyDown','key':key,'code':key,'windowsVirtualKeyCode':code});main.call('Input.dispatchKeyEvent',{'type':'keyUp','key':key,'code':key,'windowsVirtualKeyCode':code});time.sleep(.08)
   else:time.sleep(.1)
  if process.poll() is None:raise TimeoutError('VS Code keyboard benchmark timeout')
  if process.returncode or not a.output.exists():raise RuntimeError('VS Code keyboard benchmark failed; see '+str(a.output.with_suffix('.host.log')))
 finally:
  if process.poll() is None:process.terminate();process.wait(timeout=15)
  log.close()
 result=json.loads(a.output.read_text())
 if a.animation_rounds:
  result['definition']='Actual isolated VS Code webview; trusted navigation keys after each authored animation finishes; no seeking or all-page wait. Render-reply intervals, not physical scanout.'
  result['animationRounds']=a.animation_rounds;a.output.write_text(json.dumps(result,indent=2)+'\n');print('Natural animation playback passed',len(result['visits']),'visits');raise SystemExit(0)
 assert all(r['trusted'] for r in result['rows']);assert len(result['rows'])>=a.samples
 result['rows']=result['rows'][:a.samples]
 assert all(isinstance(r['firstVisibleMs'],(int,float)) and __import__('math').isfinite(r['firstVisibleMs']) and r['firstVisibleMs']>0 for r in result['rows']),'Every accepted page change needs a visible-frame timestamp'
 def quantile(values,p):
  values=sorted(x for x in values if x is not None)
  if not values:return None
  return (values[(len(values)-1)//2]+values[len(values)//2])/2 if p==.5 else values[__import__('math').ceil(len(values)*p)-1]
 result['summary']={}
 for kind,rows in [('all',result['rows']),('firstTraversal',[r for r in result['rows'] if r['round']==0]),('repeat',[r for r in result['rows'] if r['round']>0]),('prepared',[r for r in result['rows'] if r['ready']])]:
  result['summary'][kind]={'samples':len(rows),**{key:{'median':quantile([r[key] for r in rows],.5),'p95':quantile([r[key] for r in rows],.95)} for key in ['firstVisibleMs','completedMs','transitionMs','queueWaitMs']}}
 a.output.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result['summary']),flush=True)
