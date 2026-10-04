#!/usr/bin/env python3
"""Exercise the actual native VS Code color webview with trusted pointer input."""
import argparse,base64,importlib.util,json,os,signal,socket,subprocess,tempfile,time,urllib.request
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('browser_smoke',ROOT/'scripts/smoke-browser.py');browser=importlib.util.module_from_spec(spec);spec.loader.exec_module(browser)
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--output',type=Path,required=True);p.add_argument('--code',default='/usr/share/code/code');a=p.parse_args();a.output.parent.mkdir(parents=True,exist_ok=True)
with tempfile.TemporaryDirectory(prefix='laymesh-native-picker-',ignore_cleanup_errors=True) as temp:
 profile=Path(temp);gate=profile/'gate.json';evidence=profile/'evidence.json'
 with socket.socket() as sock:sock.bind(('127.0.0.1',0));port=sock.getsockname()[1]
 env={**os.environ,'LAYMESH_PICKER_GATE':str(gate),'LAYMESH_PICKER_EVIDENCE':str(evidence)};env.pop('ELECTRON_RUN_AS_NODE',None)
 command=[a.code,'--no-sandbox','--disable-gpu','--disable-updates','--ozone-platform=headless','--remote-debugging-port='+str(port),'--remote-allow-origins=*','--user-data-dir='+str(profile/'profile'),'--extensions-dir='+str(profile/'extensions'),'--extensionDevelopmentPath='+str(ROOT/'extensions/vscode'),'--extensionTestsPath='+str(ROOT/'scripts/vscode-picker.cjs'),'--skip-welcome','--skip-release-notes','--new-window']
 log=(profile/'host.log').open('w');host=subprocess.Popen(command,env=env,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
 try:
  opener=urllib.request.build_opener(urllib.request.ProxyHandler({}))
  def targets():return json.load(opener.open(f'http://127.0.0.1:{port}/json/list'))
  def until(fn,description):
   deadline=time.monotonic()+30
   while time.monotonic()<deadline:
    try:
     value=fn()
     if value:return value
    except (OSError,ValueError,StopIteration):pass
    time.sleep(.1)
   raise RuntimeError(description+'\n'+(profile/'host.log').read_text()[-2000:])
  until(lambda:gate.exists(),'Native fixture startup')
  root=browser.CDP(next(t['webSocketDebuggerUrl'] for t in targets() if t['type']=='page'));root.call('Runtime.enable');root.call('Page.enable');root.call('Emulation.setDeviceMetricsOverride',{'width':1440,'height':1000,'deviceScaleFactor':1,'mobile':False})
  def attach():
   for t in targets():
    if t['type']=='iframe':
     c=browser.CDP(t['webSocketDebuggerUrl']);c.call('Runtime.enable')
     if c.evaluate("!!document.querySelector('iframe')?.contentDocument?.querySelector('.laymesh-color-panel')"):return c
  c=until(attach,'Native color iframe')
  def evaluate(expression):return c.evaluate("(()=>{const d=document.querySelector('iframe').contentDocument;return ("+expression+");})()")
  def wait(expression):return until(lambda:evaluate(expression),expression)
  def phase(value):gate.write_text(json.dumps({'phase':value}))
  def wait_phase(value):return until(lambda:json.loads(gate.read_text()).get('phase')==value,'Gate '+value)
  wait("!d.querySelector('.laymesh-color-apply').disabled")
  frame=root.evaluate("document.querySelector('iframe').getBoundingClientRect().toJSON()")
  plane=evaluate(r"""(()=>{const p=d.querySelector('.laymesh-color-sv'),thumb=p.querySelector('.laymesh-color-thumb'),r=p.getBoundingClientRect(),fields=d.querySelector('.laymesh-color-channel').parentNode,w=d.defaultView;w.pickerMetrics={latencies:[],frames:[],rebuilds:0};w.originalInputs=[...fields.querySelectorAll('input')];let target,last=0;new w.MutationObserver(ms=>w.pickerMetrics.rebuilds+=ms.filter(m=>m.type==='childList').length).observe(fields,{childList:true});p.addEventListener('pointermove',e=>{target={x:e.clientX-r.left,y:e.clientY-r.top,at:w.performance.now(),done:false}},{capture:true});new w.MutationObserver(()=>{if(target&&!target.done&&Math.hypot(parseFloat(thumb.style.left)/100*r.width-target.x,parseFloat(thumb.style.top)/100*r.height-target.y)<1){target.done=true;w.pickerMetrics.latencies.push(w.performance.now()-target.at)}}).observe(thumb,{attributes:true,attributeFilter:['style']});function tick(now){if(last)w.pickerMetrics.frames.push(now-last);last=now;if(!w.pickerStopped)w.requestAnimationFrame(tick)}w.requestAnimationFrame(tick);return r.toJSON()})()""")
  def point(fx,fy):return {'x':frame['x']+plane['x']+plane['width']*fx,'y':frame['y']+plane['y']+plane['height']*fy}
  root.call('Input.dispatchMouseEvent',{'type':'mousePressed','button':'left','clickCount':1,**point(.15,.2)})
  start=time.monotonic()
  for i in range(120):
   last=point(.15+.7*(i%60)/59,.2+.6*((i//60)^1));root.call('Input.dispatchMouseEvent',{'type':'mouseMoved','button':'left','buttons':1,**last})
   delay=start+(i+1)/60-time.monotonic()
   if delay>0:time.sleep(delay)
  root.call('Input.dispatchMouseEvent',{'type':'mouseReleased','button':'left','clickCount':1,**last})
  metrics=evaluate("(()=>{const w=d.defaultView;w.pickerStopped=true;const p=(vs)=>[...vs].sort((a,b)=>a-b)[Math.floor(vs.length*.95)]??null;return {latency_samples:w.pickerMetrics.latencies.length,latency_p95_ms:p(w.pickerMetrics.latencies),frame_p95_ms:p(w.pickerMetrics.frames),input_rebuilds:w.pickerMetrics.rebuilds,stable_inputs:w.originalInputs.every(n=>n.isConnected)}})()")
  assert metrics['latency_samples']>=100 and metrics['latency_p95_ms']<=34 and metrics['input_rebuilds']==0 and metrics['stable_inputs'],metrics
  # Continuous real input on every RGB channel/alpha slider and the hue strip.
  for rect in evaluate("[...d.querySelectorAll('input[type=range]')].map(n=>n.getBoundingClientRect().toJSON())"):
   def at(f):return {'x':frame['x']+rect['x']+rect['width']*f,'y':frame['y']+rect['y']+rect['height']/2}
   root.call('Input.dispatchMouseEvent',{'type':'mousePressed','button':'left','clickCount':1,**at(.2)})
   for i in range(15):root.call('Input.dispatchMouseEvent',{'type':'mouseMoved','button':'left','buttons':1,**at(.2+.6*i/14)})
   root.call('Input.dispatchMouseEvent',{'type':'mouseReleased','button':'left','clickCount':1,**at(.8)})
  wait("!d.querySelector('.laymesh-color-apply').disabled")
  evaluate("(()=>{d.querySelector('[data-space=hsv]').click();const alpha=d.querySelector('[data-channel=alpha]');alpha.value='40';alpha.dispatchEvent(new d.defaultView.Event('input',{bubbles:true}));d.querySelector('.laymesh-color-output select').value='hsv';return true})()")
  wait("!d.querySelector('.laymesh-color-apply').disabled")
  phase('check-preview');wait_phase('preview-verified');evaluate("(()=>{d.querySelector('.laymesh-color-apply').click();return true})()")
  time.sleep(.3);phase('applied');wait_phase('reopened');c=until(attach,'Reopened panel');wait("!d.querySelector('.laymesh-color-apply').disabled")
  phase('concurrent');wait_phase('stale-ready');evaluate("(()=>{d.querySelector('.laymesh-color-apply').click();return true})()")
  wait("d.querySelector('.laymesh-color-status').textContent.includes('Document changed')");evaluate("(()=>{d.querySelector('.laymesh-color-actions button').click();return true})()")
  phase('cancelled');wait_phase('complete');host.wait(timeout=20);assert host.returncode==0
  result=json.loads(evidence.read_text());result['drag_metrics']=metrics;result['artifact_sha256']=browser.provenance(__file__);a.output.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result))
 finally:
  if host.poll() is None:os.killpg(host.pid,signal.SIGTERM);host.wait(timeout=10)
  log.close()
