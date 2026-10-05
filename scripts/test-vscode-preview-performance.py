#!/usr/bin/env python3
"""Measure native request-to-visible preview performance in isolated VS Code."""
import argparse, json, os, subprocess, tempfile, socket, time, urllib.request, importlib.util
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--output',required=True,type=Path);p.add_argument('--binary',type=Path);p.add_argument('--code',default='/usr/share/code/code');a=p.parse_args()
a.output=a.output.resolve();a.output.parent.mkdir(parents=True,exist_ok=True)
with tempfile.TemporaryDirectory(prefix='laymesh-perf-host-') as temp:
 root=Path(temp);user=root/'profile/User';user.mkdir(parents=True)
 settings={'window.autoDetectColorScheme':False}
 if a.binary:settings['laymesh.executable']=str(a.binary.resolve())
 (user/'settings.json').write_text(json.dumps(settings))
 env={**os.environ,'LAYMESH_PERFORMANCE_OUTPUT':str(a.output),'LAYMESH_PERFORMANCE_BINARY':str(a.binary or 'bundled')};env.pop('ELECTRON_RUN_AS_NODE',None)
 with socket.socket() as sock:sock.bind(('127.0.0.1',0));port=sock.getsockname()[1]
 command=[a.code,'--remote-debugging-port='+str(port),'--remote-allow-origins=*','--no-sandbox','--disable-gpu','--disable-updates','--ozone-platform=headless','--force-device-scale-factor=2','--user-data-dir='+str(root/'profile'),'--extensions-dir='+str(root/'extensions'),'--extensionDevelopmentPath='+str(ROOT/'extensions/vscode'),'--extensionTestsPath='+str(ROOT/'scripts/vscode-preview-performance.cjs'),'--skip-welcome','--skip-release-notes','--new-window']
 process=subprocess.Popen(command,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
 spec=importlib.util.spec_from_file_location('native_cdp',ROOT/'scripts/smoke-browser.py');native=importlib.util.module_from_spec(spec);spec.loader.exec_module(native)
 opener=urllib.request.build_opener(urllib.request.ProxyHandler({}));connected=False;debug={}
 deadline=time.monotonic()+100
 debugged=False
 while process.poll() is None and time.monotonic()<deadline:
  if not connected:
   try:
    targets=json.load(opener.open(f'http://127.0.0.1:{port}/json/list'))
    target=next(t for t in targets if t['type']=='page');c=native.CDP(target['webSocketDebuggerUrl']);c.call('Runtime.enable');c.call('Page.enable');c.call('Emulation.setDeviceMetricsOverride',{'width':1440,'height':1000,'deviceScaleFactor':2,'mobile':False});c.call('Page.bringToFront');connected=True
   except (OSError,StopIteration):pass
  elif not debugged and time.monotonic()>deadline-88:
   try:
    targets=json.load(opener.open(f'http://127.0.0.1:{port}/json/list'))
    for target in targets:
     if target['type']=='iframe':
      c=native.CDP(target['webSocketDebuggerUrl']);c.call('Runtime.enable');debug=c.evaluate("(()=>{const d=document.querySelector('iframe')?.contentDocument;return d&&{text:d.body.innerText,painted:d.querySelector('.preview')?.dataset.painted,state:d.querySelector('.preview')?.dataset.state,images:[...d.querySelectorAll('image')].map(n=>n.getAttribute('href'))}})()")
      if debug:debugged=True;break
   except (OSError,RuntimeError):pass
  time.sleep(.5)
 if process.poll() is None:process.terminate()
 output=process.communicate(timeout=15)[0]
 if process.returncode or not a.output.exists():raise RuntimeError(json.dumps(debug,ensure_ascii=False)+'\n'+output[-3000:])
 print(a.output.read_text())
