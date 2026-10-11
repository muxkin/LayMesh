// A wheel notch and a touchpad fling both represent one deliberate gesture.
// Wait for the tail to stop before accepting the next gesture, so inertia never
// finishes an animation and then silently skips its next step or page.
export class PresentationWheel {
 constructor(){this.reset();}
 reset(){this.last=-Infinity;this.sum=0;this.fired=false;}
 consume(event,now,pageHeight){
  if(event.ctrlKey||event.metaKey||event.altKey||event.shiftKey||Math.abs(event.deltaX)>=Math.abs(event.deltaY)||!Number.isFinite(event.deltaY))return 0;
  if(now-this.last>180){this.sum=0;this.fired=false;}
  this.last=now;
  if(this.fired)return 0;
  const scale=event.deltaMode===1?16:event.deltaMode===2?Math.max(48,pageHeight):1;
  this.sum+=event.deltaY*scale;
  if(Math.abs(this.sum)<48)return 0;
  this.fired=true;return Math.sign(this.sum);
 }
}
