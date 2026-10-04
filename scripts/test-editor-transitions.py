#!/usr/bin/env python3
"""Regression test real editor transitions and on-demand Worker resource loading.

Uses the existing isolated Chromium/CDP runner; keeps original audit outputs intact.
"""
import argparse,base64,importlib.util,json,os,signal,subprocess,tempfile,time,urllib.request
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('browser_smoke',ROOT/'scripts/smoke-browser.py');browser=importlib.util.module_from_spec(spec);spec.loader.exec_module(browser)
p=argparse.ArgumentParser();p.add_argument('--url',required=True);p.add_argument('--chrome',default='/usr/bin/google-chrome');p.add_argument('--output',type=Path,default=ROOT/'release/comparison/revision-editor');a=p.parse_args();a.output.mkdir(parents=True,exist_ok=True)
with tempfile.TemporaryDirectory(prefix='laymesh-editor-regression-',ignore_cleanup_errors=True) as profile:
 chrome=subprocess.Popen([a.chrome,'--headless=new','--no-sandbox','--disable-gpu','--no-proxy-server','--remote-debugging-port=0','--remote-allow-origins=*','--user-data-dir='+profile,'about:blank'],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,start_new_session=True)
 try:
  active=Path(profile)/'DevToolsActivePort';deadline=time.monotonic()+25
  while not active.exists():
   if time.monotonic()>deadline:raise RuntimeError('Chrome startup timed out')
   time.sleep(.1)
  port=int(active.read_text().splitlines()[0]);opener=urllib.request.build_opener(urllib.request.ProxyHandler({}));pages=json.load(opener.open(f'http://127.0.0.1:{port}/json'));cdp=browser.CDP(next(p['webSocketDebuggerUrl'] for p in pages if p['type']=='page'));cdp.call('Page.enable');cdp.call('Runtime.enable');cdp.call('Log.enable');cdp.call('Emulation.setDeviceMetricsOverride',{'width':1440,'height':1000,'deviceScaleFactor':1,'mobile':False});cdp.call('Page.navigate',{'url':a.url})
  def wait(previous=None,error=False):
   return cdp.evaluate('''new Promise((resolve,reject)=>{const previous='''+json.dumps(previous)+''',wantError='''+json.dumps(error)+''',deadline=Date.now()+25000;function tick(){const m=document.querySelector('.example-module');if(m){const state=m.dataset.liveState,url=m.querySelector('.live-image').src;if(wantError&&state==='error')return resolve({state,url,message:m.querySelector('.live-diagnostics').textContent});if(!wantError&&state==='success'&&(!previous||url!==previous))return resolve({state,url,metrics:m.querySelector('.live-metrics').textContent});if(!wantError&&state==='error')return reject(Error(m.querySelector('.live-diagnostics').textContent));}if(Date.now()>deadline)return reject(Error('Editor transition timeout'));setTimeout(tick,40)}tick()})''')
  wait();cdp.evaluate("document.querySelector('[data-source-mode=source]').click()")
  original=cdp.evaluate("document.querySelector('.source-panel:not([hidden]) .code-raw').defaultValue")
  def image_kind():return cdp.evaluate(r"(async()=>{const svg=await(await fetch(document.querySelector('.live-image').src)).text();const href=svg.match(/data:image\/[^\"']+/)?.[0]||'';let hash=0;for(let i=0;i<href.length;i++)hash=(Math.imul(hash,31)+href.charCodeAt(i))|0;return {png:svg.includes('data:image/png'),fingerprint:hash.toString(16)};})()")
  def key(name,code,modifiers=2):
   for typ in ['keyDown','keyUp']:cdp.call('Input.dispatchKeyEvent',{'type':typ,'key':name,'code':'Key'+name.upper(),'modifiers':modifiers,'windowsVirtualKeyCode':code})
  def replace(source,error=False):
   previous=cdp.evaluate("document.querySelector('.live-image').src");cdp.evaluate("document.querySelector('.cm-content').focus()");key('a',65);cdp.call('Input.insertText',{'text':source});return wait(previous,error)
  states=[{'action':'original','image':image_kind()}];assert states[-1]['image']['png']
  jpeg=original.replace('../assets/sample.png','../assets/sample.jpg');states.append({'action':'new registered JPEG reference',**replace(jpeg),'image':image_kind()});assert states[-1]['image']['fingerprint']!=states[0]['image']['fingerprint']
  previous=states[-1]['url'];key('z',90);states.append({'action':'undo resource edit',**wait(previous),'image':image_kind()});assert states[-1]['image']['fingerprint']==states[0]['image']['fingerprint']
  previous=states[-1]['url'];key('z',90,10);states.append({'action':'redo resource edit',**wait(previous),'image':image_kind()});assert states[-1]['image']['fingerprint']!=states[0]['image']['fingerprint']
  previous=states[-1]['url'];states.append({'action':'unregistered resource error',**replace(jpeg.replace('sample.jpg','not-registered.jpg'),True)});assert states[-1]['url']==previous;assert 'E_ASSET' in states[-1]['message']
  key('z',90);states.append({'action':'undo error and rerender',**wait(previous),'image':image_kind()});assert states[-1]['image']['fingerprint']!=states[0]['image']['fingerprint']
  previous=states[-1]['url'];cdp.evaluate("document.querySelector('.live-reset').click()");states.append({'action':'reset files and history',**wait(previous),'image':image_kind()});assert states[-1]['image']['fingerprint']==states[0]['image']['fingerprint'];assert cdp.evaluate("document.querySelector('.example-module').dataset.modified")=='false'
  # Exercise actual shared Worker API, with imports outside this example's
  # initial dependency list and an unsaved module overriding the manifest file.
  worker=cdp.evaluate('''(async()=>{
   const base=new URL('live/',document.querySelector('script[src$="app.js"]').src),worker=new Worker(new URL('worker.js',base),{type:'module'});let serial=0;const pending=new Map();
   await new Promise((resolve,reject)=>{worker.onerror=e=>reject(Error(e.message));worker.onmessage=({data})=>{if(data.type==='ready')resolve();else if(data.type==='result'||data.type==='error'){pending.get(data.id)?.(data);pending.delete(data.id)}};worker.postMessage({type:'init',base:base.href})});
   const run=files=>new Promise((resolve,reject)=>{const id=++serial,t=setTimeout(()=>reject(Error('Worker transition timeout')),20000);pending.set(id,d=>{clearTimeout(t);resolve(d)});worker.postMessage({type:'run',id,entry:'examples/gallery/images/png.lay',files})});
   const source='import {card} from "../containers/card-component.lay"\\npage=canvas(size=(120,80),stylesheet="../../language/paper.lcss")\\npage.add(card("Imported"))';
   const result=await run({'examples/gallery/images/png.lay':source});
   const edited=await run({'examples/gallery/images/png.lay':source,'examples/gallery/containers/card-component.lay':'export function card(label) {return rect(size=(20,10),fill="#e31234")}'});
   const cycleSource='import {card} from "../containers/card-component.lay"\\npage=canvas(size=(120,80))\\npage.add(card("Cycle"))';
   const cycle=await run({'examples/gallery/images/png.lay':cycleSource,'examples/gallery/containers/card-component.lay':'import {card} from "../containers/card-component.lay"\\nexport alias=card'});
   const recovered=await run({'examples/gallery/images/png.lay':source});worker.terminate();
   return {imported:{type:result.type,error:result.error,metrics:result.metrics},edited:{type:edited.type,error:edited.error,hasUnsavedColor:edited.svg?.includes('#e31234')},cycle:{type:cycle.type,error:cycle.error},recovered:{type:recovered.type,error:recovered.error}};
  })()''')
  assert worker['imported']['type']=='result',worker;assert worker['edited']['type']=='result' and worker['edited']['hasUnsavedColor'],worker;assert worker['cycle']['type']=='error' and worker['cycle']['error']['code']=='E_IMPORT',worker;assert worker['recovered']['type']=='result',worker
  metrics=worker['imported']['metrics'];assert metrics['compile']>0 and metrics['svg']>0 and metrics['resources']>0,metrics;assert abs(metrics['compile']+metrics['svg']-metrics['total'])<.0001
  errors=[e for e in cdp.events if e.get('method')=='Runtime.exceptionThrown' or e.get('method')=='Log.entryAdded' and e['params']['entry']['level']=='error'];assert not errors,errors
  cdp.evaluate("document.querySelector('.example-module').scrollIntoView({block:'center'})")
  screenshot=cdp.call('Page.captureScreenshot',{'format':'png'});(a.output/'editor-transitions.png').write_bytes(base64.b64decode(screenshot['data']))
  result={'status':'passed','artifact_sha256':browser.provenance(__file__),'states':states,'worker':worker,'console_errors':errors,'browser':'isolated Chromium via Python CDP'};(a.output/'verification.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n');print(json.dumps(result,ensure_ascii=False))
 finally:
  try:os.killpg(chrome.pid,signal.SIGTERM)
  except ProcessLookupError:pass
  try:chrome.wait(timeout=5)
  except subprocess.TimeoutExpired:os.killpg(chrome.pid,signal.SIGKILL);chrome.wait()
