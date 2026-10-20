import assert from 'node:assert/strict';
import {spreadAlpha,spreadRoundedRectPath} from '../site/live/shadow-mask.mjs';
// Compare against the definition, including holes, translucency, decal edges,
// anisotropic kernels, and radii larger than the source.
for(const [w,h]of [[1,1],[3,7],[13,9]])for(const [rx,ry]of [[0,0],[1,2],[7,12]])for(const dilate of [false,true]){
 const pixels=Uint8Array.from({length:w*h},(_,i)=>(i*127+i*i*23)%256),expected=new Uint8Array(w*h);
 for(let y=0;y<h;y++)for(let x=0;x<w;x++){
  let value=dilate?0:255;for(let dy=-ry;dy<=ry;dy++)for(let dx=-rx;dx<=rx;dx++){
   const xx=x+dx,yy=y+dy,alpha=xx<0||xx>=w||yy<0||yy>=h?0:pixels[yy*w+xx];value=dilate?Math.max(value,alpha):Math.min(value,alpha);
  }expected[y*w+x]=value;
 }
 assert.deepEqual(spreadAlpha(pixels,w,h,rx,ry,dilate),expected);
}
console.log('Separable shadow spread matches the pixel definition.');

const rectangle=[{MoveTo:{x:0,y:0}},{LineTo:{x:20,y:0}},{LineTo:{x:20,y:10}},{LineTo:{x:0,y:10}},'ClosePath'];
assert.deepEqual(spreadRoundedRectPath(rectangle,[0,0,20,10],2,3),[{MoveTo:{x:-2,y:-3}},{LineTo:{x:22,y:-3}},{LineTo:{x:22,y:13}},{LineTo:{x:-2,y:13}},'ClosePath']);
assert.equal(spreadRoundedRectPath(rectangle,[0,0,20,10],-11,-6),null);
const concave=structuredClone(rectangle);concave[2].LineTo.x=8;assert.equal(spreadRoundedRectPath(concave,[0,0,20,10],2,3),null);
assert.equal(spreadRoundedRectPath([...rectangle,...rectangle],[0,0,20,10],2,3),null);
console.log('Contour shortcut rejects concave, multi-contour and degenerate morphology.');
