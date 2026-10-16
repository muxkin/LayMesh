import assert from 'node:assert/strict';
import {PresentationWorkers} from '../site/live/presentation-workers.mjs';
const canvas=()=>({style:{},classList:{add(){},remove(){},toggle(){}},cloneNode:canvas,after(){},remove(){this.removed=true;},transferControlToOffscreen:()=>({})});
const workers=[];let release,started;
const entered=new Promise(resolve=>started=resolve),blocked=new Promise(resolve=>release=resolve);
class Worker {
 constructor(){this.messages=[];this.pages=new Set();this.queue=Promise.resolve();workers.push(this);}
 postMessage(data,transfer){
  this.messages.push(data);
  if(['target','background'].includes(data.type))return;
  this.queue=this.queue.then(async()=>{
   if(data.type==='load'&&data.page.id==='b'){started();await blocked;}
   if(data.type==='load')this.pages.add(data.page.id);
   if(data.type==='retain')this.pages=new Set([...this.pages].filter(id=>data.pages.includes(id)));
   if(data.type==='activate'){
    if(data.previousFrame){assert.equal(transfer[0],data.previousFrame.bitmap);data.previousFrame.bitmap.close();}
    this.onmessage({data:{type:'navigation-visible',pageId:data.pageId,serial:data.serial}});
   }
   const bitmap=data.type==='snapshot'?{close(){this.closed=true;}}:undefined;
   this.onmessage({data:{type:'done',id:data.id,bitmap,resident:[...this.pages],readyResident:[],entryFrames:[]}});
  });
 }
 terminate(){this.closed=true;}
}
const view={canvas:canvas()},pool=new PresentationWorkers(view,()=>new Worker());
await pool.run({type:'init'},[]);assert.equal(workers[0].messages.find(m=>m.type==='init').preserveDrawingBuffer,true,'Keep displayed pixels available for navigation');assert.equal(workers.length,1,'The first page never waits for a second runtime');
pool.postMessage({type:'target',serial:1});
await pool.run({type:'load',serial:1,priority:true,page:{id:'a',list:{nodes:[]}}},[]);
await pool.run({type:'activate',serial:1,pageId:'a'},[]);
const background=pool.run({type:'load',serial:1,page:{id:'b',list:{nodes:[]}}},[]);
await entered;
const frame=await pool.run({type:'paint',serial:1,pageId:'a'},[]);
assert.equal(frame.renderer,0,'Animation completes while background preparation is still blocked');
pool.postMessage({type:'background',serial:1,paused:true});
assert.equal(workers[1].messages.at(-1).paused,false,'Painting does not pause the independent preparation worker');
release();await background;
pool.postMessage({type:'target',serial:2});
await pool.run({type:'activate',serial:2,pageId:'b',transition:{reverse:false}},[]);
assert.equal(pool.active,pool.renderers[1]);assert.equal(view.canvas,pool.renderers[1].canvas);
assert.equal(workers[1].messages.filter(m=>m.type==='load'&&m.page.id==='b').length,1,'Handoff does not reinstall prepared geometry');
assert(workers[1].messages.find(m=>m.type==='activate').previousFrame.bitmap.closed,'The transferred transition bitmap is released');
await pool.run({type:'load',serial:2,page:{id:'c',list:{nodes:[]}}},[]);
assert.equal(pool.owners.get('c'),pool.renderers[0],'The former display worker becomes the preparation worker');
const before=workers[1].messages.length;
assert((await pool.run({type:'prepare',serial:2,pageId:'b'},[])).deferred);
assert.equal(workers[1].messages.length,before,'Background work never enters the active renderer queue');
await pool.run({type:'retain',pages:['b','c']},[]);assert(!pool.owners.has('a'));
pool.terminate();assert(workers.every(w=>w.closed));assert.equal(pool.owners.size,0);
console.log('Independent animation/preparation, retained-page handoff, bitmap ownership, pruning and shutdown passed.');
