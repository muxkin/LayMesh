#!/usr/bin/env python3
"""Measure stalled font initialization and bounded viewport paint requests.

Uses an isolated Chromium profile and the repository CDP harness. The injected
worker shim models a stalled CDN and holds paint replies during a zoom burst;
the real bundled fonts, compiler, renderer and Canvas path still run.
"""
import argparse, base64, importlib.util, json, os, signal, subprocess, tempfile, time, urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('preview_cdp', ROOT/'scripts/smoke-browser.py')
browser = importlib.util.module_from_spec(spec); spec.loader.exec_module(browser)

SHIM = r'''
window.__paintRequests=[];window.__held=[];window.__holdPaint=false;
window.__fontSources=[];window.__workerReplies=[];
window.__holdResults=false;window.__heldResults=[];
const OriginalWorker=Worker;
window.Worker=class extends OriginalWorker{
 constructor(url,options){
  const script=`const queued=[];const queue=e=>{e.stopImmediatePropagation();queued.push(e);};addEventListener('message',queue);const originalFetch=fetch;globalThis.fetch=(url,options)=>String(url).includes('cdn.jsdelivr.net')?new Promise((resolve,reject)=>{const signal=options?.signal;if(signal?.aborted)reject(new DOMException('Aborted','AbortError'));else signal?.addEventListener('abort',()=>reject(new DOMException('Aborted','AbortError')),{once:true});}):originalFetch(url,options);await import(${JSON.stringify(String(url))});removeEventListener('message',queue);for(const event of queued)onmessage(event);`;
  const blob=URL.createObjectURL(new Blob([script],{type:'text/javascript'}));super(blob,options);
  this.addEventListener('message',event=>{
   const data=event.data;
   if(data.type==='bundled-font')window.__fontSources.push(data);
   if(data.type==='result'&&window.__holdResults){window.__heldResults.push(()=>this.__handler?.(event));return;}
   if(['painted','paint-error','paint-skipped'].includes(data.type)){
    window.__workerReplies.push({type:data.type,id:data.id,generation:data.generation});
    if(window.__holdPaint){window.__held.push(()=>this.__handler?.(event));return;}
   }
   this.__handler?.(event);
  });
 }
 set onmessage(handler){this.__handler=handler;}
 get onmessage(){return this.__handler;}
 postMessage(data,...rest){if(data.type==='paint')window.__paintRequests.push({...data});super.postMessage(data,...rest);}
};
'''
READY = r'''new Promise((resolve,reject)=>{const end=Date.now()+45000;function tick(){
 const m=document.querySelector('.example-module');if(m?.dataset.liveState==='error')return reject(Error(m.querySelector('.live-diagnostics').textContent));
 if(m?.dataset.liveState==='success'&&m.dataset.painted==='true')return resolve({generation:m.dataset.generation,metrics:m.querySelector('.live-metrics').textContent});
 if(Date.now()>end)return reject(Error('Canvas paint timeout: '+JSON.stringify({state:m?.dataset,fonts:window.__fontSources,requests:window.__paintRequests,replies:window.__workerReplies})));setTimeout(tick,20);}tick();})'''

def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--url',required=True)
    p.add_argument('--output',required=True,type=Path);p.add_argument('--baseline',action='store_true',help='Record the old unbounded queue without asserting the new bound')
    a=p.parse_args()
    with tempfile.TemporaryDirectory(prefix='laymesh-paint-perf-',ignore_cleanup_errors=True) as profile:
        chrome=subprocess.Popen(['/usr/bin/google-chrome','--headless=new','--no-sandbox','--disable-gpu','--no-proxy-server','--remote-debugging-port=0','--remote-allow-origins=*','--user-data-dir='+profile,'about:blank'],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,start_new_session=True)
        try:
            active=Path(profile)/'DevToolsActivePort';deadline=time.monotonic()+30
            while not active.exists():
                if time.monotonic()>deadline:raise RuntimeError('Chromium startup timeout')
                time.sleep(.1)
            port=int(active.read_text().splitlines()[0]);opener=urllib.request.build_opener(urllib.request.ProxyHandler({}))
            pages=json.load(opener.open(f'http://127.0.0.1:{port}/json'))
            cdp=browser.CDP(next(page['webSocketDebuggerUrl'] for page in pages if page['type']=='page'))
            cdp.call('Page.enable');cdp.call('Runtime.enable');cdp.call('Page.addScriptToEvaluateOnNewDocument',{'source':SHIM})
            cdp.call('Emulation.setDeviceMetricsOverride',{'width':1440,'height':1000,'deviceScaleFactor':2,'mobile':False})
            cdp.call('Page.navigate',{'url':a.url})
            cdp.evaluate("new Promise(r=>{function tick(){const b=document.querySelector('[data-source-mode=source]');if(b&&document.querySelector('.live-image')?.localName==='canvas'){b.click();r();}else setTimeout(tick,20);}tick();})")
            first=cdp.evaluate(READY)
            zoom=cdp.evaluate(r'''(async()=>{
             const wait=ms=>new Promise(r=>setTimeout(r,ms)),raf=()=>new Promise(r=>requestAnimationFrame(r));
             window.__paintRequests=[];window.__holdPaint=true;
             document.querySelector('.preview-zoom-in').click();await wait(50);
             for(let i=0;i<12;i++){document.querySelector(i%2?'.preview-zoom-in':'.preview-zoom-out').click();await raf();}
             await wait(120);const during=window.__paintRequests.length,held=window.__held.length;
             window.__holdPaint=false;for(const release of window.__held.splice(0))release();
             return {during,held};
            })()''')
            latest=cdp.evaluate(READY)
            final=cdp.evaluate(r'''(()=>{const c=document.querySelector('.live-image'),requests=window.__paintRequests,last=requests.at(-1);return {requests:requests.length,canvas:[c.width,c.height],latest:[last.width,last.height],generation:document.querySelector('.example-module').dataset.generation,font_sources:window.__fontSources,initialization_ms:Number(document.querySelector('.live-metrics').textContent.match(/(?:初始化|Initialization)\s*([\d.]+)/)?.[1]),worker_replies:window.__workerReplies};})()''')
            assert final['canvas']==final['latest'],final
            assert all(font['source']=='page' for font in final['font_sources']) and len(final['font_sources'])==2,final
            if not a.baseline:
                assert zoom['during']==1 and final['requests']==2,(zoom,final)
                # A compile queued before an old-generation paint must return
                # an acknowledgement, otherwise bounded paint dispatch stalls.
                cdp.evaluate("window.__holdResults=true;document.querySelector('.live-run').click()")
                cdp.evaluate("new Promise((r,j)=>{let n=0;function tick(){if(window.__heldResults.length){document.querySelector('.preview-zoom-out').click();return r();}if(++n>1000)return j(Error('Compile timeout'));setTimeout(tick,20);}tick();})")
                cdp.evaluate("new Promise((r,j)=>{let n=0;function tick(){if(window.__workerReplies.some(e=>e.type==='paint-skipped')){window.__holdResults=false;for(const release of window.__heldResults.splice(0))release();return r();}if(++n>1000)return j(Error('Old paint was not acknowledged'));setTimeout(tick,20);}tick();})")
                cdp.evaluate("new Promise((resolve,reject)=>{const previous="+json.dumps(latest['generation'])+";let n=0;function tick(){const m=document.querySelector('.example-module');if(m.dataset.generation!==previous&&m.dataset.painted==='true')return resolve(true);if(++n>1000)return reject(Error('New generation paint stalled'));setTimeout(tick,20);}tick();})")
                generation=cdp.evaluate("({generation:document.querySelector('.example-module').dataset.generation,replies:window.__workerReplies})")
                assert any(reply['type']=='paint-skipped' for reply in generation['replies']),generation
            else:generation=None
            result={'definition':'Real page/WASM/Canvas; stalled CDN fetch respecting AbortSignal; 13 zoom changes with paint replies held; DPR 2','url':a.url,'artifact_sha256':browser.provenance(__file__),'first':first,'zoom_burst':zoom,'final':final,'generation_change':generation}
            a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n')
            print(json.dumps({'initialization_ms':final['initialization_ms'],'during':zoom['during'],'total_requests':final['requests'],'canvas':final['canvas'],'generation_change':bool(generation)},ensure_ascii=False))
        finally:
            try:os.killpg(chrome.pid,signal.SIGTERM)
            except ProcessLookupError:pass
            chrome.wait(timeout=10)

if __name__=='__main__':main()
