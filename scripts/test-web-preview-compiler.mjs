import assert from 'node:assert/strict';
import { PreviewCompiler } from '../site/live/web-preview-compiler.mjs';
const gate = () => { let resolve; const promise = new Promise(r => resolve = r); return { promise, resolve }; };
const nextTick = () => new Promise(r => setTimeout(r, 0));
let files = { '/doc.laydoc': 'a,b', '/a.lay': 'a', '/b.lay': 'b', '/theme.lcss': 'red' };
let active = 'b', revision = 0, composing = false, fontVersion = 0, assetKeys = {};
const calls = [], updates = [], commits = [];
let held, started, failure;
const c = new PreviewCompiler({
  pages: [{id:'a'}, {id:'b'}],
  snapshot: () => ({ entry: '/doc.laydoc', files: {...files}, assets: {}, assetKeys: {...assetKeys}, fontVersion }),
  revision: () => revision, active: () => active, paused: () => composing,
  async compile(s) {
    assert.equal(s.activePageId,c.current?.key,'background work must identify the active drawing baseline');
    calls.push([s.targetPageId, s.files['/theme.lcss']]);
    const wait = held; held = null; started?.resolve();
    if (wait) await wait.promise;
    if (failure === s.targetPageId) throw Object.assign(Error('syntax error'),{dependencies:['/missing.svg']});
    const directory = s.files['/doc.laydoc'].split(',').map(id => ({id}));
    const id = directory.some(p => p.id === s.targetPageId) ? s.targetPageId : directory[0].id;
    return { directory, pages: [{id, dependencies:['/doc.laydoc','/'+id+'.lay','/theme.lcss'],fonts:[],requestedFonts:[],warnings:[]}] };
  },
  update: r => {updates.push(r.pages[0].id);},
  commit: r => commits.push(r.pages[0].id),
});
const edit = (path, value, delay=0) => { files[path]=value; revision++; c.schedule(delay); };
try {
  edit('/b.lay','b1'); await c.ensure();
  assert.deepEqual(calls.map(v=>v[0]),['b','a'], 'active page must compile first');
  const previousA = c.records[0].value;
  calls.length=0;edit('/b.lay','b2');await c.ensure();
  assert.deepEqual(calls.map(v=>v[0]),['b']);assert.equal(c.records[0].value,previousA);
  const beforeUpdates=updates.length;
  calls.length=0;held=gate();const old=held;started=gate();edit('/theme.lcss','green');await started.promise;
  edit('/theme.lcss','blue');edit('/theme.lcss','yellow');active='a';old.resolve();await c.ensure();
  assert.deepEqual(calls,[['b','green'],['a','yellow'],['b','yellow']], 'latest edit and selected page replace queued work');
  assert.equal(updates.length-beforeUpdates,2,'superseded results must never be committed');
  assert.deepEqual(updates.slice(-2),['a','b']);
  assert.deepEqual(commits,updates,'only accepted results can become transport baselines');
  calls.length=0;composing=true;edit('/a.lay','IME');await nextTick();assert.equal(calls.length,0);
  await assert.rejects(c.ensure(),/完成输入/);composing=false;c.schedule(0);await c.ensure();assert.equal(calls.length,1);
  failure='b';edit('/b.lay','bad');await assert.rejects(c.ensure(),/syntax error/);
  assert.equal(c.records.find(r=>r.key==='a').valid,true);assert.equal(c.complete,false);
  assert(c.records.find(r=>r.key==='b').dependencies.includes('/missing.svg'));
  failure=null;edit('/b.lay','fixed');await c.ensure();assert.equal(c.complete,true);
  calls.length=0;fontVersion++;c.schedule(0);await c.ensure();assert.equal(calls.length,2);
  calls.length=0;assetKeys['/@laymesh/font-bindings.json']='changed';c.schedule(0);await c.ensure();assert.equal(calls.length,2);
  edit('/doc.laydoc','b,c');await c.ensure();assert.deepEqual(c.records.map(r=>r.key),['b','c']);
  calls.length=0;edit('/unused.lay','ignored');await c.ensure();assert.equal(calls.length,0);
  console.log('Preview compiler: priority, dependency reuse, coalescing, IME, errors, fonts and page changes passed');
} finally { c.dispose(); }

// Directory-only responses retain drawings but must replace planning reads.
{
  let files={'/doc':'first','/catalog':'a'},version=0,calls=0;
  const value={id:'a',listId:'accepted'};
  const compiler=new PreviewCompiler({pages:[value],snapshot:()=>({entry:'/doc',files:{...files},assetKeys:{}}),revision:()=>version,active:()=> 'a',paused:()=>false,
    compile:async()=>{calls++;return {directory:[{id:'a',name:'New name'}],pages:[],unchanged:['a'],unchangedDependencies:{a:['/doc','/catalog']}};},update:()=>{}});
  try {
    compiler.schedule(0);await compiler.ensure();assert.equal(compiler.records[0].value,value);
    files['/catalog']='b';version++;compiler.schedule(0);await compiler.ensure();assert.equal(calls,2,'new document module must invalidate retained directory');
  }finally{compiler.dispose();}
}

// A valid transport reply can still fail during rendering or be superseded
// while painting. Neither reply is an accepted baseline for the next request.
{
  let revision=0,source='initial',hold,started,fail=false;
  const commits=[];
  const compiler=new PreviewCompiler({pages:[{id:'a'}],snapshot:()=>({entry:'/a',files:{'/a':source},assetKeys:{}}),revision:()=>revision,active:()=> 'a',paused:()=>false,
    compile:async s=>({directory:[{id:'a'}],pages:[{id:'a',listId:s.files['/a'],dependencies:['/a'],fonts:[],requestedFonts:[]}]}),
    update:async()=>{const wait=hold;hold=null;started?.resolve();if(wait)await wait.promise;if(fail)throw Error('render failed');},
    commit:r=>commits.push(r.pages[0].listId)});
  const edit=value=>{source=value;revision++;compiler.schedule(0);};
  try {
    fail=true;edit('failed');await assert.rejects(compiler.ensure(),/render failed/);assert.deepEqual(commits,[]);
    fail=false;hold=gate();const old=hold;started=gate();edit('superseded');await started.promise;edit('latest');old.resolve();await compiler.ensure();assert.deepEqual(commits,['latest']);
  }finally{compiler.dispose();}
}
