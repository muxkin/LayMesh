const fs=require('fs'),path=require('path'),os=require('os'),http=require('http'),crypto=require('crypto'),zlib=require('zlib');
// Compiler processes and completed artifacts outlive a preparation revision.
// Revisions own cancellation/delivery; the cache owns immutable page snapshots.
class NativePresentationCache {
 constructor(createWorker){this.createWorker=createWorker;this.workers=[];this.pages=new Map();this.resources=new Set();}
 worker(index=0){return this.workers[index]??(this.workers[index]=this.createWorker());}
 signature(file,overlays){if(Object.hasOwn(overlays,file))return 'text:'+crypto.createHash('sha256').update(overlays[file]).digest('hex');try{const st=fs.statSync(file);return `disk:${st.ctimeMs}:${st.mtimeMs}:${st.size}`;}catch{return 'missing';}}
 valid(item,overlays){return item&&item.reads.every(([file,value])=>this.signature(file,overlays)===value)&&fs.existsSync(item.file);}
 prune(keys){for(const [key,item]of this.pages)if(!keys.has(key)){this.pages.delete(key);try{fs.unlinkSync(item.file);}catch{}}const used=new Set([...this.pages.values()].flatMap(item=>item.resources.map(([,file])=>file)));for(const file of this.resources)if(!used.has(file)){try{fs.unlinkSync(file);}catch{}this.resources.delete(file);}}
 releaseWorkers(){const live=new Set([this.workers[0],...[...this.pages.values()].map(p=>p.worker)]);this.workers=this.workers.filter(w=>{if(live.has(w))return true;w.dispose();return false;});}
 dispose(){for(const w of this.workers)w.dispose();this.workers=[];this.pages.clear();this.resources.clear();}
}
class NativePresentation {
 constructor({worker,createWorker,canPrepare=()=>true,cache,knownPages=[],dist,uri,notify,onBrowserClose=()=>{}}){
  Object.assign(this,{worker,createWorker,canPrepare,dist,uri,onBrowserClose});this.cache=cache;this.knownPages=new Set(knownPages);this.workers=cache?cache.workers:[worker];this.limit=createWorker?Math.max(1,os.availableParallelism?.()||os.cpus().length):1;
  this.revision=crypto.randomBytes(12).toString('hex');this.notify=data=>notify({...data,revision:this.revision});this.pages=[];this.resources=new Map();this.resourceMimes=new Map();this.dependencies=new Set();this.pending=new Map();this.acks=new Map();this.cancelled=false;this.directory=path.join(worker.resourceDir,'presentation');fs.mkdirSync(this.directory,{recursive:true});this.delivery=Promise.resolve();
 }
 ack(id){const key=id??this.acks.keys().next().value;this.acks.get(key)?.();this.acks.delete(key);}
 priority(id,retry=false){const item=this.jobs?.find(j=>j.specs.some(p=>p.id===id));if(!item)return;if(retry&&item.status==='error'){item.status='pending';for(const p of item.specs){p.status='pending';delete p.error;}}item.priority=performance.now()+1;this.pump();}
 async idle(){while(!this.canPrepare()){if(this.cancelled)throw Error('Presentation preparation cancelled');await new Promise(r=>setTimeout(r,25));}if(this.cancelled)throw Error('Presentation preparation cancelled');}
 interruptForPreview(){for(const worker of this.workers)if(worker.active||worker.queue?.size){this.interrupted=(this.interrupted||0)+1;(worker.interrupt||worker.fail).call(worker,{code:'E_PRESENTATION_YIELD',message:'Yielding background preparation to editing preview'});}}
 async compile(worker,file,source,request){for(;;){await this.idle();if(this.cancelled)throw Error('Presentation preparation cancelled');try{return await new Promise((resolve,reject)=>{const key=Symbol();this.pending.set(key,reject);worker.enqueue({file,source,overlays:this.overlays,request:{protocol:3,presentation_revision:this.revision,resource_dir:worker.resourceDir,...this.defaults,...(this.config?{config:this.config}:{}),preview:this.settings,parallelism:Math.max(1,Math.floor(this.limit/Math.min(this.limit,Math.max(1,this.jobs?.filter(j=>j.status==='pending'||j.status==='working').length||1)))),...request},timeout:120000,done:r=>{this.pending.delete(key);if(this.cancelled)reject(Error('Presentation preparation cancelled'));else if(r.error)reject(Object.assign(Error(r.error.message),{code:r.error.code}));else resolve(r);}});});}catch(error){if(error.code!=='E_PRESENTATION_YIELD')throw error;}}}

 record(file){if(file in this.overlays)return;let stat;try{stat=fs.statSync(file);}catch{}const value=stat?`${stat.ctimeMs}:${stat.mtimeMs}:${stat.size}`:'missing';if(stat?.ctimeMs>this.startedAt||this.signatures.has(file)&&this.signatures.get(file)!==value)throw Error(`Source changed while preparing: ${file}. Retry to use the latest document.`);this.signatures.set(file,value);}
 async deliver(item,specs,reused){
  for(const [id,file,mime] of item.resources){this.resources.set(id,file);this.resourceMimes.set(id,mime);}
  for(const [file] of item.reads){this.dependencies.add(file);this.record(file);}
  for(const selected of specs){await(this.delivery=this.delivery.catch(()=>{}).then(async()=>{
   if(this.cancelled)throw Error('Presentation preparation cancelled');await this.idle();
   Object.assign(selected,{file:item.file,artifactId:item.id,status:'ready'});delete selected.error;
   const consumed=new Promise(resolve=>this.acks.set(selected.id,resolve));
   if(this.knownPages.has(item.id))this.notify({type:'deck-page-reuse',artifactId:item.id,page:{id:selected.id,name:selected.name||selected.id}});
   else {const page=JSON.parse(await fs.promises.readFile(item.file,'utf8'));Object.assign(page,{id:selected.id,name:selected.name||selected.id});this.notify({type:'deck-page',artifactId:item.id,page:{...page,resourceUrls:Object.fromEntries(Object.keys(page.list.resources).map(id=>{const file=this.resources.get(id);if(!file)throw Error('Missing presentation resource: '+id);return [id,this.uri(file)];}))}});}
   await consumed;this.knownPages.add(item.id);if(selected.index===0){this.ready=true;this.firstPageMs=performance.now()-this.started;}
   this.notify({type:'deck-progress',completed:this.pages.filter(p=>p.status==='ready').length,total:this.pages.length});
  }));}
 }
 async reuse(item,specs){this.reusedPages+=specs.length;await this.deliver(item,specs,true);}
 async accept(frame,worker,specs){
  if(this.cancelled)throw Error('Presentation preparation cancelled');
  const reads=(frame.pageDependencies||frame.dependencies||[]).filter(d=>d!==this.config);
  for(const d of frame.dependencies||[]){if(d===this.config)continue;this.dependencies.add(d);this.record(d);}
  const resources=[];for(const r of frame.resources||[]){if(!/^[-a-z0-9]+$/.test(r.id)||path.dirname(path.resolve(r.path))!==worker.resourceDir)throw Error('Invalid presentation resource');resources.push([r.id,r.path,r.mime]);}
  const list=frame.displayList||JSON.parse(zlib.gunzipSync(Buffer.from(frame.displayListCompressed.data,'base64')).toString('utf8'));
  const page={id:specs[0].id,name:specs[0].name||specs[0].id,inspection:frame.inspection,list};
  const signatures=reads.map(file=>[file,this.cache?this.cache.signature(file,this.overlays):'']);
  const id=crypto.randomBytes(16).toString('hex'),filename=path.join(this.directory,id+'.json');await fs.promises.writeFile(filename,JSON.stringify(page));
  if(this.cancelled){await fs.promises.unlink(filename).catch(()=>{});throw Error('Presentation preparation cancelled');}
  for(const file of reads)this.record(file);
  const item={id,file:filename,worker,resources,reads:signatures};
  if(this.cache){for(const [,file]of resources)this.cache.resources.add(file);const old=this.cache.pages.get(specs[0].cacheKey);this.cache.pages.set(specs[0].cacheKey,item);if(old&&old.file!==filename)try{fs.unlinkSync(old.file);}catch{}}
  this.compiledPages+=specs.length;await this.deliver(item,specs,false);
 }
 async prepare(file,source,overlays,defaults,startPageId){
  this.entry=file;this.entrySource=source;
  this.started=performance.now();this.startedAt=Date.now();this.overlays={...overlays,[file]:source};this.defaults=defaults;this.signatures=new Map();this.reusedPages=0;this.compiledPages=0;
  let first;try{first=await this.compile(this.worker,file,source,{type:'plan',...(startPageId?{page_id:startPageId}:{})});}catch(error){if(startPageId&&/Selected page ID no longer exists/.test(error.message))first=await this.compile(this.worker,file,source,{});else throw error;}if(this.cancelled)throw Error('Presentation preparation cancelled');
  this.settings=first.previewSettings||{};this.config=path.join(this.directory,'defaults.json');fs.writeFileSync(this.config,JSON.stringify({preview:this.settings}));
  this.pages=(first.document?.pages||[{id:'page',name:path.basename(file),src:file}]).map((p,index)=>({...p,index,status:'pending'}));
  for(const p of this.pages)if(!(p.src in this.overlays)){this.record(p.src);this.overlays[p.src]=fs.readFileSync(p.src,'utf8');}
  this.notify({type:'deck-plan',pages:this.pages.map(({id,name,src})=>({id,name,src}))});
  const seen=new Map();this.jobs=[];for(const p of this.pages){const key=JSON.stringify([p.src,p.mediaKey,p.transition&&{args:p.transition.args,file:p.transition.file},first.configFingerprint||defaults,this.settings]);p.cacheKey=key;if(seen.has(key))seen.get(key).specs.push(p);else{const job={specs:[p],status:'pending'};seen.set(key,job);this.jobs.push(job);}}
  for(const d of first.dependencies||[]){this.dependencies.add(d);this.record(d);}
  for(const job of this.jobs){const item=this.cache?.pages.get(job.specs[0].cacheKey);if(this.cache?.valid(item,this.overlays)){job.status='ready';await this.reuse(item,job.specs);}else if(item)job.worker=item.worker;}
  // Legacy engines return a complete first page for the plan request.
  if(first.displayList||first.displayListCompressed){const job=this.jobs.find(j=>j.specs.some(p=>p.id===first.document?.pageId))||this.jobs[0];if(job.status!=='ready'){await this.accept(first,this.worker,job.specs);job.status='ready';}}
  if(startPageId){const job=this.jobs.find(j=>j.specs.some(p=>p.id===startPageId));if(job)job.priority=performance.now()+1;}
  for(let i=this.workers.length;i<Math.min(this.limit,this.jobs.length);i++)if(this.cache)this.cache.worker(i);else this.workers.push(this.createWorker());
  this.cache?.prune(new Set(this.pages.map(p=>p.cacheKey)));
  this.preparation={workers:this.workers.length,firstPageMs:this.firstPageMs};
  await new Promise((resolve,reject)=>{this.finish=resolve;this.rejectFinished=reject;this.pump();});return this;
 }
 pump(){
  if(this.cancelled)return;clearTimeout(this.timer);if(!this.canPrepare()){this.timer=setTimeout(()=>this.pump(),25);return;}
  for(const worker of this.workers){if(worker.deckBusy)continue;const job=this.jobs.filter(j=>j.status==='pending'&&(!j.worker||j.worker===worker)).sort((a,b)=>(b.priority||0)-(a.priority||0)||a.specs[0].index-b.specs[0].index)[0];if(!job)continue; // Another retained worker may own the only affected page.
   worker.deckBusy=this.revision;job.status='working';const p=job.specs[0];this.compile(worker,this.entry.endsWith(".laydoc")?this.entry:p.src,this.entry.endsWith(".laydoc")?this.entrySource:this.overlays[p.src],{page:p.index+1}).then(frame=>this.accept(frame,worker,job.specs)).then(()=>{job.status='ready';},error=>{job.status='error';if(!this.cancelled)for(const p of job.specs){p.status='error';p.error=`${p.index+1}/${this.pages.length}. ${p.name||p.id}: ${error.message}`;this.notify({type:'deck-error',pageId:p.id,error:p.error});}}).finally(()=>{if(worker.deckBusy===this.revision)worker.deckBusy=false;this.pump();});
  }
  if(this.jobs.every(j=>j.status==='ready'||j.status==='error')){this.cache?.prune(new Set(this.pages.map(p=>p.cacheKey)));if(this.cache){this.cache.releaseWorkers();this.workers=this.cache.workers;}this.prepareMs=performance.now()-this.started;this.preparation.interrupted=this.interrupted||0;this.preparation.reusedPages=this.reusedPages;this.preparation.compiledPages=this.compiledPages;this.notify({type:'deck-ready',prepareMs:this.prepareMs,preparation:this.preparation});this.finish?.();this.finish=null;if(!this.cache)for(const worker of this.workers)worker.stop?.();}
 }
 async browser(){if(!this.pages.length)throw Error('Presentation is not prepared');if(this.url)return this.url;
  const addresses=Object.values(os.networkInterfaces()).flat().filter(Boolean).map(n=>n.address);const host=addresses.find(a=>/^100\.(\d+)\./.test(a)&&Number(a.split('.')[1])>=64&&Number(a.split('.')[1])<=127);if(!host)throw Error('Tailscale IPv4 address is unavailable');
  const token=crypto.randomBytes(24).toString('hex'),prefix='/'+token+'/';
  this.server=http.createServer((req,res)=>{try{const url=new URL(req.url,'http://localhost');if(!url.pathname.startsWith(prefix)){res.writeHead(404).end();return;}const route=url.pathname.slice(prefix.length);res.setHeader('Cache-Control','no-store');
   if(route==='exit'&&req.method==='POST'){res.end('OK');this.server?.close();this.server=null;this.url=null;this.onBrowserClose();return;}
   if(route==='manifest.json'){res.setHeader('Content-Type','application/json');res.end(JSON.stringify({pages:this.pages.map((p,i)=>({id:p.id,name:p.name,url:'pages/'+i+'.json'})),prepareMs:this.prepareMs,firstPageMs:this.firstPageMs,preparation:this.preparation}));return;}
   if(route===''){res.setHeader('Content-Type','text/html; charset=utf-8');res.end('<!doctype html><html lang="zh-CN"><meta charset="utf-8"><meta name="viewport" content="width=device-width"><title>LayMesh 放映</title><body style="font:16px system-ui;background:#15191e;color:#eee;padding:24px"><h1>LayMesh 放映</h1><p id="status">正在载入已准备的演示…</p><script type="module" src="assets/presentation-browser.mjs"></script></body></html>');return;}
   let file;if(/^pages\/\d+\.json$/.test(route)){const p=this.pages[Number(route.slice(6,-5))];if(!p){res.writeHead(404).end();return;}if(p.status!=='ready'){this.priority(p.id,url.searchParams.get('retry')==='1');res.setHeader('Content-Type','application/json');res.writeHead(p.status==='error'?422:202).end(JSON.stringify({error:p.error}));return;}const page=JSON.parse(fs.readFileSync(p.file));page.id=p.id;page.name=p.name;page.resourceUrls=Object.fromEntries(Object.keys(page.list.resources).map(id=>[id,prefix+'resources/'+id]));res.setHeader('Content-Type','application/json');res.end(JSON.stringify(page));return;}
   if(route.startsWith('resources/'))file=this.resources.get(route.slice(10));
   else if(route.startsWith('assets/')){const relative=route.slice(7);if(!/^[a-zA-Z0-9_./-]+$/.test(relative)||relative.split('/').includes('..')){res.writeHead(404).end();return;}file=path.join(this.dist,relative);if(!/\.(mjs|js|css|wasm|json)$/.test(file))file=null;}
   if(!file||!fs.existsSync(file)){res.writeHead(404).end();return;}const ext=path.extname(file),mime={'.mjs':'text/javascript','.js':'text/javascript','.css':'text/css','.wasm':'application/wasm','.json':'application/json'};const resourceMime=route.startsWith('resources/')?this.resourceMimes.get(route.slice(10)):null;
   res.setHeader('Content-Type',resourceMime||mime[ext]||'application/octet-stream');
   if(resourceMime&&/^(video|audio)\//.test(resourceMime)){
    const size=fs.statSync(file).size;res.setHeader('Accept-Ranges','bytes');let start=0,end=size-1;
    if(req.headers.range){const m=/^bytes=(\d*)-(\d*)$/.exec(req.headers.range);if(!m||(!m[1]&&!m[2])){res.writeHead(416,{'Content-Range':`bytes */${size}`}).end();return;}
     if(!m[1]){start=Math.max(0,size-Number(m[2]));}else{start=Number(m[1]);if(m[2])end=Math.min(end,Number(m[2]));}
     if(!Number.isSafeInteger(start)||!Number.isSafeInteger(end)||start<0||start>=size||end<start){res.writeHead(416,{'Content-Range':`bytes */${size}`}).end();return;}
     res.statusCode=206;res.setHeader('Content-Range',`bytes ${start}-${end}/${size}`);
    }res.setHeader('Content-Length',Math.max(0,end-start+1));if(req.method==='HEAD'||size===0){res.end();return;}fs.createReadStream(file,{start,end}).on('error',()=>res.destroy()).pipe(res);return;
   }if(/\bgzip\b/.test(req.headers['accept-encoding']||'')&&fs.existsSync(file+'.gz')){file+='.gz';res.setHeader('Content-Encoding','gzip');res.setHeader('Vary','Accept-Encoding');}fs.createReadStream(file).on('error',()=>res.destroy()).pipe(res);
  }catch{res.writeHead(500).end('Presentation resource unavailable');}});
  await new Promise((resolve,reject)=>{this.server.once('error',reject);this.server.listen(0,host,resolve);});this.url=`http://${host}:${this.server.address().port}${prefix}`;return this.url;
 }
 dispose(){this.cancelled=true;clearTimeout(this.timer);const error=Error('Presentation preparation cancelled');for(const reject of this.pending.values())reject(error);this.pending.clear();this.rejectFinished?.(error);for(const resolve of this.acks.values())resolve();this.acks.clear();this.server?.close();this.server?.closeAllConnections();this.server=null;if(!this.cache)for(const worker of this.workers)worker.dispose();else for(const worker of this.workers)if(worker.deckBusy===this.revision){worker.interrupt?.({code:'E_PRESENTATION_CANCELLED',message:'Presentation preparation cancelled'});worker.deckBusy=false;}this.pages=[];this.resources.clear();}
}
module.exports={NativePresentation,NativePresentationCache};
