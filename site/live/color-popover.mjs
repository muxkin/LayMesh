// Web-only view. VS Code keeps color-panel.mjs and its existing apply/cancel flow.
import {pickerColor,parseHex,colorRamp} from './color-math.mjs';
const clamp=n=>Math.max(0,Math.min(1,n));
const rgbaCss=([r,g,b,a])=>`rgba(${r*255},${g*255},${b*255},${a})`;
const checker='conic-gradient(#bbc8c2 25%,#edf1ef 0 50%,#bbc8c2 0 75%,#edf1ef 0) 0 0/8px 8px';
const percent=(space,i)=>space==='hsv'&&i>0||space==='oklch'&&i===0;
const equal=(a,b)=>a.length===b.length&&a.every((v,i)=>v===b[i]);
const css=`
.laymesh-color-popover{position:fixed;inset:auto;margin:0;border:0;padding:0;background:transparent;overflow:visible;z-index:10000}
.laymesh-color-popover .laymesh-color-panel{--picker-bg:var(--surface,#fff);--picker-soft:var(--soft,#f5f7f9);--picker-text:var(--text,#20332f);--picker-line:var(--line,#dce3e8);--picker-accent:var(--accent,#376da8);box-sizing:border-box;width:min(500px,calc(100vw - 16px));max-height:calc(100dvh - 16px);overflow:auto;padding:12px;border:1px solid var(--picker-line);border-radius:10px;background:var(--picker-bg);color:var(--picker-text);font:12px/1.4 system-ui;box-shadow:0 8px 28px #0003;contain:layout paint}
.laymesh-color-popover *{box-sizing:border-box}.laymesh-color-popover button,.laymesh-color-popover input,.laymesh-color-popover select{font:inherit;color:inherit;border:1px solid var(--picker-line);border-radius:5px;background:var(--picker-soft);padding:5px 6px;min-width:0}.laymesh-color-popover button{cursor:pointer}.laymesh-color-popover :focus-visible{outline:2px solid var(--picker-accent);outline-offset:2px}
.laymesh-color-editor{display:grid;grid-template-columns:minmax(0,1fr) minmax(0,1.1fr);gap:12px}.laymesh-color-picker,.laymesh-color-fields{min-width:0}.laymesh-color-plane-row{display:grid;grid-template-columns:minmax(0,1fr) 16px;gap:8px;height:178px}.laymesh-color-sv,.laymesh-color-brightness{position:relative;overflow:hidden;border:1px solid #8886;border-radius:6px;touch-action:none;cursor:crosshair}.laymesh-color-sv{background:linear-gradient(to top in srgb,#000,transparent),linear-gradient(to right in srgb,#fff,transparent),var(--picker-hue)}.laymesh-color-thumb{position:absolute;width:13px;height:13px;border:2px solid white;border-radius:50%;box-shadow:0 0 0 1px #172333,0 1px 3px #0005;transform:translate(-50%,-50%);pointer-events:none}.laymesh-color-brightness{background:linear-gradient(to bottom in srgb,#fff,#000);cursor:ns-resize}.laymesh-color-brightness .laymesh-color-thumb{left:50%;width:14px;height:5px;border-radius:3px;background:#253342}
.laymesh-color-hue-row{display:grid;grid-template-columns:26px minmax(0,1fr) 30px;gap:5px;align-items:center;margin-top:10px}.laymesh-color-hue-row output{text-align:right;font-size:10px}.laymesh-color-range{appearance:none;width:100%;height:12px;padding:0!important;border:1px solid #8885!important;border-radius:4px!important;background:var(--track)!important;cursor:pointer}.laymesh-color-range::-webkit-slider-thumb{appearance:none;width:15px;height:15px;border:2px solid white;border-radius:50%;background:var(--thumb,#496783);box-shadow:0 0 0 1px #344254}.laymesh-color-range::-moz-range-thumb{width:11px;height:11px;border:2px solid white;border-radius:50%;background:var(--thumb,#496783);box-shadow:0 0 0 1px #344254}
.laymesh-color-hex-card{display:flex;gap:6px;align-items:center}.laymesh-color-preview{width:24px;height:24px;flex:none;border:1px solid #8885;border-radius:50%;overflow:hidden;background:${checker}}.laymesh-color-preview div{height:100%}.laymesh-color-hex-label{min-width:0;flex:1}.laymesh-color-hex-label input{width:100%;font-family:ui-monospace,monospace;background:var(--picker-bg)}.laymesh-color-output select{width:65px;font-size:10px;padding:5px 2px}.laymesh-color-tabs{display:flex;margin:10px 0 8px}.laymesh-color-tabs button{flex:1;border-radius:0}.laymesh-color-tabs button:first-child{border-radius:5px 0 0 5px}.laymesh-color-tabs button:last-child{border-radius:0 5px 5px 0}.laymesh-color-tabs button[aria-pressed=true]{background:var(--picker-accent);color:#fff}
.laymesh-color-channel{display:grid;grid-template-columns:32px minmax(35px,1fr) 70px;gap:6px;align-items:center;margin:8px 0}.laymesh-color-channel>span{font-size:10px}.laymesh-color-channel input[type=number]{width:100%;padding:4px}.laymesh-color-status{font-size:11px;color:var(--muted,#65766f);margin-top:8px}.laymesh-color-status:empty{display:none}@media(max-width:520px){.laymesh-color-editor{grid-template-columns:1fr;gap:10px}.laymesh-color-plane-row{height:155px}.laymesh-color-channel{grid-template-columns:45px minmax(0,1fr) 85px}}
`;
export function colorPopover(root,initial,apply,close,locale='en') {
 const zh=locale.startsWith('zh'),t=(a,b)=>zh?a:b;
 const initialState={space:initial.space==='hex'?'rgb':initial.space,channels:[...(initial.channels||initial.rgba.slice(0,3).map(v=>v*255))],alpha:initial.alpha??initial.rgba[3]};
 let state=initialState,result=pickerColor(state),model=initialState.space,valid=true,dead=false,stale=false,frame=0,submission=null,hueMemory=result.hsv[0],snapshot;
 const style=document.createElement('style');style.textContent=css;root.append(style);
 const el=(tag,text,parent,cls='')=>{const n=document.createElement(tag);if(text)n.textContent=text;if(cls)n.className=cls;parent.append(n);return n;};
 const panel=el('section','',root,'laymesh-color-panel');panel.setAttribute('aria-label',t('编辑 LayMesh 颜色','Edit LayMesh color'));panel.dataset.valid='true';
 const editor=el('div','',panel,'laymesh-color-editor'),picker=el('div','',editor,'laymesh-color-picker'),planeRow=el('div','',picker,'laymesh-color-plane-row');
 const plane=el('div','',planeRow,'laymesh-color-sv'),thumb=el('span','',plane,'laymesh-color-thumb');plane.tabIndex=0;plane.setAttribute('role','slider');plane.setAttribute('aria-label',t('饱和度与明度','Saturation and value'));
 const brightness=el('div','',planeRow,'laymesh-color-brightness'),valueThumb=el('span','',brightness,'laymesh-color-thumb');brightness.tabIndex=0;brightness.setAttribute('role','slider');brightness.setAttribute('aria-label',t('明度','Value'));brightness.setAttribute('aria-valuemin','0');brightness.setAttribute('aria-valuemax','100');
 const hueRow=el('label',t('色相','Hue'),picker,'laymesh-color-hue-row'),hue=el('input','',hueRow,'laymesh-color-range');hue.type='range';hue.min=0;hue.max=360;hue.step=.1;hue.setAttribute('aria-label',t('色相','Hue'));hue.style.setProperty('--track','linear-gradient(to right in srgb,#f00,#ff0,#0f0,#0ff,#00f,#f0f,#f00)');const hueOutput=el('output','',hueRow);
 const right=el('div','',editor,'laymesh-color-fields'),card=el('div','',right,'laymesh-color-hex-card'),preview=el('div','',card,'laymesh-color-preview'),paint=el('div','',preview);
 const hexLabel=el('label','',card,'laymesh-color-hex-label'),hex=el('input','',hexLabel);hex.setAttribute('aria-label','HEX');hex.title='HEX';hex.spellcheck=false;
 const output=el('label','',card,'laymesh-color-output'),format=el('select','',output);format.setAttribute('aria-label',t('源码格式','Source format'));for(const space of ['hex','rgb','hsv','oklch']){const n=el('option',space.toUpperCase(),format);n.value=space;}format.value=initial.space;
 const tabs=el('div','',right,'laymesh-color-tabs'),channelRoot=el('div','',right),status=el('div','',panel,'laymesh-color-status');status.setAttribute('role','status');
 let fields=[],alphaInput;const rampCache=new Map(),rects=new Map(),cleanup=[];
 // Compare before writes: property writes can trigger style/layout even if equal.
 const attr=(n,k,v)=>{v=String(v);if(n.getAttribute(k)!==v)n.setAttribute(k,v);};
 const prop=(n,k,v)=>{v=String(v);if(n[k]!==v)n[k]=v;};
 const cssValues=new WeakMap();
 const cssProp=(n,k,v)=>{let values=cssValues.get(n);if(!values){values=new Map();cssValues.set(n,values);}if(values.get(k)!==v){n.style.setProperty(k,v);values.set(k,v);}};
 const text=(n,v)=>{if(n.textContent!==v)n.textContent=v;};
 const channels=()=>state.space===model?state.channels:result[model];
 const number=n=>{const v=n.dataset.raw??n.value;return v.trim()===''?NaN:Number(v);};
 const inputValue=(n,v)=>{prop(n,'value',String(Number(v.toPrecision(6))));attr(n,'data-raw',v);};
 const hsv=()=>{const v=result.hsv;return [state.space==='hsv'?(state.channels[0]===360?360:v[0]):v[1]>1e-8&&v[2]>1e-8?v[0]:hueMemory,v[1],v[2]];};
 function row(label,value,max,index,isPercent=false) {
  const row=el('label','',channelRoot,'laymesh-color-channel');el('span',label+(isPercent?' %':''),row);
  const range=el('input','',row,'laymesh-color-range');range.type='range';range.min=0;range.max=max;range.step='any';range.setAttribute('aria-label',label+(isPercent?' (%)':'')+' slider');
  const input=el('input','',row);input.type='number';input.step='any';input.setAttribute('aria-label',label+(isPercent?' (%)':''));inputValue(input,value);input._range=range;range.value=value;
  input.addEventListener('input',()=>{input.dataset.raw=input.value;submitChannels(input);});
  range.addEventListener('input',()=>{inputValue(input,Number(range.value));submitChannels(input);});
  range.addEventListener('pointerup',flush);return input;
 }
 function renderFields() {
  channelRoot.replaceChildren();for(const tab of tabs.children)attr(tab,'aria-pressed',tab.dataset.space===model);
  const labels=model==='rgb'?['R','G','B']:model==='hsv'?['H','S','V']:['L','C','H'],values=channels();
  fields=labels.map((label,i)=>row(label,values[i]*(percent(model,i)?100:1),model==='rgb'?255:label==='H'?360:percent(model,i)?100:Math.max(.4,values[i]),i,percent(model,i)));
  alphaInput=row(t('透明度','Alpha'),state.alpha*100,100,3,true);alphaInput.dataset.channel='alpha';
 }
 function track(index,values,max) {
  // A channel ramp depends on the other two channels, not its own position.
  const key=[model,index,max,...values.filter((_,i)=>i!==index)].join(':');let value=rampCache.get(key);
  if(!value){value=`linear-gradient(to right in srgb,${colorRamp(model,values,index,max).map(rgbaCss).join(',')})`;if(rampCache.size>=128)rampCache.delete(rampCache.keys().next().value);rampCache.set(key,value);}return value;
 }
 function prepare(fromInputs=false) {
  const values=channels(),p=hsv(),rgb=result.rgba.slice(0,3),maxima=values.map((v,i)=>model==='oklch'&&i===1?Math.max(.4,v):Number(fields[i]._range.max)/(percent(model,i)?100:1));
  snapshot={fromInputs,values:[...values],maxima,hsv:p,hex:result.hex,paint:rgbaCss(result.rgba),opaque:rgbaCss([...rgb,1]),tracks:values.map((_,i)=>track(i,values,maxima[i])),alphaTrack:`linear-gradient(to right in srgb,${rgbaCss([...rgb,0])},${rgbaCss([...rgb,1])}),${checker}`,mapped:result.mapped};
 }
 function flush() {
  if(frame)cancelAnimationFrame(frame);frame=0;if(dead||!snapshot)return;
  const s=snapshot;snapshot=null;const [h,saturation,v]=s.hsv;hueMemory=h;
  // One synchronous DOM commit: no early thumb or delayed tracks/values.
  cssProp(plane,'--picker-hue',`hsl(${h} 100% 50%)`);cssProp(thumb,'left',`${saturation*100}%`);cssProp(thumb,'top',`${(1-v)*100}%`);cssProp(thumb,'background',s.opaque);cssProp(valueThumb,'top',`${(1-v)*100}%`);
  attr(plane,'aria-valuetext',`${t('饱和度','Saturation')} ${Math.round(saturation*100)}%, ${t('明度','Value')} ${Math.round(v*100)}%`);attr(brightness,'aria-valuenow',v*100);prop(hue,'value',h);prop(hueOutput,'value',`${Math.round(h)}°`);cssProp(hue,'--thumb',`hsl(${h} 100% 50%)`);cssProp(paint,'background',s.paint);
  fields.forEach((input,i)=>{const value=s.values[i]*(percent(model,i)?100:1),isHue=model==='hsv'&&i===0||model==='oklch'&&i===2;if(!s.fromInputs)inputValue(input,value);prop(input._range,'max',s.maxima[i]*(percent(model,i)?100:1));prop(input._range,'value',isHue&&(value<0||value>360)?((value%360)+360)%360:value);cssProp(input._range,'--track',s.tracks[i]);cssProp(input._range,'--thumb',s.opaque);});
  if(!s.fromInputs)inputValue(alphaInput,state.alpha*100);prop(alphaInput._range,'value',state.alpha*100);cssProp(alphaInput._range,'--track',s.alphaTrack);cssProp(alphaInput._range,'--thumb',s.opaque);
  if(document.activeElement!==hex)prop(hex,'value',s.hex);text(status,s.mapped?t('已映射到 sRGB；保留原始通道。','Mapped to sRGB; source channels retained.'):'');
 }
 function invalid(error) {valid=false;attr(panel,'data-valid','false');snapshot=null;if(frame)cancelAnimationFrame(frame);frame=0;text(status,String(error.message||error));}
 function update(next,fromInputs=false) {
  if(dead||submission)return;
  try{const converted=pickerColor(next,result);state=next;result=converted;valid=true;attr(panel,'data-valid','true');prepare(fromInputs);if(!frame)frame=requestAnimationFrame(flush);}catch(error){invalid(error);}
 }
 function submitChannels(input) {
  if(input===alphaInput&&valid){update({...state,alpha:number(alphaInput)/100},true);return;}
  update({space:model,channels:fields.map((n,i)=>number(n)/(percent(model,i)?100:1)),alpha:number(alphaInput)/100},true);
 }
 function editHsv(h,s,v){hueMemory=h;update({space:'hsv',channels:[h,s,v],alpha:state.alpha});}
 function invalidateRects(){rects.clear();}
 function drag(node,action) {
  let pointer=null;
  const move=e=>{let rect=rects.get(node);if(!rect){rect=node.getBoundingClientRect();rects.set(node,rect);}action(e,rect);};
  node.addEventListener('pointerdown',e=>{if(e.button!==0)return;e.preventDefault();node.focus({preventScroll:true});pointer=e.pointerId;rects.delete(node);node.setPointerCapture(pointer);move(e);});
  node.addEventListener('pointermove',e=>{if(e.pointerId===pointer)move(e);});
  const end=e=>{if(e.pointerId!==pointer)return;pointer=null;flush();};for(const type of ['pointerup','pointercancel','lostpointercapture'])node.addEventListener(type,end);
 }
 drag(plane,(e,r)=>{const [h]=hsv();editHsv(h,clamp((e.clientX-r.left)/r.width),1-clamp((e.clientY-r.top)/r.height));});
 drag(brightness,(e,r)=>{const [h,s]=hsv();editHsv(h,s,1-clamp((e.clientY-r.top)/r.height));});
 plane.addEventListener('keydown',e=>{let [h,s,v]=hsv();if(e.key==='ArrowLeft')s-=.01;else if(e.key==='ArrowRight')s+=.01;else if(e.key==='ArrowUp')v+=.01;else if(e.key==='ArrowDown')v-=.01;else return;e.preventDefault();editHsv(h,clamp(s),clamp(v));});
 brightness.addEventListener('keydown',e=>{let [h,s,v]=hsv();if(e.key==='ArrowUp')v+=.01;else if(e.key==='ArrowDown')v-=.01;else if(e.key==='Home')v=1;else if(e.key==='End')v=0;else return;e.preventDefault();editHsv(h,s,clamp(v));});
 hue.addEventListener('input',()=>{const [,s,v]=hsv();editHsv(Number(hue.value),s,v);});hue.addEventListener('pointerup',flush);
 hex.addEventListener('input',()=>{try{update(parseHex(hex.value));}catch(error){invalid(error);}});
 for(const space of ['rgb','hsv','oklch']){const tab=el('button',space.toUpperCase(),tabs);tab.dataset.space=space;tab.type='button';tab.addEventListener('click',()=>{flush();if(!valid||space===model)return;model=space;renderFields();invalidateRects();prepare();flush();});}
 const resize=new ResizeObserver(invalidateRects);resize.observe(panel);
 for(const type of ['scroll','resize']){window.addEventListener(type,invalidateRects,true);cleanup.push(()=>window.removeEventListener(type,invalidateRects,true));}
 async function commit() {
  if(dead)return true;if(submission)return submission;if(stale){dispose();return true;}flush();if(!valid)return false;
  const unchanged=format.value===initial.space&&state.space===initialState.space&&state.alpha===initialState.alpha&&equal(state.channels,initialState.channels);
  panel.inert=true;
  const outputChannels=format.value==='hex'?null:format.value===state.space?state.channels:result[format.value];
  submission=(async()=>{try{await apply({rgba:result.rgba,space:format.value,channels:outputChannels,alpha:state.alpha,unchanged});dispose();return true;}catch(error){stale=error.code==='STALE_COLOR';text(status,String(error.message||error));return false;}finally{panel.inert=false;submission=null;}})();return submission;
 }
 function dispose(){if(dead)return;dead=true;if(frame)cancelAnimationFrame(frame);resize.disconnect();cleanup.forEach(fn=>fn());panel.remove();style.remove();close?.();}
 renderFields();prepare();flush();prop(hex,'value',initial.hex||result.hex);hex.focus({preventScroll:true});
 return {element:panel,commit,dispose,invalidateRects};
}
