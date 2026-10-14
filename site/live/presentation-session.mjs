// Page readiness is independent of document readiness. A failed future page
// never invalidates an already visible page or advances the presentation clock.
const deferred=()=>{let resolve,reject;const promise=new Promise((a,b)=>{resolve=a;reject=b;});promise.catch(()=>{});return {promise,resolve,reject};};
export class PresentationSession {
 constructor(store,{onProgress=()=>{},request=()=>{},stop=()=>{}}={}){
  Object.assign(this,{store,onProgress,request,stop});this.started=performance.now();this.pages=[];this.planned=deferred();this.planReady=this.planned.promise;this.gates=new Map();this.first=deferred();this.firstReady=this.first.promise;this.done=deferred();this.complete=this.done.promise;
  const dispose=store.dispose.bind(store);store.dispose=async()=>{if(this.closed)return;this.closed=true;this.stop();const error=Error('Presentation preparation cancelled');this.planned.reject(error);this.first.reject(error);this.done.reject(error);for(const gate of this.gates.values())gate.reject(error);await dispose();};
 }
 plan(pages){if(!pages.length){this.fail(null,Error('Presentation contains no pages'));return;}this.pages=pages.map(p=>({...p,status:'pending'}));for(const p of this.pages)this.gates.set(p.id,deferred());this.planned.resolve(this);this.progress();}
 ready(page){if(this.closed)return;const p=this.pages.find(p=>p.id===page.id);if(p.status==='error')this.gates.set(p.id,deferred());Object.assign(p,page,{status:'ready'});delete p.error;this.gates.get(p.id).resolve(p);if(p===this.pages[0]){this.firstPageMs=performance.now()-this.started;this.first.resolve(this);}this.progress();}
 fail(id,error){if(this.closed)return;const p=this.pages.find(p=>p.id===id);if(!p){for(const pending of this.pages.filter(p=>p.status==='pending'))this.fail(pending.id,error);this.planned.reject(error);this.first.reject(error);this.done.reject(error);return;}p.status='error';p.error=error.message;this.gates.get(id).reject(error);if(p===this.pages[0])this.first.reject(error);this.progress();}
 progress(){const completed=this.pages.filter(p=>p.status==='ready').length,failed=this.pages.filter(p=>p.status==='error').length;this.onProgress({completed,total:this.pages.length,failed,page:this.pages.find(p=>p.status==='error')?.name});if(completed+failed===this.pages.length&&this.pages.length){this.prepareMs=performance.now()-this.started;this.done.resolve(this);}}
 wait(index,{retry=false}={}){const p=this.pages[index];if(!p||this.closed)return Promise.reject(Error('Presentation page unavailable'));if(retry&&p.status==='error'){p.status='pending';delete p.error;this.gates.set(p.id,deferred());}if(p.status==='pending')this.request(p.id,retry);return this.gates.get(p.id).promise;}
}
