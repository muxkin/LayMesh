"""Shared real-page motion probe for browser and VS Code profiling."""
MOTION = r'''(async()=>{
 const d=DOCUMENT,w=d.defaultView,site=SITE,root=d.querySelector(site?'.example-module':'.preview'),viewport=d.querySelector(site?'.preview-viewport':'.viewport');
 const pause=ms=>new Promise(r=>w.setTimeout(r,ms)),raf=()=>new Promise(r=>w.requestAnimationFrame(r));
 const until=async check=>{for(let i=0;i<600;i++){if(check())return;await pause(20);}throw Error('Motion final paint timeout');};
 const painted=()=>root.dataset.painted==='true',figure=()=>d.querySelector(site?'.preview-canvas':'.figure').getBoundingClientRect();
 const canvas=d.querySelector(site?'.live-image':'.figure-pixels'),originalPost=w.Worker.prototype.postMessage;
 let submissions=0,acknowledged=0;const observed=new WeakSet();w.Worker.prototype.postMessage=function(data,...args){if(data.type==='paint'){submissions++;if(!observed.has(this)){observed.add(this);this.addEventListener('message',e=>{if(['painted','done'].includes(e.data.type))acknowledged++;});}}return originalPost.call(this,data,...args);};
 const tasks=[];const observer=new w.PerformanceObserver(list=>tasks.push(...list.getEntries().map(e=>e.duration)));observer.observe({entryTypes:['longtask']});
 if(site){d.querySelector('.preview-zoom-in').click();d.querySelector('.preview-zoom-in').click();}
 else{d.querySelector('[data-action=actual]').click();for(let i=0;i<3;i++)d.querySelector('[data-action=in]').click();}
 await until(painted);await pause(250);
 const quantile=(values,p)=>{const sorted=values.slice().sort((a,b)=>a-b);return sorted[Math.min(sorted.length-1,Math.floor(p*sorted.length))];};
 const rows=[];
 async function motion(name,action){
  const intervals=[],handlers=[],latencies=[],positions=[];let previous=await raf(),startSubmissions=submissions,startAcknowledged=acknowledged,startTasks=tasks.length,uncovered=0;
  for(let i=0;i<60;i++){
   const start=w.performance.now();action(i);handlers.push(w.performance.now()-start);
   const at=await raf();intervals.push(at-previous);previous=at;
   const b=figure();positions.push([b.x,b.y,b.width]);latencies.push(w.performance.now()-start);
   const c=canvas.getBoundingClientRect(),v=viewport.getBoundingClientRect();
   const o=d.querySelector(site?'.live-overview':'.figure-overview'),overview=o&&!o.hidden&&o.width>0&&w.CSS.supports('clip-path',o.style.clipPath);
   if(!overview&&(c.left>Math.max(b.left,v.left)+1||c.top>Math.max(b.top,v.top)+1||c.right<Math.min(b.right,v.right)-1||c.bottom<Math.min(b.bottom,v.bottom)-1))uncovered++;
  }
  await until(painted);await pause(120);
  const actual=tasks.slice(startTasks);
  rows.push({scenario:name,steps:60,frame_median_ms:quantile(intervals,.5),frame_p95_ms:quantile(intervals,.95),frame_max_ms:Math.max(...intervals),frames_above_25_ms:intervals.filter(v=>v>25).length,handler_p95_ms:quantile(handlers,.95),input_to_layout_p95_ms:quantile(latencies,.95),paint_submissions:submissions-startSubmissions,paint_acknowledgements:acknowledged-startAcknowledged,long_tasks:actual.length,long_task_total_ms:actual.reduce((a,b)=>a+b,0),uncovered_layout_frames:uncovered,positions,intervals_ms:intervals});
 }
 let lastX=0,lastY=0;
 await motion('pan',i=>{
  const x=Math.sin(i/60*Math.PI*2)*40,y=Math.cos(i/60*Math.PI*2)*20;
  if(site)viewport.dispatchEvent(new w.KeyboardEvent('keydown',{bubbles:true,cancelable:true,key:i%2?'ArrowLeft':'ArrowRight'}));
  else viewport.dispatchEvent(new w.WheelEvent('wheel',{bubbles:true,cancelable:true,deltaX:lastX-x,deltaY:lastY-y}));
  lastX=x;lastY=y;
 });
 await motion('zoom',i=>{const b=viewport.getBoundingClientRect();viewport.dispatchEvent(new w.WheelEvent('wheel',{bubbles:true,cancelable:true,ctrlKey:true,deltaY:i%2?3:-3,clientX:b.x+b.width/2,clientY:b.y+b.height/2}));});
 await motion('cursor',i=>{const b=viewport.getBoundingClientRect();viewport.dispatchEvent(new w.PointerEvent('pointermove',{bubbles:true,clientX:b.x+b.width*(.3+.4*i/60),clientY:b.y+b.height*.5}));});
 await motion('pan-uncovered',i=>{
  const x=Math.sin(i/60*Math.PI*2)*240,y=Math.cos(i/60*Math.PI*2)*120;
  if(site)viewport.dispatchEvent(new w.KeyboardEvent('keydown',{bubbles:true,cancelable:true,key:i<30?'ArrowRight':'ArrowLeft'}));
  else viewport.dispatchEvent(new w.WheelEvent('wheel',{bubbles:true,cancelable:true,deltaX:lastX-x,deltaY:lastY-y}));
  lastX=x;lastY=y;
 });
 observer.disconnect();w.Worker.prototype.postMessage=originalPost;
 const snapshot=d.createElement('canvas');snapshot.width=canvas.width;snapshot.height=canvas.height;const context=snapshot.getContext('2d');context.drawImage(canvas,0,0);const pixels=context.getImageData(0,0,snapshot.width,snapshot.height).data;
 const c=canvas.getBoundingClientRect(),f=figure(),v=viewport.getBoundingClientRect(),epsilon=1;
 const o=d.querySelector(site?'.live-overview':'.figure-overview');
 let overlap=false;if(o&&!o.hidden){const front=canvas.style.pointerEvents,back=o.style.pointerEvents;canvas.style.pointerEvents='none';o.style.pointerEvents='auto';overlap=d.elementsFromPoint((Math.max(c.left,v.left)+Math.min(c.right,v.right))/2,(Math.max(c.top,v.top)+Math.min(c.bottom,v.bottom))/2).includes(o);canvas.style.pointerEvents=front;o.style.pointerEvents=back;}
 return {definition:'60 actions per scenario, real UI event listeners, requestAnimationFrame intervals and input-to-next-layout latency; headless software composition, excludes compilation; final sharp paint awaited',dpr:w.devicePixelRatio,rows,final:{painted:painted(),canvas:[canvas.width,canvas.height],pixel_ratio:[canvas.width/c.width,canvas.height/c.height],viewport_covered:c.left<=Math.max(f.left,v.left)+epsilon&&c.top<=Math.max(f.top,v.top)+epsilon&&c.right>=Math.min(f.right,v.right)-epsilon&&c.bottom>=Math.min(f.bottom,v.bottom)-epsilon,visible:pixels.some((v,i)=>i%4===3&&v>0),overview_clip_supported:!o||w.CSS.supports('clip-path',o.style.clipPath),overview_overlaps_front:overlap,overview_pixels:o?o.width*o.height:0,generation:root.dataset.generation}};
})()'''
