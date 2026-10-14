#!/usr/bin/env python3
"""Exercise media in an isolated, real VS Code webview with CDP user gestures."""
import argparse,base64,importlib.util,io,json,os,signal,socket,subprocess,tempfile,time,urllib.request
from pathlib import Path
from PIL import Image,ImageChops,ImageStat
ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('cdp',ROOT/'scripts/smoke-browser.py');cdp=importlib.util.module_from_spec(spec);spec.loader.exec_module(cdp)
p=argparse.ArgumentParser();p.add_argument('--extension-dir',type=Path,default=ROOT/'target/media-vscode');p.add_argument('--file',type=Path,default=ROOT/'target/media-trial/first.lay');p.add_argument('--output',type=Path,default=Path('/tmp/laymesh-media-vscode'));p.add_argument('--code',default='/usr/share/code/code');p.add_argument('--controls',action='store_true');a=p.parse_args();
if a.controls and a.file==ROOT/'target/media-trial/first.lay':a.file=ROOT/'examples/media/controls.laydoc'
unbound=a.controls and a.file.name=='manual.lay'
a.output.mkdir(parents=True,exist_ok=True)
with tempfile.TemporaryDirectory(prefix='laymesh-media-vscode-')as temp:
    t=Path(temp);user=t/'profile/User';user.mkdir(parents=True);(user/'settings.json').write_text(json.dumps({'laymesh.language':'zh-CN','workbench.colorTheme':'VS Code Dark','window.autoDetectColorScheme':False,'security.workspace.trust.enabled':False,'security.workspace.trust.startupPrompt':'never','security.workspace.trust.untrustedFiles':'open'}))
    import shutil
    source=t/'sources';source.mkdir();
    for file in a.file.parent.iterdir():
        if file.is_file()and file.suffix in ['.lay','.laydoc','.mp4','.wav']:shutil.copy2(file,source/file.name)
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
            if 'evaluate' in globals():
                try:(a.output/'failure.json').write_text(json.dumps(evaluate('({hidden:d.hidden,text:d.body.innerText.slice(-1500),media:d.defaultView.__LAYMESH_PREVIEW_MEDIA.tracks.map(t=>({target:t.cue.target,started:t.started,paused:t.element.paused,time:t.element.currentTime})),presentation:(()=>{const p=d.defaultView.laymeshPresentation;return {opened:p.opened,ready:p.ready,error:p.root?.dataset.error,message:p.message?.textContent,media:p.media?.tracks.map(t=>({target:t.cue.target,paused:t.element.paused,time:t.element.currentTime}))};})()})'),indent=2))
                except Exception:pass
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
        until(lambda:evaluate('d.defaultView.__LAYMESH_PREVIEW_MEDIA.tracks.length==='+str(1 if unbound else 5 if a.controls else 2)+'&&d.defaultView.__LAYMESH_PREVIEW_MEDIA.tracks.every(t=>t.element.readyState>=2)'), 'Media decoding')
        def click(selector):
            outer=root.evaluate("[...document.querySelectorAll('iframe')].find(e=>e.getBoundingClientRect().width>0).getBoundingClientRect().toJSON()");inner=c.evaluate("document.querySelector('iframe').getBoundingClientRect().toJSON()");button=evaluate('d.querySelector('+json.dumps(selector)+').getBoundingClientRect().toJSON()');point={'x':outer['x']+inner['x']+button['x']+button['width']/2,'y':outer['y']+inner['y']+button['y']+button['height']/2}
            root.call('Input.dispatchMouseEvent',{'type':'mouseMoved',**point})
            root.call('Page.bringToFront')
            for kind in ['mousePressed','mouseReleased']:root.call('Input.dispatchMouseEvent',{'type':kind,'button':'left','clickCount':1,**point})
            c.call('Runtime.evaluate',{'expression':'document.querySelector(\"iframe\").contentDocument.querySelector('+json.dumps(selector)+').click()','userGesture':True})
        if a.controls:
            until(lambda:evaluate('d.querySelector("main").dataset.painted==="true"'), 'Initial native canvas')
            def action(selector):
                result=c.call('Runtime.evaluate',{'expression':'document.querySelector("iframe").contentDocument.querySelector('+json.dumps(selector)+').click()','userGesture':True})
                assert 'exceptionDetails' not in result, result.get('exceptionDetails')
            def input_value(selector,value):
                c.call('Runtime.evaluate',{'expression':'(()=>{const e=document.querySelector("iframe").contentDocument.querySelector('+json.dumps(selector)+');e.value='+json.dumps(str(value))+';e.dispatchEvent(new Event("input",{bubbles:true}));e.dispatchEvent(new Event("change",{bubbles:true}));})()','userGesture':True})
            target='v' if unbound else 'manual';base='.media-controls[data-video-target="'+target+'"] ';track='d.defaultView.__LAYMESH_PREVIEW_MEDIA.tracks.find(t=>t.cue.target==='+json.dumps(target)+')'
            action(base+'.media-central-play')
            until(lambda:evaluate('('+track+').element.currentTime>.2'),'Independent native video')
            assert evaluate('d.defaultView.__LAYMESH_PREVIEW_MEDIA.tracks.every(t=>t.cue.target==='+json.dumps(target)+'||t.element.paused)&&d.defaultView.__LAYMESH_PREVIEW_MEDIA.clock.time===0')
            if unbound:assert evaluate('d.defaultView.__LAYMESH_PREVIEW_MEDIA.list.media.cues.length===0')
            until(lambda:evaluate('(()=>{const t='+track+';return t.sound?.readyState>=2&&Math.abs(t.element.currentTime-t.sound.currentTime)<.08;})()'),'Compatible audio sync')
            input_value(base+'.media-progress',4)
            until(lambda:evaluate('(()=>{const t='+track+';return !t.element.seeking&&t.element.currentTime>=4&&Math.abs(t.element.currentTime-t.sound.currentTime)<.08;})()'),'Native local seek and sound')
            input_value(base+'.media-volume',.3)
            assert evaluate('(()=>{const t='+track+';return t.volume===.3&&t.sound.volume===.3&&t.element.muted;})()')
            action(base+'[data-media-action="mute"]');assert evaluate('('+track+').sound.muted')
            action(base+'[data-media-action="mute"]');assert evaluate('!('+track+').sound.muted&&('+track+').element.muted')
            action(base+'.media-controlbar [data-media-action="play"]');assert until(lambda:evaluate('('+track+').element.paused&&('+track+').sound.paused'),'Local native pause')
            if not unbound:
                action('.media-controls[data-video-target="step_video"] .media-central-play')
                until(lambda:evaluate('d.defaultView.__LAYMESH_PREVIEW_MEDIA.tracks.find(t=>t.cue.target==="step_video").element.currentTime>1.1'),'Manual play before native step')
                assert evaluate('d.defaultView.__LAYMESH_PREVIEW_MEDIA.clock.active===-1&&d.defaultView.__LAYMESH_PREVIEW_MEDIA.tracks.find(t=>t.cue.scope==="document").element.paused')
                action('.media-controls[data-video-target="step_video"] .media-controlbar [data-media-action="play"]')
            until(lambda:evaluate('d.querySelector("main").dataset.painted==="true"'), 'Native painted geometry')
            geometry=evaluate('({figure:d.querySelector(".figure-pixels").getBoundingClientRect().toJSON(),view:d.defaultView.__LAYMESH_PREVIEW_MEDIA.getViewport(),boxes:[...d.querySelectorAll(".media-controls")].map(e=>({target:e.dataset.videoTarget,rect:e.getBoundingClientRect().toJSON()}))})')
            assert abs(geometry['figure']['width']-(100 if unbound else 160)*geometry['view']['matrix'][0])<2,'Canvas CSS size must match control coordinates at high DPR'
            assert all(abs(b['rect']['width']-(80 if unbound else 64)*geometry['view']['matrix'][0])<2 for b in geometry['boxes'])
            (a.output/'geometry.json').write_text(json.dumps(geometry,indent=2))
            image=root.call('Page.captureScreenshot',{'format':'png','captureBeyondViewport':False});(a.output/'controls.png').write_bytes(base64.b64decode(image['data']))
            if not unbound:
                action(base+'.media-central-play')
                until(lambda:evaluate('!('+track+').element.paused'), 'Local playback before native menu')
                evaluate('(()=>{d.querySelector(".presentation-choice").open=true;return true;})()')
                until(lambda:evaluate('d.defaultView.__LAYMESH_PREVIEW_MEDIA.locked&&('+track+').element.paused'), 'Native menu suspends local video')
                action('.presentation-choice .toolbar-popover button')
                until(lambda:evaluate('d.defaultView.laymeshPresentation.ready'), 'Native windowed presentation')
                until(lambda:evaluate('d.defaultView.laymeshPresentation.media.tracks.length===5&&d.defaultView.laymeshPresentation.media.tracks.every(t=>t.element.readyState>=2)'), 'Native windowed media decoding')
                assert evaluate('d.defaultView.__LAYMESH_PREVIEW_MEDIA.tracks.every(t=>t.element.paused)'),'Closing the entry menu must keep preview media suspended'
                evaluate('(()=>{d.defaultView.laymeshPresentation.media.pause();return true;})()')
                action('.presentation [data-video-target="manual"] .media-central-play')
                assert evaluate('d.defaultView.laymeshPresentation.media.tracks.find(t=>t.cue.target==="manual").manualOverride')
                until(lambda:evaluate('!d.defaultView.laymeshPresentation.media.tracks.find(t=>t.cue.target==="manual").element.paused'), 'Native windowed independent video')
                assert evaluate('d.defaultView.laymeshPresentation.clock.time===0&&d.defaultView.laymeshPresentation.media.tracks.every(t=>t.cue.target==="manual"||t.element.paused)')
                c.call('Runtime.evaluate',{'expression':'document.querySelector("iframe").contentDocument.defaultView.laymeshPresentation.close()','awaitPromise':True})
                until(lambda:evaluate('!('+track+').element.paused&&d.defaultView.__LAYMESH_PREVIEW_MEDIA.tracks.every(t=>t.cue.target==="manual"||t.element.paused)'), 'Native preview restores local state without music')
                action(base+'.media-controlbar [data-media-action="play"]')
            (a.output/'results.json').write_text(json.dumps({'passed':True,'host':'VS Code Linux actual webview','checks':['page without cues' if unbound else 'five tracks including unbound video','independent controls without music/page clock','MP3 compatible audio sync','trim-relative local seek','sidecar volume/mute','independent pause','high DPR canvas/control alignment']+([] if unbound else ['play before step','menu suspension','windowed local controls','preview restore without music'])},indent=2))
            print('Native video controls, isolated transport, seek, volume/mute and compatible audio passed')
        else:
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
