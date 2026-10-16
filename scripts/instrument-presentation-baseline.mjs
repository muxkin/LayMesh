// Add first-transition-frame timing to an archived runtime without changing its
// scheduling, rendering, caches, compilation, or original artifact.
import fs from 'node:fs';
import assert from 'node:assert/strict';
import {webPayload,webUnzip,webZip,webUniversal} from '../site/live/web-archive.mjs';
const [input,output]=process.argv.slice(2);assert(input&&output&&input!==output);
if(fs.statSync(input).isDirectory()){
 assert(!fs.existsSync(output),'Output must be a new directory');fs.cpSync(input,output,{recursive:true});
 for(const name of ['presentation-worker.mjs','presentation.mjs']){
  const file=output+'/dist/'+name;let s=fs.readFileSync(file,'utf8');
  if(name==='presentation-worker.mjs')s=s.replace('async play(reduced=false)','async play(reduced=false,onVisible=()=>{})').replace('if(reduced){this.cancel();return {};}', 'if(reduced){this.cancel();onVisible();return {};}').replace('if(!pending)return this.error?{transitionError:this.error}:{};', 'if(!pending){onVisible();return this.error?{transitionError:this.error}:{};}').replace('let frames=0,cancelled=false,started;','let frames=0,visible=false,cancelled=false,started;').replace('draw(reverse?1-p:p);frames++;','draw(reverse?1-p:p);if(!visible&&t>0){visible=true;onVisible();}frames++;').replace('transitions.play(data.reduced)','transitions.play(data.reduced,()=>renderer.skia.completeResources().then(()=>notify({type:"navigation-visible",serial:data.serial,pageId:data.pageId})))');
  else s=s.replaceAll("if(data.type==='context')", "if(data.type==='navigation-visible'){this.navigationVisible?.(data);return;}if(data.type==='context')").replace('canvas:this.canvas},data=>{if(generation!==this.generation)return;',"canvas:this.canvas},data=>{if(generation!==this.generation)return;if(data.type==='navigation-visible'){this.navigationVisible?.(data);return;}");
  fs.writeFileSync(file,s);
 }
 console.log(output);
}else{
const files=webUnzip(webPayload(fs.readFileSync(input))),decode=b=>new TextDecoder().decode(b);
for(const name of ['presentation-transitions.mjs','presentation-worker.mjs','presentation.mjs','web-player.mjs']){
 const key='runtime/'+name;let s=decode(files[key]);
 if(name==='presentation-transitions.mjs'){
  assert(s.includes('async play(reduced=false)'));
  s=s.replace('async play(reduced=false)','async play(reduced=false,onVisible=()=>{})').replace('if(reduced){this.cancel();return {};}', 'if(reduced){this.cancel();onVisible();return {};}').replace('if(!pending)return this.error?{transitionError:this.error}:{};', 'if(!pending){onVisible();return this.error?{transitionError:this.error}:{};}').replace('let frames=0,cancelled=false,started;','let frames=0,visible=false,cancelled=false,started;').replace('draw(reverse?1-p:p);frames++;','draw(reverse?1-p:p);if(!visible&&t>0){visible=true;onVisible();}frames++;');
 }else if(name==='presentation-worker.mjs')s=s.replace('transitions.play(data.reduced)','transitions.play(data.reduced,()=>renderer.skia.completeResources().then(()=>notify({type:"navigation-visible",serial:data.serial,pageId:data.pageId})))');
 else if(name==='presentation.mjs')s=s.replaceAll("if(data.type==='context')", "if(data.type==='navigation-visible'){this.navigationVisible?.(data);return;}if(data.type==='context')").replace('canvas:this.canvas},data=>{if(generation!==this.generation)return;',"canvas:this.canvas},data=>{if(generation!==this.generation)return;if(data.type==='navigation-visible'){this.navigationVisible?.(data);return;}");
 else s=s.replace('if (generation !== p.generation) return;', 'if (generation !== p.generation) return;\n      if (data.type === "navigation-visible") {p.navigationVisible?.(data);return;}');
 files[key]=new TextEncoder().encode(s);
}
fs.writeFileSync(output,Buffer.from(await webUniversal(webZip(files),decode(files['runtime/bootstrap.js'])).arrayBuffer()),{flag:'wx'});
console.log(output);
}
