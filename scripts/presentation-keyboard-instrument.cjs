// Shared instrumentation for trusted browser and VS Code keyboard tests.
function instrument(v){
 const data=window.keyboardMeasure={inputs:[],rows:[],calls:[],started:performance.now(),openedMs:null};
 let lastKey,round=0;
 if(!v.navigationVisible)v.navigationVisible=data=>{const serial=data.serial;requestAnimationFrame(()=>setTimeout(()=>{if(serial===v.serial){v.metrics.firstVisibleMs=performance.now()-v.navigationStarted;v.metrics.visibleSerial=serial;}},0));};
 document.addEventListener('keydown',event=>{if(v.opened){lastKey={key:event.key,at:performance.now(),trusted:event.isTrusted,switching:v.switching,index:v.index,completed:v.clock.completed,active:v.clock.active};data.inputs.push(lastKey);if(event.key==='Home')round++;}},true);
 const call=v.call.bind(v);v.call=async(request,...args)=>{const at=performance.now(),r=await call(request,...args);data.calls.push({type:request.type,stage:request.stage,page:request.pageId||request.page?.id,at,ms:performance.now()-at,result:r.type,queueWaitMs:r.queueWaitMs,preparation:r.preparation,transitionMs:r.transitionMs,frameCacheHit:r.frameCacheHit});return r;};
 const go=v.go.bind(v);v.go=async(index,options)=>{
  const from=v.index,key=lastKey,at=performance.now(),offset=data.calls.length,initial=!v.ready;
  const ready=v.entryFrames?.has(v.frameKey?.(v.deck.pages[index]?.id,options?.end?v.deck.pages[index]?.inspection.animation?.duration||0:options?.time||0));
  v.navigationStarted=at;
  const result=await go(index,options);
  if(initial)data.openedMs=performance.now()-data.started;
  if(result&&!initial&&index!==from&&key&&['ArrowDown','ArrowRight','ArrowUp','ArrowLeft'].includes(key.key)){
   const transition=v.metrics.transitions.at(-1),calls=data.calls.slice(offset),done=performance.now();
   data.rows.push({round,from,to:index,key:key.key,trusted:key.trusted,at:key.at,ready:!!ready,firstVisibleMs:(transition?.firstVisibleMs??(v.metrics.visibleSerial===v.serial?v.metrics.firstVisibleMs:null))==null?null:(transition?.firstVisibleMs??v.metrics.firstVisibleMs)+at-key.at,completedMs:done-key.at,transitionMs:v.metrics.lastFrame?.transitionMs||0,queueWaitMs:calls.reduce((n,c)=>n+(c.queueWaitMs||0),0),frameCacheHit:!!v.metrics.lastFrame?.frameCacheHit,calls});
  }
  return result;
 };
 return data;
}
module.exports=instrument;
