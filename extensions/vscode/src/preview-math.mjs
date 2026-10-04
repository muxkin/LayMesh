// All geometry in inspection is mm. Screen pixels are CSS pixels, never export pixels.
export function mmPerUnit(unit, dpi=96) {
 return ({mm:1,cm:10,in:25.4,inch:25.4,pt:25.4/72,px:25.4/dpi})[unit]??1;
}
export function apply(m, x, y) {return [m[0]*x+m[2]*y+m[4],m[1]*x+m[3]*y+m[5]];}
export function invert(m) {
 const d=m[0]*m[3]-m[1]*m[2];
 if(!Number.isFinite(d)||Math.abs(d)<1e-20)return null;
 return [m[3]/d,-m[1]/d,-m[2]/d,m[0]/d,(m[2]*m[5]-m[3]*m[4])/d,(m[1]*m[4]-m[0]*m[5])/d];
}
export function canvasAt(page, rect, x, y) {
 if(rect.width<=0||rect.height<=0||x<rect.left||x>rect.left+rect.width||y<rect.top||y>rect.top+rect.height)return null;
 const mm=[(x-rect.left)/rect.width*page.width,(y-rect.top)/rect.height*page.height];
 const factor=mmPerUnit(page.unit,page.layout_dpi);
 return {mm,display:mm.map(v=>v/factor)};
}
export function niceStep(minimum) {
 const base=10**Math.floor(Math.log10(minimum));
 return ([1,2,5,10].find(n=>n*base>=minimum)||10)*base;
}
const transform=(v,scale,c)=>scale==='log'?Math.log(v):scale==='symlog'?Math.sign(v)*Math.log1p(Math.abs(v)/c):v;
const inverse=(v,scale,c)=>scale==='log'?Math.exp(v):scale==='symlog'?Math.sign(v)*Math.expm1(Math.abs(v))*c:v;
export function axisValue(axis, pixel) {
 for(const s of axis.segments||[]) {
  const [a,b]=s.range;
  if(pixel<Math.min(a,b)-1e-9||pixel>Math.max(a,b)+1e-9||a===b)continue;
  const scale=axis.scale||'linear',c=axis.constant??1;
  const lo=transform(s.domain[0],scale,c),hi=transform(s.domain[1],scale,c);
  const value=inverse(lo+(pixel-a)/(b-a)*(hi-lo),scale,c);
  return Number.isFinite(value)?value:null;
 }
 return null;
}
const inRect=(p,r)=>p[0]>=r.x-1e-9&&p[1]>=r.y-1e-9&&p[0]<=r.x+r.width+1e-9&&p[1]<=r.y+r.height+1e-9;
export function preparePlots(inspection) {
 return (inspection.plots||[]).map(p=>({...p,inverse:invert(p.page_transform),clips:(p.clips||[]).map(c=>({...c,inverse:invert(c.transform)}))}));
}
function polarValues(m, p) {
 const dx=p[0]-m.center[0],dy=m.center[1]-p[1],rho=Math.hypot(dx,dy),inner=m.innerRadius||0,outer=m.outerRadius;
 if(rho<inner-1e-9||rho>outer+1e-9)return null;
 const period=m.angleUnit==='rad'?Math.PI*2:360,lo=m.theta[0],hi=m.theta[1];
 let theta=rho<1e-10?null:(Math.atan2(dy,dx)-m.thetaZero)/m.direction*period/(Math.PI*2);
 if(theta!==null){theta=lo+((theta-lo)%period+period)%period;if(theta>hi+1e-8)return null;}
 const a=m.radial;
 let f=(rho-inner)/(outer-inner);if(a.reverse)f=1-f;
 const r=inverse(transform(a.domain[0],a.scale,a.constant??1)+f*(transform(a.domain[1],a.scale,a.constant??1)-transform(a.domain[0],a.scale,a.constant??1)),a.scale,a.constant??1);
 return {theta,r};
}
function radarValues(plot, m, p, pagePoint, cssPerMm) {
 const center=m.center;
 if(Math.hypot(p[0]-center[0],p[1]-center[1])<1e-10)return {};
 const origin=apply(plot.page_transform,...center);let best=null;
 for(let i=0;i<m.categories.length;i++) {
  const angle=m.thetaZero+m.direction*i*2*Math.PI/m.categories.length;
  const end=apply(plot.page_transform,center[0]+m.outerRadius*Math.cos(angle),center[1]-m.outerRadius*Math.sin(angle));
  const vx=end[0]-origin[0],vy=end[1]-origin[1],length=vx*vx+vy*vy;
  const f=((pagePoint[0]-origin[0])*vx+(pagePoint[1]-origin[1])*vy)/length;
  if(f<0||f>1)continue;
  const distance=Math.hypot(pagePoint[0]-origin[0]-f*vx,pagePoint[1]-origin[1]-f*vy)*cssPerMm;
  if(distance<=6&&(!best||distance<best.distance))best={distance,name:m.categories[i],value:m.ranges[i][0]+f*(m.ranges[i][1]-m.ranges[i][0])};
 }
 return best?{[best.name]:best.value}:{};
}
function radarContains(m,p) {
 const dx=p[0]-m.center[0],dy=p[1]-m.center[1];
 if(m.radarFrame!=='polygon')return Math.hypot(dx,dy)<=m.outerRadius+1e-9;
 const n=m.categories.length;let sign=0;
 for(let i=0;i<n;i++) {
  const a=m.thetaZero+m.direction*i*2*Math.PI/n,b=m.thetaZero+m.direction*(i+1)*2*Math.PI/n;
  const x1=m.outerRadius*Math.cos(a),y1=-m.outerRadius*Math.sin(a),x2=m.outerRadius*Math.cos(b),y2=-m.outerRadius*Math.sin(b);
  const cross=(x2-x1)*(dy-y1)-(y2-y1)*(dx-x1);
  if(Math.abs(cross)<1e-9)continue;
  if(sign&&Math.sign(cross)!==sign)return false;sign=Math.sign(cross);
 }
 return true;
}
export function plotAt(plots, pagePoint, cssPerMm, containsPath) {
 for(let i=plots.length-1;i>=0;i--) {
  const plot=plots[i];if(!plot.inverse)continue;
  const p=apply(plot.inverse,...pagePoint);
  if(!inRect(p,plot.plot_area))continue;
  if(!plot.clips.every(c=>{if(!c.inverse)return false;const q=apply(c.inverse,...pagePoint);return c.rect?inRect(q,c.rect):!!containsPath?.(c.path,q);}))continue;
  const m=plot.projection;let values;
  if(m?.kind==='polar') {values=polarValues(m,p);if(!values)continue;}
  else if(m?.kind==='radar') {if(!radarContains(m,p))continue;values=radarValues(plot,m,p,pagePoint,cssPerMm);}
  else {values={};for(const [name,axis] of Object.entries(plot.axes||{})){const horizontal=['top','bottom'].includes(axis.side);values[name]=axisValue(axis,p[horizontal?0:1]-plot.plot_area[horizontal?'x':'y']);}}
  if(plot.clip_boundary&&containsPath&&!containsPath(plot.clip_boundary,p))continue;
  return {plot,values,angleUnit:m?.angleUnit};
 }
 return null;
}
export function formatData(value) {return value===null||!Number.isFinite(value)?'—':value.toPrecision(6);}
