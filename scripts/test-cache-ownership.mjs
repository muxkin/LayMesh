// Exercise actual retained resource buffers beyond the former 128-item/256 MiB
// quotas, and verify release on replacement. GPU handles use a lifetime oracle.
import fs from 'node:fs';
import assert from 'node:assert/strict';
import {PageTransitions} from '../site/live/presentation-transitions.mjs';
import {transitions} from '../site/live/presentation-transitions-data.mjs';
const file=new URL('../site/live/skia-renderer.mjs',import.meta.url);
const source=fs.readFileSync(file,'utf8').replace("import CanvasKitInit from './skia/canvaskit.mjs';",'const CanvasKitInit=undefined;').replace(/from '(\.\/[^']+)'/g,(_,relative)=>`from '${new URL(relative,file).href}'`);
const {SkiaRenderer}=await import('data:text/javascript;base64,'+Buffer.from(source).toString('base64'));
let live=0,decoded=0;
const ck={PictureRecorder:class{beginRecording(){return {};}finishRecordingAsPicture(){return{delete(){}};}delete(){}},MakeImageFromEncoded(){decoded++;live++;return{delete(){live--;}};}};
const r=new SkiaRenderer(ck,{}, {software:true});
const resources=Object.fromEntries(Array.from({length:130},(_,i)=>['image-'+i,{mime:'image/png'}]));
const list={schemaVersion:1,width:10,height:10,nodes:[],resources};
let reads=0;
await r.install(list,async()=>{reads++;return new Uint8Array(2*1024*1024).fill(reads%256);},1);
assert.equal(r.images.size,130);assert.equal(live,130);
const bytes=[...r.imageBytes.values()].reduce((n,b)=>n+b.byteLength,0);assert.equal(bytes,260*1024*1024);
await r.update({schemaVersion:1,width:10,height:10,nodes:[],roots:null,resources:null},()=>{throw Error('Unchanged resource read again');},2);
assert.equal(decoded,130);assert.equal(r.images.size,130);
await r.install({...list,resources:{}},()=>{},3);assert.equal(live,0);assert.equal(r.imageBytes.size,0);
r.dispose();
let effects=0,deleted=0;
const t=new PageTransitions({ck:{RuntimeEffect:{Make(){effects++;return{delete(){deleted++;}};}}}});
const names=Object.keys(transitions).slice(0,12);for(const name of names)t.effect(name);for(const name of names)t.effect(name);
assert.equal(effects,12);assert.equal(t.effects.size,12);assert.equal(deleted,0);
t.retain(new Set(names.slice(0,2)));assert.equal(t.effects.size,2);assert.equal(deleted,10);t.dispose();assert.equal(deleted,12);
console.log(JSON.stringify({resourceCount:130,retainedBytes:bytes,unchangedDecodeCount:decoded,releasedImages:130,transitionEffects:12,releasedEffects:deleted}));

// Tile allocation oracle: a live layer may exceed the previous 100M-pixel
// admission threshold. Individual surface allocations still decide success.
let tiles=0;
const layer=new SkiaRenderer({ColorType:{RGBA_8888:1},AlphaType:{Premul:1},ColorSpace:{SRGB:1}}, {}, {software:true});
layer.surface={makeSurface:()=>({getCanvas:()=>({clear(){},translate(){},scale(){},drawPicture(){}}),makeImageSnapshot:()=>{tiles++;return{delete(){tiles--;}};},dispose(){}})};
const shape={uid:1,bounds:[0,0,10000,10000],picture:{}};
const built=layer.buildLayer(shape,1);
assert(built&&built.bytes>400_000_000);assert.equal(layer.buildLayer(shape,1),built);
layer.clearLayerTextures();assert.equal(tiles,0);
console.log('Large tiled layer retained above 100M pixels and released; surface handles mocked.');
