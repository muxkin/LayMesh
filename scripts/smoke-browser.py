#!/usr/bin/env python3
"""Exercise the live WASM UI through Chromium CDP using Python stdlib only.

No Node runtime, Playwright driver, npm package, or user browser profile is used.
Start a static preview bound to a Tailscale address, then pass its URL here.
"""
from __future__ import annotations
import argparse,base64,hashlib,json,os,socket,struct,subprocess,shutil,signal,tempfile,time,urllib.request
from pathlib import Path
from urllib.parse import urlparse
ROOT=Path(__file__).resolve().parents[1]
def provenance(script):
 paths=[Path(script).resolve(),Path(__file__).resolve()]
 paths += [p for p in (ROOT/'site/dist/site/live').rglob('*') if p.is_file() and p.suffix in ('.js','.mjs','.wasm')]
 return {str(p.relative_to(ROOT)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(set(paths))}
class CDP:
 def __init__(self,url):
  parsed=urlparse(url);self.sock=socket.create_connection((parsed.hostname,parsed.port),timeout=30);self.sock.settimeout(60);key=base64.b64encode(os.urandom(16)).decode();self.sock.sendall(f'GET {parsed.path} HTTP/1.1\r\nHost: {parsed.hostname}:{parsed.port}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: {key}\r\nSec-WebSocket-Version: 13\r\n\r\n'.encode());header=b''
  while b'\r\n\r\n' not in header:header+=self.sock.recv(1)
  assert b'101' in header.split(b'\r\n')[0],header
  self.seq=0;self.events=[]
 def read(self,n):
  out=b''
  while len(out)<n:
   data=self.sock.recv(n-len(out))
   if not data:raise RuntimeError('Browser connection closed')
   out+=data
  return out
 def message(self):
  first,second=self.read(2);n=second&127
  if n==126:n=struct.unpack('!H',self.read(2))[0]
  elif n==127:n=struct.unpack('!Q',self.read(8))[0]
  mask=self.read(4) if second&128 else None;data=self.read(n)
  if mask:data=bytes(v^mask[i%4] for i,v in enumerate(data))
  if first&15==8:raise RuntimeError('Browser websocket closed')
  return json.loads(data)
 def call(self,method,params=None):
  self.seq+=1;ident=self.seq;data=json.dumps({'id':ident,'method':method,'params':params or {}}).encode();mask=os.urandom(4);n=len(data);header=bytes([129,128|n]) if n<126 else bytes([129,254])+struct.pack('!H',n) if n<65536 else bytes([129,255])+struct.pack('!Q',n);self.sock.sendall(header+mask+bytes(v^mask[i%4] for i,v in enumerate(data)))
  while True:
   msg=self.message()
   if msg.get('id')==ident:
    if 'error' in msg:raise RuntimeError(msg['error'])
    return msg.get('result',{})
   self.events.append(msg)
 def evaluate(self,expression):
  r=self.call('Runtime.evaluate',{'expression':expression,'awaitPromise':True,'returnByValue':True})
  if 'exceptionDetails' in r:raise RuntimeError(r['exceptionDetails'])
  return r['result'].get('value')
def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--url',required=True);p.add_argument('--all-examples',action='store_true');p.add_argument('--chrome',default='/usr/bin/google-chrome');p.add_argument('--output',type=Path,default=ROOT/'release/comparison/browser');a=p.parse_args();a.output.mkdir(parents=True,exist_ok=True)
 with tempfile.TemporaryDirectory(prefix='laymesh-chrome-',ignore_cleanup_errors=True) as profile:
  chrome=subprocess.Popen([a.chrome,'--headless=new','--no-sandbox','--disable-gpu','--no-proxy-server','--remote-debugging-port=0','--remote-allow-origins=*','--user-data-dir='+profile,'about:blank'],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,start_new_session=True)
  try:
   active=Path(profile)/'DevToolsActivePort';deadline=time.monotonic()+20
   while not active.exists():
    if time.monotonic()>deadline:raise RuntimeError('Chromium did not expose CDP')
    time.sleep(.1)
   port=int(active.read_text().splitlines()[0]);opener=urllib.request.build_opener(urllib.request.ProxyHandler({}));pages=json.load(opener.open(f'http://127.0.0.1:{port}/json'));cdp=CDP(next(p['webSocketDebuggerUrl'] for p in pages if p['type']=='page'));cdp.call('Page.enable');cdp.call('Runtime.enable');cdp.call('Log.enable');cdp.call('Emulation.setDeviceMetricsOverride',{'width':1440,'height':1000,'deviceScaleFactor':1,'mobile':False});cdp.call('Page.navigate',{'url':a.url})
   ready="""new Promise((resolve,reject)=>{const deadline=Date.now()+45000;const tick=()=>{const m=document.querySelector('.example-module');if(m?.dataset.liveState==='success')return resolve({title:document.title,url:location.href,status:m.querySelector('.live-status').textContent,diagnostics:m.querySelector('.live-diagnostics').textContent});if(m?.dataset.liveState==='error')return reject(Error(m.querySelector('.live-diagnostics').textContent));if(Date.now()>deadline)return reject(Error('Preview timeout: '+document.body.innerText.slice(-800)));setTimeout(tick,100)};tick()})"""
   first=cdp.evaluate(ready);cdp.evaluate("document.querySelector('[data-source-mode=source]').click()")
   state=cdp.evaluate("({mode:document.querySelector('.example-module').dataset.mode,editor:!!document.querySelector('.cm-editor'),image:document.querySelector('.live-image').src,warnings:document.querySelector('.live-diagnostics').textContent})");assert state['mode']=='source' and state['editor'] and state['image'].startswith('blob:'),state
   source='page=canvas(size=(80,50),background="#ffffff")\npage.add(text("Rust 中文 $E=mc^2$",font_size=12),offset=(4,8))\npage.add(rect(size=(25,12),fill="#245447"),offset=(4,25))'
   before_edit=cdp.evaluate("document.querySelector('.live-image').src")
   cdp.evaluate("document.querySelector('.cm-content').focus()")
   cdp.call('Input.dispatchKeyEvent',{'type':'keyDown','key':'a','code':'KeyA','modifiers':2,'windowsVirtualKeyCode':65});cdp.call('Input.dispatchKeyEvent',{'type':'keyUp','key':'a','code':'KeyA','modifiers':2,'windowsVirtualKeyCode':65});cdp.call('Input.insertText',{'text':source});edited=cdp.evaluate("new Promise((resolve,reject)=>{const previous="+json.dumps(before_edit)+";let n=0;const tick=()=>{const m=document.querySelector('.example-module'),image=m.querySelector('.live-image');if(m.dataset.liveState==='error')return reject(Error(m.querySelector('.live-diagnostics').textContent));if(m.dataset.liveState==='success'&&image.src!==previous&&image.complete)return resolve({title:document.title,url:location.href,status:m.querySelector('.live-status').textContent,diagnostics:m.querySelector('.live-diagnostics').textContent});if(++n>450)return reject(Error('Edited preview timeout'));setTimeout(tick,100)};tick()})");assert 'W_FONT' not in edited['diagnostics'],edited
   svg=cdp.evaluate("fetch(document.querySelector('.live-image').src).then(r=>r.text()).then(t=>({hasChinese:t.includes('中文'),hasEmbeddedFont:t.includes('data:font/')}))")
   assert svg['hasChinese'] and svg['hasEmbeddedFont'],svg
   cdp.call('Input.dispatchKeyEvent',{'type':'keyDown','key':'Escape','code':'Escape','windowsVirtualKeyCode':27});cdp.call('Input.dispatchKeyEvent',{'type':'keyUp','key':'Escape','code':'Escape','windowsVirtualKeyCode':27});shot=cdp.call('Page.captureScreenshot',{'format':'png','captureBeyondViewport':False});(a.output/'desktop.png').write_bytes(base64.b64decode(shot['data']))
   # User-font route: programmatically select the dedicated test fixture through
   # the browser file input. It is never packaged in the site runtime.
   before_font=cdp.evaluate("document.querySelector('.live-image').src");cdp.call('DOM.enable');doc=cdp.call('DOM.getDocument');node=cdp.call('DOM.querySelector',{'nodeId':doc['root']['nodeId'],'selector':'.live-font-file'});cdp.call('DOM.setFileInputFiles',{'nodeId':node['nodeId'],'files':[str(ROOT/'tests/assets/GFSNeohellenic.otf')]})
   fonts=cdp.evaluate("new Promise((resolve,reject)=>{let n=0;const tick=()=>{const t=document.querySelector('.font-note').textContent;if(t.includes('GFSNeohellenic'))return resolve(t);if(++n>100)return reject(Error(t));setTimeout(tick,100)};tick()})")
   font_render=cdp.evaluate("new Promise((resolve,reject)=>{const previous="+json.dumps(before_font)+";let n=0;const tick=()=>{const m=document.querySelector('.example-module');if(m.dataset.liveState==='success'&&m.querySelector('.live-image').src!==previous)return resolve(m.querySelector('.live-diagnostics').textContent);if(++n>200)return reject(Error('Font rerender timeout'));setTimeout(tick,100)};tick()})")
   cdp.call('Emulation.setDeviceMetricsOverride',{'width':390,'height':844,'deviceScaleFactor':1,'mobile':True});cdp.evaluate("document.querySelector('.example-module').scrollIntoView()")
   shot=cdp.call('Page.captureScreenshot',{'format':'png','captureBeyondViewport':False});(a.output/'mobile.png').write_bytes(base64.b64decode(shot['data']))
   corpus=[]
   if a.all_examples:
    entries=cdp.evaluate("""(async()=>{
      const base=new URL('live/',document.querySelector('script[src$="app.js"]').src);
      const manifest=await(await fetch(new URL('manifest.json',base))).json();
      let worker,sequence=10000;const pending=new Map();
      async function connect(){
        worker=new Worker(new URL('worker.js',base),{type:'module'});
        await new Promise((resolve,reject)=>{
          const timeout=setTimeout(()=>reject(Error('Corpus worker initialization timeout')),20000);
          worker.onmessage=({data})=>{
            if(data.type==='ready'){clearTimeout(timeout);resolve()}
            else if(data.type==='result'||data.type==='error'){pending.get(data.id)?.(data);pending.delete(data.id)}
          };
          worker.onerror=error=>reject(Error(error.message));
          worker.postMessage({type:'init',base:base.href});
        });
      }
      await connect();
      window.__laymeshCorpus=async names=>{
        const results=[];
        for(const name of names){
          if(!worker)await connect();
          const example=manifest.examples[name],files={};
          for(const file of example.files)files[file]=await(await fetch(new URL(manifest.resources['/'+file],base))).text();
          const id=++sequence,start=performance.now();
          const response=await new Promise(resolve=>{
            const timeout=setTimeout(()=>{worker.terminate();worker=null;pending.delete(id);resolve({type:'error',error:{code:'TIMEOUT',message:'Render exceeded 20 seconds'}})},20000);
            pending.set(id,data=>{clearTimeout(timeout);resolve(data)});
            worker.postMessage({type:'render',id,entry:example.entry,files});
          });
          results.push(response.type==='error'?{example:name,elapsed_ms:performance.now()-start,error:response.error}:{example:name,elapsed_ms:performance.now()-start,width:response.inspection.page.width,height:response.inspection.page.height,warnings:response.warnings.length});
        }
        return results;
      };
      window.__laymeshCorpusStop=()=>worker?.terminate();
      return Object.keys(manifest.examples);
    })()""")
    for start in range(0,len(entries),2):
     corpus.extend(cdp.evaluate('window.__laymeshCorpus('+json.dumps(entries[start:start+2])+')'));(a.output/'wasm-corpus.json').write_text(json.dumps(corpus,ensure_ascii=False,indent=2)+'\n');print(f'WASM corpus: {min(start+2,len(entries))}/{len(entries)}',flush=True)
    cdp.evaluate('window.__laymeshCorpusStop()')
   errors=[e for e in cdp.events if e.get('method')=='Runtime.exceptionThrown' or e.get('method')=='Log.entryAdded' and e['params']['entry']['level']=='error'];assert not errors,errors
   result={'browser':'Chromium via Python stdlib CDP','artifact_sha256':provenance(__file__),'url':a.url,'first_render':first,'editor':state,'edited_render':edited,'font_file_input':fonts,'font_render_warnings':font_render,'desktop':[1440,1000],'mobile':[390,844],'console_errors':errors,'node_runtime':False,'node_on_path':bool(shutil.which('node')),'npm_on_path':bool(shutil.which('npm')),'wasm_examples':corpus};(a.output/'verification.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n');print(json.dumps(result,ensure_ascii=False))
   assert all('error' not in item for item in corpus),[item for item in corpus if 'error' in item]
  finally:
   try:
    if os.name=='posix':os.killpg(chrome.pid,signal.SIGTERM)
    else:chrome.terminate()
   except ProcessLookupError:pass
   try:chrome.wait(timeout=5)
   except subprocess.TimeoutExpired:
    if os.name=='posix':os.killpg(chrome.pid,signal.SIGKILL)
    else:chrome.kill()
    chrome.wait()
if __name__=='__main__':main()
