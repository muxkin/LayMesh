// Exercise native-handle lifetime when an update fails after creating geometry.
// The CanvasKit allocator is replaced with handles that detect use after free.
import fs from 'node:fs';
import assert from 'node:assert/strict';
const file=new URL('../site/live/skia-renderer.mjs',import.meta.url);
const source=fs.readFileSync(file,'utf8')
 .replace("import CanvasKitInit from './skia/canvaskit.mjs';",'const CanvasKitInit=undefined;')
 .replace(/from '(\.\/[^']+)'/g,(_,relative)=>`from '${new URL(relative,file).href}'`);
const {SkiaRenderer}=await import('data:text/javascript;base64,'+Buffer.from(source).toString('base64'));
const created=[];
class PathBuilder {
 moveTo(){} lineTo(){} setFillType(){} delete(){}
 detach(){const path={deleted:false,delete(){assert(!this.deleted,'Handle freed twice');this.deleted=true;}};created.push(path);return path;}
}
const r=new SkiaRenderer({PathBuilder,FillType:{Winding:0,EvenOdd:1}},{},{software:true});
const empty={schemaVersion:1,width:10,height:10,nodes:[],resources:{}};
r.recordPictures=()=>{};r.planScene=()=>{};
let fail=false,lastPath;
r.bind=node=>{
 const entry={objects:[],dependencies:new Set()};r.entries.add(entry);r.owner=entry;
 lastPath=r.path(node.kind.path);r.owner=null;
 assert(!lastPath.deleted,'Recovery reused deleted geometry');
 if(fail)throw Error('injected renderer allocation failure');
 return entry;
};
await r.install(empty,()=>{},1);
const node={kind:{kind:'path',path:[{MoveTo:{x:0,y:0}},{LineTo:{x:1,y:1}}]}};
fail=true;
await assert.rejects(r.install({...empty,nodes:[node]},()=>{},2),/injected/);
assert.equal(r.list,empty);assert.equal(r.generation,1);assert(created[0].deleted);
fail=false;
await r.install({...empty,nodes:[node]},()=>{},3);
assert.equal(r.generation,3);assert.notEqual(lastPath,created[0]);assert(!lastPath.deleted);
r.clearCommands();assert(created.every(path=>path.deleted));
console.log('Failed drawing updates release new handles and recover without reusing freed geometry.');
