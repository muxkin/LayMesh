// Foreground input must never depend on an unrelated neighbor's preparation.
import assert from 'node:assert/strict';
import {Presentation} from '../site/live/presentation.mjs';
import {PresentationClock} from '../site/live/presentation-clock.mjs';
const deferred=()=>{let resolve;const promise=new Promise(r=>resolve=r);return {promise,resolve};};
const blocked=deferred(),messages=[];
globalThis.document={hidden:false};
globalThis.requestAnimationFrame=fn=>setImmediate(fn);
const v=Object.assign(Object.create(Presentation.prototype),{
 opened:true,ready:true,generation:1,serial:1,index:0,resident:new Set(),readyResident:new Set(),
 deck:{pages:[{id:'a',inspection:{}},{id:'b',inspection:{}}]},
 clock:new PresentationClock(),reduced:{matches:false},metrics:{transitions:[]},
 root:{dataset:{}},overlay:{},menu:{hidden:true},mask:{hidden:true},
 worker:{postMessage:m=>messages.push(m)},sync(){},dimensions:()=>({width:10,height:10}),
 preload:blocked.promise,preloading:true,load:async()=>{},
 call:async()=>({type:'done'}),paint(){this.paints=(this.paints||0)+1;},
 fail(error){throw error;},
});
v.action('next');
await new Promise(r=>setImmediate(r));await new Promise(r=>setImmediate(r));
assert.equal(v.index,1,'A ready target must appear before the unrelated preload resolves');
assert.equal(v.switching,false);
assert(messages.some(m=>m.type==='background'&&m.paused));
// Preserve deliberate input protection: no queued extra page after a transition.
v.switching=true;const serial=v.serial;v.action('next');v.action('previous');
assert.equal(v.serial,serial);assert.equal(v.index,1);v.switching=false;
v.clock.install({duration:1,steps:[{start:0,duration:1}],reducedSteps:[]});
v.preload=blocked.promise;v.preloading=true;v.action('next');
assert.equal(v.clock.running,true,'Starting the active animation cannot wait for a neighbor');
assert.equal(v.paints,1);
blocked.resolve();await new Promise(r=>setImmediate(r));
assert.equal(v.index,1,'Completing a preload must not replay earlier input');
// Navigation during asynchronous artifact reads must not install the old target.
const reading=deferred();let loads=0;
v.resident.clear();v.clock.running=false;v.deck.store={page:()=>reading.promise};
v.call=async()=>{loads++;return {resident:[],readyResident:[]};};
const stale=Presentation.prototype.load.call(v,0,true);await new Promise(r=>setImmediate(r));
v.serial++;reading.resolve({id:'a'});await stale;assert.equal(loads,0);
console.log('Foreground navigation/animation bypass preloads; transition input protection and stale-read cancellation pass.');
