#!/usr/bin/env python3
"""Compare retained WASM pixels with fresh full paints after real source edits.

Native frames/resources remain in ignored target storage. Only pixel summaries
and input/artifact fingerprints belong in a shareable verification report.
"""
import argparse, hashlib, importlib.util, json, os, signal, subprocess, tempfile, time, urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('update_cdp', ROOT/'scripts/smoke-browser.py')
browser = importlib.util.module_from_spec(spec); spec.loader.exec_module(browser)

def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--source', required=True, type=Path)
    p.add_argument('--replace', required=True, nargs=2, metavar=('OLD','NEW'))
    p.add_argument('--url', required=True, help='Tailscale URL serving the repository root')
    p.add_argument('--output', required=True, type=Path)
    a = p.parse_args(); source = a.source.resolve(); text = source.read_text()
    assert a.replace[0] in text
    with tempfile.TemporaryDirectory(prefix='update-pixels-', dir=ROOT/'target') as temporary:
        work = Path(temporary); frames = []; known = []
        process = subprocess.Popen([str(ROOT/'target/release/laymesh'),'preview','--stdio'],
                                   stdin=subprocess.PIPE,stdout=subprocess.PIPE,text=True)
        process.stdout.readline(); base = None
        try:
            for i in range(7):
                edited = text if i == 0 else text.replace(a.replace[0],a.replace[1].replace('{i}',str(i)))
                request = dict(id=i,generation=i,protocol=3,file=str(source),source=edited,
                               resource_dir=str(work),display_list_updates=True,
                               known_display_list=base,known_resources=known)
                process.stdin.write(json.dumps(request)+'\n'); process.stdin.flush()
                frame = json.loads(process.stdout.readline()); assert 'error' not in frame
                for r in frame['resources']:
                    r['url'] = str(Path(r.pop('path')).relative_to(ROOT)); known.append(r['id'])
                base = frame['displayListId']; frames.append(frame)
        finally:
            process.stdin.close(); process.wait(timeout=15)
        (work/'frames.json').write_text(json.dumps(frames))
        with tempfile.TemporaryDirectory(prefix='laymesh-update-chrome-',ignore_cleanup_errors=True) as profile:
            chrome = subprocess.Popen(['/usr/bin/google-chrome','--headless=new','--no-sandbox','--disable-gpu','--no-proxy-server','--remote-debugging-port=0','--remote-allow-origins=*','--user-data-dir='+profile,'about:blank'],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,start_new_session=True)
            try:
                active = Path(profile)/'DevToolsActivePort'; deadline=time.monotonic()+30
                while not active.exists():
                    if time.monotonic()>deadline: raise RuntimeError('Chromium startup timeout')
                    time.sleep(.1)
                port=int(active.read_text().splitlines()[0]); opener=urllib.request.build_opener(urllib.request.ProxyHandler({}))
                pages=json.load(opener.open(f'http://127.0.0.1:{port}/json'))
                c=browser.CDP(next(v['webSocketDebuggerUrl'] for v in pages if v['type']=='page'))
                c.call('Page.enable'); c.call('Runtime.enable'); c.call('Page.navigate',{'url':a.url})
                time.sleep(.3)
                result=c.evaluate(r'''(async()=>{
                 const base=BASE,wasm=await import(new URL('extensions/vscode/dist/wasm/laymesh_wasm.js',base));
                 await wasm.default({module_or_path:new URL('extensions/vscode/dist/wasm/laymesh_wasm_bg.wasm',base)});
                 const frames=await (await fetch(new URL(FRAMES,base))).json(),rows=[];let draw,id;
                 for(const frame of frames){
                  for(const r of frame.resources)wasm.register_render_resource(r.id,new Uint8Array(await (await fetch(new URL(r.url,base))).arrayBuffer()));
                  if(frame.displayListUpdate){if(frame.displayListUpdate.base!==id)throw Error('baseline mismatch');draw.update_display_list(JSON.stringify(frame.displayListUpdate.update));}
                  else {draw?.free();draw=wasm.prepare_display_list(JSON.stringify(frame.displayList));}
                  id=frame.displayListId;
                  const p=frame.inspection.page,viewports=[[0,0,p.width,p.height],[p.width*.02,p.height*.01,p.width*.91,p.height*.87]];
                  for(const rect of viewports){
                   const w=1200,h=Math.round(w*(rect[3]-rect[1])/(rect[2]-rect[0])),start=performance.now();
                   const actual=draw.raster_retained(JSON.stringify(rect),w,h).into_rgba(),paintMs=performance.now()-start;
                   const fresh=wasm.prepare_display_list(draw.display_list()),expected=fresh.raster(JSON.stringify(rect),w,h).into_rgba();fresh.free();
                   let max=0,above=0,changed=0;for(let i=0;i<actual.length;i++){const delta=Math.abs(actual[i]-expected[i]);max=Math.max(max,delta);above+=delta>1;changed+=delta>0;}
                   if(above)throw Error('retained/full pixels differ: '+JSON.stringify({generation:frame.generation,rect,max,above}));
                   rows.push({generation:frame.generation,incremental:!!frame.displayListUpdate,width:w,height:h,maximumDelta:max,differingChannels:changed,retainedPaintMs:paintMs});
                  }
                 }
                 draw.free();return {rows,maximumDelta:Math.max(...rows.map(r=>r.maximumDelta)),frames:frames.length,paints:rows.length};
                })()'''.replace('BASE',json.dumps(a.url)).replace('FRAMES',json.dumps(str((work/'frames.json').relative_to(ROOT)))))
            finally:
                try: os.killpg(chrome.pid,signal.SIGTERM)
                except ProcessLookupError: pass
                chrome.wait(timeout=10)
    result['fixture_sha256']=hashlib.sha256(source.read_bytes()).hexdigest()
    result['native_sha256']=hashlib.sha256((ROOT/'target/release/laymesh').read_bytes()).hexdigest()
    result['wasm_sha256']=hashlib.sha256((ROOT/'extensions/vscode/dist/wasm/laymesh_wasm_bg.wasm').read_bytes()).hexdigest()
    result['definition']='Seven genuine source versions; same resources; whole-page and shifted viewport; fresh full WASM paint compared with retained update; no precision/quality reduction.'
    a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps({k:result[k] for k in ['frames','paints','maximumDelta']}))

if __name__=='__main__': main()
