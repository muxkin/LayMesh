import {uiIcon} from './presentation-icons.mjs';
const text=(zh,en)=>/^en/.test(document.documentElement.lang)?en:zh;
const sheet=`
.media-controls-layer{position:absolute;inset:0;z-index:5;pointer-events:none;overflow:hidden}
.media-controls-layer[hidden],.media-controls[hidden],.media-controls [hidden]{display:none!important}
.media-controls{position:absolute;left:0;top:0;transform-origin:0 0;pointer-events:none;color:#fff;font:12px system-ui,sans-serif}
.media-controls button{all:unset;box-sizing:border-box;display:flex;align-items:center;justify-content:center;cursor:pointer;color:#fff;pointer-events:auto;flex:none}
.media-controls button:focus-visible,.media-controls input:focus-visible{outline:2px solid #fff;outline-offset:2px}
.media-controls button:disabled{opacity:.5;cursor:wait}
.media-controls svg{width:18px;height:18px;flex:none}
.media-controls .media-central-play{position:absolute;z-index:1;left:50%;top:50%;width:46px;height:46px;border-radius:50%;background:rgba(10,16,24,.78)!important;box-shadow:0 0 0 1px #ffffff70;transform:translate(-50%,-50%)}
.media-central-play svg{width:24px;height:24px}
.media-controlbar{position:absolute;box-sizing:border-box;left:0;right:0;bottom:0;display:grid;gap:4px;padding:7px 8px;background:linear-gradient(transparent,rgba(8,13,20,.92) 20%);pointer-events:auto}
.media-control-row{display:flex;align-items:center;gap:5px;flex-wrap:wrap}
.media-controlbar button{width:24px;height:24px;border-radius:4px}
.media-controlbar button:hover{background:#ffffff25}
.media-controlbar input{box-sizing:border-box;min-width:0;height:14px;margin:0;padding:0;accent-color:#fff;cursor:pointer;pointer-events:auto}
.media-progress{width:100%}
.media-volume{width:64px;flex:1 1 48px;max-width:110px}
.media-time{flex:1 0 58px;font-size:11px;font-variant-numeric:tabular-nums;white-space:nowrap;line-height:24px}
.media-sound{display:flex;align-items:center;gap:5px;position:relative}
.media-compact .media-controlbar{padding:4px;gap:2px}
.media-compact .media-control-row{flex-wrap:nowrap;gap:3px}
.media-compact .media-controlbar button{width:20px!important;height:20px!important}
.media-compact .media-time{font-size:10px;line-height:20px;flex:1 1 48px;min-width:0}
.media-compact .media-progress{height:8px!important}
.media-compact .media-central-play{width:28px!important;height:28px!important}
.media-compact .media-central-play svg{width:18px;height:18px}
.media-compact .media-volume{position:absolute;bottom:28px;right:0;width:90px!important;max-width:90px;padding:4px!important;height:22px!important;background:rgba(8,13,20,.92)!important;border-radius:4px}
.media-compact .media-sound:not(:hover):not(:focus-within):not(.show-volume) .media-volume{display:none}
`;
const time=value=>{value=Math.max(0,Math.floor(value||0));return Math.floor(value/60)+':'+String(value%60).padStart(2,'0');};
function inside(quad,x,y){let sign=0;for(let i=0;i<4;i++){const a=quad[i],b=quad[(i+1)%4],cross=(b[0]-a[0])*(y-a[1])-(b[1]-a[1])*(x-a[0]);if(Math.abs(cross)<1e-6)continue;const s=Math.sign(cross);if(sign&&s!==sign)return false;sign=s;}return true;}
// One overlay shared by native preview, portable playback and presentation.
export class MediaControls {
 constructor(controller,parent){
  this.controller=controller;this.parent=parent;this.views=new Map();this.keyboard=false;
  if(!document.getElementById('laymesh-media-controls-style')){const style=document.createElement('style');style.id='laymesh-media-controls-style';style.textContent=sheet;document.head.append(style);}
  if(getComputedStyle(parent).position==='static')parent.style.position='relative';
  this.layer=document.createElement('div');this.layer.className='media-controls-layer';parent.append(this.layer);
  this.events=new AbortController();const options={signal:this.events.signal};
  document.addEventListener('keydown',e=>{if(e.key==='Tab'||e.target.closest?.('.media-controls'))this.keyboard=true;},{...options,capture:true});
  parent.addEventListener('pointermove',e=>this.pointer(e),options);
  parent.addEventListener('pointerdown',e=>{this.keyboard=false;if(e.pointerType==='touch')this.pointer(e);},{...options,capture:true});
  parent.addEventListener('pointerleave',()=>{for(const v of this.views.values())if(!v.bar.hidden&&!v.dragging&&!v.adjusting&&!this.focused(v))v.deadline=performance.now()+2000;},options);
 }
 add(track){
  const box=document.createElement('div');box.className='media-controls';box.dataset.videoTarget=track.cue.target;box.hidden=true;this.layer.append(box);
  const button=(action,central=false)=>{const b=document.createElement('button');b.type='button';b.dataset.mediaAction=action;Object.assign(b.style,{minWidth:'0',minHeight:'0',width:central?'46px':'24px',height:central?'46px':'24px',padding:'0',margin:'0',border:'0',background:'transparent',color:'#fff'});if(central)b.className='media-central-play';box.append(b);return b;};
  const center=button('play',true),bar=document.createElement('div');bar.className='media-controlbar';bar.hidden=true;box.append(bar);
  const inputStyle={minWidth:'0',minHeight:'0',height:'14px',padding:'0',margin:'0',border:'0',background:'transparent',accentColor:'#fff'};
  const progress=document.createElement('input');Object.assign(progress.style,inputStyle,{width:'100%'});progress.type='range';progress.className='media-progress';progress.min=0;progress.max=track.cue.trimEnd-track.cue.trimStart;progress.step=.01;progress.value=0;progress.setAttribute('aria-label',track.cue.target+': '+text('视频进度','Video progress'));bar.append(progress);
  const row=document.createElement('div');row.className='media-control-row';bar.append(row);
  const play=button('play'),label=document.createElement('span');label.className='media-time';label.setAttribute('aria-live','off');const mute=button('mute'),volume=document.createElement('input'),sound=document.createElement('div');sound.className='media-sound';Object.assign(volume.style,inputStyle,{width:'64px'});volume.type='range';volume.className='media-volume';volume.min=0;volume.max=1;volume.step=.01;volume.setAttribute('aria-label',track.cue.target+': '+text('视频音量','Video volume'));sound.append(mute,volume);row.append(play,label,sound);
  const v={track,box,center,bar,progress,play,label,mute,volume,deadline:0,dragging:false};this.views.set(track,v);
  for(const b of [center,play])b.onclick=()=>{this.controller.toggleTrack(track);this.reveal(v);};
  mute.onclick=()=>{this.controller.setMuted(track,!track.muted);sound.classList.add('show-volume');this.reveal(v);};
  volume.oninput=()=>{this.controller.setVolume(track,Number(volume.value));this.reveal(v);};
  volume.addEventListener('pointerdown',e=>{v.adjusting=true;volume.setPointerCapture(e.pointerId);this.reveal(v);});
  for(const name of ['pointerup','pointercancel','lostpointercapture'])volume.addEventListener(name,()=>{v.adjusting=false;this.reveal(v);});
  const start=()=>{if(v.dragging)return;v.dragging=true;this.controller.beginScrub(track);this.reveal(v);};
  const end=()=>{if(!v.dragging)return;v.dragging=false;this.controller.endScrub(track);this.reveal(v);};
  progress.addEventListener('pointerdown',e=>{start();progress.setPointerCapture(e.pointerId);});
  for(const name of ['pointerup','pointercancel','lostpointercapture'])progress.addEventListener(name,end);
  progress.oninput=()=>{const discrete=!v.dragging;if(discrete)start();this.controller.seekTrack(track,Number(progress.value));if(discrete)end();};progress.onchange=end;
  box.addEventListener('focusin',()=>this.reveal(v));box.addEventListener('pointermove',()=>this.reveal(v));
  for(const event of ['click','dblclick','pointerdown','pointerup','pointermove','wheel','keydown','keyup','contextmenu'])box.addEventListener(event,e=>{e.stopPropagation();if(event==='wheel')e.preventDefault();},{passive:false});
  this.sync(track);
 }
 focused(v){return this.keyboard&&v.box.contains(document.activeElement);}
 reveal(v){v.deadline=performance.now()+2000;v.bar.hidden=false;}
 pointer(e){if(this.layer.hidden)return;const r=this.parent.getBoundingClientRect(),matches=[...this.views.values()].filter(v=>!v.box.hidden&&v.quad&&inside(v.quad,e.clientX-r.left,e.clientY-r.top)),v=matches.sort((a,b)=>b.order-a.order)[0];if(v)this.reveal(v);}
 sync(track){
  const v=this.views.get(track);if(!v)return;const playing=this.controller.wantsPlay(track),name=track.cue.target+': ',action=track.done?text('重新播放视频','Replay video'):playing?text('暂停视频','Pause video'):text('播放视频','Play video');
  for(const b of [v.center,v.play]){const icon=playing?'pause':'play';if(b.dataset.icon!==icon){b.replaceChildren(uiIcon(icon));b.dataset.icon=icon;}b.setAttribute('aria-label',name+action);b.title=action;b.disabled=track.element.readyState<2;}
  v.center.hidden=playing||track.scrubbing;
  const span=track.cue.trimEnd-track.cue.trimStart,position=track.done?span:Math.max(0,Math.min(span,track.element.currentTime-track.cue.trimStart));
  if(!v.dragging)v.progress.value=String(position);v.label.textContent=time(position)+' / '+time(span);v.progress.setAttribute('aria-valuetext',v.label.textContent);
  v.volume.value=String(track.muted?0:track.volume);const muted=track.muted||track.volume===0;
  const muteIcon=muted?'volume-off':'volume';if(v.mute.dataset.icon!==muteIcon){v.mute.replaceChildren(uiIcon(muteIcon));v.mute.dataset.icon=muteIcon;}v.mute.setAttribute('aria-label',name+(muted?text('取消视频静音','Unmute video'):text('视频静音','Mute video')));v.mute.setAttribute('aria-pressed',String(muted));
 }
 layout(track,{matrix,quad,width,height,order,visible}){
  const v=this.views.get(track);if(!v)return;v.order=order;v.quad=quad;v.box.hidden=!visible;if(!visible)return;v.box.classList.toggle('media-compact',width<220);v.center.style.top=Math.min(height/2,Math.max(14,height-(width<220?38:59)-(width<220?16:25)))+'px';
  const[a,b,c,d,e,f,g,h,i]=matrix,transform=`matrix3d(${a},${d},0,${g},${b},${e},0,${h},0,0,1,0,${c},${f},0,${i})`;
  if(v.transform!==transform){v.box.style.transform=transform;v.transform=transform;}if(v.width!==width){v.box.style.width=width+'px';v.width=width;}if(v.height!==height){v.box.style.height=height+'px';v.height=height;}
  v.box.style.zIndex=String(order);v.bar.hidden=performance.now()>v.deadline&&!v.dragging&&!v.adjusting&&!this.focused(v);if(v.bar.hidden)v.mute.parentElement.classList.remove('show-volume');
 }
 viewport(view){this.layer.hidden=!view?.visible||this.controller.locked;if(!view?.clip)return;const r=this.parent.getBoundingClientRect(),[x0,y0,x1,y1]=view.clip;this.layer.style.clipPath=`inset(${Math.max(0,y0)}px ${Math.max(0,r.width-x1)}px ${Math.max(0,r.height-y1)}px ${Math.max(0,x0)}px)`;}
 focusFirst(){const v=[...this.views.values()].find(v=>!v.box.hidden);if(!v||this.layer.hidden)return false;this.keyboard=true;this.reveal(v);v.play.focus({preventScroll:true});return true;}
 remove(track){this.views.get(track)?.box.remove();this.views.delete(track);}
 dispose(){this.events.abort();this.layer.remove();this.views.clear();}
}
