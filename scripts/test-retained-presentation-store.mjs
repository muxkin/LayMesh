import assert from 'node:assert/strict';
import {RetainedPresentationCache} from '../site/live/presentation-store.mjs';
const cache=new RetainedPresentationCache();
const page=(id,text,resource)=>({id,name:text,inspection:{speakerNotes:text},list:{nodes:[{kind:{kind:'path',path:text}}],resources:resource?{[resource]:{mime:'image/png'}}:{}},resources:resource?{[resource]:new Uint8Array([1,2,3])}:{}});
try {
 const first=cache.session();await first.add(page('a','first','r1'),'artifact-a');await first.add(page('b','second','r2'),'artifact-b');
 await first.dispose();assert.deepEqual(cache.known(),['artifact-a','artifact-b'],'Invalidating a preparation must retain its artifacts');
 const next=cache.session();next.bind({id:'renamed',name:'New title'},'artifact-a');next.bind({id:'alias',name:'Alias'},'artifact-a');
 assert.equal((await next.page('renamed')).name,'New title');assert.equal((await next.page('alias')).list.nodes[0].kind.path,'first');
 const packed=await next.page('alias',{packed:true});assert(packed.packedPage instanceof ArrayBuffer);assert(!packed.list);assert.equal(packed.id,'alias');assert.equal(packed.name,'Alias');assert.deepEqual(Object.keys(packed.resources),['r1']);
 await cache.prune();assert.deepEqual(cache.known(),['artifact-a']);assert(cache.archive.resources.has('r1'));assert(!cache.archive.resources.has('r2'));
 const leased=cache.session();leased.bind({id:'old',name:'Old active snapshot'},'artifact-a');await next.add(page('renamed','changed','r3'),'artifact-c');await next.dispose();await cache.prune();assert.deepEqual(cache.known(),['artifact-a']);assert.equal((await leased.page('old')).list.nodes[0].kind.path,'first');
 await leased.dispose();const empty=cache.session();await cache.prune();assert.equal(cache.archive.bytes,0);assert.equal(cache.archive.resources.size,0);await empty.dispose();
 console.log('Presentation artifacts: reuse, aliases, immutable leases, removal and document lifecycle cleanup passed');
}finally{await cache.dispose();}
