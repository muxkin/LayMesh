import {PresentationStore} from './presentation-store.mjs';
import {PresentationSession} from './presentation-session.mjs';
// First-page compilation has priority over pool startup. Subsequent workers
// compile independent resolved sources; one artifact per worker is in flight.
export function webPresentationProvider(editor,base,{concurrency}={}){return async({signal,onProgress})=>{
 const store=new PresentationStore(),workers=[],files={...editor.files},entry=editor.entry;
 const limit=Math.max(1,Math.min(4,Math.floor(concurrency??Math.max(1,(navigator.hardwareConcurrency||2)-2)),navigator.deviceMemory&&navigator.deviceMemory<=4?2:4));
 let queue=[],planned=false,storage=Promise.resolve(),expanded=false;
 const timings={workers:1,pages:[],storageMs:0};
 const session=new PresentationSession(store,{onProgress,stop:()=>{signal.removeEventListener('abort',abort);workers.forEach(w=>w.terminate());},request:(id,retry)=>{const item=queue.find(q=>q.page.id===id||q.aliases.some(p=>p.id===id));if(!item)return;if(retry&&item.status==='error')item.status='pending';item.priority=performance.now()+1;if(workers.every(w=>w.closed))spawn();else for(const w of workers)dispatch(w);}});session.preparation=timings;
 const abort=()=>store.dispose();signal.addEventListener('abort',abort,{once:true});
 const fatal=e=>{for(const q of queue)if(q.status!=='ready')q.status='error';session.fail(null,e);for(const w of workers){w.closed=true;w.terminate();}if(session.pages[0]?.status!=='ready')store.dispose();};
 const dispatch=w=>{if(session.closed||!w.initialized||w.closed||w.item)return;const item=queue.filter(q=>q.status==='pending').sort((a,b)=>(b.priority||0)-(a.priority||0)||a.index-b.index)[0];if(!item)return;item.status='working';w.item=item;w.postMessage({type:'compile-deck-page',page:item.page,index:item.index,total:session.pages.length,resourceIds:[...store.resources]});};
 const spawn=(planner=false)=>{
  const w=new Worker(new URL('worker.js',base),{type:'module'});workers.push(w);timings.workers=Math.max(timings.workers,workers.filter(w=>!w.closed).length);
  w.onerror=e=>fatal(Error(e.message));w.onmessage=async({data})=>{try{
   if(data.type==='ready'){
    for(const font of editor.loadedFonts)w.postMessage({type:'font',...font});
    w.postMessage({type:'deck-context',files,entry,fullFonts:editor.module.dataset.modified==='true'});w.initialized=true;
    if(planner)w.postMessage({type:'prepare-deck',files,entry,planOnly:true});else dispatch(w);
   }else if(data.type==='deck-plan'&&!planned){
    planned=true;timings.planMs=performance.now()-session.started;session.plan(data.pages);const seen=new Map();
    data.pages.forEach((page,index)=>{if(page.src&&seen.has(page.src))seen.get(page.src).aliases.push(page);else{const item={page,index,aliases:[],status:'pending'};queue.push(item);if(page.src)seen.set(page.src,item);}});dispatch(w);
   }else if(data.type==='deck-page'){
    const item=w.item;await(storage=storage.catch(()=>{}).then(async()=>{if(session.closed)return;const before=performance.now();await store.add(data.page);session.ready(store.pages.find(p=>p.id===data.page.id));for(const alias of item.aliases){await store.alias(alias.id,alias.name,item.page.id);session.ready(store.pages.find(p=>p.id===alias.id));}timings.storageMs+=performance.now()-before;}));
    if(session.closed)return;timings.pages.push(data.timings);item.status='ready';w.item=null;
    if(!expanded){expanded=true;for(let i=1;i<Math.min(limit,queue.length);i++)spawn();}dispatch(w);
    if(queue.every(q=>['ready','error'].includes(q.status))){timings.totalMs=performance.now()-session.started;for(const worker of workers){worker.closed=true;worker.terminate();}}
   }else if(data.type==='deck-error'){
    const item=w.item;if(!item)return fatal(Error(data.error.message));const error=Error(data.error.message);item.status='error';w.item=null;session.fail(item.page.id,error);for(const p of item.aliases)session.fail(p.id,error);dispatch(w);if(queue.every(q=>['ready','error'].includes(q.status)))for(const worker of workers){worker.closed=true;worker.terminate();}
   }else if(data.type==='init-error')fatal(Error(data.error.message));
  }catch(e){fatal(e);}};w.postMessage({type:'init',base:base.href,compilerOnly:true});
 };
 if(signal.aborted)abort();else spawn(true);
 try{return await session.firstReady;}catch(error){await store.dispose();throw error;}
};}
