import {initializeSkia} from './skia-renderer.mjs';
let sharedSkia;
import {createPresentationRuntime} from './presentation-worker.mjs';
// A small software surface shares the already loaded CanvasKit/WASM module.
// Presenter static previews can use the audience's existing CanvasKit instance.
export class PagePreviewRenderer {
 constructor({canvas=document.createElement('canvas'),runtime,page,skiaInstance}){Object.assign(this,{canvas,runtime,page,skiaInstance});this.serial=0;this.queue=Promise.resolve();this.resident=new Set();}
 draw(index,width,time=null,reduced=false){
  const work=async()=>{if(this.closed)return;if(!this.run){const runtime=await this.runtime();const skia=this.skiaInstance||(sharedSkia??=initializeSkia(runtime.skiaBytes||runtime.skiaURL).catch(error=>{sharedSkia=null;throw error;}));this.run=createPresentationRuntime({...runtime,skiaInstance:await skia,canvas:this.canvas,software:true}).catch(error=>{this.run=null;throw error;});}const run=await this.run;if(this.closed)return;
   const page=await this.page(index);if(this.closed)return;const height=Math.max(1,Math.round(width*page.list.height/page.list.width));
   if(!this.resident.has(page.id)){const result=await run({type:'load',page,installOnly:true,width,height});this.resident=new Set(result.resident);}
   await run({type:'activate',serial:++this.serial,pageId:page.id,width,height,time,reduced});return this.canvas;
  };const next=this.queue.then(work);this.queue=next.catch(()=>{});return next;
 }
 async dispose(){this.closed=true;await this.queue;const run=await this.run?.catch(()=>null);await run?.({type:'dispose'});this.canvas.width=this.canvas.height=1;this.resident.clear();}
}
export {PageThumbnails} from "./page-thumbnails.mjs";
