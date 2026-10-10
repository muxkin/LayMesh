// Count distinct animation images at most once per browser refresh, not worker
// replies (several replies can arrive before a single image reaches the screen).
export class PlaybackStats {
 constructor(){this.frames=[];this.fps=0;}
 record(key,now){
  if(key===this.key)return;this.key=key;
  if(this.frames.length&&now-this.frames.at(-1)>250)this.frames=[];
  this.frames.push(now);while(this.frames.length>1&&this.frames[0]<now-1000)this.frames.shift();
  const span=now-this.frames[0];if(span>=250)this.fps=(this.frames.length-1)*1000/span;
 }
}
