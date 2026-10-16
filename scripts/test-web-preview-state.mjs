import assert from 'node:assert/strict';
import {makePreviewState,restorePreviewState} from '../site/live/web-preview-state.mjs';
import {PreviewCompiler} from '../site/live/web-preview-compiler.mjs';
const encode=s=>new TextEncoder().encode(s),files={'runtime/compiler/laymesh.wasm':encode('compiler'),'a':encode('a'),'b':encode('b'),'font':encode('font'),'pa':encode('page a'),'pb':encode('page b')};
const manifest={project:{entry:'/doc',files:{'/a':'a','/b':'b'}},fonts:[{path:'/font',file:'font'}],pages:[{id:'a',file:'pa'},{id:'b',file:'pb'}]};
const records=['a','b'].map(key=>({key,valid:true,dependencies:['/doc','/'+key],fonts:[],requestedFonts:[],warnings:[]}));
const state=makePreviewState(manifest,files,records);files['preview-state.json']=encode(JSON.stringify(state));assert.equal(Object.keys(restorePreviewState(manifest,files)).length,2);
for(const key of ['a','b','font','pa','runtime/compiler/laymesh.wasm']){const before=files[key];files[key]=encode('changed');assert.deepEqual(restorePreviewState(manifest,files),{});files[key]=before;}
assert.deepEqual(restorePreviewState({...manifest,fontBindings:{x:'y'}},files),{});assert.deepEqual(restorePreviewState({...manifest,project:{...manifest.project,entry:'/other'}},files),{});
let source={'/a':'a','/b':'b'},revision=0;const calls=[];
const c=new PreviewCompiler({pages:[{id:'a'},{id:'b'}],initial:restorePreviewState(manifest,files),snapshot:()=>({entry:'/doc',files:{...source},assetKeys:{}}),revision:()=>revision,active:()=> 'a',paused:()=>false,compile:async s=>{calls.push(s.targetPageId);return {directory:manifest.pages,pages:[{id:'a',dependencies:['/doc','/a'],fonts:[],requestedFonts:[],warnings:[]}]};},update:()=>{}});
try{source['/a']='edited';revision++;c.schedule(0);await c.ensure();assert.deepEqual(calls,['a']);}finally{c.dispose();}
console.log('Preview state: exact source/font/page binding and cold local edit invalidation passed');
