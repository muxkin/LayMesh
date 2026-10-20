// Conservative intervals in which sampling cannot change visible artwork.
// The authored clock still advances through holds and step boundaries.
export function nextVisualChange(animation,time,reduced=false){
 if(time===null)return Infinity;
 const timeline=animation?.[reduced?'reduced':'normal'];
 if(!timeline)return Infinity;
 let next=Infinity;
 for(const step of timeline.steps||[]){
  const stepEnd=step.start+step.duration;
  if(step.start>time)next=Math.min(next,step.start);
  if(stepEnd>time)next=Math.min(next,stepEnd);
  for(const clip of step.clips||[]){
   const start=step.start+clip.delay,iterations=clip.iterations===null?Infinity:clip.iterations??1;
   const end=Math.min(stepEnd,start+clip.duration*iterations);
   if(start>time)next=Math.min(next,start);
   if(!clip.paused&&clip.tracks?.length&&time>=start&&time<end)return time;
   if(end>time)next=Math.min(next,end);
  }
 }
 return next;
}
