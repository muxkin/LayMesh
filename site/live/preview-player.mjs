import {AnimationPlayer} from './animation-player.mjs';
import {PresentationClock} from './presentation-clock.mjs';

// Preview retains the static view until an explicit action. After that it uses
// the same step boundaries, holds and automatic links as detached presentation.
export class PreviewPlayer extends AnimationPlayer {
 constructor(parent,change,options){super(parent,change,options);this.mode.value='manual';this.mode.onchange=()=>{this.clock.mode=this.mode.value;};this.speedSelect=this.bar.querySelector('[aria-label="Playback speed"]');this.speedSelect.onchange=()=>{this.speed=Number(this.speedSelect.value);this.clock.speed=this.speed;};}
 update(metadata){
  this.clock??=new PresentationClock();cancelAnimationFrame(this.raf);this.raf=null;this.running=false;this.metadata=metadata;this.bar.style.display=metadata?'flex':'none';
  this.clock.mode=this.mode?.value||'manual';this.clock.install(metadata,this.reduced);this.clock.running=false;this.clock.active=-1;this.time=null;this.resume=false;
  if(metadata)this.sync();
 }
 seek(time,pinnedStep=null,frame=false){
  cancelAnimationFrame(this.raf);this.raf=null;this.clock.mode=this.mode.value;
  if(time===null){this.clock.install(this.metadata,this.reduced);this.clock.running=false;this.clock.active=-1;this.time=null;this.running=false;this.resume=false;this.pinnedStep=null;this.sync(frame);this.change();return;}
  const timeline=this.timeline;this.clock.steps=timeline?.steps||[];this.clock.duration=timeline?.duration??1;this.clock.staticPage=!this.clock.steps.length;
  this.clock.seek(time);this.time=this.clock.time;this.pinnedStep=pinnedStep;this.refresh(frame);
 }
 restorePage(resume){
  this.clock.mode=this.mode.value;this.clock.install(this.metadata,this.reduced,!!resume.end);
  const start=!resume.end&&(resume.automatic||this.clock.running);this.clock.running=false;
  if(!start&&!resume.end)this.clock.active=-1;
  this.time=this.clock.time;this.refresh();return start;
 }
 refresh(frame=false){
  if(this.time!==null)this.time=this.clock.time;
  this.running=this.clock.running;this.pinnedStep=this.clock.active<0&&this.clock.completed>0?this.clock.completed-1:null;
  this.sync(frame);this.change();
  if(this.running&&!this.raf)this.raf=requestAnimationFrame(now=>this.tick(now));
  if(!this.running){cancelAnimationFrame(this.raf);this.raf=null;}
 }
 tick(now){
  this.raf=null;if(!this.clock.running)return;const next=this.clock.tick((now-this.last)/1000);this.last=now;this.refresh(true);if(next)this.changePage(1);
 }
 start(){
  this.clock.mode=this.mode.value;
  if(this.time===null||this.clock.time>=this.clock.duration){this.clock.install(this.metadata,this.reduced);this.clock.running=false;this.clock.active=-1;}
  this.time=this.clock.time;if(!this.clock.running)this.clock.toggle();this.last=performance.now();this.refresh();
 }
 changePage(direction){
  if(this.page(direction,this.clock.mode==='auto')){this.resume={end:direction<0,automatic:this.clock.mode==='auto'};return true;}return false;
 }
 navigate(id){
  this.clock.mode=this.mode.value;
  if(this.time===null){this.clock.install(this.metadata,this.reduced);this.clock.running=false;this.clock.active=-1;this.time=0;}
  if(id==='next'){if(this.clock.advance())this.changePage(1);}
  else if(this.clock.previous()&&this.clock.completed===0)this.changePage(-1);
  this.last=performance.now();this.refresh();
 }
 action(id){
  if(id==='static'){this.seek(null);return;}
  if(id==='previous'||id==='next'){this.navigate(id);return;}
  if(!this.metadata)return;
  if(id==='play'){if(this.clock.running){this.clock.running=false;this.refresh();}else this.start();return;}
  if(id==='replay'){this.seek(0);this.start();return;}
  if(id==='skip'){if(this.time===null)this.navigate('next');if(this.clock.active>=0)this.clock.advance();this.refresh();}
 }
}
