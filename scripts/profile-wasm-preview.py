#!/usr/bin/env python3
"""Measure shared WASM compile/lowering/paint phases in an isolated Chromium."""
import argparse, base64, importlib.util, json, signal, subprocess, tempfile, time, urllib.request
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('preview_cdp', ROOT/'scripts/smoke-browser.py')
browser = importlib.util.module_from_spec(spec); spec.loader.exec_module(browser)

def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--url', required=True, help='Built site served on its Tailscale address')
    p.add_argument('--wasm-dir', type=Path, default=ROOT/'site/dist/site/live/wasm')
    p.add_argument('--example', default='showcase')
    p.add_argument('--width', type=int, default=960, help='Canvas pixel width')
    p.add_argument('--output', type=Path, required=True)
    a = p.parse_args()
    glue = 'data:text/javascript;base64,'+base64.b64encode((a.wasm_dir/'laymesh_wasm.js').read_bytes()).decode()
    binary = base64.b64encode((a.wasm_dir/'laymesh_wasm_bg.wasm').read_bytes()).decode()
    with tempfile.TemporaryDirectory(prefix='laymesh-wasm-profile-', ignore_cleanup_errors=True) as profile:
        chrome = subprocess.Popen(['/usr/bin/google-chrome','--headless=new','--no-sandbox','--disable-gpu',
            '--no-proxy-server','--remote-debugging-port=0','--remote-allow-origins=*',
            '--user-data-dir='+profile,'about:blank'], stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,start_new_session=True)
        try:
            active=Path(profile)/'DevToolsActivePort'; deadline=time.monotonic()+30
            while not active.exists():
                if time.monotonic()>deadline: raise RuntimeError('Chromium startup timeout')
                time.sleep(.1)
            port=int(active.read_text().splitlines()[0]); opener=urllib.request.build_opener(urllib.request.ProxyHandler({}))
            pages=json.load(opener.open(f'http://127.0.0.1:{port}/json'))
            cdp=browser.CDP(next(page['webSocketDebuggerUrl'] for page in pages if page['type']=='page'))
            cdp.call('Page.enable'); cdp.call('Runtime.enable')
            # The static resource manifest supplies all inputs; no editor worker
            # runs in this page, avoiding a second concurrent compilation.
            cdp.call('Page.navigate',{'url':a.url.rstrip('/')+'/site/live/manifest.json'})
            cdp.evaluate("new Promise(r=>document.readyState==='complete'?r():addEventListener('load',r,{once:true}))")
            result=cdp.evaluate('''(async()=>{
                const base=new URL('/site/live/',location.href),manifest=await(await fetch(new URL('manifest.json',base))).json();
                const wasm=await import(GLUE);const runtime=await wasm.default({module_or_path:Uint8Array.from(atob(BINARY),c=>c.charCodeAt(0))});
                for(const font of manifest.fonts){const bytes=new Uint8Array(await(await fetch(new URL(font.file,base))).arrayBuffer());wasm.register_preview_font('/@bundled/'+font.file.split('/').pop(),bytes);}
                const example=manifest.examples[EXAMPLE],files={};
                for(const file of example.dependencies){const response=await fetch(new URL(manifest.resources['/'+file],base));files['/'+file]=/\\.(lay|lcss|csv|json)$/.test(file)?await response.text():Array.from(new Uint8Array(await response.arrayBuffer()));}
                const json=JSON.stringify(files),rows=[];
                const take=frame=>{if(typeof frame.into_rgba==='function')return frame.into_rgba();const bytes=frame.rgba;frame.free();return bytes;};
                for(let i=0;i<8;i++){
                    let before=performance.now();const job=wasm.prepare_render(files['/'+example.entry]+'\\n# performance edit '+i,'/'+example.entry,json),compile=performance.now()-before;
                    before=performance.now();const draw=job.draw_job(),lower=performance.now()-before;
                    before=performance.now();const serialized=draw.display_list(),inspection=JSON.parse(job.inspection()).inspection,metadata=performance.now()-before;
                    const height=Math.round(WIDTH*inspection.page.height/inspection.page.width);
                    before=performance.now();const frame=draw.raster(JSON.stringify([0,0,inspection.page.width,inspection.page.height]),WIDTH,height),drawMs=performance.now()-before;
                    before=performance.now();const rgba=take(frame),readbackMs=performance.now()-before,raster=drawMs+readbackMs;
                    const repeatPaint=[];for(let view=0;view<5;view++){
                        const start=performance.now(),px=draw.raster(JSON.stringify([inspection.page.width*view*.01,0,inspection.page.width*(1+view*.01),inspection.page.height]),WIDTH,height),paintMs=performance.now()-start;
                        const read=performance.now();take(px);repeatPaint.push({paintMs,copyMs:performance.now()-read});
                    }
                    let hash=2166136261;for(const b of rgba)hash=Math.imul(hash^b,16777619)>>>0;
                    rows.push({compile,lower,metadata,raster,drawMs,readbackMs,repeatPaint,total:compile+lower+metadata+raster,display_list_bytes:serialized.length,rgba_hash:hash.toString(16),wasm_memory_bytes:runtime.memory.buffer.byteLength});draw.free();job.free();
                }
                return {example:EXAMPLE,files_json_bytes:json.length,rows};
            })()'''.replace('GLUE',json.dumps(glue)).replace('BINARY',json.dumps(binary)).replace('EXAMPLE',json.dumps(a.example)).replace('WIDTH',str(a.width)))
            result['wasm_sha256']=__import__('hashlib').sha256((a.wasm_dir/'laymesh_wasm_bg.wasm').read_bytes()).hexdigest()
            result['definition']=f'Same dependencies/fonts; isolated Chromium; {a.width} px paint and five viewport repaints; first compile and seven comment edits; excludes initialization/network/debounce'
            result['hot_median_ms']={phase:__import__('statistics').median(r[phase] for r in result['rows'][1:]) for phase in ['compile','lower','metadata','raster','total']}
            a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(json.dumps(result,indent=2)+'\n')
            print(json.dumps({'cold':result['rows'][0],'hot_median_ms':result['hot_median_ms']}))
        finally:
            try: os_kill=signal.SIGTERM; __import__('os').killpg(chrome.pid,os_kill)
            except ProcessLookupError: pass
            chrome.wait(timeout=10)

if __name__=='__main__':main()
