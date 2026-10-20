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
const layer=new SkiaRenderer({ClipOp:{Intersect:0},ColorType:{RGBA_8888:1},AlphaType:{Premul:1},ColorSpace:{SRGB:1}}, {}, {software:true});
layer.surface={makeSurface:()=>({getCanvas:()=>({clipRect(){},clear(){},getSaveCount(){return 1;},save(){},restore(){},restoreToCount(){},translate(){},scale(){},drawPicture(){}}),flush(){},makeImageSnapshot:()=>{tiles++;return{delete(){tiles--;}};},dispose(){}})};
const shape={uid:1,bounds:[0,0,10000,10000],picture:{}};
const built=layer.buildLayer(shape,1);
assert(built&&built.bytes>400_000_000);assert.equal(layer.buildLayer(shape,1),built);
layer.clearLayerTextures();assert.equal(tiles,0);
console.log('Large tiled layer retained above 100M pixels and released; surface handles mocked.');
const sharedRenderer=new SkiaRenderer(ck,{}, {software:true});
const first=sharedRenderer.acquireImage('same',new Uint8Array([1,2,3]),'image/png');
const second=sharedRenderer.acquireImage('same',new Uint8Array([1,2,3]),'image/png');
assert.equal(first.image,second.image);assert.equal(first.bytes,second.bytes);assert.equal(sharedRenderer.imageDecodes,1);
const changed=sharedRenderer.acquireImage('same',new Uint8Array([1,2,4]),'image/png');
assert.notEqual(changed.image,first.image,'Changed bytes with the same resource ID must invalidate decoding');
sharedRenderer.releaseImage(first.image);assert.equal(first.references,1);assert.equal(sharedRenderer.imagePool.get('same').length,2);
sharedRenderer.releaseImage(second.image);sharedRenderer.releaseImage(changed.image);assert.equal(sharedRenderer.imagePool.size,0);
console.log('Immutable decoded images and encoded bytes are shared by identity and released by the last owner.');
// Placement aliases share content while stable bindings keep unrelated static
// run identities across a fresh display-list decode or an opacity-only edit.
const paths=[];
class ShapeBuilder{moveTo(){}lineTo(){}close(){}setFillType(){}delete(){}detach(){const p={getBounds:()=>[0,0,10,10],delete(){this.deleted=true;}};paths.push(p);return p;}}
const bindings=new SkiaRenderer({PathBuilder:ShapeBuilder,FillType:{Winding:0}}, {}, {software:true});
const subject={name:'one',opacity:1,transform:[1,0,0,1,0,0],effects:[],kind:{kind:'path',path:[{MoveTo:[0,0]},{LineTo:[10,0]},{LineTo:[10,10]},'ClosePath'],fill:{kind:'none'},stroke:null}};
const initial=bindings.prepare(subject),same=bindings.prepare(structuredClone(subject));assert.equal(same.uid,initial.uid);
const faded=bindings.prepare({...structuredClone(subject),opacity:.4});assert.notEqual(faded.uid,initial.uid);assert.equal(faded.textureKey,initial.uid);assert.equal(faded.path,initial.path);
const moved=bindings.prepare({...structuredClone(subject),transform:[1,0,0,1,20,0]});assert.equal(moved.textureKey,initial.uid);assert.deepEqual(moved.matrix,[1,0,20,0,1,0,0,0,1]);
assert.equal(bindings.prepare(structuredClone(subject)).uid,initial.uid);assert.equal(paths.length,1);bindings.clearCommands();assert(paths.every(p=>p.deleted));
console.log('Unchanged decoded bindings retain IDs; placement and opacity share one immutable geometry resource.');
const sparse=Object.create(SkiaRenderer.prototype),simple={name:'paper',opacity:1},label={name:'label',opacity:1},effect={name:'effect',opacity:1};
Object.assign(sparse,{list:{nodes:[simple,label,effect]},animatedNames:new Set(),staticTextures:new Map(),layerTextures:new Map(),entries:new Set(),updateCacheLimit(){},prepare:n=>({uid:n===simple?1:n===label?2:3,cost:n===simple?5:100,hasEffects:n===effect,bounds:n===simple?[0,0,320,180]:n===label?[12,14,80,25]:[20,40,100,100],matrix:[1,0,0,0,1,0,0,0,1]})});
sparse.planScene();assert.equal(sparse.sceneRuns.length,3);assert.equal(sparse.sceneRuns[0].cacheable,false);assert(sparse.sceneRuns.slice(1).every(r=>r.cacheable));
assert.deepEqual(sparse.sceneRuns[1].bounds,[12,14,80,25]);
console.log('Sparse static content retains tight object bounds without a duplicate page-background texture.');
// Identical static artwork on two resident pages owns one physical tile. Page
// removal must keep the other owner's pixels alive; a changed view cannot use it.
layer.drawNode=()=>{};
const run={key:'page-a',contentKey:'same artwork and resources',bounds:[0,0,10,10],nodes:[]};
const finish=work=>{let step;do{step=work.next();}while(!step.done);return step.value;};
const a=finish(layer.staticTasks(run,[0,0,10,10],100,100)),mapA=layer.staticTextures,bytesA=layer.staticTextureBytes,allocated=layer.textureAllocated;
layer.staticTextures=new Map();layer.staticTextureBytes=0;
const b=finish(layer.staticTasks({...run,key:'page-b'},[0,0,10,10],100,100)),mapB=layer.staticTextures;
assert.equal(a,b);assert.equal(layer.textureAllocated,allocated);assert.equal(a.references,2);
layer.staticTextures=mapA;layer.staticTextureBytes=bytesA;layer.clearStaticTextures();assert.equal(tiles,1);assert.equal(a.references,1);
layer.staticTextures=mapB;layer.staticTextureBytes=b.bytes;
const resized=finish(layer.staticTasks({...run,key:'page-b'},[0,0,10,10],200,200));assert.notEqual(resized,b);assert.equal(layer.staticTexturePool.size,1);
layer.clearStaticTextures();assert.equal(tiles,0);assert.equal(layer.staticTexturePool.size,0);layer.releaseScratch();
console.log('Resident pages share identical static pixels; removal and resolution changes preserve exact ownership.');
let driverLimit;
layer.gr={getResourceCacheUsageBytes:()=>0,setResourceCacheLimitBytes:n=>driverLimit=n};
layer.canvas={width:1920,height:920};layer.cacheViewportPixels=1920*920;layer.updateCacheLimit();const beforeHide=driverLimit;
layer.canvas.width=layer.canvas.height=1;layer.updateCacheLimit();assert.equal(driverLimit,beforeHide);
console.log('Releasing a hidden display framebuffer does not lower the driver cache allowance.');

// A CPU tile changes representation, not content or cache identity. Later
// rotation restores Skia once; disposal releases either representation.
let nativeTiles=0;
const pixelContent=Uint8Array.from({length:100},(_,i)=>i);
const nativeTile=(pixels,readable=true)=>{nativeTiles++;let alive=true;return {readPixels:()=>readable?pixels.slice():null,delete(){assert(alive);alive=false;nativeTiles--;}};};
const cpu=new SkiaRenderer({ColorType:{RGBA_8888:1},AlphaType:{Premul:1},ColorSpace:{SRGB:1},MakeImage(info,pixels){assert.deepEqual(pixels,pixelContent);return nativeTile(pixels);}}, {}, {software:true});
const addCPUTile=(key,readable=true)=>{const entry={bytes:100,tiles:[{w:1,h:1,image:nativeTile(pixelContent,readable)}]};cpu.layerTextures.set(key,entry);cpu.layerTextureBytes+=100;cpu.textureAllocated=(cpu.textureAllocated||0)+100;cpu.textureImages=(cpu.textureImages||0)+1;return entry;};
const cpuEntry=addCPUTile(1);
assert(cpu.cpuLayerPixels(cpuEntry));assert.equal(nativeTiles,0);assert.equal(cpu.cpuTileBytes,100);assert.equal(cpuEntry.tiles[0].image,null);assert.deepEqual(cpuEntry.tiles[0].pixels,pixelContent);
const promoted=cpu.textureAllocated;assert(cpu.cpuLayerPixels(cpuEntry));assert.equal(cpu.textureAllocated,promoted,'Repeated translation must reuse owned pixels');
cpu.restoreLayerImages(cpuEntry);assert.equal(nativeTiles,1);assert.equal(cpu.cpuTileBytes,0);assert.equal(cpuEntry.tiles[0].pixels,null);assert(cpuEntry.cpuBlitDisabled);
const restored=cpu.textureAllocated;cpu.restoreLayerImages(cpuEntry);assert.equal(cpu.textureAllocated,restored);
cpu.deleteLayerTexture(1,cpuEntry);assert.equal(nativeTiles,0);
const pixelsOnly=addCPUTile(2);cpu.cpuLayerPixels(pixelsOnly);cpu.deleteLayerTexture(2,pixelsOnly);assert.equal(cpu.cpuTileBytes,0);assert.equal(nativeTiles,0);
const unreadable=addCPUTile(3,false);assert.equal(cpu.cpuLayerPixels(unreadable),false);assert.equal(nativeTiles,1);assert(unreadable.cpuBlitDisabled);cpu.deleteLayerTexture(3,unreadable);
assert.equal(nativeTiles,0);assert.equal(cpu.textureImages,0);assert.equal(cpu.textureAllocated,cpu.textureReleased);assert.equal(cpu.layerTextureBytes,0);
console.log('CPU tile promotion, one-time fallback, failed readback and release retain one owned representation.');
