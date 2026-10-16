import assert from 'node:assert/strict';
import fs from 'node:fs';
import {Presentation} from '../site/live/presentation.mjs';
const source=fs.readFileSync(new URL('../site/live/presentation-renderer.mjs',import.meta.url),'utf8').replace(/^import .*;\n/gm,'');
const {PresentationRenderer}=await import('data:text/javascript;base64,'+Buffer.from(source).toString('base64'));
const renderer=Object.create(PresentationRenderer.prototype);
renderer.pages=new Map(Array.from({length:12},(_,i)=>[String(i),{baseBytes:257*1048576,state:{},frames:new Map()}]));
assert.deepEqual(renderer.evict(new Set(['0'])),[]);
assert.equal(renderer.pages.size,12,'Live pages survive old count and byte thresholds');
renderer.skia={transitions:{retain(){}}};
renderer.remove=id=>renderer.pages.delete(id);renderer.retain(new Set(['0','1']));assert.equal(renderer.pages.size,2);
const messages=[];
const view=Object.assign(Object.create(Presentation.prototype),{
 opened:true,ready:true,generation:1,serial:1,index:0,painting:false,switching:false,
 clock:{running:false,time:0,completed:0,staticPage:true},deck:{pages:[{id:'a',inspection:{}}]},
 worker:{postMessage:m=>messages.push(m)},metrics:{frames:0},reduced:{matches:false},dimensions:()=>({width:10,height:10}),
 resident:new Set(['a']),readyResident:new Set(),entryFrames:new Set(),
 call:async request=>{messages.push(request);return {type:'done',resident:['a'],readyResident:[],entryFrames:['a/10/10/0/false']};},
 fail:e=>{throw e;}
});
await view.load(0,false);assert.equal(messages.at(-1).type,'prepare','Installed unfinished pages resume');
messages.length=0;
view.call=async()=>({type:'done',skipped:true});await view.paint();await view.preload;
assert(messages.some(m=>m.type==='background'&&!m.paused),'Idle drawing resumes background preparation');
await view.preload;
messages.length=0;view.clock.running=true;
const lastSuccessful={key:'a/visit-0/step-0',time:123};view.lastCompletion=lastSuccessful;
view.schedulePreload();await view.load(0,false,'warm');await view.paint();
assert.equal(view.lastCompletion,lastSuccessful,'A skipped submission must not hide the gap since the last successful frame');
assert(!messages.some(m=>m.type==='background'&&!m.paused),'Completing an animation frame must not restart background preparation');
assert(!messages.some(m=>m.type==='prepare'),'No neighbor preparation while the animation owns the renderer');
view.clock.running=false;await view.paint();await view.preload;
assert(messages.some(m=>m.type==='background'&&!m.paused),'Animation completion resumes background preparation');
await view.preload;messages.length=0;view.worker.parallel=true;view.worker.canPrepare=id=>id!=='a';view.clock.running=true;
view.deck.pages.push({id:'b',inspection:{}});view.resident.add('b');
view.call=async request=>{messages.push(request);return {type:'done',resident:['a','b'],readyResident:[],entryFrames:[]};};
view.schedulePreload();await view.preload;
assert(messages.some(m=>m.pageId==='b'&&m.type==='prepare'),'Independent background work continues during animation');
assert(!messages.some(m=>m.pageId==='a'),'No preparation is inserted into the current renderer');
console.log('Document ownership, serial fallback, parallel preload during playback, and skipped-frame timing passed.');
