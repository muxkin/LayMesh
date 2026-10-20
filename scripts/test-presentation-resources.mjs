import assert from 'node:assert/strict';
import fs from 'node:fs';
const rendererFile=new URL('../site/live/skia-renderer.mjs',import.meta.url);
const rendererSource=fs.readFileSync(rendererFile,'utf8')
 .replace("import CanvasKitInit from './skia/canvaskit.mjs';",'const CanvasKitInit=undefined;')
 .replace(/from '(\.\/[^']+)'/g,(_,relative)=>"from '"+new URL(relative,rendererFile).href+"'");
const rendererURL='data:text/javascript;base64,'+Buffer.from(rendererSource).toString('base64');
const planSource=fs.readFileSync(new URL('../site/live/presentation-resources.mjs',import.meta.url),'utf8')
 .replace("'./skia-renderer.mjs'",JSON.stringify(rendererURL));
const {layerPlan}=await import('data:text/javascript;base64,'+Buffer.from(planSource).toString('base64'));
const identity=[1,0,0,0,1,0,0,0,1];
const node={name:'animated',kind:{kind:'group',children:[]}};
const prepared={uid:1,matrix:identity,bounds:[0,0,10,10],cost:128,picture:{},dynamicChildren:false};
const timeline={steps:[{start:0,duration:1,clips:[{target:'animated',delay:0,duration:1,tracks:[{property:'transform',frames:[{offset:0},{offset:1}]}]}]}]};
const skia={list:{width:100,height:100,nodes:[node],animation:{normal:timeline,reduced:timeline}},animatedNames:new Set(['animated']),prepare:()=>prepared};
let calls=0;
const plan=await layerPlan(skia,100,100,t=>{calls++;return {'0':{matrix:[1+t,0,0,0,1+t,0,0,0,1]}};});
assert.equal(plan.layers.length,1);assert.equal(plan.layers[0].density,2);assert.equal(plan.probes,calls);
skia.animatedNames.clear();calls=0;
assert.equal((await layerPlan(skia,100,100,()=>{calls++;throw Error('No cacheable animation needs sampling');})).probes,0);assert.equal(calls,0);
skia.animatedNames.add('animated');let cancelled=false;
setTimeout(()=>cancelled=true,0);
const partial=await layerPlan(skia,100,100,()=>{
 const end=performance.now()+2;while(performance.now()<end){}
 return {};
},()=>cancelled);
assert.equal(partial.cancelled,true);assert(partial.probes<plan.probes,'Planning yields before the whole timeline completes');
console.log('Transform density, empty-scene fast path and cooperative cancellation passed.');
const resourceURL='data:text/javascript;base64,'+Buffer.from(planSource).toString('base64');
const presentationSource=fs.readFileSync(new URL('../site/live/presentation-renderer.mjs',import.meta.url),'utf8')
 .replace("'./skia-renderer.mjs'",JSON.stringify(rendererURL))
 .replace("'./presentation-resources.mjs'",JSON.stringify(resourceURL));
const {PresentationRenderer}=await import('data:text/javascript;base64,'+Buffer.from(presentationSource).toString('base64'));
const cpu=new PresentationRenderer({}, {}, {software:true});
const state={list:{width:100,height:100,nodes:[node],animation:{normal:timeline,reduced:timeline}},pictureBytes:0,staticTextureBytes:0,layerTextureBytes:0,animatedNames:new Set(['animated']),layerTextures:new Map(),sceneRuns:[]};
cpu.pages.set('cpu',{id:'cpu',state,baseBytes:0,frames:new Map()});
const built=[];cpu.skia.ensureSurface=()=>{};cpu.skia.prepare=()=>prepared;
cpu.skia.layerTasks=function*(p,density){if(this.layerTextures.get(p.uid)?.density>=density)return this.layerTextures.get(p.uid);built.push({id:p.uid,density});yield {bytes:400};this.layerTextures.set(p.uid,{complete:true,density});};
cpu.skia.completeResources=async()=>{};
const preparedCPU=await cpu.prepare('cpu',100,100,t=>({'0':{matrix:[1+t,0,0,0,1+t,0,0,0,1]}}),true);
assert.equal(preparedCPU.complete,true);assert.deepEqual(built,[{id:1,density:2}]);
await cpu.prepare('cpu',100,100,()=>{throw Error('Ready content must be reused');},true);
assert.equal(built.length,1);
assert.equal(cpu.pages.get('cpu').frames.size,0,'Warm content must not allocate duplicate page snapshots');
console.log('CPU preparation uses the same projected content density and retained tiles, without endpoint snapshots.');

const entryState={...state,layerTextures:new Map()};
cpu.pages.set('entry',{id:'entry',state:entryState,baseBytes:0,frames:new Map()});
const entry=await cpu.prepareEntry('entry',100,100,0,false,t=>({'0':{matrix:[1+t,0,0,0,1+t,0,0,0,1]}}));
assert.equal(entry.complete,true);assert.equal(built.length,2);assert.equal(built[1].density,2,'Entry plans the peak density, not only time zero');
assert.equal(cpu.pages.get('entry').frames.size,1);assert.equal(cpu.pages.get('entry').frames.values().next().value.image,undefined);
assert.equal(cpu.pages.get('entry').ready,true);
console.log('First-frame preparation covers future scale without duplicate endpoint bitmaps.');
const sizes=[];cpu.skia.ensureSurface=(w,h)=>sizes.push([w,h]);
cpu.pages.set('hidden',{id:'hidden',state:{...state,layerTextures:new Map()},baseBytes:0,frames:new Map()});
await cpu.prepareEntry('hidden',100,100,0,false,t=>({'0':{matrix:[1+t,0,0,0,1+t,0,0,0,1]}}),()=>false,false,()=>0,true);
assert.deepEqual(sizes,[[1,1]]);assert.equal(built.at(-1).density,2);
console.log('Hidden preparation retains full projected tile density without a duplicate display framebuffer.');
