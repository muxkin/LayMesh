#!/usr/bin/env python3
"""Exercise media in an isolated, real VS Code webview with CDP user gestures."""
import argparse,base64,importlib.util,io,json,os,signal,socket,subprocess,tempfile,time,urllib.request
from pathlib import Path
from PIL import Image,ImageChops,ImageStat
ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('cdp',ROOT/'scripts/smoke-browser.py');cdp=importlib.util.module_from_spec(spec);spec.loader.exec_module(cdp)
p=argparse.ArgumentParser();p.add_argument('--extension-dir',type=Path,default=ROOT/'target/media-vscode');p.add_argument('--file',type=Path,default=ROOT/'target/media-trial/first.lay');p.add_argument('--output',type=Path,default=Path('/tmp/laymesh-media-vscode'));p.add_argument('--code',default='/usr/share/code/code');a=p.parse_args();a.output.mkdir(parents=True,exist_ok=True)
with tempfile.TemporaryDirectory(prefix='laymesh-media-vscode-')as temp:
    t=Path(temp);user=t/'profile/User';user.mkdir(parents=True);(user/'settings.json').write_text(json.dumps({'laymesh.language':'zh-CN','workbench.colorTheme':'VS Code Dark','window.autoDetectColorScheme':False,'security.workspace.trust.enabled':False,'security.workspace.trust.startupPrompt':'never','security.workspace.trust.untrustedFiles':'open'}))
    import shutil
    source=t/'sources';source.mkdir();
    for name in ['first.lay','demo.mp4','music.wav']:shutil.copy2(a.file.parent/name,source/name)
    fixture=t/'extensions/local-media-qa';fixture.mkdir(parents=True);(fixture/'package.json').write_text(json.dumps({'name':'local-media-qa','publisher':'local','version':'0.0.1','engines':{'vscode':'^1.85.0'},'main':'index.cjs','activationEvents':['onStartupFinished']}));(fixture/'index.cjs').write_text('exports.activate=async()=>{const v=require("vscode");const d=await v.workspace.openTextDocument('+json.dumps(str(source/a.file.name))+');await v.window.showTextDocument(d);await v.commands.executeCommand("workbench.action.closeAuxiliaryBar");await v.commands.executeCommand("workbench.action.closeSidebar");await v.commands.executeCommand("laymesh.openPreview");};')
    with socket.socket()as s:s.bind(('127.0.0.1',0));port=s.getsockname()[1]
    env={**os.environ,'XDG_RUNTIME_DIR':'/run/user/1000','PULSE_SERVER':'unix:/run/user/1000/pulse/native'};env.pop('ELECTRON_RUN_AS_NODE',None)
    log=(a.output/'host.log').open('w');host=subprocess.Popen([a.code,'--no-sandbox','--mute-audio','--disable-gpu','--disable-updates','--force-device-scale-factor=2','--ozone-platform=headless','--remote-debugging-port='+str(port),'--remote-allow-origins=*','--user-data-dir='+str(t/'profile'),'--extensions-dir='+str(t/'extensions'),'--extensionDevelopmentPath='+str(a.extension_dir.resolve()),'--skip-welcome','--skip-release-notes','--new-window'],env=env,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
    sessions=[]
    try:
        opener=urllib.request.build_opener(urllib.request.ProxyHandler({}))
        def until(fn,label,seconds=60):
            deadline=time.monotonic()+seconds
            while time.monotonic()<deadline:
                try:
                    value=fn()
                    if value:return value
                except(Exception):pass
                time.sleep(.05)
            raise AssertionError('Timeout: '+label)
        def targets():return json.load(opener.open(f'http://127.0.0.1:{port}/json/list'))
        root=until(lambda:next((cdp.CDP(x['webSocketDebuggerUrl'])for x in targets()if x['type']=='page'),None),'VS Code window');sessions.append(root)
        root.call('Emulation.setDeviceMetricsOverride',{'width':1440,'height':960,'deviceScaleFactor':2,'mobile':False})
        def attach():
            for target in targets():
                if target['type']!='iframe':continue
                c=cdp.CDP(target['webSocketDebuggerUrl']);sessions.append(c)
                if c.evaluate("!!document.querySelector('iframe')?.contentDocument?.defaultView?.__LAYMESH_PREVIEW_MEDIA"):return c
            return None
        c=until(attach,'Native media webview')
        def evaluate(code):return c.evaluate("(()=>{const d=document.querySelector('iframe').contentDocument;return ("+code+");})()")
        until(lambda:evaluate('d.defaultView.__LAYMESH_PREVIEW_MEDIA.tracks.length===2&&d.defaultView.__LAYMESH_PREVIEW_MEDIA.tracks.every(t=>t.element.readyState>=2)'), 'Media decoding')
        def click(selector):
            outer=root.evaluate("[...document.querySelectorAll('iframe')].find(e=>e.getBoundingClientRect().width>0).getBoundingClientRect().toJSON()");inner=c.evaluate("document.querySelector('iframe').getBoundingClientRect().toJSON()");button=evaluate('d.querySelector('+json.dumps(selector)+').getBoundingClientRect().toJSON()');point={'x':outer['x']+inner['x']+button['x']+button['width']/2,'y':outer['y']+inner['y']+button['y']+button['height']/2}
            root.call('Input.dispatchMouseEvent',{'type':'mouseMoved',**point})
            root.call('Page.bringToFront')
            for kind in ['mousePressed','mouseReleased']:root.call('Input.dispatchMouseEvent',{'type':kind,'button':'left','clickCount':1,**point})
            c.call('Runtime.evaluate',{'expression':'document.querySelector(\"iframe\").contentDocument.querySelector('+json.dumps(selector)+').click()','userGesture':True})
        root.call('Page.bringToFront')
        click('[aria-label="开始"]')
        time.sleep(.5)
        image=root.call('Page.captureScreenshot',{'format':'png','captureBeyondViewport':False});(a.output/'after-click.png').write_bytes(base64.b64decode(image['data']))
        until(lambda:evaluate('d.defaultView.__LAYMESH_PREVIEW_MEDIA.tracks.every(t=>t.started&&!t.element.paused)&&d.defaultView.__LAYMESH_PREVIEW_MEDIA.tracks[0].element.currentTime>.2'),'Actual media playback')
        playing=evaluate('d.defaultView.__LAYMESH_PREVIEW_MEDIA.tracks.map(t=>({scope:t.cue.scope,time:t.element.currentTime,paused:t.element.paused,compatibleAudio:!!t.sound,soundReady:t.sound?.readyState,soundTime:t.sound?.currentTime}))')
        assert playing[0]['compatibleAudio']and playing[0]['soundReady']>=2
        def screenshot():return base64.b64decode(root.call('Page.captureScreenshot',{'format':'png','captureBeyondViewport':False})['data'])
        first=screenshot();time.sleep(.7);second=screenshot();(a.output/'playing.png').write_bytes(second)
        outer=root.evaluate("[...document.querySelectorAll('iframe')].find(e=>e.getBoundingClientRect().width>0).getBoundingClientRect().toJSON()");inner=c.evaluate("document.querySelector('iframe').getBoundingClientRect().toJSON()");canvas=evaluate('d.querySelector(".figure-pixels").getBoundingClientRect().toJSON()')
        one=Image.open(io.BytesIO(first)).convert('RGB');two=Image.open(io.BytesIO(second)).convert('RGB');scale=one.width/1440
        x=max(0,outer['x']+inner['x']+canvas['x']);y=max(0,outer['y']+inner['y']+canvas['y']);right=min(1440,x+canvas['width']);bottom=min(940,y+canvas['height']);crop=tuple(round(v*scale)for v in [x,y,right,bottom])
        assert right>x and bottom>y
        assert sum(ImageStat.Stat(ImageChops.difference(one.crop(crop),two.crop(crop))).mean)>1,'Rendered video pixels must change'
        click('[aria-label="暂停"]');assert until(lambda:evaluate('d.defaultView.__LAYMESH_PREVIEW_MEDIA.tracks.every(t=>t.element.paused&&(!t.sound||t.sound.paused))'),'Global pause')
        click('[aria-label="开始"]');assert until(lambda:evaluate('d.defaultView.__LAYMESH_PREVIEW_MEDIA.tracks.every(t=>!t.element.paused&&(!t.sound||!t.sound.paused))'),'Resume media')
        click('[aria-label="暂停"]')
        (a.output/'results.json').write_text(json.dumps({'passed':True,'host':'VS Code Linux actual webview','playing':playing,'checks':['media resource decoding','H.264 video with MP3 compatible audio','CDP user gesture play action','rendered video pixels change','global pause and resume']},indent=2));print('Actual VS Code video/audio playback, changing pixels, pause and resume passed')
    finally:
        for c in sessions:
            try:c.close()
            except Exception:pass
        try:os.killpg(host.pid,signal.SIGTERM)
        except ProcessLookupError:pass
        try:host.wait(timeout=10)
        except subprocess.TimeoutExpired:os.killpg(host.pid,signal.SIGKILL);host.wait()
        log.close()
