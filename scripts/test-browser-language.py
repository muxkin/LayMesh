#!/usr/bin/env python3
"""Verify actual browser language UI, WASM protocol, and output-mutation rejection."""
import argparse, base64, hashlib, importlib.util, json, os, shutil, signal, subprocess, tempfile, time, urllib.request
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('browser_smoke',ROOT/'scripts/smoke-browser.py');browser=importlib.util.module_from_spec(spec);spec.loader.exec_module(browser)

def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--url',required=True);p.add_argument('--chrome',default='/usr/bin/google-chrome');p.add_argument('--output',type=Path,default=ROOT/'release/comparison/revision-language-browser');a=p.parse_args();a.output.mkdir(parents=True,exist_ok=True)
 with tempfile.TemporaryDirectory(prefix='laymesh-language-browser-',ignore_cleanup_errors=True) as profile:
  chrome=subprocess.Popen([a.chrome,'--headless=new','--no-sandbox','--disable-gpu','--no-proxy-server','--remote-debugging-port=0','--remote-allow-origins=*','--user-data-dir='+profile,'about:blank'],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,start_new_session=True)
  try:
   active=Path(profile)/'DevToolsActivePort';deadline=time.monotonic()+25
   while not active.exists():
    if time.monotonic()>deadline:raise RuntimeError('Chromium startup timeout')
    time.sleep(.1)
   port=int(active.read_text().splitlines()[0]);opener=urllib.request.build_opener(urllib.request.ProxyHandler({}));pages=json.load(opener.open(f'http://127.0.0.1:{port}/json'));cdp=browser.CDP(next(p['webSocketDebuggerUrl'] for p in pages if p['type']=='page'));cdp.call('Page.enable');cdp.call('Runtime.enable');cdp.call('Log.enable');cdp.call('Emulation.setDeviceMetricsOverride',{'width':1440,'height':1000,'deviceScaleFactor':1,'mobile':False});cdp.call('Page.navigate',{'url':a.url})
   def wait(expression):
    return cdp.evaluate('new Promise((resolve,reject)=>{const end=Date.now()+12000;function poll(){const result=('+expression+');if(result)return resolve(result);if(Date.now()>end)return reject(Error("Timed out: "+'+json.dumps(expression)+'));setTimeout(poll,40)}poll()})')
   wait("document.querySelector('.example-module')?.dataset.liveState==='success'");cdp.evaluate("document.querySelector('[data-source-mode=source]').click()");wait("!!document.querySelector('.cm-content')")
   def key(name,code,modifiers=2):
    for kind in ['keyDown','keyUp']:cdp.call('Input.dispatchKeyEvent',{'type':kind,'key':name,'code':{32:'Space',9:'Tab',27:'Escape'}.get(code,'Key'+name.upper()),'modifiers':modifiers,'windowsVirtualKeyCode':code})
   def replace(source):
    cdp.evaluate("document.querySelector('.cm-content').focus()");key('a',65);cdp.call('Input.insertText',{'text':source})
   def tooltip(selector,needle):return wait('(()=>{const t=document.querySelector('+json.dumps(selector)+')?.textContent;return t?.includes('+json.dumps(needle)+')?t:null})()')
   ui=[]
   for label,source in [('many candidates','page=canvas(size=(30,20))\nr=rect('),('one candidate','page=canvas(size=(30,20))\nr=rect(si')]:
    replace(source)
    candidates=tooltip('.cm-tooltip-autocomplete','size')
    details=wait("document.querySelector('.cm-completionInfo')?.textContent")
    cdp.evaluate('new Promise(resolve=>setTimeout(resolve,450))')
    assert not cdp.evaluate("!!document.querySelector('.signature-help')"),label+' opened full call help while typing'
    ui.append({'action':label+' while typing','candidates':candidates,'details':details,'full_signature':False})
    if label=='one candidate':
     shot=cdp.call('Page.captureScreenshot',{'format':'png'});(a.output/'completion-one-candidate.png').write_bytes(base64.b64decode(shot['data']))
   replace('## @')
   doc_tags=tooltip('.cm-tooltip-autocomplete','@param')
   cdp.evaluate('new Promise(resolve=>setTimeout(resolve,450))')
   assert not cdp.evaluate("!!document.querySelector('.signature-help')"),'documentation tags opened full call help'
   ui.append({'action':'documentation tags while typing','candidates':doc_tags,'full_signature':False})
   for label,source,expected in [
    ('physical units','page=canvas(size=(30,20))\ntext(font_size=','pt'),
    ('style classes','style { .title {color:red;} }\ntext(class="ti','title'),
    ('factory return types','## @returns {plot} A plot.\nfunction makeplot() {return plot(size=(20,20))}\np=makeplot()\np.','line')]:
    replace(source);key(' ',32);text=tooltip('.cm-tooltip-autocomplete',expected);ui.append({'action':label,'tooltip':text})
   replace('page=canvas(size=(30,20))\ntext(content="hello",font_size=');key(' ',32,10);ui.append({'action':'signature help','tooltip':tooltip('.signature-help','font_size')})
   def hover(offset):
    point=cdp.evaluate('(async()=>{const base=new URL("live/",document.querySelector("script[src$=\\\"app.js\\\"]").src),{EditorView}=await import(new URL("vendor/@codemirror/view/index.js",base));const view=EditorView.findFromDOM(document.querySelector(".cm-editor"));view.contentDOM.scrollIntoView({block:"center"});const p=view.coordsAtPos('+str(offset)+');return {x:p.left+2,y:(p.top+p.bottom)/2}})()');cdp.call('Input.dispatchMouseEvent',{'type':'mouseMoved',**point})
   def editor():
    return cdp.evaluate(r'''(async()=>{const base=new URL('live/',document.querySelector('script[src$="app.js"]').src),{EditorView}=await import(new URL('vendor/@codemirror/view/index.js',base));return EditorView.findFromDOM(document.querySelector('.cm-editor')).state.doc.toString()})()''')
   bare='page=canvas(size=(80mm,60mm))\npage.add(line(length=40mm,start_cap=ro'
   replace(bare);key(' ',32);tooltip('.cm-tooltip-autocomplete','round');key('Tab',9,0)
   wait("document.querySelector('.cm-content').textContent.includes('start_cap=round')")
   assert editor()==bare[:-2]+'round'
   ui.append({'action':'Named option completion inserts the ordinary predefined variable without quotes','passed':True})
   quoted='# 中文 😀\npage=canvas(size=(80mm,60mm))\npage.add(line(length=40mm,start_cap="roxx"),anchor=top_left)'
   replace(quoted);key('Escape',27,0)
   cdp.evaluate(r'''(async()=>{const base=new URL('live/',document.querySelector('script[src$="app.js"]').src),{EditorView}=await import(new URL('vendor/@codemirror/view/index.js',base));const view=EditorView.findFromDOM(document.querySelector('.cm-editor'));view.dispatch({selection:{anchor:view.state.doc.toString().indexOf('roxx')+2}});view.focus()})()''')
   key(' ',32);tooltip('.cm-tooltip-autocomplete','round');key('Tab',9,0)
   wait("document.querySelector('.cm-content').textContent.includes('start_cap=\"round\"')")
   assert editor()==quoted.replace('roxx','round')
   ui.append({'action':'Quoted completion replaces the full suffix and preserves quotes after Chinese and emoji text','passed':True})
   variable_source='page=canvas(size=(80mm,60mm))\npage.add(line(length=40mm,start_cap=round),anchor=top_left)'
   replace(variable_source);hover(variable_source.index('round')+2)
   variable_hover=tooltip('.cm-tooltip-hover','round: string = "round"')
   assert 'line_cap' not in variable_hover
   ui.append({'action':'Predefined variable hover shows only its ordinary string type and value','tooltip':variable_hover})
   hover(variable_source.index('start_cap')+3);parameter_hover=tooltip('.cm-tooltip-hover','line_cap')
   assert all(value in parameter_hover for value in ['butt','round','square'])
   ui.append({'action':'Parameter hover explains all cap choices and the inherited default','tooltip':parameter_hover})
   replace('page=canvas(size=(80mm,60mm))\npage.add(line(length=40mm),anchor=');key(' ',32)
   anchor_choices=tooltip('.cm-tooltip-autocomplete','top_left');assert 'self' in anchor_choices
   ui.append({'action':'Anchor value completion offers the nine-point variables and self selectors','tooltip':anchor_choices})
   replace('page=canvas(size=(30,20))');hover(8);ui.append({'action':'Chinese hover documentation','tooltip':tooltip('.cm-tooltip-hover','创建页面')})
   replace('# 😀\nstyle {text {font-size:12;}}');wait("!!document.querySelector('.cm-lintRange-error')");hover(18);ui.append({'action':'LCSS diagnostic underline','underline':True,'tooltip':tooltip('.cm-tooltip-lint','font_size')})
   # Web popover: closing commits one guarded editor edit.
   color_source='page=canvas(size=(80,60))\npage.add(rect(size=(20,10),fill="#0072b290"))'
   replace(color_source);wait("!!document.querySelector('.laymesh-color-swatch')")
   cdp.evaluate("document.querySelector('.laymesh-color-swatch').click()")
   wait("document.querySelector('.laymesh-color-panel input[aria-label=HEX]')?.value==='#0072b290'")
   wait("document.querySelector('.laymesh-color-panel')?.dataset.valid==='true'")
   cdp.evaluate(r"""(async()=>{const base=new URL('live/',document.querySelector('script[src$="app.js"]').src),{EditorView}=await import(new URL('vendor/@codemirror/view/index.js',base));globalThis.pickerInitialDoc=EditorView.findFromDOM(document.querySelector('.cm-editor')).state.doc;})()""")
   cdp.evaluate("document.dispatchEvent(new KeyboardEvent('keydown',{key:'Escape',bubbles:true}))")
   wait("!document.querySelector('.laymesh-color-panel')")
   assert cdp.evaluate(r"""(async()=>{const base=new URL('live/',document.querySelector('script[src$="app.js"]').src),{EditorView}=await import(new URL('vendor/@codemirror/view/index.js',base));return EditorView.findFromDOM(document.querySelector('.cm-editor')).state.doc===globalThis.pickerInitialDoc;})()""")
   ui.append({'action':'Unchanged close leaves the editor document and undo history untouched','passed':True})
   cdp.evaluate("document.querySelector('.laymesh-color-swatch').click()")
   wait("document.querySelector('.laymesh-color-panel')?.dataset.valid==='true'")
   # Drafts stay in the same focused input; raw precision is independent of
   # the rounded display after a tab switch. Invalid input cannot be applied.
   def input_text(selector,value):
    cdp.evaluate('document.querySelector('+json.dumps(selector)+').focus()');key('a',65);cdp.call('Input.insertText',{'text':value})
   cdp.evaluate("window.pickerRed=document.querySelector('.laymesh-color-channel input[type=number]')")
   input_text('.laymesh-color-channel input[type=number]','256')
   wait("document.querySelector('.laymesh-color-panel')?.dataset.valid==='false'&&document.querySelector('.laymesh-color-status').textContent.length>0")
   assert cdp.evaluate("pickerRed.isConnected&&document.activeElement===pickerRed&&pickerRed.value==='256'")
   input_text('.laymesh-color-channel input[type=number]','20.123456789')
   wait("document.querySelector('.laymesh-color-panel')?.dataset.valid==='true'")
   assert cdp.evaluate("pickerRed.value==='20.123456789'")
   cdp.evaluate("document.querySelector('[data-space=hsv]').click();document.querySelector('[data-space=rgb]').click()")
   assert cdp.evaluate("Math.abs(Number(document.querySelector('.laymesh-color-channel input[type=number]').dataset.raw)-20.123456789)<1e-10")
   input_text('input[aria-label=HEX]','#12')
   wait("document.querySelector('.laymesh-color-panel')?.dataset.valid==='false'")
   assert cdp.evaluate("document.querySelector('input[aria-label=HEX]').value==='#12'")
   input_text('input[aria-label=HEX]','#0072b290')
   wait("document.querySelector('.laymesh-color-panel')?.dataset.valid==='true'")
   assert editor()==color_source
   ui.append({'action':'Invalid channels and unfinished HEX preserve focused drafts; space switching preserves raw precision','passed':True})
   # Pointer input on the actual two-dimensional picker, with no source edits.
   point=cdp.evaluate("(()=>{const r=document.querySelector('.laymesh-color-sv').getBoundingClientRect();return {x:r.left+r.width*.7,y:r.top+r.height*.3}})()")
   cdp.call('Input.dispatchMouseEvent',{'type':'mouseMoved',**point})
   cdp.call('Input.dispatchMouseEvent',{'type':'mousePressed','button':'left','clickCount':1,**point})
   cdp.call('Input.dispatchMouseEvent',{'type':'mouseReleased','button':'left','clickCount':1,**point})
   wait("document.querySelector('.laymesh-color-panel')?.dataset.valid==='true'&&document.querySelector('input[aria-label=HEX]').value!=='#0072b290'")
   assert cdp.evaluate("document.querySelector('.cm-content').textContent.includes('#0072b290')")
   cdp.evaluate("(()=>{const input=document.querySelector('input[aria-label=HEX]');input.focus();input.value='#0072b290';input.dispatchEvent(new Event('input',{bubbles:true}))})()")
   wait("document.querySelector('.laymesh-color-panel')?.dataset.valid==='true'&&Math.abs(Number(document.querySelector('[data-channel=alpha]').value)-144/255*100)<0.001")
   wait("document.querySelector('.laymesh-color-channel input[type=range]').style.getPropertyValue('--track').includes('linear-gradient')")
   ui.append({'action':'Mosaic SV pointer input, colored tracks, HEX-alpha synchronization and preview-only edits','passed':True})
   cdp.evaluate("document.querySelector('[data-space=hsv]').click();const input=document.querySelector('[data-channel=alpha]');input.value='40';input.dispatchEvent(new Event('input',{bubbles:true}));document.querySelector('.laymesh-color-output select').value='hsv'")
   wait("document.querySelector('.laymesh-color-panel')?.dataset.valid==='true'")
   assert not cdp.evaluate("document.querySelector('.cm-content').textContent.includes('hsv(')")
   wait("[...document.querySelectorAll('.laymesh-color-channel input[type=range]')].every(input=>input.style.getPropertyValue('--track').includes('linear-gradient'))")
   shot=cdp.call('Page.captureScreenshot',{'format':'png'});(a.output/'color-panel-desktop.png').write_bytes(base64.b64decode(shot['data']))
   cdp.evaluate("document.dispatchEvent(new KeyboardEvent('keydown',{key:'Escape',bubbles:true}))")
   wait("!document.querySelector('.laymesh-color-panel')")
   assert cdp.evaluate("document.querySelector('.cm-content').textContent.includes('hsv(')&&document.querySelector('.cm-content').textContent.includes('/ 0.4')")
   key('z',90);wait("document.querySelector('.cm-content').textContent.includes('#0072b290')")
   ui.append({'action':'Color swatch, HSV and alpha, preview, close commit and one-step undo','passed':True})
   cdp.evaluate("document.querySelector('.laymesh-color-swatch').click()")
   wait("document.querySelector('.laymesh-color-panel')?.dataset.valid==='true'")
   cdp.evaluate("document.querySelector('[data-space=oklch]').click()")
   wait("!!document.querySelector('.laymesh-color-channel')")
   wait("[...document.querySelectorAll('.laymesh-color-channel input[type=range]')].every(input=>input.style.getPropertyValue('--track').includes('linear-gradient'))")
   cdp.call('Emulation.setDeviceMetricsOverride',{'width':390,'height':844,'deviceScaleFactor':1,'mobile':True})
   shot=cdp.call('Page.captureScreenshot',{'format':'png'});(a.output/'color-panel-mobile.png').write_bytes(base64.b64decode(shot['data']))
   cdp.evaluate("document.dispatchEvent(new KeyboardEvent('keydown',{key:'Escape',bubbles:true}))")
   wait("!document.querySelector('.laymesh-color-panel')")
   assert cdp.evaluate("document.querySelector('.cm-content').textContent.includes('#0072b290')")
   cdp.call('Emulation.setDeviceMetricsOverride',{'width':1440,'height':1000,'deviceScaleFactor':1,'mobile':False})
   if cdp.evaluate("document.documentElement.dataset.theme!=='dark'"):cdp.evaluate("document.querySelector('.theme-toggle').click()")
   wait("document.documentElement.dataset.theme==='dark'")
   cdp.evaluate("document.querySelector('.laymesh-color-swatch').click()")
   wait("document.querySelector('.laymesh-color-panel')?.dataset.valid==='true'")
   wait("[...document.querySelectorAll('.laymesh-color-channel input[type=range]')].every(input=>input.style.getPropertyValue('--track').includes('linear-gradient'))")
   shot=cdp.call('Page.captureScreenshot',{'format':'png'});(a.output/'color-panel-dark.png').write_bytes(base64.b64decode(shot['data']))
   cdp.evaluate(r'''(async()=>{const base=new URL('live/',document.querySelector('script[src$="app.js"]').src),{EditorView}=await import(new URL('vendor/@codemirror/view/index.js',base));const view=EditorView.findFromDOM(document.querySelector('.cm-editor'));view.dispatch({changes:{from:view.state.doc.length,insert:'\n# concurrent edit'}});})()''')
   cdp.evaluate("document.dispatchEvent(new KeyboardEvent('keydown',{key:'Escape',bubbles:true}))")
   tooltip('.laymesh-color-status','文档已变化')
   assert cdp.evaluate("document.querySelector('.cm-content').textContent.includes('#0072b290')&&document.querySelector('.cm-content').textContent.includes('concurrent edit')")
   cdp.evaluate("document.dispatchEvent(new KeyboardEvent('keydown',{key:'Escape',bubbles:true}))")
   ui.append({'action':'OKLCH tab, mobile layout, unchanged close and stale-document refusal','passed':True})
   # The same Worker envelope used by the editor checks UTF-16, localization,
   # aliases, in-memory dependency replacement, and error recovery.
   protocol=cdp.evaluate(r'''(async()=>{
    const base=new URL('live/',document.querySelector('script[src$="app.js"]').src),w=new Worker(new URL('language-worker.js',base),{type:'module'});let seq=0;const pending=new Map();w.onmessage=({data})=>{pending.get(data.id)?.(data);pending.delete(data.id)};
    const query=(files,file,method,offset=0,locale='en')=>new Promise((resolve,reject)=>{const id=++seq,t=setTimeout(()=>reject(Error('Language worker timeout')),8000);pending.set(id,d=>{clearTimeout(t);d.error?reject(Error(d.error)):resolve(d.result)});w.postMessage({id,files,file,method,offset,locale})});
    const clone=v=>JSON.parse(JSON.stringify(v)),checks=[],faults=[];
    function verify(name,actual,predicate,mutations){if(!predicate(actual))throw Error(name+': '+JSON.stringify(actual));checks.push(name);for(const [label,mutate] of mutations){const broken=clone(actual);mutate(broken);if(predicate(broken))throw Error('Mutation survived '+name+': '+label);faults.push({check:name,mutation:label,rejected:true})}}
    try{
     const file='/main.lay',source='# 😀\nstyle {text {nonsense:1pt;}}\npage=canvas(size=(10deg,20))\nt=text(font_family=["sans",3])\nr=rect(border_width=true)',diagnostics=await query({[file]:source},file,'diagnostics');
     const from=source.indexOf('nonsense');verify('LCSS/types UTF16 diagnostics',diagnostics,d=>Array.isArray(d)&&d.map(v=>v.code).join(',')==='E_LCSS,E_UNIT,E_TYPE,E_TYPE'&&d[0].from===from&&d[0].to>from,[['drop diagnostic',d=>d.shift()],['wrong UTF16 offset',d=>d[0].from++],['missing code',d=>delete d[0].code]]);
     const unit='text(font_size=',units=await query({[file]:unit},file,'completions',unit.length);verify('unit completion envelope',units,d=>Array.isArray(d)&&['pt','mm','cm','auto'].every(label=>d.some(o=>o.label===label&&typeof o.apply==='string'&&o.from===unit.length)),[['remove pt',d=>d.splice(d.findIndex(o=>o.label==='pt'),1)],['missing insertion',d=>delete d.find(o=>o.label==='pt').apply]]);
     const lib='/lib.lay',caller='import {card as tile} from "./lib.lay"\ntile(',library='## @lang zh-CN\n## 中文面板。\n## @param title - 中文标题。\n## @returns {group} 组合。\n## @lang en\n## English panel.\n## @param title - English title.\n## @returns {group} Group.\nexport function card(title, size=(40,25)) {return group()}';
     for(const [locale,expected]of [['zh-CN','中文面板'],['en','English panel']]){const s=await query({[file]:caller,[lib]:library},file,'signature',caller.length,locale);verify('localized signature '+locale,s,d=>d&&d.summary?.includes(expected)&&d.name==='tile'&&d.parameters?.[0]?.name==='title'&&typeof d.parameters[0].description==='string',[['wrong locale text',d=>d.summary='wrong'],['missing parameter list',d=>delete d.parameters]])}
     const hover=await query({[file]:'page=canvas(size=(30,20))'},file,'hover',8,'en');verify('hover source range',hover,d=>d?.contents?.includes('Create a page')&&d.from===5&&d.to===11,[['wrong source range',d=>d.from=0],['missing documentation',d=>delete d.contents]]);
     const edited=await query({[file]:caller,[lib]:'## Changed unsaved dependency.\nexport function card(buffer=2) {return buffer}'},file,'signature',caller.length);if(!edited.label.includes('buffer=2'))throw Error('Unsaved module was not used');const missing=await query({[file]:caller},file,'signature',caller.length);if(missing!==null)throw Error('Deleted module survived');const recovered=await query({[file]:caller,[lib]:library},file,'signature',caller.length);if(!recovered.label.includes('title'))throw Error('Module recovery failed');checks.push('unsaved dependency -> missing -> restored state');
     const invalid='# 😀\n😀',bad=await query({[file]:invalid},file,'diagnostics');if(bad[0].from!==5||bad[0].to!==7)throw Error('Unicode diagnostic conversion failed');const restored=await query({[file]:'page=canvas(size=(30,20))'},file,'diagnostics');if(restored.length)throw Error('Unicode failure recovery failed');checks.push('Unicode syntax error -> valid recovery');
     return {checks,output_mutations:faults,product_code_mutation:false};
    }finally{w.terminate()}
   })()''')
   # Verify document language reaches the real help widget after page navigation.
   english=a.url.replace('/examples/','/en/examples/');cdp.call('Page.navigate',{'url':english});wait("document.documentElement.lang==='en'&&document.querySelector('.example-module')?.dataset.liveState==='success'");cdp.evaluate("document.querySelector('[data-source-mode=source]').click()");wait("!!document.querySelector('.cm-content')");replace('page=canvas(size=(30,20))');hover(8);ui.append({'action':'English hover documentation','tooltip':tooltip('.cm-tooltip-hover','Create a page')})
   shot=cdp.call('Page.captureScreenshot',{'format':'png'});(a.output/'language-ui.png').write_bytes(base64.b64decode(shot['data']))
   errors=[e for e in cdp.events if e.get('method')=='Runtime.exceptionThrown' or e.get('method')=='Log.entryAdded' and e['params']['entry']['level']=='error'];assert not errors,errors
   result={'status':'passed','artifact_sha256':browser.provenance(__file__),'ui':ui,'wasm_protocol':protocol,'console_errors':errors,'node_on_path':bool(shutil.which('node')),'npm_on_path':bool(shutil.which('npm')),'wasm_sha256':hashlib.sha256((ROOT/'site/dist/site/live/wasm/laymesh_wasm_bg.wasm').read_bytes()).hexdigest()};(a.output/'verification.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n');print(json.dumps(result,ensure_ascii=False))
  except Exception:
   if 'cdp' in locals():
    print(json.dumps(cdp.evaluate("({tooltips:[...document.querySelectorAll('.cm-tooltip')].map(e=>({class:e.className,text:e.textContent})),editor:document.querySelector('.cm-content')?.textContent})"),ensure_ascii=False),flush=True)
    print(json.dumps(cdp.evaluate(r'''(async()=>{const base=new URL('live/',document.querySelector('script[src$="app.js"]').src),{EditorView}=await import(new URL('vendor/@codemirror/view/index.js',base)),cm=await import(new URL('vendor/@codemirror/autocomplete/index.js',base)),core=await import(new URL('wasm/laymesh_wasm.js',base));await core.default();const view=EditorView.findFromDOM(document.querySelector('.cm-editor')),text=view.state.doc.toString(),pos=view.state.selection.main.head;return {source:text,selection:pos,completion_status:cm.completionStatus(view.state),backend:JSON.parse(core.language_query(JSON.stringify({'/main.lay':text}),'/main.lay','completions',pos,'zh-CN'))}})()'''),ensure_ascii=False),flush=True)
    shot=cdp.call('Page.captureScreenshot',{'format':'png'});(a.output/'failed-language-ui.png').write_bytes(base64.b64decode(shot['data']))
   raise
  finally:
   try:os.killpg(chrome.pid,signal.SIGTERM)
   except ProcessLookupError:pass
   try:chrome.wait(timeout=5)
   except subprocess.TimeoutExpired:os.killpg(chrome.pid,signal.SIGKILL);chrome.wait()
if __name__=='__main__':main()
