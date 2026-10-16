import assert from 'node:assert/strict';
import {WebCompilerPool} from '../site/live/web-compiler-pool.mjs';
const tick=()=>new Promise(r=>setTimeout(r,0)),workers=[];
const pool=new WebCompilerPool({parallelism:()=>2,initialize:()=>({fonts:[],wasm:new Uint8Array()}),knownPages:()=>({a:'accepted'}),
 create(){const worker={requests:[],postMessage(data){this.requests.push(data);if(data.type==='init')queueMicrotask(()=>this.reply(data,{}));},reply(request,data){this.onmessage({data:{id:request.id,...data}});},terminate(){this.terminated=true;}};workers.push(worker);return worker;}});
const snapshot={type:'compile',targetPageId:'a',version:1,files:{'/a':'a'},assets:{'/image':new Uint8Array([1])},assetKeys:{'/image':'one'}};
try{
 const first=pool.call(snapshot);await tick();const worker=workers[0],request=worker.requests.find(r=>r.type==='compile');
 assert.deepEqual(request.fileChanges,{'/a':'a'});assert.equal(request.knownPages.a,'accepted');assert(!pool.canRun('a'));
 worker.reply(request,{pages:[],wasmMemoryBytes:100});await first;
 assert(!pool.canRun('a',['a']),'Reserved jobs still checking fonts occupy their owner');
 const second=pool.call({...snapshot,version:2,files:{'/a':'changed'}});await tick();const changed=worker.requests.at(-1);
 assert.deepEqual(changed.fileChanges,{'/a':'changed'});assert.deepEqual(changed.assetChanges,{});
 worker.reply(changed,{pages:[],wasmMemoryBytes:110});await second;assert.equal(pool.memoryBytes,110,'Memory reflects current allocation, not cumulative traffic');
 const helper=pool.call({type:'fontInfo',bytes:new Uint8Array()});await tick();pool.retain([]);assert(!worker.terminated,'Do not terminate an in-flight resource request');
 worker.reply(worker.requests.findLast(r=>r.type==='fontInfo'),{faces:[]});await helper;pool.retain([]);assert(worker.terminated);assert.equal(pool.clients.size,0);
}finally{pool.dispose();}
console.log('Compiler pool: stable owners, input deltas, reservations, live accounting and lifecycle passed');
{
 let release;
 const initialization=new Promise(r=>release=r);
 const pool=new WebCompilerPool({create:()=>({postMessage(){assert.fail('Disposed worker received initialization');},terminate(){}}),initialize:()=>initialization,knownPages:()=>({})});
 const request=pool.call({type:'fontInfo'});pool.dispose();release({});
 await assert.rejects(request,/closed/);
}

{
 let attempts=0;
 const pool=new WebCompilerPool({create:()=>({postMessage(data){queueMicrotask(()=>this.onmessage({data:{id:data.id}}));},terminate(){}}),initialize:()=>{if(!attempts++)throw Error('Initialization failed');return {};},knownPages:()=>({})});
 await assert.rejects(pool.call({type:'fontInfo'}),/Initialization failed/);
 assert.equal(pool.clients.size,0,'A failed initialization must release its worker');
 await pool.call({type:'fontInfo'});pool.dispose();
}
