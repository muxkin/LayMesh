#!/usr/bin/env python3
"""Check protocol 3 and native Skia/CanvasKit pixel parity in the existing Chromium harness."""
import argparse,base64,importlib.util,json,os,signal,subprocess,tempfile,time,urllib.request
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('cdp',ROOT/'scripts/smoke-browser.py');browser=importlib.util.module_from_spec(spec);spec.loader.exec_module(browser)
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--url',required=True,help='Tailscale-bound built site URL');p.add_argument('--binary',type=Path,default=ROOT/'target/release/laymesh');p.add_argument('--output',type=Path,default=ROOT/'release/verification/rust-renderer-migration/parity');a=p.parse_args();a.output.mkdir(parents=True,exist_ok=True)
staging=ROOT/'site/dist/renderer-contract';staging.mkdir(exist_ok=True);resources=staging/'resources';resources.mkdir(exist_ok=True)
files=['examples/typography.lay','examples/vector.lay','examples/effects/art-text.lay','examples/effects/shadow-glow.lay','examples/effects/images-groups.lay','examples/gallery/images/svg.lay','examples/plot/scientific.lay']
wide=staging/'wide.lay';wide.write_text('p=canvas(size=(7000mm,0.2mm))\np.add(rect(size=(7000mm,0.2mm),fill=linear_gradient(stops=[(0,"#00ff0080"),(1,"#0000ffff")],start=(0,0),end=(1,0))))\n');files.append(str(wide))
fixtures=[]
subprocess.run(['cargo','build','--locked','--offline','--release','-p','laymesh-render','--example','render-display-list'],cwd=ROOT,check=True)
oracle=ROOT/'target/release/examples/render-display-list'
engine=subprocess.Popen([str(a.binary),'preview','--stdio'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,text=True);ready=json.loads(engine.stdout.readline());assert 'display-list-v3' in ready['capabilities']
try:
 for i,name in enumerate(files):
  file=Path(name);file=file if file.is_absolute() else ROOT/file;dpi=254 if file==wide else 96
  request={'id':i,'file':str(file),'source':file.read_text(),'protocol':3,'generation':i,'resource_dir':str(resources)}
  engine.stdin.write(json.dumps(request)+'\n');engine.stdin.flush();frame=json.loads(engine.stdout.readline());assert frame['protocol']==3 and 'svg' not in frame and 'error' not in frame,frame
  assert frame['generation']==i
  snapshot=a.output/f'{i}.json';snapshot.write_text(json.dumps(frame));subprocess.run([str(oracle),str(snapshot),str(staging/f'{i}.rgba'),str(dpi)],check=True)
  for resource in frame['resources']:resource['url']='renderer-contract/resources/'+Path(resource.pop('path')).name
  (staging/f'{i}.json').write_text(json.dumps(frame));png=staging/f'{i}.png'
  subprocess.run([str(a.binary),'render',str(file),'-o',str(png),'--dpi',str(dpi),'--warnings','hide'],check=True,stdout=subprocess.DEVNULL)
  fixtures.append({'name':name,'index':i,'dpi':dpi})
 # Explicit old protocols remain available; unknown versions fail, with no implicit fallback.
 for protocol in [1,2,4]:
  request.update(protocol=protocol,id=100+protocol);engine.stdin.write(json.dumps(request)+'\n');engine.stdin.flush();result=json.loads(engine.stdout.readline());assert ('error' in result) if protocol==4 else ('svg' in result)
finally:engine.stdin.close();engine.wait(timeout=10)
with tempfile.TemporaryDirectory(prefix='laymesh-parity-chrome-',ignore_cleanup_errors=True) as profile:
 chrome=subprocess.Popen(['/usr/bin/google-chrome','--headless=new','--no-sandbox','--disable-gpu','--no-proxy-server','--remote-debugging-port=0','--remote-allow-origins=*','--user-data-dir='+profile,'about:blank'],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,start_new_session=True)
 try:
  active=Path(profile)/'DevToolsActivePort';deadline=time.monotonic()+25
  while not active.exists():
   if time.monotonic()>deadline:raise RuntimeError('Chromium timeout')
   time.sleep(.1)
  port=int(active.read_text().splitlines()[0]);opener=urllib.request.build_opener(urllib.request.ProxyHandler({}));pages=json.load(opener.open(f'http://127.0.0.1:{port}/json'));cdp=browser.CDP(next(page['webSocketDebuggerUrl'] for page in pages if page['type']=='page'));cdp.call('Page.enable');cdp.call('Runtime.enable');cdp.call('Log.enable');cdp.call('Page.navigate',{'url':a.url});cdp.evaluate("new Promise(r=>{const t=()=>document.querySelector('script[src$=\"app.js\"]')?r():setTimeout(t,30);t()})")
  result=cdp.evaluate('''(async()=>{
   const base=new URL('../',new URL('site/app.js',location.origin)),wasm=await import(new URL('site/live/wasm/laymesh_wasm.js',base));await wasm.default();const {initializeSkia,SkiaRenderer}=await import(new URL('site/live/skia-renderer.mjs',base)),meta=await(await fetch(new URL('site/live/skia/manifest.json',base))).json(),ck=await initializeSkia(new URL('site/live/skia/'+meta.wasm,base).href);const cases=[];
   for(const c of FIXTURES){const frame=await(await fetch(new URL(`renderer-contract/${c.index}.json`,base))).json();
    for(const resource of frame.resources)wasm.register_render_resource(resource.id,new Uint8Array(await(await fetch(new URL(resource.url,base))).arrayBuffer()));
    const job=wasm.prepare_display_list(JSON.stringify(frame.displayList)),page=frame.inspection.page,width=Math.max(1,Math.round(page.width*c.dpi/25.4)),height=Math.max(1,Math.round(page.height*c.dpi/25.4)),start=performance.now(),renderer=new SkiaRenderer(ck,new OffscreenCanvas(width,height),{software:true});await renderer.install(JSON.parse(job.display_list()),id=>job.resource_bytes(id),1);renderer.render([0,0,page.width,page.height],width,height);const rgba=renderer.readPixels();
    const native=new Uint8Array(await(await fetch(new URL(`renderer-contract/${c.index}.rgba`,base))).arrayBuffer());
    let max=0,sum=0,changed=0;for(let i=0;i<rgba.length;i++){const delta=Math.abs(rgba[i]-native[i]);max=Math.max(max,delta);sum+=delta;if(delta>2)changed++;}
    const samples=[1023,1024,2047,2048,65534,65535,65536].filter(x=>x<width).map(x=>({x,rgba:[...rgba.slice(x*4,x*4+4)]}));cases.push({name:c.name,width,height,rasterMs:performance.now()-start,maximumDelta:max,meanDelta:sum/rgba.length,channelsAboveTwo:changed,samples});renderer.dispose();job.free();
   }return {cases};
  })()'''.replace('FIXTURES',json.dumps(fixtures)))
  result['status']='passed' if all(c['meanDelta']<=1.0 and c['channelsAboveTwo']/(c['width']*c['height']*4)<.02 for c in result['cases']) else 'failed';result['protocols']={'default_client':3,'explicit_compatibility':[1,2],'unknown_rejected':True};result['native_oracle_sha256']=__import__('hashlib').sha256(oracle.read_bytes()).hexdigest();result['wasm_sha256']=__import__('hashlib').sha256((ROOT/'site/dist/site/live/wasm'/json.loads((ROOT/'site/dist/site/live/wasm/manifest.json').read_text())['wasm']).read_bytes()).hexdigest();(a.output/'verification.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n');print(json.dumps(result,ensure_ascii=False));assert result['status']=='passed'
 finally:
  os.killpg(chrome.pid,signal.SIGTERM);chrome.wait(timeout=10)
