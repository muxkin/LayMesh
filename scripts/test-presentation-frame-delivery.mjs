// Test frame ownership and the ready barrier without depending on a GPU driver.
import fs from 'node:fs';
import vm from 'node:vm';
import assert from 'node:assert/strict';
import {nextVisualChange} from '../site/live/presentation-visual-state.mjs';
const source=fs.readFileSync(new URL('../site/live/presentation-worker.mjs',import.meta.url),'utf8')
 .replace(/^import .*;\n/gm,'').replace('export async function','async function');
let renderer,resolveBitmap,rejectBitmap,closed=0,failNotification=false,planning=0,geometry=0;
const frames=[];
class Renderer {
 constructor(ck,canvas,options){renderer=this;this.options=options;this.pages=new Map();this.skip=false;this.skia={canvas,gr:{},interruptPreparationWait(){},setMediaFrames(){},gl:{getContextAttributes:()=>({preserveDrawingBuffer:true})},transitions:{cancel(){},capture(){},async play(){return {};}},completeFrame:async()=>true};}
 async install(id){this.pages.set(id,{ready:true,readySize:[10,10]});}
 async prepare(id,w,h,sample,priority,viewport,cancelled){
  sample(0,false);await new Promise(resolve=>setImmediate(resolve));
  return {cancelled:cancelled()};
 }
 evict(){} use(){} cost(){return 0;} residentBytes(){return 0;} dispose(){}
 render(){this.draws=(this.draws||0)+1;if(this.skip)return {skipped:true};this.options.onPresent();return {};}
}
const context=vm.createContext({document:{},performance,Promise,JSON,Map,Set,Error,nextVisualChange,
 init:async()=>({memory:{buffer:{byteLength:0}}}),initializeSkia:async()=>({HEAPU8:{byteLength:0},ColorType:{RGBA_8888:0},AlphaType:{Premul:0},ColorSpace:{SRGB:0}}),
 rendering_memory_stats:()=>[0,0,0,0],setSurfaceRasterizer(){},raster_surface3d(){},surface_texture_region(){},refine_surface3d(){},surface_material_texture(){},
 PresentationRenderer:Renderer,retain_render_resources(){},register_render_resource(){},
 prepare_display_list:()=>({free(){},frame_state:()=>{geometry++;return '{}';},transform_state:()=>{planning++;return '{}';}}),
 createImageBitmap:()=>new Promise((resolve,reject)=>{resolveBitmap=resolve;rejectBitmap=reject;}),
});
vm.runInContext(source,context);
const runtime=await context.createPresentationRuntime({canvas:{width:10,height:10},bitmapFrames:true},frame=>{
 if(failNotification)throw Error('injected transfer failure');
 frames.push(frame);
});
await runtime({type:'load',installOnly:true,page:{id:'a',resources:{},list:{nodes:[],resources:{}}}});
const request={type:'activate',pageId:'a',serial:1,width:10,height:10,time:null};
let complete=false;
const activation=runtime(request).then(value=>{complete=true;return value;});
await new Promise(resolve=>setImmediate(resolve));
assert.equal(complete,false,'Ready must wait for the bitmap');assert.equal(frames.length,0);
resolveBitmap({close(){closed++;}});
assert.equal((await activation).presentationConfirmed,true);assert.equal(frames.length,1);
assert.equal(frames[0].serial,1);assert.equal(frames[0].sequence,1);
renderer.skip=true;
await assert.rejects(runtime({...request,serial:2}),/drawing interrupted/);
assert.equal(frames.length,1,'Skipped activation must not publish the old bitmap');
const skipped=await runtime({...request,type:'paint',serial:2,running:true});
assert.equal(skipped.skipped,true);assert.equal(skipped.presentationConfirmed,false);
renderer.skip=false;
const failed=runtime({...request,serial:3});
rejectBitmap(Error('injected bitmap failure'));
await assert.rejects(failed,/injected bitmap failure/);
const recovered=runtime({...request,serial:4});
resolveBitmap({close(){closed++;}});
assert.equal((await recovered).presentationConfirmed,true,'A failed frame must not poison the next delivery');
assert.equal(frames.at(-1).serial,4);
const stale=runtime({...request,serial:5});
await runtime({type:'target',serial:6});
resolveBitmap({close(){closed++;}});
assert.equal((await stale).type,'stale');assert.equal(frames.at(-1).serial,5);
assert.equal(closed,0,'Successful transfer belongs to the receiver');
failNotification=true;
const undelivered=runtime({...request,serial:6});
resolveBitmap({close(){closed++;}});
await assert.rejects(undelivered,/injected transfer failure/);
assert.equal(closed,1,'Failed transfer must release the undelivered bitmap');
failNotification=false;
const retry=runtime({...request,serial:7});
resolveBitmap({close(){closed++;}});
assert.equal((await retry).presentationConfirmed,true);
const beforeSnapshotDraws=renderer.draws,beforeSnapshotSamples=geometry;
const snapshot=runtime({type:'snapshot'});resolveBitmap({close(){closed++;}});const transferred=await snapshot;
assert.equal(renderer.draws,beforeSnapshotDraws,'A retained display buffer transfers without redrawing');
assert.equal(geometry,beforeSnapshotSamples,'A retained display buffer transfers without sampling');transferred.bitmap.close();
const rawPixels=new Uint8Array(400);renderer.skia.gr=null;renderer.skia.surface={getCanvas:()=>({readPixels:()=>rawPixels})};
const raw=await runtime({type:'snapshot'});assert.equal(raw.rgba,rawPixels);assert.equal(raw.width,10);assert.equal(raw.height,10);assert.equal(raw.bitmap,undefined);assert.equal(renderer.draws,beforeSnapshotDraws,'CPU snapshot does not redraw');
await runtime({type:'target',serial:8});
await runtime({type:'background',serial:8,paused:false});
const beforeGeometry=geometry;
const background=runtime({type:'prepare',pageId:'a',serial:8,width:10,height:10});
await runtime({type:'background',serial:8,paused:true});
assert.equal((await background).preparation.cancelled,true,'Foreground input interrupts background preparation');
assert.equal(planning,1);assert.equal(geometry,beforeGeometry,'Planning must not sample animated geometry');
assert.equal((await runtime({type:'prepare',pageId:'a',serial:7})).type,'stale');
assert.equal((await runtime({type:'prepare',pageId:'a',serial:8})).type,'stale','Queued background work stays paused');
const foreground=await runtime({type:'prepare',pageId:'a',serial:8,priority:true});
assert.equal(foreground.preparation.cancelled,false,'The requested page keeps priority');
await runtime({type:'dispose'});
console.log('Frame delivery waits for the bitmap, rejects skipped activations, survives failures and preserves serials.');
