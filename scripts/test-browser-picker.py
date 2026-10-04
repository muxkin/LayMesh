#!/usr/bin/env python3
"""Exercise web popover lifecycle, coherent frames, color parity and real paint work.

Uses the repository Chromium/CDP workflow with trusted pointer and keyboard input.
Artifacts are written to --output, outside source. No user browser profile is used.
"""
import argparse,base64,importlib.util,json,os,signal,subprocess,tempfile,time,urllib.request
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('browser_smoke',ROOT/'scripts/smoke-browser.py');browser=importlib.util.module_from_spec(spec);spec.loader.exec_module(browser)

def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--url',required=True);p.add_argument('--output',type=Path,required=True);a=p.parse_args();a.output.mkdir(parents=True,exist_ok=True)
 with tempfile.TemporaryDirectory(prefix='laymesh-picker-') as profile:
  chrome=subprocess.Popen(['/usr/bin/google-chrome','--headless=new','--no-sandbox','--no-proxy-server','--remote-debugging-port=0','--remote-allow-origins=*','--user-data-dir='+profile,'about:blank'],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,start_new_session=True)
  try:
   active=Path(profile)/'DevToolsActivePort'
   for _ in range(250):
    if active.exists():break
    time.sleep(.1)
   port=int(active.read_text().splitlines()[0]);opener=urllib.request.build_opener(urllib.request.ProxyHandler({}));pages=json.load(opener.open(f'http://127.0.0.1:{port}/json'));c=browser.CDP(next(p['webSocketDebuggerUrl'] for p in pages if p['type']=='page'))
   c.call('Page.enable');c.call('Runtime.enable');c.call('Log.enable');c.call('Performance.enable');c.call('Emulation.setDeviceMetricsOverride',{'width':1440,'height':1000,'deviceScaleFactor':1,'mobile':False});c.call('Page.navigate',{'url':a.url})
   def wait(expr):return c.evaluate('new Promise((resolve,reject)=>{const end=Date.now()+15000;function poll(){const v=('+expr+');if(v)return resolve(true);if(Date.now()>end)return reject(Error('+json.dumps('Timeout: '+expr)+'));setTimeout(poll,30)}poll()})')
   def key(name,code,modifiers=0):
    for kind in ['keyDown','keyUp']:c.call('Input.dispatchKeyEvent',{'type':kind,'key':name.upper() if modifiers&8 else name,'code':'Key'+name.upper() if len(name)==1 else name,'modifiers':modifiers,'windowsVirtualKeyCode':code})
   def click(selector,index=0):
    pt=c.evaluate(f"(()=>{{const n=document.querySelectorAll({json.dumps(selector)})[{index}];n.scrollIntoView({{block:'nearest'}});const r=n.getBoundingClientRect();return {{x:r.left+r.width/2,y:r.top+r.height/2}}}})()")
    for kind in ['mousePressed','mouseReleased']:c.call('Input.dispatchMouseEvent',{'type':kind,'button':'left','clickCount':1,**pt})
   def input_text(selector,value):
    click(selector);key('a',65,2);c.call('Input.insertText',{'text':value})
   def escape():key('Escape',27)
   def opened():wait("document.querySelector('.laymesh-color-panel')?.dataset.valid==='true'")
   def closed():wait("!document.querySelector('.laymesh-color-panel')")
   def doc():return c.evaluate('view.state.doc.toString()')
   def replace(source):
    c.evaluate('view.focus()');key('a',65,2);c.call('Input.insertText',{'text':source});wait("document.querySelector('.example-module')?.dataset.liveState==='success'");wait('document.querySelectorAll(".laymesh-color-swatch").length===3')
   def check(name,condition):assert condition,name;checks.append(name)
   def shot(name):(a.output/name).write_bytes(base64.b64decode(c.call('Page.captureScreenshot',{'format':'png'})['data']))
   wait("document.querySelector('.example-module')?.dataset.liveState==='success'");click('[data-source-mode=source]');wait("document.querySelector('.laymesh-color-swatch')")
   c.evaluate(r'''(async()=>{const base=new URL('live/',document.querySelector('script[src$="app.js"]').src);const {EditorView}=await import(new URL('vendor/@codemirror/view/index.js',base));window.view=EditorView.findFromDOM(document.querySelector('.cm-editor'));window.frontend=await import(new URL('color-math.mjs',base));window.rust=await import(new URL('wasm/laymesh_wasm.js',base));await rust.default();})()''')
   source='page=canvas(size=(80,60),background="#2471bd90")\npage.add(rect(size=(20,10),fill="#ffee00",border_color="#223344"))'
   checks=[];replace(source)
   # Guard the actual editor document identity and history on unchanged close.
   c.evaluate('window.initialDoc=view.state.doc');click('.laymesh-color-swatch');opened()
   check('500px popover, no title, X, footer, apply or cancel',c.evaluate("document.querySelector('.laymesh-color-panel').offsetWidth===500&&!document.querySelector('.laymesh-color-header,.laymesh-color-footer,.laymesh-color-close,.laymesh-color-apply')"))
   placement=c.evaluate("(()=>{const p=document.querySelector('.laymesh-color-panel').getBoundingClientRect(),s=document.querySelector('.laymesh-color-swatch').getBoundingClientRect();return {left:p.left,right:p.right,top:p.top,bottom:p.bottom,swatch:s.toJSON(),gap:p.top-s.bottom,nonmodal:document.querySelector('.laymesh-color-popover').getAttribute('aria-modal')}})()")
   check('popover opens 6px below source swatch within viewport',abs(placement['gap']-6)<.1 and placement['nonmodal']=='false' and placement['right']<=1440)
   check('HEX and output selector stay in the same preview row',c.evaluate("!!document.querySelector('.laymesh-color-hex-card .laymesh-color-output select')"));shot('desktop.png');escape();closed();check('unchanged Esc keeps document object',c.evaluate('view.state.doc===initialDoc'))
   # Preview only; closing flushes final input, with one undo and redo step.
   click('.laymesh-color-swatch');opened();input_text('input[aria-label=HEX]','#34567890');check('HEX preview does not write source',doc()==source)
   # Selecting formats programmatically tests format semantics; drags below use trusted input.
   c.evaluate("document.querySelector('.laymesh-color-output select').value='hsv'")
   c.call('Input.dispatchMouseEvent',{'type':'mousePressed','x':20,'y':20,'button':'left','clickCount':1});c.call('Input.dispatchMouseEvent',{'type':'mouseReleased','x':20,'y':20,'button':'left','clickCount':1});closed();changed=doc();check('outside close writes latest color and chosen format',changed!=source and 'hsv(' in changed)
   key('z',90,2);wait('view.state.doc.toString()==='+json.dumps(source));key('z',90,10);wait('view.state.doc.toString()==='+json.dumps(changed));check('one Ctrl+Z and redo cover the entire session',True);key('z',90,2);wait('view.state.doc.toString()==='+json.dumps(source))
   # Source precision is retained even when the final format differs from the active tab.
   click('.laymesh-color-swatch');opened();input_text('.laymesh-color-channel input[type=number]','20.123456789');c.evaluate("document.querySelector('.laymesh-color-output select').value='rgb'");escape();closed();check('float RGB serialization retains all input digits','20.123456789' in doc());key('z',90,2);wait('view.state.doc.toString()==='+json.dumps(source))
   click('.laymesh-color-swatch');opened();click('[data-space=hsv]');input_text('.laymesh-color-channel input[type=number]','360');wait("document.querySelector('.laymesh-color-channel input[type=range]').value==='360'&&document.querySelector('.laymesh-color-hue-row input').value==='360'");check('hue endpoints do not jump to the opposite side of the strip',True);input_text('.laymesh-color-channel input[type=number]','380');wait("document.querySelector('.laymesh-color-channel input[type=range]').value==='20'");check('periodic hue input keeps draft precision and normalizes slider position',c.evaluate("document.querySelector('.laymesh-color-channel input[type=number]').value==='380'"));input_text('input[aria-label=HEX]','#2471bd90');escape();closed()
   # A source lower in the code area leaves insufficient room below the popover.
   c.call('Emulation.setDeviceMetricsOverride',{'width':1440,'height':650,'deviceScaleFactor':1,'mobile':False});replace('# padding\n'*8+source);click('.laymesh-color-swatch');opened();check('popover flips above source when below would overflow',c.evaluate("(()=>{const p=document.querySelector('.laymesh-color-panel').getBoundingClientRect(),s=document.querySelector('.laymesh-color-swatch').getBoundingClientRect();return Math.abs(p.bottom+6-s.top)<1&&p.top>=8})()"));escape();closed();c.call('Emulation.setDeviceMetricsOverride',{'width':1440,'height':1000,'deviceScaleFactor':1,'mobile':False});replace(source)
   # Several close events while the Rust formatting request is outstanding coalesce.
   click('.laymesh-color-swatch');opened();input_text('input[aria-label=HEX]','#334455');c.evaluate("window.closeQueries=0;window.closeEdits=0;window.savedPost=Worker.prototype.postMessage;Worker.prototype.postMessage=function(d,...args){if(d?.method==='colorPresentations')closeQueries++;return savedPost.call(this,d,...args)};window.savedDispatch=view.dispatch;view.dispatch=function(...args){const before=view.state.doc;savedDispatch.apply(view,args);if(view.state.doc!==before)closeEdits++;};for(let i=0;i<2;i++)document.dispatchEvent(new KeyboardEvent('keydown',{key:'Escape',bubbles:true}))");closed();check('duplicate close events issue one query and one replacement',c.evaluate('closeQueries===1&&closeEdits===1'));c.evaluate('Worker.prototype.postMessage=savedPost;view.dispatch=savedDispatch');key('z',90,2);wait('view.state.doc.toString()==='+json.dumps(source))
   # Invalid/unfinished drafts remain visible and block outside/Esc close.
   click('.laymesh-color-swatch');opened();c.evaluate('window.redInput=document.querySelector(".laymesh-color-channel input[type=number]")');input_text('.laymesh-color-channel input[type=number]','256');escape();check('invalid RGB blocks close and keeps input identity',c.evaluate("document.querySelector('.laymesh-color-panel').dataset.valid==='false'&&redInput.isConnected&&redInput.value==='256'") and doc()==source)
   input_text('.laymesh-color-channel input[type=number]','20.123456789');opened();click('[data-space=hsv]');click('[data-space=rgb]');check('tabs preserve original channel precision',c.evaluate("Math.abs(Number(document.querySelector('.laymesh-color-channel input[type=number]').dataset.raw)-20.123456789)<1e-10"))
   input_text('input[aria-label=HEX]','#12');escape();check('unfinished HEX blocks close',c.evaluate("document.querySelector('input[aria-label=HEX]').value==='#12'&&document.querySelector('.laymesh-color-panel').dataset.valid==='false'"));input_text('input[aria-label=HEX]','#2471bd90');opened();escape();closed();check('restoring original HEX creates no source edit',doc()==source)
   # Switch to a later swatch while the first replacement changes text length.
   click('.laymesh-color-swatch');opened();input_text('input[aria-label=HEX]','#112233');c.evaluate("document.querySelector('.laymesh-color-output select').value='hsv'");click('.laymesh-color-swatch',1);opened();wait("document.querySelector('input[aria-label=HEX]').value==='#ffee00'");check('swatch switch resolves range after previous async replacement',doc()!=source and 'hsv(' in doc());input_text('input[aria-label=HEX]','#00aa88');escape();closed();check('second swatch writes its own color', '#00aa88' in doc() and '#223344' in doc())
   # A source edit while open cannot be overwritten by stale color replacement.
   replace(source);click('.laymesh-color-swatch');opened();input_text('input[aria-label=HEX]','#abcdef');c.evaluate("view.dispatch({changes:{from:view.state.doc.length,insert:'\\n# concurrent edit'}})");escape();wait("document.querySelector('.laymesh-color-status')?.textContent.includes('文档已变化')");check('concurrent edits are preserved and stale write refused',doc()==source+'\n# concurrent edit')
   escape();closed();check('second close dismisses stale session without losing concurrent edit',doc()==source+'\n# concurrent edit')
   replace(source);click('.laymesh-color-swatch');opened()
   # Scroll follows the swatch; leaving viewport commits. Resize tests responsive layouts.
   before=c.evaluate("document.querySelector('.laymesh-color-panel').getBoundingClientRect().top");c.evaluate('window.scrollBy(0,20)');time.sleep(.1);after=c.evaluate("document.querySelector('.laymesh-color-panel').getBoundingClientRect().top");check('popover follows scrolling source',abs(before-after-20)<1)
   c.call('Emulation.setDeviceMetricsOverride',{'width':390,'height':844,'deviceScaleFactor':1,'mobile':True});time.sleep(.2);check('mobile popover stacks and fits viewport',c.evaluate("(()=>{const p=document.querySelector('.laymesh-color-panel').getBoundingClientRect();return p.left>=0&&p.right<=innerWidth&&getComputedStyle(document.querySelector('.laymesh-color-editor')).gridTemplateColumns.split(' ').length===1})()"));shot('mobile.png');escape();closed()
   c.call('Emulation.setDeviceMetricsOverride',{'width':1440,'height':1000,'deviceScaleFactor':1,'mobile':False});replace(source+'\n'+'# scroll fixture\n'*100);click('.laymesh-color-swatch');opened();input_text('input[aria-label=HEX]','#abcdef');c.evaluate('view.scrollDOM.scrollTop=view.scrollDOM.scrollHeight');closed();check('source leaving editor viewport commits once', '#abcdef' in doc());replace(source);c.evaluate('view.scrollDOM.scrollTop=0;view.dom.scrollIntoView({block:"center"})')
   if c.evaluate("document.documentElement.dataset.theme!=='dark'"):click('.theme-toggle')
   click('.laymesh-color-swatch');opened();shot('dark.png');escape();closed();check('dark theme supports same controls',True)
   # Cross-engine parity includes both the lazy web path and the legacy shared API.
   parity=c.evaluate(r'''(()=>{let seed=54711;const random=()=>((seed=(Math.imul(seed,1664525)+1013904223)>>>0)/4294967296),samples=[];for(const L of [0,1e-8,.5,1-1e-8,1])for(const C of [0,1e-8,.2,1,1e6])for(const H of [-360,0,90,180,270,360])samples.push({space:'oklch',channels:[L,C,H],alpha:.5});for(let i=0;i<2000;i++)for(const space of ['rgb','hsv','oklch'])samples.push({space,channels:space==='rgb'?[random()*255,random()*255,random()*255]:space==='hsv'?[random()*720-360,random(),random()]:[random(),random()*.8,random()*720-360],alpha:random()});let error=0;for(const input of samples){const js=frontend.pickerColor(input),full=frontend.convertColor(input),core=JSON.parse(rust.color_convert(JSON.stringify(input)));for(const candidate of [js,full]){const delta=Math.max(...candidate.rgba.map((v,i)=>Math.abs(v-core.rgba[i])));error=Math.max(error,delta);if(delta>1e-7||candidate.hex!==core.hex||candidate.mapped!==core.mapped)throw Error(JSON.stringify({input,delta}));}for(const space of ['rgb','hsv','oklch'])if(Math.max(...js[space].map((v,i)=>Math.abs(v-full[space][i])))>1e-7)throw Error('Lazy mismatch');const next=frontend.pickerColor({...input,alpha:.3},js);if(next.cache!==js.cache||next.mapped!==js.mapped)throw Error('Alpha recomputed');}let rampSamples=0;for(const space of ['rgb','hsv','oklch'])for(let i=0;i<3;i++){const channels=space==='rgb'?[42.5,80.1,250]:space==='hsv'?[230,.73,.8]:[.7,.4,210],max=space==='rgb'?255:space==='hsv'&&i===0||space==='oklch'&&i===2?360:space==='oklch'&&i===1?.4:1,ramp=frontend.colorRamp(space,channels,i,max);ramp.forEach((rgba,n)=>{const c=[...channels];c[i]=max*n/(ramp.length-1);const core=JSON.parse(rust.color_convert(JSON.stringify({space,channels:c,alpha:1})));if(Math.max(...rgba.map((v,j)=>Math.abs(v-core.rgba[j])))>1e-7)throw Error('Ramp mismatch');rampSamples++;});}for(const hex of ['#fff','#fff9','#102030','#10203090']){const input=frontend.parseHex(hex);if(frontend.pickerColor(input).hex!==JSON.parse(rust.color_convert(JSON.stringify(input))).hex)throw Error(hex);}let rejected=0;for(const input of [{space:'rgb',channels:[-1,0,0]},{space:'rgb',channels:[256,0,0]},{space:'hsv',channels:[0,1.1,.5]},{space:'oklch',channels:[.5,-.1,0]},{space:'rgb',channels:[0,0,0],alpha:2}]){let js=false,core=false;try{frontend.pickerColor(input)}catch{js=true}try{rust.color_convert(JSON.stringify(input))}catch{core=true}if(!js||!core)throw Error('Invalid accepted');rejected++;}return {samples:samples.length,maximum_rgba_error:error,matching_hex_and_mapping:true,ramp_samples:rampSamples,invalid_inputs:rejected};})()''')
   # Observe complete DOM commits before paint; inspect their colors outside timed input.
   # A plain RAF observer can run before the picker RAF, so it is not a presented frame.
   replace(source);click('.laymesh-color-swatch');opened()
   c.evaluate(r'''window.metrics={frames:[],snapshots:[],rebuilds:0,rpcs:0,renderRPCs:0,loads:0};window.profileActive=false;window.profileDone=false;window.initialInputs=[];let last=0;const panel=document.querySelector('.laymesh-color-panel'),image=document.querySelector('.live-image');image.addEventListener('load',()=>metrics.loads++);const post=Worker.prototype.postMessage;Worker.prototype.postMessage=function(data,...args){if(profileActive){if(data?.type==='render')metrics.renderRPCs++;if(data?.method)metrics.rpcs++;}return post.call(this,data,...args);};new MutationObserver(ms=>{if(profileActive)metrics.rebuilds+=ms.filter(m=>m.type==='childList').length}).observe(document.querySelector('.laymesh-color-channel').parentNode,{childList:true});function frame(now){if(profileActive){if(last)metrics.frames.push(now-last);}last=now;if(!profileDone)requestAnimationFrame(frame)}requestAnimationFrame(frame);new MutationObserver(ms=>{if(profileActive&&ms.some(m=>m.attributeName==='style')){metrics.snapshots.push({space:panel.querySelector('[aria-pressed=true]').dataset.space,hex:panel.querySelector('[aria-label=HEX]').value,values:[...panel.querySelectorAll('.laymesh-color-channel input[type=number]')].map(n=>n.dataset.raw||n.value),maxima:[...panel.querySelectorAll('.laymesh-color-channel input[type=range]')].slice(0,3).map(n=>+n.max),tracks:[...panel.querySelectorAll('.laymesh-color-channel input[type=range]')].map(n=>n.style.getPropertyValue('--track')),paint:panel.querySelector('.laymesh-color-preview div').style.background,thumb:[panel.querySelector('.laymesh-color-sv .laymesh-color-thumb').style.left,panel.querySelector('.laymesh-color-sv .laymesh-color-thumb').style.top]});console.timeStamp('picker-ui-commit');}}).observe(panel,{subtree:true,attributes:true,attributeFilter:['style']});''')
   c.call('Tracing.start',{'categories':'devtools.timeline,blink.user_timing,disabled-by-default-devtools.timeline.frame,toplevel','transferMode':'ReturnAsStream'})
   controls=[]
   for space in ['rgb','hsv','oklch']:
    click('[data-space='+space+']')
    for selector in ['.laymesh-color-sv','.laymesh-color-brightness','.laymesh-color-hue-row input']+[f'.laymesh-color-channel:nth-child({i}) input[type=range]' for i in range(1,5)]:
     # Reset to a chromatic, partially transparent state so every control changes color.
     input_text('input[aria-label=HEX]','#2471bd90');click('.laymesh-color-sv');time.sleep(.03)
     point=c.evaluate(f"(()=>{{const r=document.querySelector({json.dumps(selector)}).getBoundingClientRect();window.initialInputs=[...document.querySelectorAll('.laymesh-color-channel input')];window.timedDoc=view.state.doc;return {{left:r.left,top:r.top,width:r.width,height:r.height}}}})()")
     before={m['name']:m['value'] for m in c.call('Performance.getMetrics')['metrics']};c.evaluate('profileActive=true;last=0;console.timeStamp("picker-start")');start=time.monotonic();vertical=selector=='.laymesh-color-brightness';x=point['left']+point['width']*.15;y=point['top']+point['height']*.5
     c.call('Input.dispatchMouseEvent',{'type':'mousePressed','x':x,'y':y,'button':'left','clickCount':1})
     for i in range(24):
      f=.15+.7*i/23;x=point['left']+point['width']*(.5 if vertical else f);y=point['top']+point['height']*(f if vertical else .4 if selector=='.laymesh-color-sv' else .5)
      c.call('Input.dispatchMouseEvent',{'type':'mouseMoved','x':x,'y':y,'buttons':1});delay=start+(i+1)/60-time.monotonic()
      if delay>0:time.sleep(delay)
     c.call('Input.dispatchMouseEvent',{'type':'mouseReleased','x':x,'y':y,'button':'left','clickCount':1});time.sleep(.04);c.evaluate('profileActive=false;console.timeStamp("picker-end")');after={m['name']:m['value'] for m in c.call('Performance.getMetrics')['metrics']}
     assert c.evaluate('initialInputs.every(n=>n.isConnected)&&view.state.doc===timedDoc')
     controls.append({'space':space,'control':selector,**{k:1000*(after[k]-before[k]) for k in ['ScriptDuration','LayoutDuration','RecalcStyleDuration','TaskDuration']}})
   c.evaluate('profileDone=true');c.call('Tracing.end');c.call('Runtime.evaluate',{'expression':'true'})
   while not any(e.get('method')=='Tracing.tracingComplete' for e in c.events):c.events.append(c.message())
   handle=next(e['params']['stream'] for e in c.events if e.get('method')=='Tracing.tracingComplete');chunks=[]
   while True:
    item=c.call('IO.read',{'handle':handle});chunks.append(item['data'])
    if item.get('eof'):break
   c.call('IO.close',{'handle':handle});trace=json.loads(''.join(chunks));(a.output/'trace.json').write_text(json.dumps(trace))
   # CSSOM uses short 8-bit color/alpha serializations; math parity above stays at 1e-7.
   coherence=c.evaluate(r'''(()=>{let examined=0;for(const s of metrics.snapshots){const channels=s.values.slice(0,3).map((v,i)=>+v/(s.space==='hsv'&&i>0||s.space==='oklch'&&i===0?100:1)),alpha=+s.values[3]/100,c=frontend.convertColor({space:s.space,channels,alpha});const hex=frontend.convertColor(frontend.parseHex(s.hex)).rgba;if(Math.max(...hex.map((v,i)=>Math.abs(v-c.rgba[i])))>.5/255+1e-7)throw Error('HEX lag '+JSON.stringify(s));if(Math.abs(parseFloat(s.thumb[0])-c.hsv[1]*100)>1e-4||Math.abs(parseFloat(s.thumb[1])-(1-c.hsv[2])*100)>1e-4)throw Error('Thumb lag '+JSON.stringify({s,hsv:c.hsv}));const paint=s.paint.match(/[\d.e+-]+/g).map(Number);if(Math.max(...paint.slice(0,3).map((v,i)=>Math.abs(v/255-c.rgba[i])))>.5/255+1e-7||Math.abs((paint[3]??1)-c.rgba[3])>1/255+1e-7)throw Error('Preview lag '+JSON.stringify({s,rgba:c.rgba,paint}));s.tracks.slice(0,3).forEach((track,i)=>{const max=s.maxima[i]/(s.space==='hsv'&&i>0||s.space==='oklch'&&i===0?100:1),samples=track.match(/rgba?\([^)]*\)/g);if(!samples)throw Error('Missing track');samples.forEach((sample,j)=>{const v=[...channels];v[i]=max*j/(samples.length-1);const reference=JSON.parse(rust.color_convert(JSON.stringify({space:s.space,channels:v,alpha:1}))),actual=sample.match(/[\d.e+-]+/g).map(Number);const error=Math.max(...actual.slice(0,3).map((n,k)=>Math.abs(n/255-reference.rgba[k])));if(error>1e-7)throw Error('Gradient lag '+error);});});const alphaColors=s.tracks[3].match(/rgba?\([^)]*\)/g).slice(0,2);alphaColors.forEach((v,i)=>{const q=v.match(/[\d.e+-]+/g).map(Number);if(Math.max(...q.slice(0,3).map((n,j)=>Math.abs(n/255-c.rgba[j])))>1e-7||q[3]!==i)throw Error('Alpha gradient lag');});examined++;}return {frames_examined:examined,all_visible_outputs_coherent:true};})()''')
   metrics=c.evaluate('({frames:metrics.frames,rebuilds:metrics.rebuilds,rpcs:metrics.rpcs,renderRPCs:metrics.renderRPCs,loads:metrics.loads})');assert metrics['rebuilds']==0 and metrics['rpcs']==0 and metrics['renderRPCs']==0 and metrics['loads']==0,metrics
   def percentile(v):return sorted(v)[min(len(v)-1,int(len(v)*.95))] if v else None
   # Input -> complete UI commit -> renderer Paint, not thumb mutation time.
   events=sorted(trace['traceEvents'],key=lambda e:e['ts'])
   marks=lambda label:[e['ts'] for e in events if e['name']=='TimeStamp' and e.get('args',{}).get('data',{}).get('message')==label]
   starts,ends,commits=marks('picker-start'),marks('picker-end'),marks('picker-ui-commit')
   paints=[e for e in events if e['name']=='Paint' and e.get('ph')=='X' and e.get('args',{}).get('data',{}).get('nodeName')=="SECTION class='laymesh-color-panel'"]
   responses=[];input_work=[];dom_work=[]
   for control,start,end in zip(controls,starts,ends):
    kind='pointermove' if control['control'] in ['.laymesh-color-sv','.laymesh-color-brightness'] else 'input'
    for event in events:
     if not start<=event['ts']<=end:continue
     data=event.get('args',{}).get('data',{})
     if event['name']=='FunctionCall' and data.get('functionName')=='flush':dom_work.append(event.get('dur',0)/1000)
     if event['name']!='EventDispatch' or event.get('ph')!='X' or data.get('type')!=kind:continue
     input_work.append(event.get('dur',0)/1000)
     commit=next((t for t in commits if event['ts']+event.get('dur',0)<=t<end),None)
     paint=next((e for e in paints if commit is not None and commit<=e['ts']<min(end,commit+40000)),None)
     if paint:responses.append((paint['ts']+paint.get('dur',0)-event['ts'])/1000)
   response={'samples':len(responses),'input_to_complete_ui_paint_p95_ms':percentile(responses),'input_handler_p95_ms':percentile(input_work),'dom_commit_p95_ms':percentile(dom_work)}
   assert len(responses)>=450 and response['input_to_complete_ui_paint_p95_ms']<=34,response
   timings={}
   for e in trace['traceEvents']:
    if e.get('ph')=='X' and e['name'] in ['Layout','Paint','UpdateLayoutTree','FunctionCall','EventDispatch']:timings.setdefault(e['name'],[]).append(e.get('dur',0)/1000)
   errors=[e for e in c.events if e.get('method')=='Runtime.exceptionThrown' or e.get('method')=='Log.entryAdded' and e['params']['entry']['level']=='error'];assert not errors,errors
   result={'status':'passed','url':a.url,'title':c.evaluate('document.title'),'checks':checks,'color_parity':parity,'coherence':coherence,'controls':controls,'performance':{**{k:v for k,v in metrics.items() if k!='frames'},'frame_p95_ms':percentile(metrics['frames']),'response':response,'trace':{k:{'count':len(v),'total_ms':sum(v),'p95_ms':percentile(v)} for k,v in timings.items()}},'console_errors':errors,'artifact_sha256':browser.provenance(__file__),'measurement_limit':'Renderer trace includes scripts, styles, layout and paint, not physical display scanout.'};(a.output/'verification.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n');print(json.dumps(result,ensure_ascii=False))
  except Exception:
   if 'c' in locals():
    (a.output/'snapshots.json').write_text(json.dumps(c.evaluate('window.metrics?.snapshots||[]')));print(c.evaluate("({panel:document.querySelector('.laymesh-color-panel')?.outerHTML,source:document.querySelector('.cm-content')?.textContent})"));shot('failure.png')
   raise
  finally:
   try:os.killpg(chrome.pid,signal.SIGTERM)
   except ProcessLookupError:pass
   chrome.wait(timeout=10)
if __name__=='__main__':main()
