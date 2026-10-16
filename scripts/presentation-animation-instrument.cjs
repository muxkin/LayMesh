// Timing hooks only. Ordinary playback clocks, durations and keys stay intact.
module.exports=function instrumentAnimation(v){
 const m=window.animationMeasure={calls:[],visits:[]},call=v.call.bind(v),go=v.go.bind(v);
 v.call=async(data,...args)=>{const start=performance.now(),running=v.clock.running,result=await call(data,...args);m.calls.push({start,end:performance.now(),type:data.type,stage:data.stage,page:data.pageId||data.page?.id,time:data.time,running,serial:data.serial,result});return result;};
 v.go=async(...args)=>{const start=performance.now(),result=await go(...args);if(result)m.visits.push({page:v.deck.pages[v.index].id,index:v.index,serial:v.serial,start,end:performance.now(),steps:v.clock.steps,duration:v.clock.duration,prepared:v.readyResident?.has(v.deck.pages[v.index].id)});return result;};
};
