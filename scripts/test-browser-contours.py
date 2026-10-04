#!/usr/bin/env python3
"""Run the shared native/WASM contour contracts in a real browser, without Node."""
import argparse, hashlib, importlib.util, json, os, shutil, signal, subprocess, tempfile, time, urllib.request
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('browser_smoke', ROOT / 'scripts/smoke-browser.py')
browser = importlib.util.module_from_spec(spec)
spec.loader.exec_module(browser)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--url', required=True)
    parser.add_argument('--chrome', default='/usr/bin/google-chrome')
    parser.add_argument('--output', type=Path, default=ROOT / 'release/comparison/revision-contours-browser')
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    fixture_path = ROOT / 'tests/fixtures/contour-contracts.json'
    fixtures = json.loads(fixture_path.read_text())
    with tempfile.TemporaryDirectory(prefix='laymesh-contour-browser-', ignore_cleanup_errors=True) as profile:
        chrome = subprocess.Popen([args.chrome, '--headless=new', '--no-sandbox', '--disable-gpu', '--no-proxy-server', '--remote-debugging-port=0', '--remote-allow-origins=*', '--user-data-dir=' + profile, 'about:blank'], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, start_new_session=True)
        try:
            active = Path(profile) / 'DevToolsActivePort'
            deadline = time.monotonic() + 25
            while not active.exists():
                if time.monotonic() > deadline:
                    raise RuntimeError('Chromium startup timed out')
                time.sleep(.1)
            port = int(active.read_text().splitlines()[0])
            opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
            pages = json.load(opener.open(f'http://127.0.0.1:{port}/json'))
            cdp = browser.CDP(next(p['webSocketDebuggerUrl'] for p in pages if p['type'] == 'page'))
            cdp.call('Page.enable'); cdp.call('Runtime.enable'); cdp.call('Log.enable')
            cdp.call('Page.navigate', {'url': args.url})
            cdp.evaluate("new Promise((resolve,reject)=>{const limit=Date.now()+20000;function ready(){if(document.querySelector('script[src$=\"app.js\"]'))return resolve();if(Date.now()>limit)return reject(Error('Page timeout'));setTimeout(ready,30)}ready()})")
            result = cdp.evaluate('''(async()=>{
                const fixtures=FIXTURES;
                const base=new URL('live/',document.querySelector('script[src$="app.js"]').src);
                const worker=new Worker(new URL('worker.js',base),{type:'module'}),pending=new Map();let serial=0;
                await new Promise((resolve,reject)=>{worker.onerror=e=>reject(Error(e.message));worker.onmessage=({data})=>{if(data.type==='ready')resolve();else if(data.type==='result'||data.type==='error'){pending.get(data.id)?.(data);pending.delete(data.id)}};worker.postMessage({type:'init',base:base.href})});
                const run=c=>new Promise((resolve,reject)=>{const id=++serial,t=setTimeout(()=>reject(Error(c.name+' timeout')),20000);pending.set(id,d=>{clearTimeout(t);resolve(d)});worker.postMessage({type:'run',id,entry:'fixtures/contour.lay',files:{'fixtures/contour.lay':c.source,...c.files}})});
                function geometry(svg,c){
                    const doc=new DOMParser().parseFromString(svg,'image/svg+xml');
                    const paths=[...doc.querySelectorAll('[data-id="plot-data"] path')].filter(p=>!p.closest('defs'));
                    if(c.data_path_count!==undefined&&paths.length!==c.data_path_count)return false;
                    if(c.minimum_closed_subpaths!==undefined&&paths.reduce((n,p)=>n+(p.getAttribute('d').match(/Z/gi)||[]).length,0)<c.minimum_closed_subpaths)return false;
                    const vertices=paths.flatMap(p=>[...p.getAttribute('d').matchAll(/[ML]\\s*([-+\\deE.]+)[ ,]+([-+\\deE.]+)/g)].map(m=>[Number(m[1])+10,Number(m[2])+10]));
                    return (c.boundary_vertices||[]).every(q=>vertices.some(p=>Math.hypot(p[0]-q[0],p[1]-q[1])<fixtures.scene_coordinate_tolerance_mm));
                }
                async function pixels(svg,c,page){
                    const canvas=document.createElement('canvas'),scale=fixtures.pixel_dpi/25.4;canvas.width=Math.round(page.width*scale);canvas.height=Math.round(page.height*scale);
                    const url=URL.createObjectURL(new Blob([svg],{type:'image/svg+xml'})),image=new Image();
                    try{await new Promise((resolve,reject)=>{image.onload=resolve;image.onerror=()=>reject(Error('SVG image decode failed'));image.src=url});const ctx=canvas.getContext('2d',{willReadFrequently:true});ctx.drawImage(image,0,0,canvas.width,canvas.height);
                    const samples=c.pixel_samples.map(s=>{const actual=[...ctx.getImageData(Math.round(s.x*scale),Math.round(s.y*scale),1,1).data].slice(0,3);return {...s,actual,passed:actual.every((v,i)=>Math.abs(v-s.rgb[i])<=s.tolerance)}});return {passed:samples.every(s=>s.passed),samples};}finally{URL.revokeObjectURL(url)}
                }
                const cases=[];let geometryFault=false,opacityFault=false;
                try{for(const c of fixtures.cases){const response=await run(c);if(response.type!=='result')throw Error(c.name+': '+JSON.stringify(response.error));const g=geometry(response.svg,c),p=await pixels(response.svg,c,response.inspection.page);cases.push({name:c.name,geometry:g,pixels:p,metrics:response.metrics,svg:response.svg});
                    if(c.name==='saddle'){const doc=new DOMParser().parseFromString(response.svg,'image/svg+xml');doc.querySelector('[data-id="plot-data"]').replaceChildren();geometryFault=!geometry(new XMLSerializer().serializeToString(doc),c)}
                    if(c.name==='equal_threshold'){const doc=new DOMParser().parseFromString(response.svg,'image/svg+xml');for(const el of doc.querySelectorAll('[data-id="plot-data"] [opacity]'))el.setAttribute('opacity','1');const fault=await pixels(new XMLSerializer().serializeToString(doc),c,response.inspection.page);opacityFault=!fault.passed}
                }}finally{worker.terminate()}
                return {cases,fault_injections:{missing_geometry_rejected:geometryFault,changed_opacity_rejected:opacityFault}};
            })()'''.replace('FIXTURES', json.dumps(fixtures)))
            for case in result['cases']:
                (args.output / (case['name'] + '.svg')).write_text(case.pop('svg'))
            errors = [e for e in cdp.events if e.get('method') == 'Runtime.exceptionThrown' or e.get('method') == 'Log.entryAdded' and e['params']['entry']['level'] == 'error']
            result.update(artifact_sha256=browser.provenance(__file__), node_on_path=bool(shutil.which('node')), npm_on_path=bool(shutil.which('npm')), fixture_sha256=hashlib.sha256(fixture_path.read_bytes()).hexdigest(), wasm_sha256=hashlib.sha256((ROOT/'site/dist/site/live/wasm/laymesh_wasm_bg.wasm').read_bytes()).hexdigest(), console_errors=errors)
            result['status'] = 'passed' if not errors and all(c['geometry'] and c['pixels']['passed'] for c in result['cases']) and all(result['fault_injections'].values()) else 'failed'
            (args.output / 'verification.json').write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n')
            print(json.dumps(result, ensure_ascii=False))
            if result['status'] != 'passed':
                raise SystemExit(1)
        finally:
            try: os.killpg(chrome.pid, signal.SIGTERM)
            except ProcessLookupError: pass
            try: chrome.wait(timeout=5)
            except subprocess.TimeoutExpired: os.killpg(chrome.pid, signal.SIGKILL); chrome.wait()


if __name__ == '__main__':
    main()
