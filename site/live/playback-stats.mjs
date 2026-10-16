// Browser refresh callbacks estimate presentation; they cannot prove hardware
// scan-out. Submission frequency is recorded separately. Hardware acceptance
// also requires compositor tracing on a real, active 60 Hz display.
// A worker's rendering step does not prove its placeholder bitmap has reached
// the owning window. Static previews cross a window rendering step as well.
export function waitForPresentation(visible=true){
 if(!visible||document.visibilityState!=='visible')return Promise.resolve(false);
 return new Promise(resolve=>{
  let frame;const timer=setTimeout(()=>{cancelAnimationFrame(frame);resolve(false);},250);
  frame=requestAnimationFrame(()=>{frame=requestAnimationFrame(()=>{clearTimeout(timer);resolve(true);});});
 });
}
export class PlaybackStats {
 constructor(){this.frames=[];this.submissions=[];this.refreshes=[];this.fps=0;this.submissionFps=0;this.missedPercent=0;}
 refresh(now){if(this.lastRefresh&&now-this.lastRefresh<100)this.refreshes.push(now-this.lastRefresh);this.lastRefresh=now;if(this.refreshes.length>120)this.refreshes.shift();}
 submit(now){this.submissions.push(now);while(this.submissions.length>1&&this.submissions[0]<now-1000)this.submissions.shift();const span=now-this.submissions[0];if(span>=250)this.submissionFps=(this.submissions.length-1)*1000/span;}
 record(key,now){
  if(key===this.key)return;this.key=key;
  if(this.frames.length&&now-this.frames.at(-1)>250)this.frames=[];
  this.frames.push(now);while(this.frames.length>1&&this.frames[0]<now-1000)this.frames.shift();
  const span=now-this.frames[0];if(span>=250){this.fps=(this.frames.length-1)*1000/span;
   const sorted=[...this.refreshes].sort((a,b)=>a-b),period=sorted[Math.floor(sorted.length*.2)];
   if(period){let missed=0;for(let i=1;i<this.frames.length;i++)missed+=Math.max(0,Math.round((this.frames[i]-this.frames[i-1])/period)-1);this.missedPercent=100*missed/(missed+this.frames.length-1);}
  }
 }
}
