import {MediaControls} from './media-controls.mjs';
// Host media elements decode; video frames enter the existing Skia scene.
export function cueStart(cue,steps=[]){return(cue.step==null?0:steps[cue.step]?.start||0)+cue.delay;}
export function mediaTime(cue,elapsed){const span=cue.trimEnd-cue.trimStart;return cue.trimStart+(cue.looped?((elapsed%span)+span)%span:Math.min(Math.max(0,elapsed),Math.max(0,span-1e-6)));}
const matrix=([a,b,c,d,e,f])=>[a,c,e,b,d,f,0,0,1];
const multiply=(a,b)=>Array.from({length:9},(_,i)=>{const r=Math.floor(i/3)*3,c=i%3;return a[r]*b[c]+a[r+1]*b[c+3]+a[r+2]*b[c+6];});
const point=(m,x,y)=>{const w=m[6]*x+m[7]*y+m[8];return w>1e-8?[(m[0]*x+m[1]*y+m[2])/w,(m[3]*x+m[4]*y+m[5])/w]:null;};
function localPoint(m,x,y){const[a,b,c,d,e,f,g,h,i]=m,det=a*(e*i-f*h)-b*(d*i-f*g)+c*(d*h-e*g);if(Math.abs(det)<1e-12)return null;return point([(e*i-f*h)/det,(c*h-b*i)/det,(b*f-c*e)/det,(f*g-d*i)/det,(a*i-c*g)/det,(c*d-a*f)/det,(d*h-e*g)/det,(b*g-a*h)/det,(a*e-b*d)/det],x,y);}
export class MediaController {
 constructor({onFrame=()=>{},onState=()=>{},onError=()=>{},parent=document.body,compatibleAudio=false,getViewport=()=>null}={}){
  Object.assign(this,{onFrame,onState,onError,parent,compatibleAudio,getViewport});this.tracks=[];this.frames=new Map();this.sources=new Map();this.enabled=false;this.pageEnabled=false;this.paused=false;this.locked=false;this.serial=0;this.speed=1;this.controls=new MediaControls(this,parent);
  this.unlock=document.createElement('button');this.unlock.textContent='点击开启声音 / Enable media';this.unlock.hidden=true;this.unlock.className='media-unlock';Object.assign(this.unlock.style,{position:'absolute',bottom:'44px',left:'16px',zIndex:'20'});parent.append(this.unlock);
  this.unlock.onclick=()=>{this.unlock.hidden=true;for(const t of this.tracks)if(this.wantsPlay(t))this.play(t);};
  this.visibility=()=>{if(document.hidden)this.pause({lock:true});else this.locked=false;};document.addEventListener('visibilitychange',this.visibility);
 }
 async install(list,read,{preserveDocument=true}={}){
  const serial=++this.serial;this.pageEnabled=false;this.list=list;this.steps=list.animation?.normal?.steps||[];this.hitNodes=[];
  const walk=(nodes,ancestors=[],prefix='')=>nodes.forEach((n,i)=>{const path=prefix+i,chain=[...ancestors,{node:n,path}];if(n.hint?.mediaId&&n.name)this.hitNodes.push({node:n,path,chain});if(n.kind.kind==='group')walk(n.kind.children,chain,path+'/');});walk(list.nodes);
  const cues=[...(list.media?.cues||[])];for(const h of this.hitNodes)if(!cues.some(c=>c.target===h.node.name)){const id=h.node.hint.mediaId,a=list.media?.assets[id];if(a)cues.push({asset:id,target:h.node.name,scope:'page',step:null,trigger:'click',delay:0,trimStart:0,trimEnd:a.duration,looped:false,volume:1,muted:false});}
  const keyFor=(c,index)=>JSON.stringify([c.scope,c.scope==='document'?index:null,c.asset,c.target,c.step,c.trigger,c.delay,c.trimStart,c.trimEnd,c.looped,c.volume,c.muted]);
  const wanted=new Set(cues.map(keyFor)),retained=preserveDocument?this.tracks.filter(t=>t.cue.scope==='document'&&wanted.has(t.key)):[];
  for(const t of this.tracks)if(!retained.includes(t))this.release(t);this.tracks=retained;for(const b of this.frames.values())b.bitmap.close();this.frames.clear();
  const imageFor=target=>{let result;const search=(nodes,inside=false)=>{for(const n of nodes){const yes=inside||n.name===target;if(yes&&n.kind.kind==='image'&&n.hint?.mediaId)result={resource:n.kind.resource,crop:n.hint.mediaCrop};if(n.kind.kind==='group')search(n.kind.children,yes);}};search(list.nodes);return result;};
  for(const[index,cue]of cues.entries()){
   const asset=list.media.assets[cue.asset],key=keyFor(cue,index);if(retained.some(t=>t.key===key))continue;
   let sound;const video=cue.target!=null,element=document.createElement(video?'video':'audio'),urls=[];element.preload='auto';element.playsInline=true;
   try{
    const id=!video&&this.compatibleAudio&&asset.audioResource?asset.audioResource:cue.asset;
    const url=await this.acquireSource(id,id===asset.audioResource?'audio/mpeg':asset.mime,read);urls.push(url);element.src=url;
    if(video&&this.compatibleAudio&&asset.audioResource){sound=document.createElement('audio');sound.preload='auto';const url=await this.acquireSource(asset.audioResource,'audio/mpeg',read);urls.push(url);sound.src=url;}
   }catch(error){for(const url of urls)this.releaseSource(url);throw error;}
   const t={cue,key,index,asset,element,sound,urls,image:video?imageFor(cue.target):null,started:false,done:false,armed:0,localPaused:false,held:false,manualOverride:false,volume:cue.volume,muted:cue.muted,serial};
   if(serial!==this.serial){this.release(t);return;}this.tracks.push(t);this.applyVolume(t);if(video)this.controls.add(t);
   element.addEventListener('error',()=>{if(!t.released)this.onError(Error('Cannot decode media: '+asset.src));});
   element.addEventListener('loadeddata',()=>{this.setTime(t,t.pendingTime??cue.trimStart);if(video)this.capture(t);if(this.wantsPlay(t))this.play(t);this.refresh(t);});
   element.addEventListener('seeked',()=>{t.videoSeekPending=false;if(video)this.capture(t);if(this.wantsPlay(t))this.play(t);this.refresh(t);});
   element.addEventListener('ended',()=>{if(!t.scrubbing)this.finish(t);});
   for(const name of ['playing','pause','timeupdate','volumechange'])element.addEventListener(name,()=>this.refresh(t));
   sound?.addEventListener('loadedmetadata',()=>{this.setTime(t,t.element.currentTime||t.pendingTime||cue.trimStart);if(this.wantsPlay(t))this.play(t);});sound?.addEventListener('seeked',()=>{t.soundSeekPending=false;if(this.wantsPlay(t))this.play(t);});
   element.load();sound?.load();if(video)this.watch(t);
  }
  this.layoutControls();
 }
 async acquireSource(id,mime,read){
  let source=this.sources.get(id);if(!source){source={refs:0,url:null};source.promise=Promise.resolve().then(()=>read(id)).then(raw=>source.url=URL.createObjectURL(new Blob([raw],{type:mime})));this.sources.set(id,source);}source.refs++;
  try{return await source.promise;}catch(error){if(--source.refs===0&&this.sources.get(id)===source)this.sources.delete(id);throw error;}
 }
 releaseSource(url){for(const[id,source]of this.sources)if(source.url===url){if(--source.refs===0){URL.revokeObjectURL(url);this.sources.delete(id);}return;}}
 refresh(t){if(t.released)return;this.controls.sync(t);this.onState();}
 wantsPlay(t){return !t.released&&t.started&&!t.done&&!t.localPaused&&!t.scrubbing&&!t.held&&!this.locked&&(t.manualOverride||this.enabled&&!this.paused);}
 async capture(t){
  if(!t.image||t.capturing||t.released||t.element.readyState<2)return;t.capturing=true;
  try{const v=t.element,c=t.image.crop,bitmap=c?.offset?await createImageBitmap(v,Math.round(c.offset[0]*v.videoWidth),Math.round(c.offset[1]*v.videoHeight),Math.max(1,Math.round(c.size[0]*v.videoWidth)),Math.max(1,Math.round(c.size[1]*v.videoHeight))):await createImageBitmap(v);if(t.released){bitmap.close();return;}this.frames.get(t.image.resource)?.bitmap.close();this.frames.set(t.image.resource,{resource:t.image.resource,bitmap});this.onFrame();}catch(e){/* A seek can invalidate a decoded frame before bitmap capture settles. The next seeked/video-frame event retries. */if(!t.released&&!t.element.seeking&&t.element.readyState>=2&&e.name!=="InvalidStateError")this.onError(e);}finally{t.capturing=false;}
 }
 watch(t){if(t.released)return;const callback=()=>{if(!t.element.paused)this.capture(t);this.watch(t);};if(t.element.requestVideoFrameCallback)t.callback=t.element.requestVideoFrameCallback(callback);else t.callback=requestAnimationFrame(callback);}
 setTime(t,value){t.pendingTime=value;try{if(Math.abs(t.element.currentTime-value)>1e-5){t.resumePending=true;t.videoSeekPending=true;t.element.currentTime=value;}if(t.sound&&Math.abs(t.sound.currentTime-value)>1e-5){t.resumePending=true;t.soundSeekPending=true;t.sound.currentTime=value;}}catch{} }
 play(t){
  if(!this.wantsPlay(t))return;if(t.videoSeekPending||t.soundSeekPending||t.element.seeking||t.sound?.seeking){t.resumePending=true;return;}if(t.resumePending){t.element.pause();t.sound?.pause();t.resumePending=false;}
  for(const e of [t.element,t.sound].filter(Boolean)){e.playbackRate=this.speed;e.play().catch(error=>{if(t.released)return;if(error.name==='NotAllowedError')this.unlock.hidden=false;else if(error.name!=='AbortError')this.onError(error);});}this.refresh(t);
 }
 pauseTrack(t){t.element.pause();t.sound?.pause();this.refresh(t);}
 toggleTrack(t){if(this.locked||t.released)return;const playing=this.wantsPlay(t);t.manualOverride=true;t.held=false;if(playing){t.localPaused=true;this.pauseTrack(t);}else{if(!t.started||t.done)this.setTime(t,t.cue.trimStart);t.started=true;t.done=false;t.localPaused=false;this.play(t);}this.refresh(t);}
 beginScrub(t){if(this.locked)return;t.scrubResume=this.wantsPlay(t);t.manualOverride=true;t.started=true;t.done=false;t.held=false;t.scrubbing=true;this.pauseTrack(t);}
 seekTrack(t,position){if(this.locked)return;const c=t.cue;t.manualOverride=true;t.started=true;t.done=false;t.held=false;this.setTime(t,c.trimStart+Math.max(0,Math.min(c.trimEnd-c.trimStart-1e-6,position)));this.capture(t);this.refresh(t);}
 endScrub(t){if(!t.scrubbing)return;t.scrubbing=false;t.localPaused=!t.scrubResume;if(this.wantsPlay(t))this.play(t);this.refresh(t);}
 applyVolume(t){t.element.volume=t.volume;t.element.muted=!!t.sound||t.muted;if(t.sound){t.sound.volume=t.volume;t.sound.muted=t.muted;}}
 setVolume(t,value){t.volume=Math.max(0,Math.min(1,value));if(t.volume>0)t.muted=false;this.applyVolume(t);this.refresh(t);}
 setMuted(t,value){t.muted=value;if(!value&&t.volume===0)t.volume=t.cue.volume||1;this.applyVolume(t);this.refresh(t);}
 begin(){this.pageEnabled=true;if(!this.enabled)this.paused=false;this.enabled=true;for(const t of this.tracks)if(this.wantsPlay(t))this.play(t);}
 pause({lock=false}={}){this.paused=true;this.locked||=lock;for(const t of this.tracks){t.held=true;this.pauseTrack(t);}}
 resume({schedule=true}={}){this.paused=false;this.locked=false;for(const t of this.tracks)t.held=false;if(schedule)this.begin();else for(const t of this.tracks)if(this.wantsPlay(t))this.play(t);}
 snapshot(){return {enabled:this.enabled,pageEnabled:this.pageEnabled,paused:this.paused,locked:this.locked,held:new Map(this.tracks.map(t=>[t,t.held]))};}
 suspend(){const snapshot=this.snapshot();this.pause({lock:true});return snapshot;}
 restore(snapshot){if(!snapshot)return;if(document.hidden){this.pause({lock:true});return;}Object.assign(this,{enabled:snapshot.enabled,pageEnabled:snapshot.pageEnabled,paused:snapshot.paused,locked:snapshot.locked});for(const t of this.tracks){t.held=snapshot.held.get(t)??false;if(this.wantsPlay(t))this.play(t);this.refresh(t);}}
 get running(){return this.tracks.some(t=>this.wantsPlay(t));}
 tick(seconds,clock={}){
  this.clock=clock;this.speed=clock.speed||1;
  for(const t of this.tracks){if(t.released)continue;const c=t.cue;if(!t.started&&!t.done&&!t.manualOverride&&this.enabled&&!this.paused&&!this.locked&&c.trigger==='auto'&&(c.scope==='document'||this.pageEnabled)){
   const eligible=c.scope==='document'||c.step==null||clock.active===c.step||clock.completed>c.step;if(eligible){t.armed+=Math.max(0,seconds)*this.speed;if(t.armed+1e-9>=c.delay){t.started=true;this.setTime(t,c.trimStart);this.play(t);}}
  }if(this.wantsPlay(t)){for(const e of [t.element,t.sound].filter(Boolean))e.playbackRate=this.speed;if(!t.element.seeking&&t.element.currentTime>=c.trimEnd-.015)this.finish(t);else if(t.sound&&!t.element.seeking&&!t.sound.seeking&&Math.abs(t.sound.currentTime-t.element.currentTime)>.02){try{t.sound.currentTime=t.element.currentTime;}catch{}}}this.controls.sync(t);}
  this.layoutControls();
 }
 finish(t){if(t.cue.looped&&this.wantsPlay(t)){this.setTime(t,t.cue.trimStart);this.play(t);}else{t.done=true;t.localPaused=true;this.pauseTrack(t);this.capture(t);}this.refresh(t);}
 world(h,states=this.states||{}){return h.chain.reduce((m,v)=>multiply(m,states[v.path]?.matrix||matrix(v.node.transform)),[1,0,0,0,1,0,0,0,1]);}
 visible(h,states=this.states||{}){return !h.chain.some(v=>(states[v.path]?.opacity??v.node.opacity)<=0);}
 click(x,y,states={}){if(this.locked)return false;for(const h of [...this.hitNodes].reverse()){const p=localPoint(this.world(h,states),x,y),b=h.node.hint.mediaBounds;if(!p||!b||!this.visible(h,states)||p[0]<b[0]||p[1]<b[1]||p[0]>b[2]||p[1]>b[3])continue;const t=this.tracks.find(t=>t.cue.target===h.node.name);if(t){this.toggleTrack(t);return true;}}return false;}
 layoutControls(){
  const view=this.getViewport();this.controls.viewport(view);if(!view?.visible)return;
  for(const t of this.tracks){if(!t.image)continue;const order=this.hitNodes.findIndex(h=>h.node.name===t.cue.target),h=this.hitNodes[order];if(!h)continue;const b=h.node.hint.mediaBounds,m=multiply(view.matrix,this.world(h)),quad=[[b[0],b[1]],[b[2],b[1]],[b[2],b[3]],[b[0],b[3]]].map(p=>point(m,...p));
   const valid=quad.every(Boolean),width=valid?Math.max(1,Math.hypot(quad[1][0]-quad[0][0],quad[1][1]-quad[0][1])):1,height=valid?Math.max(1,Math.hypot(quad[3][0]-quad[0][0],quad[3][1]-quad[0][1])):1,clip=view.clip||[-Infinity,-Infinity,Infinity,Infinity];
   const visible=valid&&this.visible(h)&&!this.locked&&Math.max(...quad.map(p=>p[0]))>clip[0]&&Math.min(...quad.map(p=>p[0]))<clip[2]&&Math.max(...quad.map(p=>p[1]))>clip[1]&&Math.min(...quad.map(p=>p[1]))<clip[3],local=[(b[2]-b[0])/width,0,b[0],0,(b[3]-b[1])/height,b[1],0,0,1];this.controls.layout(t,{matrix:multiply(m,local),quad,width,height,order,visible});
  }
 }
 seek(time,documentOffset=0,{preserveManual=false,preserveDocument=false}={}){
  this.enabled=true;this.pageEnabled=true;if(!preserveManual)this.paused=true;for(const t of this.tracks){if(preserveManual&&t.manualOverride||preserveDocument&&t.cue.scope==='document')continue;const c=t.cue;t.element.pause();t.sound?.pause();t.held=true;t.manualOverride=false;t.localPaused=false;t.resumePending=true;if(c.trigger==='click'){t.started=false;t.done=false;this.setTime(t,c.trimStart);}else{const elapsed=(c.scope==='document'?documentOffset:0)+time-cueStart(c,this.clock?.steps||this.steps);t.started=elapsed>=0;t.done=!c.looped&&elapsed>=c.trimEnd-c.trimStart;t.armed=Math.max(0,time);this.setTime(t,mediaTime(c,elapsed));}this.capture(t);this.refresh(t);}
 }
 takeFrames(){const f=[...this.frames.values()];this.frames.clear();return f;}
 release(t){t.released=true;this.controls.remove(t);t.element.pause();t.sound?.pause();if(t.element.cancelVideoFrameCallback)t.element.cancelVideoFrameCallback(t.callback);else cancelAnimationFrame(t.callback);for(const e of [t.element,t.sound].filter(Boolean)){e.removeAttribute('src');e.load();}for(const u of t.urls)this.releaseSource(u);}
 leavePage(){this.pageEnabled=false;for(const t of this.tracks)if(t.cue.scope!=='document')this.release(t);this.tracks=this.tracks.filter(t=>t.cue.scope==='document');for(const f of this.frames.values())f.bitmap.close();this.frames.clear();}
 stop(){this.enabled=false;this.pause();}
 dispose(){this.stop();this.serial++;for(const t of this.tracks)this.release(t);this.tracks=[];for(const f of this.frames.values())f.bitmap.close();this.frames.clear();this.unlock.remove();this.controls.dispose();document.removeEventListener('visibilitychange',this.visibility);}
}
