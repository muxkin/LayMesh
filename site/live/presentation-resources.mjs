import {projectedDensity} from './skia-renderer.mjs';
const multiply=(a,b)=>Array.from({length:9},(_,i)=>{const r=Math.floor(i/3)*3,c=i%3;return a[r]*b[c]+a[r+1]*b[c+3]+a[r+2]*b[c+6];});
// These are transform probes, not frames: no drawing, clock, rehearsal or
// readback. The renderer still checks actual density on every real frame, so
// an unsampled extremum can rebuild but can never display an undersized layer.
export function layerPlan(skia,width,height,sample){
 const scale=Math.min(width/skia.list.width,height/skia.list.height),root=[scale,0,0,0,scale,0,0,0,1],layers=new Map(),unrecorded=new Set();let probes=0;
 const visit=(nodes,states,parent,prefix='')=>nodes.forEach((node,i)=>{
  const id=prefix+i,p=skia.prepare(node),state=states[id],world=multiply(parent,state?.matrix||p.matrix);
  if(skia.animatedNames.has(node.name)&&!p.dynamicChildren&&node.kind.kind==='group'&&p.cost>=128){
   if(!p.picture){unrecorded.add(p.uid);return;}
   const density=projectedDensity(world,p.bounds),old=layers.get(p.uid);
   if(Number.isFinite(density)&&density>0&&(!old||density>old.density))layers.set(p.uid,{p,density});
  }
  if(node.kind.kind==='group'&&p.dynamicChildren)visit(node.kind.children,states,world,id+'/');
 });
 visit(skia.list.nodes,{},root);
 for(const [reduced,timeline]of [[false,skia.list.animation?.normal],[true,skia.list.animation?.reduced]]){
  const times=new Set([0]);
  for(const step of timeline?.steps||[]){
   times.add(step.start);times.add(step.start+step.duration);
   for(const clip of step.clips){
    const tracks=clip.tracks.filter(t=>['transform','transform-origin'].includes(t.property));if(!tracks.length||clip.duration<=0)continue;
    const offsets=[...new Set([0,1,...tracks.flatMap(t=>t.frames.map(f=>f.offset))])].sort((a,b)=>a-b),cycle=Math.max(0,Math.floor(-clip.delay/clip.duration));
    for(let iteration=cycle;iteration<=cycle+1;iteration++)for(let i=1;i<offsets.length;i++)for(let k=0;k<=16;k++){
     const f=offsets[i-1]+(offsets[i]-offsets[i-1])*k/16,time=step.start+clip.delay+(iteration+f)*clip.duration;
     if(time>=step.start&&time<=step.start+step.duration)times.add(time);
    }
   }
  }
  // Bound planning work even for adversarial timelines; runtime admission and
  // density checks remain authoritative outside this planning envelope.
  const ordered=[...times].sort((a,b)=>a-b),stride=Math.max(1,Math.ceil(ordered.length/1024));
  for(let i=0;i<ordered.length;i+=stride){visit(skia.list.nodes,sample(ordered[i],reduced),root);probes++;}
 }
 return {layers:[...layers.values()],probes,unrecorded:unrecorded.size};
}
