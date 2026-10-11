// Pure presentation state; no timers, DOM, compiler, or renderer dependencies.
export class PresentationClock {
 constructor(){this.mode='manual';this.speed=1;this.running=false;this.time=0;this.completed=0;this.active=-1;this.steps=[];}
 install(animation,reduced=false,end=false){this.steps=animation?(reduced?animation.reducedSteps:animation.steps):[];this.duration=animation?(reduced?animation.reducedDuration:animation.duration):1;this.time=end?this.duration:0;this.completed=end?this.steps.length:0;this.active=-1;this.running=false;this.staticPage=!this.steps.length;if(!end&&this.steps[0]?.trigger==='after-previous'){this.active=0;this.running=true;}}
 advance(){if(this.staticPage)return 1;if(this.active>=0){this.time=this.steps[this.active].start+this.steps[this.active].duration;this.completed=this.active+1;this.active=-1;this.running=false;return 0;}if(this.completed>=this.steps.length)return 1;this.active=this.completed;this.time=this.steps[this.active].start;this.running=true;return 0;}
 previous(){this.running=false;if(this.active>=0){this.completed=this.active;this.active=-1;}else if(this.completed>0)this.completed--;else return -1;this.time=this.steps[this.completed]?.start??0;return 0;}
 tick(seconds){if(!this.running)return 0;this.time+=Math.max(0,seconds)*this.speed;if(this.mode==='auto'){if(this.time>=this.duration){this.time=this.duration;this.completed=this.steps.length;this.active=-1;this.running=false;return 1;}this.active=this.steps.findLastIndex(s=>s.start<=this.time);this.completed=Math.max(0,this.active);return 0;}
  while(this.active>=0){const s=this.steps[this.active],end=s.start+s.duration;if(this.time<end)break;this.completed=this.active+1;const next=this.steps[this.active+1];if(next?.trigger==='after-previous'){if(this.time<next.start)break;this.active++;continue;}this.time=end;this.active=-1;this.running=false;break;}return 0;}

 toggle(){if(this.running){this.running=false;return;}if(this.mode==='auto'){this.running=this.time<this.duration;return;}if(this.active>=0)this.running=true;else this.advance();}
 seek(time){this.time=Math.max(0,Math.min(this.duration,time));this.running=false;this.active=this.steps.findIndex(s=>this.time>=s.start&&this.time<s.start+s.duration);this.completed=this.steps.filter(s=>this.time>=s.start+s.duration).length;}
}
