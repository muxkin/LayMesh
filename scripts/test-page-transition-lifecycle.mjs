import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {fileURLToPath} from 'node:url';
import {PageTransitions} from '../site/live/presentation-transitions.mjs';
const ck=await createRequire(import.meta.url)('../vendor/canvaskit/canvaskit.js')({locateFile:()=>fileURLToPath(new URL('../vendor/canvaskit/canvaskit.wasm',import.meta.url))});
let now=0,callback;globalThis.performance={now:()=>now};globalThis.requestAnimationFrame=fn=>{callback=fn;return 1;};globalThis.cancelAnimationFrame=()=>callback=null;
const surface=ck.MakeSurface(32,24),s={ck,surface,canvas:{width:32,height:24},images:new Map(),imageBytes:new Map(),budget:1048576},p=new PageTransitions(s);
const spec={name:'fade',durationMs:100},draw=color=>{surface.getCanvas().clear(color);surface.flush();};
const rgba=()=>{const image=surface.makeImageSnapshot();try{return image.readPixels(0,0,{width:32,height:24,colorType:ck.ColorType.RGBA_8888,alphaType:ck.AlphaType.Unpremul,colorSpace:ck.ColorSpace.SRGB});}finally{image.delete();}};
function prepare(){draw(ck.RED);p.capture(spec);draw(ck.BLUE);now=0;return rgba();}
let expected=prepare(),done=p.play();now=25;callback();p.cancel();assert.equal((await done).transitionCancelled,true);assert.deepEqual(rgba(),expected);assert.equal(p.pending,null);assert.equal(p.active,null);
prepare();await p.play(true);assert.equal(p.pending,null);assert.equal(p.active,null);
p.navigate({pageTransition:spec},{pageId:'a',pageNumber:1});assert.equal(p.pending,null);p.navigate({pageTransition:spec},{pageId:'a',pageNumber:1});assert.equal(p.pending,null);
expected=prepare();const original=p.effect.bind(p);p.effect=()=>({getUniformFloatCount:()=>1,getUniformCount:()=>1,getUniformName:()=> 'progress',getUniform:()=>({slot:0}),makeShaderWithChildren:()=>{throw Error('injected shader failure');}});
p.warmed.add('fade');done=p.play();now=25;callback();const failed=await done;assert.match(failed.transitionError,/injected/);assert.deepEqual(rgba(),expected);p.effect=original;
prepare();done=p.play();s.lost=true;p.cancel();await done;assert.equal(p.active,null);s.lost=false;
prepare();done=p.play();p.dispose();await done;assert.equal(p.effects.size,0);assert.equal(p.active,null);surface.dispose();
console.log('Cancellation, reduced motion, first/same page, runtime fallback, context loss and disposal passed.');
