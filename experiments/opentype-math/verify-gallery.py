#!/usr/bin/env python3
"""Verify the complete formula gallery through real Chromium Canvas rendering."""
import argparse, base64, json, subprocess, sys, tempfile, time, urllib.request
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/"scripts"))
import importlib.util
spec=importlib.util.spec_from_file_location("smoke_browser",ROOT/"scripts/smoke-browser.py")
module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
CDP=module.CDP

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--url",required=True)
    parser.add_argument("--output",type=Path,default=ROOT/"release/verification/rust-renderer-migration/math-gallery")
    args=parser.parse_args(); args.output.mkdir(parents=True,exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="laymesh-math-chrome-",ignore_cleanup_errors=True) as profile:
        chrome=subprocess.Popen(["/usr/bin/google-chrome","--headless=new","--no-sandbox","--disable-gpu","--no-proxy-server","--remote-debugging-port=0","--remote-allow-origins=*","--user-data-dir="+profile,"about:blank"],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
        try:
            active=Path(profile)/"DevToolsActivePort";deadline=time.monotonic()+30
            while not active.exists():
                assert time.monotonic()<deadline,"Chromium startup timeout"
                time.sleep(.1)
            port=int(active.read_text().splitlines()[0]);opener=urllib.request.build_opener(urllib.request.ProxyHandler({}))
            pages=json.load(opener.open(f"http://127.0.0.1:{port}/json"))
            cdp=CDP(next(page["webSocketDebuggerUrl"] for page in pages if page["type"]=="page"))
            cdp.call("Page.enable");cdp.call("Runtime.enable");cdp.call("Log.enable")
            cdp.call("Emulation.setDeviceMetricsOverride",{"width":1440,"height":1000,"deviceScaleFactor":1,"mobile":False})
            cdp.call("Page.navigate",{"url":args.url})
            deadline=time.monotonic()+60
            while not cdp.evaluate("!!window.mathGalleryData && window.mathCanvasRendered>0"):
                assert time.monotonic()<deadline,"Gallery load timeout"
                time.sleep(.2)
            time.sleep(1)
            args.output.joinpath("desktop.png").write_bytes(base64.b64decode(cdp.call("Page.captureScreenshot",{"format":"png"})["data"]))
            total=cdp.evaluate("window.mathAuditCases=window.mathGalleryData.renders.flatMap(r=>['variants','text_variants'].flatMap(policy=>Object.values(r[policy]).flatMap(styles=>Object.values(styles)))).filter(r=>r.drawing);window.mathAuditCases.length")
            started=time.monotonic();results=[]
            for start in range(0,total,100):
                result=cdp.evaluate("""(async()=>{let checked=0, failures=[],mismatches=[];for(const result of window.mathAuditCases.slice(START,START+100)){const canvas=document.createElement('canvas');await window.mathGalleryDraw(canvas,result,16);if(canvas.dataset.rendered!=='true')failures.push({drawing:result.drawing,error:canvas.dataset.error});else{checked++;if(canvas.dataset.nativeMatch!=='true')mismatches.push({drawing:result.drawing,maximum_delta:Number(canvas.dataset.maximumDelta),different_channels:Number(canvas.dataset.differentChannels)});}}return {checked,failures,mismatches};})()""".replace("START",str(start)))
                results.append(result)
                if start%1000==0:print(f"Canvas {start+result['checked']}/{total}",flush=True)
            summary={"url":args.url,"case_entries":cdp.evaluate("window.mathGalleryData.cases.length"),"distinct_formulas":cdp.evaluate("window.mathGalleryData.renders.length"),"canvas_checks":sum(r["checked"] for r in results),"failures":[v for r in results for v in r["failures"]],"native_hash_mismatches":[v for r in results for v in r["mismatches"]],"seconds":time.monotonic()-started,"renderer_fingerprint":cdp.evaluate("window.mathGalleryData.meta.renderer_fingerprint")}
            cdp.evaluate("document.querySelector('[data-suite=text]').click();document.querySelector('[data-style=inline]').click();document.querySelector('#baselines').checked=true;document.querySelector('#baselines').dispatchEvent(new Event('change',{bubbles:true}));document.querySelector('#size').value=48;document.querySelector('#size').dispatchEvent(new Event('input',{bubbles:true}))")
            time.sleep(1)
            summary["interaction"]=cdp.evaluate("({url:location.href,count:document.querySelector('#result-count').textContent,inline:document.querySelector('[data-style=inline]').getAttribute('aria-pressed'),baseline:document.querySelector('#baselines').checked,canvases:document.querySelectorAll('canvas[data-rendered=true]').length})")
            assert "suite=text" in summary["interaction"]["url"] and summary["interaction"]["inline"]=="true" and summary["interaction"]["canvases"]>0
            summary["console_errors"]=[event for event in cdp.events if event.get("method")=="Runtime.exceptionThrown" or (event.get("method")=="Log.entryAdded" and event.get("params",{}).get("entry",{}).get("level")=="error")]
            assert not summary["console_errors"],summary["console_errors"]
            cdp.call("Emulation.setDeviceMetricsOverride",{"width":390,"height":844,"deviceScaleFactor":2,"mobile":True})
            cdp.evaluate("document.querySelector('#size').dispatchEvent(new Event('input',{bubbles:true}))")
            time.sleep(1)
            args.output.joinpath("mobile.png").write_bytes(base64.b64decode(cdp.call("Page.captureScreenshot",{"format":"png"})["data"]))
            summary["page_failures"]=cdp.evaluate("window.mathCanvasFailures")
            args.output.joinpath("verification.json").write_text(json.dumps(summary,ensure_ascii=False,indent=2))
            print(json.dumps({k:v for k,v in summary.items() if k!="native_hash_mismatches"} | {"native_hash_mismatches":len(summary["native_hash_mismatches"])},ensure_ascii=False),flush=True)
            assert summary["canvas_checks"]==total and not summary["failures"] and not summary["page_failures"],summary
            assert all(r["maximum_delta"]<=1 for r in summary["native_hash_mismatches"]),"Native/WASM differences exceed one channel level"
        finally:
            chrome.terminate();chrome.wait(timeout=15)
if __name__=="__main__":main()
