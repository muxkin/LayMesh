import assert from 'node:assert/strict';
import {PreviewCompiler} from '../site/live/web-preview-compiler.mjs';
import {createWebPresentationSession} from '../site/live/web-presentation-session.mjs';
const gate=()=>{let resolve;return {promise:new Promise(r=>resolve=r),resolve:()=>resolve()};};
const tick=()=>new Promise(r=>setTimeout(r,0));
let source='initial',revision=0;
const started=[],holds={a:gate(),b:gate(),c:gate()},committed=[];
const compiler=new PreviewCompiler({pages:['a','b','c'].map(id=>({id,inspection:{}})),snapshot:()=>({entry:'/doc',files:{'/doc':source},assetKeys:{}}),revision:()=>revision,active:()=> 'a',paused:()=>false,concurrency:()=>3,
  async compile(s,plan){started.push(s.targetPageId);await plan({directory:['a','b','c'].map(id=>({id}))});await holds[s.targetPageId].promise;return {directory:['a','b','c'].map(id=>({id})),pages:[{id:s.targetPageId,listId:s.targetPageId+'@'+s.version,inspection:{},dependencies:['/doc'],fonts:[],requestedFonts:[]}]};},
  async update(_,records){await tick();},commit:r=>committed.push(r.pages[0].id)});
try {
  source='changed';revision++;compiler.schedule(0);
  const deck=await createWebPresentationSession({compiler,startPageId:'a',load:(p,id)=>({...p,id})});
  assert.deepEqual(started,['a'],'Directory is available while first evaluation is held');
  holds.a.resolve();await deck.wait(0);await compiler.ensure('a');
  for(let i=0;started.length<3&&i<20;i++)await tick();assert.deepEqual(started,['a','b','c'],'Background evaluations overlap after first commit');
  assert.equal(compiler.complete,false,'First page plays while unrelated pages are blocked');
  assert.equal((await deck.store.page('a')).listId,'a@1');
  await assert.rejects(compiler.ensure('missing'),/不存在/);
  holds.c.resolve();await deck.wait(2);assert.equal(compiler.complete,false);
  holds.b.resolve();await compiler.ensure();assert.deepEqual(committed,['a','c','b']);
  assert(compiler.records.every(r=>r.valid),'Concurrent commits merge into the latest records');
  source='new';revision++;compiler.schedule(0);
  await assert.rejects(deck.wait(0),/版本/,'Old readiness cannot leak into new source version');
  await deck.store.dispose();assert.equal(compiler.listeners.size,0);
  await compiler.ensure();
}finally{compiler.dispose();}
console.log('Progressive compile: early directory, blocked neighbor, concurrent merge, missing page, version and disposal passed');

// Supersede multiple active pages together. Out-of-order old replies must not
// overwrite the new version, including replies already queued for painting.
{
 let source='0',revision=0,phase=0;
 const holds=new Map(),started=[],accepted=[];
 const c=new PreviewCompiler({pages:['a','b','c'].map(id=>({id})),snapshot:()=>({entry:'/doc',files:{'/doc':source},assetKeys:{}}),revision:()=>revision,active:()=> 'a',paused:()=>false,concurrency:()=>3,
  async compile(s){started.push([s.version,s.targetPageId]);if(s.version===1&&s.targetPageId!=='a'){const hold=gate();holds.set(s.targetPageId,hold);await hold.promise;}return {directory:['a','b','c'].map(id=>({id})),pages:[{id:s.targetPageId,listId:String(s.version),dependencies:['/doc'],fonts:[],requestedFonts:[]}]};},
  update:async()=>tick(),commit:r=>accepted.push([c.version,r.pages[0].id])});
 try{
  source='1';revision++;c.schedule(0);await c.ensure('a');
  while(holds.size<2)await tick();source='2';revision++;c.schedule(0);
  holds.get('c').resolve();holds.get('b').resolve();await c.ensure();
  assert(c.records.every(r=>r.value.listId==='2'));assert.equal(accepted.filter(([v])=>v===1).length,1);
 }finally{c.dispose();}
}
{
 const c=new PreviewCompiler({pages:[{id:'a'}],snapshot:()=>({entry:'/doc',files:{},assetKeys:{}}),revision:()=>0,active:()=> 'a',paused:()=>false,
  compile:async()=>({directory:[],pages:[]}),update:()=>{}});
 try{c.schedule(0);await assert.rejects(c.ensure(),/没有可用页面/);assert.equal(c.records[0].value.id,'a');}
 finally{c.dispose();}
}
