// Exercise the real preview class with failed runtime initialization, without
// requiring a GPU. Mock only its renderer dependencies through the ESM linker.
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {SourceTextModule, SyntheticModule} from 'node:vm';

let attempts=0,disposed=0,installs=0;
const canvas={width:40,height:20},runtime={};
const module=new SourceTextModule(await readFile(new URL('../site/live/page-previews.mjs',import.meta.url),'utf8'));
await module.link(async specifier=>{
 const exports=specifier==='./skia-renderer.mjs'?{initializeSkia:async()=>({})}:
  specifier==='./page-thumbnails.mjs'?{PageThumbnails:class{}}:
  {createPresentationRuntime:async()=>{
   attempts++;
   if(attempts===1)throw Error('Temporary WASM initialization failure');
   return async message=>{
    if(message.type==='dispose'){disposed++;return {};}
    if(message.type==='load'){installs++;return {resident:[message.page.id]};}
    if(message.type==='activate'){canvas.width=message.width;canvas.height=message.height;}
    return {};
   };
  }};
 return new SyntheticModule(Object.keys(exports),function(){for(const [key,value]of Object.entries(exports))this.setExport(key,value);});
});
await module.evaluate();
const renderer=new module.namespace.PagePreviewRenderer({canvas,runtime:async()=>runtime,page:async()=>({id:'page',list:{width:160,height:90}})});
await assert.rejects(renderer.draw(0,320),/Temporary WASM/);
assert.equal(await renderer.draw(0,320),canvas);
assert.equal(attempts,2,'a failed initialization must be retried');
assert.deepEqual([canvas.width,canvas.height],[320,180]);
await renderer.draw(0,160);
assert.equal(attempts,2,'successful initialization is reused');
assert.equal(installs,1,'a retry must not duplicate the page installation');
await renderer.dispose();
assert.equal(disposed,1);
assert.equal(await renderer.draw(0,160),undefined);
assert.equal(attempts,2,'disposed surfaces cannot restart');
console.log('Page previews: failed initialization recovers, successful runtime is reused, disposal is final.');
