// Two independent roles, not a page/cache quota. Each page keeps its renderer;
// showing a prepared page changes the foreground role instead of reinstalling it.
export class PresentationWorkers {
 constructor(view,createWorker){
  this.view=view;this.createWorker=createWorker;this.parallel=true;this.renderers=[];this.owners=new Map();this.sequence=0;this.serial=0;this.closed=false;
 }
 async create(canvas){
  canvas.classList.add('presentation-buffer');
  const worker=this.createWorker(),r={worker,canvas,pending:new Map(),resident:[],readyResident:[],entryFrames:[]};
  this.renderers.push(r);
  worker.onmessage=({data})=>{
   if(this.closed){data.bitmap?.close();return;}
   if(data.type==='context'){
    r.readyResident=[];r.entryFrames=[];
    if(r===this.active)this.onmessage?.({data});return;
   }
   if(data.type==='navigation-visible'){
    if(data.serial===this.serial)this.show(r);
    this.onmessage?.({data});return;
   }
   const pending=r.pending.get(data.id);if(!pending){data.bitmap?.close();return;}
   r.pending.delete(data.id);
   for(const key of ['resident','readyResident','entryFrames'])if(data[key])r[key]=data[key];
   data.error?pending.reject(Error(data.error)):pending.resolve(data);
  };
  worker.onerror=error=>{for(const p of r.pending.values())p.reject(Error(error.message));r.pending.clear();this.onerror?.(error);};
  const offscreen=canvas.transferControlToOffscreen();
  try{await this.request(r,{...this.init,type:'init',canvas:offscreen,preserveDrawingBuffer:true},[offscreen]);}
  catch(error){worker.terminate();this.renderers=this.renderers.filter(item=>item!==r);throw error;}
  worker.postMessage({type:'target',serial:this.serial});
  return r;
 }
 request(r,data,transfer=[]){
  if(this.closed)return Promise.reject(Error('Presentation renderer closed'));
  return new Promise((resolve,reject)=>{const id=++this.sequence;r.pending.set(id,{resolve,reject});
   const started=performance.now();
   if(data.page?.list){const {list,...page}=data.page;data={...data,page:{...page,listJSON:JSON.stringify(list)}};}
   if(data.page?.packedPage){const bytes=data.page.packedPage;transfer=[...transfer,bytes.buffer||bytes];}
   try{r.worker.postMessage({...data,id},transfer);r.dispatchMs=performance.now()-started;}
   catch(error){r.pending.delete(id);reject(error);}});
 }
 async backgroundRenderer(){
  if(this.renderers.length===1&&!this.creating){
   const canvas=this.view.canvas.cloneNode(false);canvas.classList.remove('presentation-canvas');canvas.style.visibility='hidden';this.view.canvas.after(canvas);
   this.creating=this.create(canvas).catch(error=>{canvas.remove();this.parallel=false;this.view.preparationFallback=String(error);}).finally(()=>{this.creating=null;});
  }
  if(this.creating)await this.creating;
  if(this.closed)throw Error('Presentation renderer closed');
  return this.renderers.find(r=>r!==this.active)||this.active;
 }
 show(r){
  if(this.closed)return;
  this.active=r;this.view.canvas=r.canvas;
  for(const item of this.renderers){item.canvas.classList.toggle('presentation-canvas',item===r);item.canvas.style.visibility=item===r?'':'hidden';item.worker.postMessage({type:'background',serial:this.serial,paused:item===r});}
 }
 state(){return Object.fromEntries(['resident','readyResident','entryFrames'].map(key=>[key,[...new Set(this.renderers.flatMap(r=>r[key]))]]));}
 canPrepare(id){return !this.parallel||this.owners.get(id)!==this.active;}
 postMessage(data,transfer=[]){
  if(this.closed)return;
  if(data.type==='target'){
   this.serial=data.serial;for(const r of this.renderers)r.worker.postMessage(data);return;
  }
  if(data.type==='background'){
   // Painting only owns the foreground. It never pauses the other worker.
   for(const r of this.renderers)r.worker.postMessage({...data,paused:this.parallel?r===this.active:data.paused});return;
  }
  this.run(data,transfer).then(result=>{if(!this.closed)this.onmessage?.({data:{...result,id:data.id}});},error=>{if(!this.closed)this.onmessage?.({data:{id:data.id,error:String(error)}});});
 }
 async run(data,transfer){
  if(data.type==='init'){
   this.init={...data};delete this.init.id;
   this.active=await this.create(this.view.canvas);return {type:'done'};
  }
  if(data.width&&this.size!==`${data.width}/${data.height}`){this.size=`${data.width}/${data.height}`;for(const r of this.renderers){r.readyResident=[];r.entryFrames=[];}}
  const pageId=data.pageId||data.page?.id;let r=this.owners.get(pageId)||this.active;
  if(data.type==='load'||data.type==='prepare'){
   if(!this.owners.has(pageId)){
    r=this.owners.get(data.page?.listUpdate?.base)||(!this.owners.size?this.active:await this.backgroundRenderer());
    this.owners.set(pageId,r);
   }
   if(!data.priority&&r===this.active&&(this.parallel||this.view.clock.running||this.view.painting))return {type:'done',deferred:true,...this.state()};
   data={...data,cpuPreparation:!data.priority};
   r.worker.postMessage({type:'background',serial:this.serial,paused:r===this.active&&this.parallel});
  }else if(data.type==='activate'){
   if(r!==this.active&&data.transition&&!data.reduced){
    const from=await this.request(this.active,{type:'snapshot'});
    if(this.closed||data.serial!==this.serial){from.bitmap.close();return {type:'stale'};}
    data={...data,previousFrame:from};transfer=[...transfer,from.bitmap];
   }
  }else if(data.type==='retain'){
   await Promise.all(this.renderers.map(r=>this.request(r,data)));
   const keep=new Set(data.pages);for(const id of this.owners.keys())if(!keep.has(id))this.owners.delete(id);
   return {type:'done',...this.state()};
  }else r=this.active;
  const result=await this.request(r,data,transfer);
  if(data.type==='activate'&&result.type!=='stale'&&data.serial===this.serial)this.show(r);
  return {...result,renderer:this.renderers.indexOf(r),dispatchMs:r.dispatchMs,...this.state()};
 }
 terminate(){
  this.closed=true;
  for(const r of this.renderers){r.worker.terminate();for(const p of r.pending.values())p.reject(Error('Presentation renderer closed'));r.pending.clear();if(r.canvas!==this.view.canvas)r.canvas.remove();}
  this.renderers=[];this.owners.clear();this.init=null;
 }
}
