// Page ownership keeps semantic sessions and drawing baselines warm. Workers
// grow with runnable document work and the browser's available parallelism.
export class WebCompilerPool {
  constructor({create, initialize, knownPages, accept, parallelism=()=>navigator.hardwareConcurrency || 1}) {
    Object.assign(this,{create,initialize,knownPages,accept,parallelism});
    this.clients = new Set(); this.owners = new Map(); this.serial = 0; this.latestVersion = 0;
  }
  get memoryBytes() { return [...this.clients].reduce((n,c)=>n+(c.memoryBytes || 0),0); }
  canRun(key, scheduled=[]) {
    const owner = this.owners.get(key);
    return owner ? !owner.busy && !scheduled.some(key=>this.owners.get(key)===owner) : [...this.clients].some(c=>!c.busy) || this.clients.size < this.parallelism();
  }
  send(c, data, onPlan, onProgress) {
    return new Promise((resolve,reject)=>{
      if(this.closed || !this.clients.has(c)){reject(Error('Compiler pool closed'));return;}
      const id = ++this.serial;
      c.pending.set(id,{resolve,reject,onPlan,onProgress});
      try{c.worker.postMessage({...data,id});}catch(error){c.pending.delete(id);reject(error);}
    });
  }
  client() {
    const c = {id:++this.serial, worker:this.create(), pending:new Map(), inputs:{files:{},assetKeys:{}}};
    this.clients.add(c);
    c.worker.onmessage = ({data}) => {
      const p = c.pending.get(data.id);
      if (!p) return;
      c.memoryBytes = data.wasmMemoryBytes || c.memoryBytes;
      if (data.type === 'compile-progress') { p.onProgress?.({...data,worker:c.id}); return; }
      if (data.type === 'compile-plan') { Promise.resolve(p.onPlan?.(data)).catch(p.reject); return; }
      c.pending.delete(data.id);
      if (data.error) p.reject(Object.assign(Error(data.error),{dependencies:data.dependencies,diagnostics:data.diagnostics}));
      else {
        try { this.accept?.(data); p.resolve(data); } catch (error) { p.reject(error); }
      }
    };
    c.worker.onerror = e => this.remove(c,Error(e.message || 'Compilation worker failed'));
    const started = performance.now();
    c.ready = Promise.resolve().then(()=>this.initialize()).then(data=>this.send(c,{type:'init',...data})).then(()=>{c.initMs=performance.now()-started;});
    c.ready.catch(error=>this.remove(c,error));
    if (this.latestVersion) c.worker.postMessage({type:'cancel',version:this.latestVersion});
    return c;
  }
  async call(data,onPlan,onProgress) {
    if (this.closed) throw Error('Compiler pool closed');
    const compile = data.type === 'compile', key = data.targetPageId;
    const c = (compile && this.owners.get(key)) || [...this.clients].find(c=>!c.busy) || (!compile && this.clients.values().next().value) || this.client();
    if (compile) { this.owners.set(key,c); c.busy = true; }
    const queuedAt = performance.now();
    try {
      await c.ready;
      const initializedAt = performance.now();
      if (compile) {
        const fileChanges={}, assetChanges={};
        for (const path of new Set([...Object.keys(c.inputs.files),...Object.keys(data.files)]))
          if (c.inputs.files[path] !== data.files[path]) fileChanges[path]=data.files[path]??null;
        for (const path of new Set([...Object.keys(c.inputs.assetKeys),...Object.keys(data.assetKeys)]))
          if (c.inputs.assetKeys[path] !== data.assetKeys[path]) assetChanges[path]=data.assets[path]??null;
        c.inputs={files:data.files,assetKeys:data.assetKeys};
        const {files,assets,assetKeys,...request}=data;
        data={...request,fileChanges,assetChanges,knownPages:this.knownPages()};
      }
      const result = await this.send(c,data,onPlan,onProgress);
      result.diagnostics={...result.diagnostics,worker:c.id,queuedAt,initializedAt,receivedAt:performance.now(),initMs:c.initMs};
      return result;
    } finally { if (compile) c.busy=false; }
  }
  cancel(version) {
    this.latestVersion=version;
    for (const c of this.clients) c.worker.postMessage({type:'cancel',version});
  }
  retain(keys) {
    const live = new Set(keys);
    for (const key of this.owners.keys()) if (!live.has(key)) this.owners.delete(key);
    for (const c of this.clients) {
      const pages=[...this.owners].filter(([,owner])=>owner===c).map(([key])=>key);
      if (!pages.length && !c.busy && !c.pending.size) this.remove(c,Error('Page owner released'));
      else c.worker.postMessage({type:'retain',pages});
    }
  }
  remove(c,error) {
    c.worker.terminate(); this.clients.delete(c);
    for (const p of c.pending.values()) p.reject(error);
    c.pending.clear();
    for (const [key,owner] of this.owners) if (owner===c) this.owners.delete(key);
  }
  dispose() {
    this.closed=true;
    for (const c of this.clients) this.remove(c,Error('Compiler pool closed'));
  }
}
