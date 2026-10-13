// Host media elements decode; video frames enter the existing Skia scene.
export function cueStart(cue, steps = []) {
  return (cue.step == null ? 0 : steps[cue.step]?.start || 0) + cue.delay;
}
export function mediaTime(cue, elapsed) {
  const span = cue.trimEnd - cue.trimStart;
  return cue.trimStart + (cue.looped ? ((elapsed % span) + span) % span : Math.min(Math.max(0, elapsed), Math.max(0, span - 1e-6)));
}
const matrix = ([a,b,c,d,e,f]) => [a,c,e,b,d,f,0,0,1];
const multiply = (a,b) => Array.from({length:9},(_,i)=>{const r=Math.floor(i/3)*3,c=i%3;return a[r]*b[c]+a[r+1]*b[c+3]+a[r+2]*b[c+6];});
function localPoint(m,x,y) {
  const [a,b,c,d,e,f,g,h,i]=m,det=a*(e*i-f*h)-b*(d*i-f*g)+c*(d*h-e*g);
  if(Math.abs(det)<1e-12)return null;
  const u=((e*i-f*h)*x+(c*h-b*i)*y+b*f-c*e)/det,v=((f*g-d*i)*x+(a*i-c*g)*y+c*d-a*f)/det,w=((d*h-e*g)*x+(b*g-a*h)*y+a*e-b*d)/det;
  return w>0?[u/w,v/w]:null;
}
export class MediaController {
  constructor({onFrame=()=>{},onError=()=>{},parent=document.body,compatibleAudio=false}={}) {
    Object.assign(this,{onFrame,onError,parent,compatibleAudio});this.tracks=[];this.frames=new Map();this.enabled=false;this.paused=false;this.serial=0;this.speed=1;
    this.unlock=document.createElement('button');this.unlock.textContent='点击开启声音 / Enable media';this.unlock.hidden=true;this.unlock.className='media-unlock';
    Object.assign(this.unlock.style,{position:'absolute',bottom:'44px',left:'16px',zIndex:'20'});parent.append(this.unlock);
    this.unlock.onclick=()=>{this.unlock.hidden=true;this.begin();for(const t of this.tracks)if(t.started&&!t.done)this.play(t);};
    this.visibility=()=>{if(document.hidden)this.pause();};document.addEventListener('visibilitychange',this.visibility);
  }
  async install(list, read, {preserveDocument=true}={}) {
    const serial=++this.serial;this.list=list;this.steps=list.animation?.normal?.steps||[];this.hitNodes=[];
    const walk=(nodes,parent= [1,0,0,0,1,0,0,0,1],prefix='',ancestors=[])=>nodes.forEach((n,i)=>{
      const path=prefix+i,world=multiply(parent,matrix(n.transform)),chain=[...ancestors,{node:n,path}];
      if(n.hint?.mediaId&&n.name)this.hitNodes.push({node:n,path,world,chain});
      if(n.kind.kind==='group')walk(n.kind.children,world,path+'/',chain);
    });walk(list.nodes);
    const keyFor=(c,index)=>JSON.stringify([c.scope,c.scope==='document'?index:null,c.asset,c.step,c.trigger,c.delay,c.trimStart,c.trimEnd,c.looped,c.volume,c.muted]);
    const wanted=new Set((list.media?.cues||[]).map(keyFor));
    const retained=preserveDocument?this.tracks.filter(t=>t.cue.scope==='document'&&wanted.has(t.key)):[];
    for(const t of this.tracks)if(!retained.includes(t))this.release(t);
    this.tracks=retained;for(const b of this.frames.values())b.bitmap.close();this.frames.clear();
    const imageFor=(nodes,target)=>{let result;const search=(ns,inside=false)=>{for(const n of ns){const yes=inside||n.name===target;if(yes&&n.kind.kind==='image'&&n.hint?.mediaId)result={resource:n.kind.resource,crop:n.hint.mediaCrop};if(n.kind.kind==='group')search(n.kind.children,yes);}};search(nodes);return result;};
    for(const [index,cue]of (list.media?.cues||[]).entries()) {
      const asset=list.media.assets[cue.asset],key=keyFor(cue,index);
      if(retained.some(t=>t.key===key))continue;
      const video=cue.target!=null,element=document.createElement(video?'video':'audio'),urls=[];
      element.preload='auto';element.playsInline=true;element.volume=cue.volume;element.muted=cue.muted;
      const raw=await read(!video&&this.compatibleAudio&&asset.audioResource?asset.audioResource:cue.asset),url=URL.createObjectURL(new Blob([raw],{type:!video&&this.compatibleAudio&&asset.audioResource?'audio/mpeg':asset.mime}));urls.push(url);element.src=url;
      let sound;
      if(video&&this.compatibleAudio&&asset.audioResource){element.muted=true;sound=document.createElement('audio');sound.preload='auto';sound.volume=cue.volume;sound.muted=cue.muted;const bytes=await read(asset.audioResource),url=URL.createObjectURL(new Blob([bytes],{type:'audio/mpeg'}));urls.push(url);sound.src=url;}
      const t={cue,key,index,asset,element,sound,urls,image:video?imageFor(list.nodes,cue.target):null,started:false,done:false,armed:null,elapsed:0,localPaused:false,serial};
      if(serial!==this.serial){this.release(t);return;}
      this.tracks.push(t);
      element.addEventListener('error',()=>this.onError(Error('Cannot decode media: '+asset.src)));
      element.addEventListener('loadeddata',()=>{if(video){this.setTime(t,cue.trimStart);this.capture(t);}});
      element.addEventListener('seeked',()=>{if(video)this.capture(t);if(this.enabled&&!this.paused&&t.started&&!t.done&&!t.localPaused)this.play(t);});
      element.addEventListener('ended',()=>this.finish(t));
      sound?.addEventListener("seeked",()=>{if(this.enabled&&!this.paused&&t.started&&!t.done&&!t.localPaused)this.play(t);});
      element.load();sound?.load();
      if(video)this.watch(t);
    }
  }
  async capture(t) {
    if(!t.image||t.capturing||t.released||t.element.readyState<2)return;t.capturing=true;
    try {
      const v=t.element,c=t.image.crop;
      const bitmap=c?.offset?await createImageBitmap(v,Math.round(c.offset[0]*v.videoWidth),Math.round(c.offset[1]*v.videoHeight),Math.max(1,Math.round(c.size[0]*v.videoWidth)),Math.max(1,Math.round(c.size[1]*v.videoHeight))):await createImageBitmap(v);
      if(t.released){bitmap.close();return;}
      this.frames.get(t.image.resource)?.bitmap.close();this.frames.set(t.image.resource,{resource:t.image.resource,bitmap});this.onFrame();
    }catch(e){this.onError(e);}finally{t.capturing=false;}
  }
  watch(t) {
    if(t.released)return;
    const callback=()=>{if(!t.element.paused)this.capture(t);this.watch(t);};
    if(t.element.requestVideoFrameCallback)t.callback=t.element.requestVideoFrameCallback(callback);
    else t.callback=requestAnimationFrame(callback);
  }
  setTime(t,value) {try{if(Math.abs(t.element.currentTime-value)>1e-5){t.resumePending=true;t.element.currentTime=value;}if(t.sound&&Math.abs(t.sound.currentTime-value)>1e-5){t.resumePending=true;t.sound.currentTime=value;}}catch{} }
  play(t) {
    if(this.paused||t.localPaused||t.released)return;
    if(t.element.seeking||t.sound?.seeking){t.resumePending=true;return;}
    if(t.resumePending){t.element.pause();t.sound?.pause();t.resumePending=false;}
    for(const e of [t.element,t.sound].filter(Boolean)){e.playbackRate=this.speed;e.play().catch(error=>{if(error.name==='NotAllowedError')this.unlock.hidden=false;else if(error.name!=='AbortError')this.onError(error);});}
  }
  begin() {if(!this.enabled)this.paused=false;this.enabled=true;for(const t of this.tracks)if(t.started&&!t.done)this.play(t);}
  pause() {this.paused=true;for(const t of this.tracks){t.element.pause();t.sound?.pause();}}
  resume() {this.paused=false;this.begin();}
  get running(){return this.enabled&&!this.paused&&this.tracks.some(t=>t.started&&!t.done&&!t.localPaused);}
  tick(seconds,clock) {
    this.clock=clock;if(!this.enabled||this.paused)return;this.speed=clock.speed||1;
    for(const t of this.tracks) {
      if(t.done)continue;const c=t.cue;
      if(!t.started&&(c.trigger==='auto'||t.clicked)) {
        const eligible=t.clicked||c.scope==='document'||c.step==null||clock.active===c.step||clock.completed>c.step;
        if(eligible){t.armed??=0;t.armed+=Math.max(0,seconds)*this.speed;if(t.armed+1e-9>=c.delay){t.started=true;this.setTime(t,c.trimStart);this.play(t);}}
      }
      if(!t.started||t.localPaused)continue;
      for(const e of [t.element,t.sound].filter(Boolean))e.playbackRate=this.speed;
      if(t.element.currentTime>=c.trimEnd-0.015)this.finish(t);
      if(t.sound&&Math.abs(t.sound.currentTime-t.element.currentTime)>.02){try{t.sound.currentTime=t.element.currentTime;}catch{}}
    }
  }
  finish(t) {if(t.cue.looped&&this.enabled&&!this.paused){this.setTime(t,t.cue.trimStart);this.play(t);}else{t.done=true;t.element.pause();t.sound?.pause();this.capture(t);}}
  click(x,y,states={}) {
    for(const h of [...this.hitNodes].reverse()) {
      const n=h.node,s=states[h.path],world=h.chain.reduce((m,v)=>multiply(m,states[v.path]?.matrix||matrix(v.node.transform)),[1,0,0,0,1,0,0,0,1]),p=localPoint(world,x,y),bounds=n.hint.mediaBounds;
      if(!p||!bounds||h.chain.some(v=>(states[v.path]?.opacity??v.node.opacity)<=0)||s?.opacity===0||n.opacity<=0||p[0]<0||p[1]<0||p[0]>bounds[2]||p[1]>bounds[3])continue;
      const t=this.tracks.find(t=>t.cue.target===n.name&&t.cue.trigger==='click');if(!t)continue;if(t.cue.step!=null&&this.clock?.active!==t.cue.step&&!(this.clock?.completed>t.cue.step))continue;
      this.begin();if(!t.started||t.done){t.clicked=true;t.armed=0;t.done=false;t.localPaused=false;this.setTime(t,t.cue.trimStart);if(t.cue.delay===0){t.started=true;this.play(t);}}else{t.localPaused=!t.localPaused;if(t.localPaused){t.element.pause();t.sound?.pause();}else this.play(t);}return true;
    }return false;
  }
  seek(time,documentOffset=0) {
    this.pause();this.enabled=true;
    for(const t of this.tracks){const c=t.cue;t.resumePending=true;if(c.trigger==='click'){t.started=false;t.done=false;this.setTime(t,c.trimStart);continue;}
      const elapsed=(c.scope==='document'?documentOffset:0)+time-cueStart(c,this.steps);t.started=elapsed>=0;t.done=!c.looped&&elapsed>=c.trimEnd-c.trimStart;t.armed=Math.max(0,time);t.localPaused=false;this.setTime(t,mediaTime(c,elapsed));this.capture(t);
    }
  }
  takeFrames(){const f=[...this.frames.values()];this.frames.clear();return f;}
  release(t){t.released=true;t.element.pause();t.sound?.pause();if(t.element.cancelVideoFrameCallback)t.element.cancelVideoFrameCallback(t.callback);else cancelAnimationFrame(t.callback);for(const e of [t.element,t.sound].filter(Boolean)){e.removeAttribute('src');e.load();}for(const u of t.urls)URL.revokeObjectURL(u);}
  stop(){this.enabled=false;this.pause();}
  dispose(){this.stop();this.serial++;for(const t of this.tracks)this.release(t);this.tracks=[];for(const f of this.frames.values())f.bitmap.close();this.frames.clear();this.unlock.remove();document.removeEventListener('visibilitychange',this.visibility);}
}
