import fs from 'node:fs';
import assert from 'node:assert/strict';
const file=new URL('../site/live/skia-renderer.mjs',import.meta.url),source=fs.readFileSync(file,'utf8').replace("import CanvasKitInit from './skia/canvaskit.mjs';",'const CanvasKitInit=undefined;').replace(/from '(\.\/[^']+)'/g,(_,relative)=>`from '${new URL(relative,file).href}'`);
const {SkiaRenderer}=await import('data:text/javascript;base64,'+Buffer.from(source).toString('base64'));
const renderer=new SkiaRenderer({}, {}, {software:true});let cancelled=false,steps=0,timer;
let unwound=false;
const work=function*(){try{steps++;yield {pixels:256,bytes:0};steps++;}finally{unwound=true;}};
const preparing=renderer.prepareTasks(work(),()=>cancelled,()=>60000);
await new Promise(setImmediate);assert.equal(typeof renderer.preparationWake,'function');
cancelled=true;renderer.interruptPreparationWait();
try{
 const result=await Promise.race([preparing,new Promise((_,reject)=>{timer=setTimeout(()=>reject(Error('Urgent target waited for background cooldown')),100);})]);
 assert.equal(result.cancelled,true);assert.equal(steps,1,'Cancellation must not generate the next tile');assert.equal(renderer.preparationWake,null);assert.equal(unwound,true,'Cancellation must unwind the suspended drawing task');assert.equal(renderer.activePreparation,false);
}finally{clearTimeout(timer);renderer.interruptPreparationWait();}
console.log('Urgent cancellation wakes a 60-second background cooldown without drawing another tile.');
