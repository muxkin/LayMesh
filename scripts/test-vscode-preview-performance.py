#!/usr/bin/env python3
"""Measure native request-to-visible preview performance in isolated VS Code."""
import argparse, json, os, subprocess, tempfile, socket, time, urllib.request, importlib.util
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--output',required=True,type=Path);p.add_argument('--binary',type=Path);p.add_argument('--source',type=Path,help='Measure a real layout using read-only links to its sibling assets');p.add_argument('--replace',nargs=2,metavar=('OLD','NEW'),help='Change actual drawing content on each edit, using {i} for the edit number');p.add_argument('--edits',type=int,default=6,help='Number of genuine edits, including one warmup');p.add_argument('--motion',action='store_true',help='Measure pan, zoom and cursor frame intervals before source edits');p.add_argument('--extension-dir',type=Path,default=ROOT/'extensions/vscode');p.add_argument('--code',default='/usr/share/code/code');p.add_argument('--test-script',type=Path,default=ROOT/'scripts/vscode-preview-performance.cjs');p.add_argument('--timeout',type=float,default=100);a=p.parse_args()
# Periodic CDP DOM inspection can force layout and perturb frame timings.
# Enable it only for a separate diagnostic run, never for acceptance timings.
debug_probe=os.environ.get('LAYMESH_PERFORMANCE_DEBUG')=='1'
if a.edits<6:p.error('Use at least six edits')
a.output=a.output.resolve();a.output.parent.mkdir(parents=True,exist_ok=True)
with tempfile.TemporaryDirectory(prefix='laymesh-perf-host-',ignore_cleanup_errors=True) as temp:
 root=Path(temp);user=root/'profile/User';user.mkdir(parents=True)
 settings={'window.autoDetectColorScheme':False,'files.autoSave':'off'}
 if a.binary:settings['laymesh.executable']=str(a.binary.resolve())
 (user/'settings.json').write_text(json.dumps(settings))
 env={**os.environ,'LAYMESH_PERFORMANCE_OUTPUT':str(a.output),'LAYMESH_PERFORMANCE_BINARY':str(a.binary or 'bundled')};env.pop('ELECTRON_RUN_AS_NODE',None)
 gate=root/'viewport-gate';env['LAYMESH_PERFORMANCE_GATE']=str(gate)
 if a.source:env['LAYMESH_PERFORMANCE_SOURCE']=str(a.source.resolve())
 if a.replace:env['LAYMESH_PERFORMANCE_REPLACE']=json.dumps(a.replace)
 env['LAYMESH_PERFORMANCE_EDITS']=str(a.edits)
 if a.motion:env['LAYMESH_PERFORMANCE_MOTION']='1'
 with socket.socket() as sock:sock.bind(('127.0.0.1',0));port=sock.getsockname()[1]
 command=[a.code,'--remote-debugging-port='+str(port),'--remote-allow-origins=*','--no-sandbox','--disable-gpu','--disable-updates','--disable-background-timer-throttling','--disable-renderer-backgrounding','--disable-backgrounding-occluded-windows','--ozone-platform=headless','--force-device-scale-factor=2','--user-data-dir='+str(root/'profile'),'--extensions-dir='+str(root/'extensions'),'--extensionDevelopmentPath='+str(a.extension_dir.resolve()),'--extensionTestsPath='+str(a.test_script.resolve()),'--skip-welcome','--skip-release-notes','--new-window']
 # Electron can emit more diagnostics than a pipe holds. A pipe that is only
 # drained after exit can stall the host in the middle of the measurements.
 log=(root/'host.log').open('w')
 process=subprocess.Popen(command,env=env,stdout=log,stderr=subprocess.STDOUT,text=True)
 spec=importlib.util.spec_from_file_location('native_cdp',ROOT/'scripts/smoke-browser.py');native=importlib.util.module_from_spec(spec);spec.loader.exec_module(native)
 opener=urllib.request.build_opener(urllib.request.ProxyHandler({}));connected=False;debug={}
 deadline=time.monotonic()+a.timeout
 debugged=False;motion=None
 while process.poll() is None and time.monotonic()<deadline:
  if not connected and gate.exists():
   try:
    targets=json.load(opener.open(f'http://127.0.0.1:{port}/json/list'))
    target=next(t for t in targets if t['type']=='page');c=native.CDP(target['webSocketDebuggerUrl']);c.call('Runtime.enable');c.call('Page.enable');c.call('Emulation.setDeviceMetricsOverride',{'width':1440,'height':1000,'deviceScaleFactor':2,'mobile':False});c.call('Page.bringToFront');gate.write_text('start');connected=True
   except (OSError,StopIteration):pass
  elif connected and a.motion and gate.read_text()=='motion':
   from preview_frame_probe import MOTION
   targets=json.load(opener.open(f'http://127.0.0.1:{port}/json/list'))
   for target in targets:
    if target['type']=='iframe':
     probe=native.CDP(target['webSocketDebuggerUrl']);probe.call('Runtime.enable')
     if probe.evaluate("!!document.querySelector('iframe')?.contentDocument?.querySelector('.preview')"):
      motion=probe.evaluate(MOTION.replace('DOCUMENT',"document.querySelector('iframe').contentDocument").replace('SITE','false'));gate.write_text('motion-done');break
  elif connected and debug_probe and not debugged and time.monotonic()>deadline-a.timeout+20:
   try:
    targets=json.load(opener.open(f'http://127.0.0.1:{port}/json/list'))
    for target in targets:
     if target['type']=='iframe':
      c=native.CDP(target['webSocketDebuggerUrl']);c.call('Runtime.enable');debug=c.evaluate("(()=>{const d=document.querySelector('iframe')?.contentDocument;if(!d?.querySelector('.preview'))return;d.defaultView.__perfMessages=[];d.defaultView.addEventListener('message',e=>{d.defaultView.__perfMessages.push({type:e.data?.type,generation:e.data?.generation,status:e.data?.status,at:Date.now()})});return {state:d.querySelector('.preview').dataset.state}})()")
      if debug:debugged=True;break
   except (OSError,RuntimeError):pass
  elif debug_probe and debugged:
   try:debug=c.evaluate("(()=>{const d=document.querySelector('iframe')?.contentDocument;return d&&{text:d.body.innerText,generation:d.querySelector('.preview')?.dataset.generation,painted:d.querySelector('.preview')?.dataset.painted,state:d.querySelector('.preview')?.dataset.state,messages:d.defaultView.__perfMessages?.slice(-10)}})()")
   except (OSError,RuntimeError):pass
  time.sleep(.5)
 if process.poll() is None:process.terminate()
 process.wait(timeout=15);log.close();output=(root/'host.log').read_text()
 if process.returncode or not a.output.exists():raise RuntimeError(json.dumps(debug,ensure_ascii=False)+'\n'+output[-3000:])
 if a.motion:
  assert motion and motion['final']['painted'] and motion['final']['visible'],motion
  assert motion['final']['viewport_covered'] and all(abs(v-motion['dpr'])<.015 for v in motion['final']['pixel_ratio']),motion['final']
  assert all(row['uncovered_layout_frames']==0 for row in motion['rows']),motion
  assert motion['final']['overview_clip_supported'] and not motion['final']['overview_overlaps_front'] and motion['final']['overview_pixels']<=500000,motion['final']
  result=json.loads(a.output.read_text());result['motion']=motion;a.output.write_text(json.dumps(result,indent=2)+'\n')
 result=json.loads(a.output.read_text());result['periodic_cdp_diagnostics']=debug_probe
 a.output.write_text(json.dumps(result,indent=2)+'\n')
 print(a.output.read_text())
