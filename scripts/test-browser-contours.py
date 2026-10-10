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
            result = cdp.evaluate(r'''(async()=>{
                const fixtures=FIXTURES;
                const base=new URL('live/',document.querySelector('script[src$="app.js"]').src);
                const worker=new Worker(new URL('worker.js',base),{type:'module'}),pending=new Map();let serial=0;
                await new Promise((resolve,reject)=>{worker.onerror=e=>reject(Error(e.message));worker.onmessage=({data})=>{if(data.type==='ready')resolve();else if(data.type==='init-error')reject(Error(JSON.stringify(data.error)));else if(['result','error','painted','paint-error','paint-skipped'].includes(data.type)){pending.get(data.id)?.(data);pending.delete(data.id)}};worker.postMessage({type:'init',base:base.href})});
                const request=(data,label)=>new Promise((resolve,reject)=>{const id=++serial,t=setTimeout(()=>{pending.delete(id);reject(Error(label+' timeout'))},20000);pending.set(id,d=>{clearTimeout(t);resolve(d)});worker.postMessage({...data,id})});
                const run=c=>request({type:'run',entry:'fixtures/contour.lay',files:{'fixtures/contour.lay':c.source,...c.files}},c.name);
                const wasm=await import(new URL('wasm/laymesh_wasm.js',base));await wasm.default();
                const manifest=await(await fetch(new URL('manifest.json',base))).json();
                for(const font of manifest.fonts){const bytes=new Uint8Array(await(await fetch(new URL(font.file,base))).arrayBuffer());wasm.register_preview_font('/@bundled/'+font.file.split('/').pop(),bytes);}
                function walk(nodes,t=[1,0,0,1,0,0],active=false,out=[]){for(const n of nodes){const u=n.transform;const tr=[t[0]*u[0]+t[2]*u[1],t[1]*u[0]+t[3]*u[1],t[0]*u[2]+t[2]*u[3],t[1]*u[2]+t[3]*u[3],t[0]*u[4]+t[2]*u[5]+t[4],t[1]*u[4]+t[3]*u[5]+t[5]],inside=active||n.name==='plot-data';if(n.kind.kind==='group')walk(n.kind.children,tr,inside,out);else if(inside&&n.kind.kind==='path'&&(n.kind.fill.kind!=='none'||n.kind.stroke))out.push({node:n,t:tr});}return out;}
                function geometry(list,c){const paths=walk(list.nodes);if(c.data_path_count!==undefined&&paths.length!==c.data_path_count)return false;
                  if(c.minimum_closed_subpaths!==undefined&&paths.reduce((count,p)=>count+p.node.kind.path.filter(el=>el==='ClosePath').length,0)<c.minimum_closed_subpaths)return false;
                  const vertices=paths.flatMap(p=>p.node.kind.path.flatMap(el=>{const q=el.MoveTo||el.LineTo;return q?[[p.t[0]*q.x+p.t[2]*q.y+p.t[4],p.t[1]*q.x+p.t[3]*q.y+p.t[5]]]:[]}));return (c.boundary_vertices||[]).every(q=>vertices.some(p=>Math.hypot(p[0]-q[0],p[1]-q[1])<fixtures.scene_coordinate_tolerance_mm));}
                function samples(rgba,c,width){const scale=fixtures.pixel_dpi/25.4,values=c.pixel_samples.map(s=>{const i=(Math.round(s.y*scale)*width+Math.round(s.x*scale))*4,actual=[...rgba.slice(i,i+3)];return {...s,actual,passed:actual.length===3&&actual.every((v,i)=>Math.abs(v-s.rgb[i])<=s.tolerance)}});return {passed:values.every(s=>s.passed),samples:values};}
                async function pixels(list,c,page,workerRgba){const job=wasm.prepare_display_list(JSON.stringify(list)),scale=fixtures.pixel_dpi/25.4,width=Math.round(page.width*scale),height=Math.round(page.height*scale),raster=job.raster(JSON.stringify([0,0,page.width,page.height]),width,height);try{const rgba=raster.rgba,result=samples(rgba,c,width);if(workerRgba){if(workerRgba.length!==rgba.length)throw Error(c.name+' worker raster size mismatch');let maximumDelta=0;for(let i=0;i<rgba.length;i++)maximumDelta=Math.max(maximumDelta,Math.abs(rgba[i]-workerRgba[i]));result.workerMaximumDelta=maximumDelta;result.passed&&=maximumDelta<=1;}return result;}finally{raster.free();job.free();}}
                const cases=[];let geometryFault=false,opacityFault=false;
                try{for(const c of fixtures.cases){const response=await run(c);if(response.type!=='result')throw Error(c.name+': '+JSON.stringify(response.error));
                    // The worker keeps drawing commands resident. Inspect an
                    // independent compilation, then compare its pixels with
                    // the actual worker paint, including incremental updates.
                    const files=Object.fromEntries(Object.entries({'fixtures/contour.lay':c.source,...c.files}).map(([k,v])=>['/'+k.replace(/^\/+/,''),v])),resourceJob=wasm.prepare_render(c.source,'/fixtures/contour.lay',JSON.stringify(files));
                    let list;try{list=JSON.parse(resourceJob.display_list());for(const [id,r]of Object.entries(list.resources))wasm.register_render_resource(id,new Uint8Array(r.data));}finally{resourceJob.free();}
                    const page=response.inspection.page,scale=fixtures.pixel_dpi/25.4,width=Math.round(page.width*scale),height=Math.round(page.height*scale);
                    const painted=await request({type:'paint',generation:response.generation,rect:[0,0,page.width,page.height],width,height},c.name+' paint');
                    if(painted.type!=='painted'||painted.width!==width||painted.height!==height)throw Error(c.name+': worker paint failed '+JSON.stringify(painted.error||painted.type));
                    const g=geometry(list,c),p=await pixels(list,c,page,painted.rgba),workerPixels=samples(painted.rgba,c,width);cases.push({name:c.name,geometry:g,pixels:p,worker_pixels:workerPixels,metrics:response.metrics,displayList:list});
                    if(c.name==='saddle'){const fault=structuredClone(list);function remove(nodes){for(const n of nodes){if(n.name==='plot-data')n.kind.children=[];else if(n.kind.children)remove(n.kind.children)}}remove(fault.nodes);geometryFault=!geometry(fault,c);}
                    if(c.name==='equal_threshold'){const fault=structuredClone(list);function alpha(nodes){for(const n of nodes){n.opacity=1;if(n.kind.children)alpha(n.kind.children)}}alpha(fault.nodes);opacityFault=!(await pixels(fault,c,response.inspection.page)).passed;}
                }}finally{worker.terminate()}
                return {cases,fault_injections:{missing_geometry_rejected:geometryFault,changed_opacity_rejected:opacityFault}};
            })()'''.replace('FIXTURES', json.dumps(fixtures)))
            for case in result['cases']:
                (args.output / (case['name'] + '.json')).write_text(json.dumps(case.pop('displayList')))
            errors = [e for e in cdp.events if e.get('method') == 'Runtime.exceptionThrown' or e.get('method') == 'Log.entryAdded' and e['params']['entry']['level'] == 'error']
            result.update(artifact_sha256=browser.provenance(__file__), node_on_path=bool(shutil.which('node')), npm_on_path=bool(shutil.which('npm')), fixture_sha256=hashlib.sha256(fixture_path.read_bytes()).hexdigest(), wasm_sha256=hashlib.sha256((ROOT/'site/dist/site/live/wasm/laymesh_wasm_bg.wasm').read_bytes()).hexdigest(), console_errors=errors)
            result['status'] = 'passed' if not errors and all(c['geometry'] and c['pixels']['passed'] and c['worker_pixels']['passed'] for c in result['cases']) and all(result['fault_injections'].values()) else 'failed'
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
