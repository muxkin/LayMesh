// CanvasKit retention, resource replacement, context recovery and 4K buffers.
const fs=require('fs'),path=require('path'),assert=require('assert/strict');
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const [url,output]=process.argv.slice(2);if(!url||!output)throw Error('Usage: test-skia-browser.cjs SITE_URL OUTPUT.json');
(async()=>{const browser=await chromium.launch({headless:true,...(process.env.CHROMIUM_PATH?{executablePath:process.env.CHROMIUM_PATH}:{}),args:['--no-sandbox','--no-proxy-server']});const report={checks:[]};
try{
 const p=await browser.newPage();await p.goto(new URL('site/live/manifest.json',url).href);
 report.renderer=await p.evaluate(async base=>{
  const {initializeSkia,SkiaRenderer}=await import(new URL('skia-renderer.mjs',base)),meta=await(await fetch(new URL('skia/manifest.json',base))).json(),ck=await initializeSkia(new URL('skia/'+meta.wasm,base).href);
  const wasm=await import(new URL('wasm/laymesh_wasm.js',base));await wasm.default();
  const job=wasm.prepare_render('p=canvas(size=(160mm,90mm),background="#ffffff")\np.add(rect(size=(80mm,45mm),fill="#3978d4"),offset=(20mm,20mm))','/test.lay','{}'),draw=job.draw_job(),list=JSON.parse(draw.display_list());
  const canvas=document.createElement('canvas');document.body.replaceChildren(canvas);let restored=false,lost=false;
  const renderer=new SkiaRenderer(ck,canvas,{onContextChange:ready=>{if(ready)restored=true;else lost=true;}});await renderer.install(list,id=>draw.resource_bytes(id),1);
  const paint=()=>renderer.render([0,0,160,90],3840,2160);paint();const before=renderer.readPixels(),commands=renderer.owned.length;
  for(let i=0;i<120;i++){paint();await new Promise(requestAnimationFrame);}if(renderer.owned.length!==commands)throw Error('Retained objects grew during playback');
  await renderer.install(structuredClone(list),id=>draw.resource_bytes(id),2);paint();if(renderer.owned.length!==commands)throw Error('Identical content rebuilt commands');
  await renderer.install(structuredClone(list),id=>draw.resource_bytes(id),3,0);if(renderer.pictureBytes!==0)throw Error('Zero cache budget retained pictures');paint();
  const old=renderer.generation;try{await renderer.install({...list,resources:{broken:{mime:'image/png'}}},()=>new Uint8Array([0]),3);throw Error('Bad resource accepted');}catch(e){if(e.message==='Bad resource accepted')throw e;}
  if(renderer.generation!==old)throw Error('Failed install replaced the old generation');paint();const after=renderer.readPixels();if(before.some((v,i)=>v!==after[i]))throw Error('Failed update changed pixels');
  if(renderer.gl){
   const gl=renderer.gl,wait=gl.clientWaitSync;gl.clientWaitSync=()=>gl.TIMEOUT_EXPIRED;
   try{for(let i=0;i<3;i++)renderer.render([0,0,160,90],3840,2160,{},renderer.generation,true);const count=renderer.frameCount;
    if(!renderer.render([0,0,160,90],3840,2160,{},renderer.generation,true).skipped||renderer.frameCount!==count)throw Error('Animation queued work behind a busy GPU');
   }finally{gl.clientWaitSync=wait;}
   paint(); // Explicit static/seek paints still complete after animation backpressure.
  }
  const extension=renderer.gl?.getExtension('WEBGL_lose_context');
  if(extension){extension.loseContext();for(let i=0;i<100&&!lost;i++)await new Promise(r=>setTimeout(r,10));if(!renderer.render([0,0,160,90],3840,2160).skipped)throw Error('Lost context still rendered');extension.restoreContext();for(let i=0;i<100&&!restored;i++)await new Promise(r=>setTimeout(r,10));if(!restored)throw Error('Context did not restore');paint();const recovered=renderer.readPixels();if(before.some((v,i)=>v!==recovered[i]))throw Error('Recovery changed pixels');}
  const result={backend:renderer.backend,device:renderer.device,width:canvas.width,height:canvas.height,commands,contextRecovery:!!extension};renderer.dispose();
  const node={name:'image',transform:[1,0,0,1,0,0],opacity:1,effects:[],semantic:null,hint:{},kind:{kind:'image',resource:'same-id',rect:[0,0,10,10],nearest:true}},imageList={schemaVersion:1,width:10,height:10,nodes:[node],resources:{'same-id':{mime:'image/x-laymesh-rgba'}}};
  const software=new SkiaRenderer(ck,new OffscreenCanvas(10,10),{software:true}),bytes=new Uint8Array(12),view=new DataView(bytes.buffer);view.setUint32(0,1,true);view.setUint32(4,1,true);bytes.set([255,0,0,255],8);
  await software.install(imageList,()=>bytes.slice(),1);software.render([0,0,10,10],10,10);if(software.readPixels()[0]!==255)throw Error('Software image failed');bytes.set([0,0,255,255],8);
  await software.install(structuredClone(imageList),()=>bytes.slice(),2,0);software.render([0,0,10,10],10,10);const pixel=software.readPixels();if(pixel[0]!==0||pixel[2]!==255)throw Error('Resource bytes under same ID stayed stale');result.softwareBackend=software.backend;software.dispose();
  const nestedJob=wasm.prepare_render('style {@keyframes fade {from{opacity:0}to{opacity:1}}}\np=canvas(size=(20,20),background="none")\na=p.add(rect(size=(20,20)))\nb=p.add(rect(size=(3,8),fill="#ff0000",border_color="none"),offset=(1,1))\ns=p.step()\ns.animate(a,name="reveal",duration=1s,fill="both",easing="linear")\ns.animate(b,name="fade",duration=1s,fill="both",easing="linear")','/nested.lay','{}');
  const initial=nestedJob.draw_job(),nested=JSON.parse(initial.display_list()),clips=nested.animation.normal.steps[0].clips;
  const red=structuredClone(nested.nodes.find(n=>n.name===clips[1].target)),blue=structuredClone(red);blue.name='';blue.transform=[1,0,0,1,5,1];blue.kind.fill={kind:'solid',rgba:[0,0,1,1]};
  const group=structuredClone(nested.nodes.find(n=>n.name===clips[0].target));group.hint={animationSize:[20,20]};group.kind={kind:'group',children:[red,blue],clip:null,mask:null};nested.nodes=[group];
  const nestedDraw=wasm.prepare_display_list(JSON.stringify(nested)),nestedRenderer=new SkiaRenderer(ck,new OffscreenCanvas(20,20),{software:true});await nestedRenderer.install(nested,id=>nestedDraw.resource_bytes(id),1);nestedRenderer.render([0,0,20,20],20,20,JSON.parse(nestedDraw.frame_state(.5,false)));const nestedPixels=nestedRenderer.readPixels();
  if(Math.abs(nestedPixels[(4*20+2)*4+3]-128)>1||nestedPixels[(4*20+7)*4+3]!==255)throw Error('Parent reveal remapped child animation or sibling opacity');
  result.nestedReveal=true;nestedRenderer.dispose();nestedDraw.free();initial.free();nestedJob.free();draw.free();job.free();return result;
 },new URL('site/live/',url).href);
 report.checks.push('3840×2160 buffer; 120 retained paints; transactional failed install; context recovery; software fallback; same-ID image replacement');await p.close();
 const context=await browser.newContext();await context.addInitScript(()=>{HTMLCanvasElement.prototype.transferControlToOffscreen=undefined;});const fallback=await context.newPage();const errors=[];fallback.on('pageerror',e=>errors.push(String(e)));await fallback.goto(new URL('examples/animation-presentation.html',url).href);await fallback.waitForFunction(()=>document.querySelector('.example-module')?.dataset.painted==='true');await fallback.locator('[data-action=animation-play]').click();await fallback.waitForFunction(()=>Number(document.querySelector('[aria-label="Animation time"]').value)>.2);await fallback.locator('[data-action=animation-play]').click();assert.deepEqual(errors,[]);report.mainThread=await fallback.locator('.example-module').evaluate(m=>({...m.dataset}));report.checks.push('Main-thread CanvasKit fallback plays directly from summary');await context.close();
 fs.mkdirSync(path.dirname(output),{recursive:true});fs.writeFileSync(output,JSON.stringify(report,null,2));console.log(JSON.stringify(report));
}finally{await browser.close();}})().catch(e=>{console.error(e);process.exitCode=1});
