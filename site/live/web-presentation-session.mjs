import {PresentationSession} from './presentation-session.mjs';
// A deck refers to one source version. Ready pages carry the exact drawing
// baseline committed by that version; future-page errors stay local.
export async function createWebPresentationSession({compiler,startPageId,signal,onProgress,load}) {
  const version=compiler.version;
  compiler.prioritize(startPageId || compiler.records[0]?.key);
  await compiler.ensurePlan({version,signal});
  const values=new Map();
  let unsubscribe=()=>{};
  const check=()=>compiler.checkWait(version,signal);
  const store={
    async page(id,options) { check(); const value=values.get(id); if(!value)throw Error('页面尚未就绪: '+id); const page=await load(value,id,options); check(); return page; },
    dispose() { values.clear(); },
  };
  const session=new PresentationSession(store,{
    onProgress,
    request(id,retry) { if(retry)compiler.retry(id);else compiler.prioritize(id); },
    stop() { unsubscribe(); signal?.removeEventListener('abort',abort); },
  });
  const abort=()=>session.store.dispose();
  const wait=session.wait.bind(session);
  session.wait=(index,options)=>{try{check();return wait(index,options);}catch(error){return Promise.reject(error);}};
  session.sourceVersion=version;
  session.plan(compiler.records.map(r=>({id:r.key,name:r.name || r.value?.name || r.key,sourceVersion:version})));
  const refresh=()=>{
    try { check(); } catch(error) { session.fail(null,error); unsubscribe(); return; }
    for(const r of compiler.records) {
      const p=session.pages.find(p=>p.id===r.key);
      if(!p)continue;
      if(r.valid && (p.status!=='ready' || values.get(r.key)!==r.value)) {
        values.set(r.key,r.value);
        session.ready({...r.value,id:r.key,sourceId:r.key,sourceVersion:version,baseline:r.value.listId || r.value.id});
      } else if(r.error && p.status!=='error')session.fail(r.key,r.error);
    }
  };
  unsubscribe=compiler.subscribe(refresh);
  signal?.addEventListener('abort',abort,{once:true});
  refresh();
  return session;
}
