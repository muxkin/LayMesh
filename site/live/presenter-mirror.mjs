// Copies completed audience frames. Never samples a timeline or renders a scene.
export class PresenterMirror {
 constructor({canvas,send,rect=()=>null,onError=()=>{}}){Object.assign(this,{canvas,send,rect,onError});this.width=1280;this.visible=true;this.sequence=0;this.generation=0;this.interval=1000/15;this.metrics={frames:0,maxInFlight:0,copyMs:0,fpsLimit:15};}
 configure({visible=true,width=1280}={}){this.visible=visible;this.width=Math.max(160,Math.min(1280,Math.round(width)));if(visible){this.failed=false;this.force();}}
 present(meta){this.latest=meta;this.flush(!meta.running);}
 force(){this.flush(true);}
 acknowledge(sequence){if(sequence!==this.pending)return;clearTimeout(this.timeout);this.pending=null;if(this.dirty){this.dirty=false;this.flush(true);}}
 flush(force=false){
  if(this.closed||!this.visible||!this.latest||this.failed)return;
  if(this.pending){if(force)this.dirty=true;return;}
  const now=performance.now();if(!force&&now-(this.last||0)<this.interval)return;
  const canvas=this.canvas();if(!canvas?.width||!canvas?.height)return;
  const sequence=++this.sequence,generation=this.generation,meta=this.latest;this.pending=sequence;this.last=now;this.metrics.maxInFlight=1;
  const [x,y,w,h]=this.rect(meta,canvas)||[0,0,canvas.width,canvas.height],width=Math.min(this.width,w),height=Math.max(1,Math.round(width*h/w));
  // The audience surface preserves its presented buffer for forced idle/resize
  // snapshots. There is no readPixels, encoding, or second scene evaluation.
  createImageBitmap(canvas,x,y,w,h,{resizeWidth:Math.round(width),resizeHeight:height}).then(bitmap=>{
   if(this.closed||generation!==this.generation){bitmap.close();return;}
   const cost=performance.now()-now;this.metrics.frames++;this.metrics.copyMs+=cost;
   this.averageCost=this.averageCost===undefined?cost:this.averageCost*.8+cost*.2;
   // Software GPUs can make a canvas copy expensive. Sacrifice monitor FPS,
   // never audience animation frames; terminal snapshots still bypass the cap.
   const fps=this.averageCost>24?5:this.averageCost>12?10:15;this.interval=1000/fps;this.metrics.fpsLimit=fps;
   try{this.send({type:'frame',...meta,sequence,bitmap},[bitmap]);this.timeout=setTimeout(()=>{this.pending=null;this.failed=true;this.onError('预览画面传输超时；观众放映继续。');},5000);}catch(error){bitmap.close();throw error;}
  }).catch(error=>{if(generation!==this.generation||this.closed)return;this.pending=null;this.failed=true;this.onError('控制台镜像暂不可用：'+error.message);});
 }
 reset(){this.generation++;clearTimeout(this.timeout);this.pending=null;this.latest=null;this.failed=false;this.dirty=false;}
 dispose(){this.closed=true;this.reset();}
}
