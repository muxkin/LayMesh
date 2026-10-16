// Compressed artifacts are owned by the current document and its active leases.
export class PresentationStore {
 constructor(){this.memory=new Map();this.bytes=0;this.pages=[];this.resources=new Set();this.closed=false;}
 async put(key,value,size){if(this.closed)throw Error('Presentation closed');const old=this.memory.get(key);if(old)this.bytes-=old.size;this.memory.delete(key);this.memory.set(key,{value,size});this.bytes+=size;
 }
 async get(key){if(this.closed)throw Error('Presentation closed');const item=this.memory.get(key);if(!item)throw Error('Missing presentation artifact: '+key);return item.value;}
 async add(page){for(const [id,bytes]of Object.entries(page.resources||{}))if(!this.resources.has(id)){const data=bytes instanceof Uint8Array?bytes:new Uint8Array(bytes);await this.put('resource:'+id,data,data.byteLength);this.resources.add(id);}
  const clean={...page};delete clean.resources;const text=JSON.stringify(clean),raw=new Blob([text]),packed=typeof CompressionStream==='function'?await new Response(raw.stream().pipeThrough(new CompressionStream('gzip'))).blob():raw;
  await this.put('page:'+page.id,{packed,gzip:packed!==raw,resourceIds:Object.keys(page.list.resources)},packed.size);const info={id:page.id,name:page.name,inspection:{speakerNotes:page.inspection.speakerNotes,warnings:page.inspection.warnings,page:page.inspection.page,animation:page.inspection.animation,pageTransition:page.list.pageTransition||page.inspection.pageTransition}};const i=this.pages.findIndex(p=>p.id===page.id);if(i<0)this.pages.push(info);else this.pages[i]=info;}
 async alias(id,name,source){await this.put('page:'+id,{alias:source},64);const original=this.pages.find(p=>p.id===source);this.pages.push({...original,id,name:name||id});}
 async page(id,{packed=false}={}){const entry=await this.get('page:'+id);if(entry.alias){const page=await this.page(entry.alias,{packed});page.id=id;page.name=this.pages.find(p=>p.id===id).name;return page;}
  const resources={};for(const key of entry.resourceIds)resources[key]=await this.get('resource:'+key);
  if(packed)return {id,name:this.pages.find(p=>p.id===id).name,packedPage:await entry.packed.arrayBuffer(),gzip:entry.gzip,resources};
  const text=entry.gzip?await new Response(entry.packed.stream().pipeThrough(new DecompressionStream('gzip'))).text():await entry.packed.text();return {...JSON.parse(text),resources};}

 async dispose(){this.closed=true;this.memory.clear();this.bytes=0;this.pages=[];this.resources.clear();}
}

// Keep compressed artifacts across preparation revisions. A session owns only
// its directory and leases; invalidating it does not discard unchanged pages.
export class RetainedPresentationCache {
 constructor(options){this.archive=new PresentationStore(options);this.artifacts=new Map();this.sessions=new Set();}
 known(){return [...this.artifacts.keys()];}
 session(){const cache=this,archive=this.archive;const store={pages:[],resources:archive.resources,closed:false,bindings:new Map(),
  async put(...args){return archive.put(...args);},async get(...args){return archive.get(...args);},
  async add(page,artifactId=page.id){if(this.closed)return;await archive.add({...page,id:artifactId});cache.artifacts.set(artifactId,{resources:Object.keys(page.list.resources),info:archive.pages.find(p=>p.id===artifactId)});if(this.closed)return;this.bind({id:page.id,name:page.name},artifactId);},
  bind(page,artifactId){if(this.closed)return;const item=cache.artifacts.get(artifactId);if(!item)throw Error('Missing retained presentation page');this.bindings.set(page.id,artifactId);const info={...item.info,...page};const i=this.pages.findIndex(p=>p.id===page.id);if(i<0)this.pages.push(info);else this.pages[i]=info;},
  async page(id,options){if(this.closed)throw Error('Presentation closed');const page=await archive.page(this.bindings.get(id),options);const info=this.pages.find(p=>p.id===id);return {...page,id,name:info.name};},
  async dispose(){this.closed=true;cache.sessions.delete(this);this.bindings.clear();this.pages=[];}
 };this.sessions.add(store);return store;}
 async prune(){const keep=new Set([...this.sessions].flatMap(s=>[...s.bindings.values()])),resources=new Set();for(const [id,item]of this.artifacts)if(keep.has(id))for(const r of item.resources)resources.add(r);else {this.artifacts.delete(id);await this.remove('page:'+id);}this.archive.pages=this.archive.pages.filter(p=>keep.has(p.id));for(const id of this.archive.resources)if(!resources.has(id)){this.archive.resources.delete(id);await this.remove('resource:'+id);}}
 async remove(key){const old=this.archive.memory.get(key);if(old){this.archive.bytes-=old.size;this.archive.memory.delete(key);}}
 async dispose(){for(const session of this.sessions)await session.dispose();this.artifacts.clear();await this.archive.dispose();}
}
