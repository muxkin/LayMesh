// Shared presentation controls. Rendering remains in the native/WASM sampler.
export class AnimationPlayer {
 constructor(parent,change,{locale=()=>document.documentElement.lang,page=()=>false,statusInterval=0}={}){
  this.change=change;this.locale=locale;this.page=page;this.time=null;this.running=false;this.speed=1;this.metadata=null;this.resume=false;
  this.statusInterval=statusInterval;
  this.motion=matchMedia('(prefers-reduced-motion: reduce)');this.reduced=this.motion.matches;
  this.bar=document.createElement('div');this.bar.className='animation-controls';this.bar.style.cssText='display:flex;flex-wrap:wrap;align-items:center;gap:6px;padding:6px 8px;font:inherit;border-bottom:1px solid #8884';parent.append(this.bar);
  const create=(tag,label)=>{const e=document.createElement(tag);e.setAttribute('aria-label',label);if(tag==='button'||tag==='select')e.style.cssText='font:inherit;color:inherit;background:transparent;border:1px solid #8886;border-radius:4px;padding:3px 6px';this.bar.append(e);return e;};this.buttons={};
  for(const [id,zh,en]of [['static','完整排版','Static'],['previous','上一步','Previous'],['play','播放','Play'],['next','下一步','Next'],['skip','跳过动画','Skip'],['replay','重播','Replay']]){const b=create('button',en);b.type='button';b.dataset.action='animation-'+id;b.textContent=/^zh/.test(locale())?zh:en;b.onclick=()=>this.action(id);this.buttons[id]=b;}
  this.mode=create('select','Playback mode');for(const [value,zh,en]of [['auto','自动','Auto'],['manual','手动','Manual']]){const o=document.createElement('option');o.value=value;o.textContent=/^zh/.test(locale())?zh:en;this.mode.append(o);}
  this.scrub=create('input','Animation time');this.scrub.type='range';this.scrub.min=0;this.scrub.step=.001;this.scrub.style.cssText='flex:1;min-width:90px';this.scrub.oninput=()=>{this.running=false;cancelAnimationFrame(this.raf);this.seek(Number(this.scrub.value));};
  const speed=create('select','Playback speed');for(const n of [.25,.5,1,1.5,2]){const o=document.createElement('option');o.value=n;o.textContent=n+'×';speed.append(o);}speed.value=1;speed.onchange=()=>{this.speed=Number(speed.value);};
  for(const option of this.bar.querySelectorAll('option'))option.style.cssText='color:CanvasText;background:Canvas';
  this.label=create('output','Current step');this.label.style.fontVariantNumeric='tabular-nums';
  this.motion.addEventListener('change',e=>{this.reduced=e.matches;this.running=false;this.seek(this.time);});this.update(null);
 }
 localize(){this.language=this.locale();const zh=/^zh/.test(this.language);const labels={static:['完整排版','Static'],previous:['上一步','Previous'],next:['下一步','Next'],skip:['跳过动画','Skip'],replay:['重播','Replay']};for(const [id,values]of Object.entries(labels)){const b=this.buttons[id];b.textContent=values[zh?0:1];b.setAttribute('aria-label',b.textContent);}this.mode.options[0].textContent=zh?'自动':'Auto';this.mode.options[1].textContent=zh?'手动':'Manual';}
 get timeline(){return this.metadata?(this.reduced?{steps:this.metadata.reducedSteps,duration:this.metadata.reducedDuration}:this.metadata):null;}
 get current(){if(this.pinnedStep!=null)return this.pinnedStep;const t=this.time??0;return Math.max(0,(this.timeline?.steps.findLastIndex(s=>s.start<=t)??0));}
 update(metadata){cancelAnimationFrame(this.raf);this.running=false;this.metadata=metadata;this.bar.style.display=metadata?'flex':'none';if(!metadata){this.time=null;return;}const resume=this.resume;this.resume=false;this.seek(resume?(resume===-1?this.timeline.steps.at(-1).start:0):null);if(resume)this.start();}
 seek(time,pinnedStep=null,frame=false){this.pinnedStep=pinnedStep;const timeline=this.timeline;this.time=time===null?null:Math.max(0,Math.min(timeline?.duration??0,time));this.sync(frame);this.change();}
 sync(frame=false){if(this.language!==this.locale())this.localize();const t=this.timeline;if(!t)return;this.scrub.max=t.duration;this.scrub.value=this.time??0;const now=performance.now();if(frame&&this.running&&now<this.nextStatus)return;this.nextStatus=now+this.statusInterval;const step=t.steps[this.current];this.label.textContent=this.time===null?(/^zh/.test(this.locale())?'完整排版':'Static'):`${this.current+1}/${t.steps.length} ${step?.name??''} · ${this.time.toFixed(2)} / ${t.duration.toFixed(2)} s`;this.buttons.play.textContent=this.running?(/^zh/.test(this.locale())?'暂停':'Pause'):(/^zh/.test(this.locale())?'播放':'Play');this.buttons.play.setAttribute('aria-label',this.buttons.play.textContent);}
 action(id){const t=this.timeline;if(!t)return;
  if(id==='play'){if(this.running){this.running=false;cancelAnimationFrame(this.raf);this.sync();this.change();}else this.start();return;}
  this.running=false;cancelAnimationFrame(this.raf);
  if(id==='static'){this.seek(null);return;}if(id==='replay'){this.seek(0);this.start();return;}
  const index=this.current,s=t.steps[index];if(id==='skip'){this.seek(s.start+s.duration,index);return;}
  const n=index+(id==='next'?1:-1);if(n<0||n>=t.steps.length){if(this.page(id==='next'?1:-1,false)){this.resume=id==='previous'?-1:1;return;}this.seek(n<0?0:t.duration);return;}
  this.seek(t.steps[n].start);this.start();
 }
 start(){const t=this.timeline;if(!t)return;if(this.time===null||this.time>=t.duration){this.time=0;this.pinnedStep=null;}else if(this.pinnedStep!=null&&this.mode.value==='manual'){this.time=t.steps[this.pinnedStep].start;this.pinnedStep=null;}this.running=true;this.last=performance.now();this.sync();
  const tick=now=>{if(!this.running)return;const old=this.current,elapsed=now-this.last;let boundary=old,next=this.time+elapsed/1000*this.speed;this.last=now;
   if(this.mode.value==='manual'){while(boundary<t.steps.length){const step=t.steps[boundary],end=step.start+step.duration;if(next<end)break;const following=t.steps[boundary+1];if(following?.trigger==='after-previous'){if(next<following.start)break;boundary++;continue;}next=end;this.running=false;break;}}
   if(next>=t.duration){next=t.duration;this.running=false;if(this.mode.value==='auto'&&this.page(1,true))this.resume=true;}
   this.seek(next,!this.running&&this.mode.value==='manual'?boundary:null,true);if(this.running)this.raf=requestAnimationFrame(tick);
  };this.raf=requestAnimationFrame(tick);
 }
}
