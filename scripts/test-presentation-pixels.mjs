import assert from 'node:assert/strict';
import {premultiplyPixels} from '../site/live/skia-cpu-surface.mjs';
import {opaquePageRect,opaqueFadeRect} from '../site/live/presentation-transitions.mjs';
for(let a=0;a<=255;a++)for(let c=0;c<=255;c++){const b=new Uint8Array([c,c,c,a]);premultiplyPixels(b);assert.deepEqual([...b],[Math.round(c*a/255),Math.round(c*a/255),Math.round(c*a/255),a]);}
const background={opacity:1,effects:[],hint:null,transform:[1,0,0,1,0,0],kind:{kind:'path',fill:{kind:'solid',rgba:[.2,.4,.6,1]},stroke:null,path:[{MoveTo:{x:0,y:0}},{LineTo:{x:10,y:0}},{LineTo:{x:10,y:10}},{LineTo:{x:0,y:10}},'ClosePath']}};
const list={width:10,height:10,nodes:[background]};
assert.deepEqual(opaquePageRect(list,{},[-.25,0,10.25,10],105,100),[4,1,101,99]);
for(const mutate of [n=>n.opacity=.5,n=>n.kind.fill.rgba[3]=.9,n=>n.effects.push({}),n=>n.transform[4]=1,n=>n.kind.path.pop(),n=>n.hint={surface3d:{}}]){const d=structuredClone(list);mutate(d.nodes[0]);assert.equal(opaquePageRect(d,{},[0,0,10,10],100,100),null);}
assert.equal(opaquePageRect(list,{'0':{opacity:.5}},[0,0,10,10],100,100),null);
const image={width:()=>100,height:()=>100};
assert.deepEqual(opaqueFadeRect(image,image,[1,2,99,98],[3,1,97,99],100,100),[3,2,97,98]);
assert.equal(opaqueFadeRect(image,image,[1,2,99,98],null,100,100),null);
assert.equal(opaqueFadeRect(image,image,[1,2,99,98],[1,2,99,98],200,100),null);
console.log('All 65,536 premultiplication channel/alpha pairs, conservative opacity proof, edge and resize fallback passed.');
