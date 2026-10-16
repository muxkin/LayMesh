// Runs inside either a real browser or the real VS Code presentation webview.
// Synthetic key events use the ordinary presentation key handler. Timings end
// after go() confirms the renderer and the following browser rendering step.
module.exports=async function navigationBenchmark(v,{samples=50,visual=false}={}){
 const pause=ms=>new Promise(r=>setTimeout(r,ms));
 const until=async(fn)=>{const end=performance.now()+120000;while(!fn()){if(v.root?.dataset.error)throw Error(v.root.dataset.error);if(performance.now()>end)throw Error('Navigation timeout');await pause(5);}};
 const visible=()=>new Promise(r=>requestAnimationFrame(()=>setTimeout(r,0)));
 const calls=[],call=v.call.bind(v);
 v.call=async data=>{const start=performance.now();const result=await call(data);calls.push({type:data.type,page:data.pageId||data.page?.id,ms:performance.now()-start,result:result.type,preparation:result.preparation,transitionMs:result.transitionMs,backend:result.backend});return result;};
 if(!v.opened)await v.open({fullscreen:false,startPageId:null});
 await until(()=>v.ready&&!v.switching);await v.deck.complete;
 const coldResources=v.metrics.resources?.slice(),rows=[];let direction=1;
 for(let i=0;i<=samples;i++){
  if(v.index===v.deck.pages.length-1)direction=-1;else if(v.index===0)direction=1;
  const from=v.index,target=from+direction,key=direction>0?(i%2?'ArrowRight':'ArrowDown'):(i%2?'ArrowLeft':'ArrowUp');
  v.clock.seek(direction>0?v.clock.duration:0);await v.paint();v.sync();v.root.focus();
  const offset=calls.length,start=performance.now();
  const event=new KeyboardEvent('keydown',{key,bubbles:true,cancelable:true});v.root.dispatchEvent(event);
  if(!event.defaultPrevented)throw Error('Navigation key was not handled');
  // Extra keys during switching must remain ignored, not queued for later.
  for(let k=0;k<4;k++)v.root.dispatchEvent(new KeyboardEvent('keydown',{key,bubbles:true,cancelable:true}));
  await until(()=>v.index===target&&!v.switching);v.clock.running=false;await visible();
  if(v.index!==target)throw Error('Input protection skipped an extra page');
  const row={iteration:i,from:v.deck.pages[from].id,to:v.deck.pages[target].id,key,elapsedMs:performance.now()-start,calls:calls.slice(offset),transition:v.metrics.transitions.at(-1)};
  row.prepareMs=row.calls.reduce((n,c)=>n+(c.preparation?.prepareMs||0),0);
  row.planMs=row.calls.reduce((n,c)=>n+(c.preparation?.planMs||0),0);
  row.transitionMs=row.calls.reduce((n,c)=>n+(c.transitionMs||0),0);
  row.withoutTransitionMs=row.elapsedMs-row.transitionMs;
  if(i)rows.push(row);
  if(i&&i%10===0)console.info('LayMesh navigation samples: '+i+'/'+samples);
 }
 const animation=[];
 if(visual)for(let index=0;index<v.deck.pages.length;index++){
  await v.go(index,{time:0,preserve:true});v.clock.running=false;await v.preload;
  for(const time of [0,1,3]){
   await v.call({type:'paint',serial:v.serial,pageId:v.deck.pages[index].id,time,reduced:false,running:false,...v.dimensions()});
   const {rgba}=await v.call({type:'capture'});
   const digest=await crypto.subtle.digest('SHA-256',rgba);
   animation.push({page:v.deck.pages[index].id,time,sha256:Array.from(new Uint8Array(digest),n=>n.toString(16).padStart(2,'0')).join('')});
  }
 }
 const q=(a,p)=>{a=a.slice().sort((x,y)=>x-y);return p===.5?(a[Math.floor((a.length-1)/2)]+a[Math.floor(a.length/2)])/2:a[Math.ceil(a.length*p)-1];};
 const summary={samples:rows.length};
 for(const key of ['elapsedMs','withoutTransitionMs','prepareMs','planMs','transitionMs'])summary[key]={median:q(rows.map(r=>r[key]),.5),p95:q(rows.map(r=>r[key]),.95)};
 return {summary,rows,coldResources,animation,backend:v.metrics.backend,dimensions:v.dimensions(),definition:'Keydown through ordinary handler to renderer confirmation and next browser rendering step; includes authored page transition, five-event directional key burst; keys after navigation starts are protected; one initial navigation excluded. No source edits or compilation during navigation.'};
};
