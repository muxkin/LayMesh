// Exact separable min/max on an 8-bit coverage mask. A monotone deque makes
// work linear in the mask area, independent of the spread radius. Outside the
// mask is transparent, as for Skia's decal morphology input.
export function spreadAlpha(source,width,height,rx,ry,dilate){
 const temporary=new Uint8Array(width*height),result=new Uint8Array(width*height);
 const deque=new Int32Array(Math.max(width+2*rx,height+2*ry)+1);
 const pass=(input,output,length,lines,radius,stride,lineStride)=>{
  for(let line=0;line<lines;line++){
   let head=0,tail=0;const base=line*lineStride;
   const value=i=>i<0||i>=length?0:input[base+i*stride];
   for(let i=-radius;i<length+radius;i++){
    while(head<tail&&deque[head]<i-2*radius)head++;
    const v=value(i);while(head<tail&&(dilate?value(deque[tail-1])<=v:value(deque[tail-1])>=v))tail--;
    deque[tail++]=i;if(i>=radius)output[base+(i-radius)*stride]=value(deque[head]);
   }
  }
 };
 pass(source,temporary,width,height,rx,1,width);pass(temporary,result,height,width,ry,width,1);return result;
}

// An axis-aligned box dilation/erosion translates each rounded-rectangle
// corner without changing its radius. Recognize the actual eight-cubic
// contour, including every control point; arbitrary paths retain morphology.
export function spreadRoundedRectPath(elements,bounds,dx,dy){
 const [x0,y0,x1,y1]=bounds,w=x1-x0,h=y1-y0,epsilon=1e-6*Math.max(1,w,h);
 const point=p=>Array.isArray(p)?p:[p?.x,p?.y],near=(p,q)=>p.every((v,i)=>Number.isFinite(v)&&Math.abs(v-q[i])<=epsilon);
 if(elements?.length===5&&elements[4]==='ClosePath'){
  const expected=[[x0,y0],[x1,y0],[x1,y1],[x0,y1]];
  if(!expected.every((p,i)=>near(point(i?elements[i]?.LineTo:elements[i]?.MoveTo),p))||w+2*dx<=0||h+2*dy<=0)return null;
  return [{MoveTo:{x:x0-dx,y:y0-dy}},{LineTo:{x:x1+dx,y:y0-dy}},{LineTo:{x:x1+dx,y:y1+dy}},{LineTo:{x:x0-dx,y:y1+dy}},'ClosePath'];
 }
 if(elements?.length!==13||elements[12]!=='ClosePath')return null;
 const start=point(elements[0]?.MoveTo),end=point(elements[2]?.CurveTo?.[2]),rx=end[0]-x0,ry=start[1]-y0;
 if(!(rx>0&&ry>0)||w+2*dx<2*rx-epsilon||h+2*dy<2*ry-epsilon)return null;
 const k=4/3*Math.tan(Math.PI/16),q=Math.SQRT1_2,quarter=[[0,1],[0,1-k],[1-q-k*q,1-q+k*q],[1-q,1-q],[1-q+k*q,1-q-k*q],[1-k,0],[1,0]];
 for(let corner=0;corner<4;corner++){
  const i=corner*3,actual=[point(corner?elements[i]?.LineTo:elements[i]?.MoveTo),...(elements[i+1]?.CurveTo||[]).map(point),...(elements[i+2]?.CurveTo||[]).map(point)];
  if(actual.length!==7)return null;
  const expected=quarter.map(([u,v])=>corner===0?[x0+rx*u,y0+ry*v]:corner===1?[x1-rx+rx*(1-v),y0+ry*u]:corner===2?[x1-rx*u,y1-ry*v]:[x0+rx*v,y1-ry*u]);
  if(!actual.every((p,j)=>near(p,expected[j])))return null;
 }
 const shift=(p,corner)=>{const [x,y]=point(p);return {x:x+([0,3].includes(corner)?-dx:dx),y:y+(corner<2?-dy:dy)};};
 return elements.map((el,i)=>{if(el==='ClosePath')return el;const corner=Math.floor(i/3);return el.MoveTo?{MoveTo:shift(el.MoveTo,corner)}:el.LineTo?{LineTo:shift(el.LineTo,corner)}:{CurveTo:el.CurveTo.map(p=>shift(p,corner))};});
}
