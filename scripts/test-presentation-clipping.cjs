// Page bounds, including cached animation and intermediate transition frames.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const pw=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const [url,output]=process.argv.slice(2);
if(!url||!output)throw Error('Usage: test-presentation-clipping.cjs SITE_URL OUTPUT.json');
(async()=>{
 const results=[];
 for(const engine of (process.env.LAYMESH_TEST_ENGINES||'chromium,firefox,webkit').split(',')){
  const browser=await pw[engine].launch({headless:true});
  try{
   const page=await browser.newPage(),errors=[];page.on('pageerror',e=>errors.push(String(e)));
   await page.goto(new URL('site/live/manifest.json',url).href);
   const checks=await page.evaluate(async()=>{
    const base=new URL('./',location),{initializeSkia,SkiaRenderer}=await import(new URL('skia-renderer.mjs',base)),{PresentationRenderer}=await import(new URL('presentation-renderer.mjs',base));
    const meta=await(await fetch(new URL('skia/manifest.json',base))).json(),ck=await initializeSkia(new URL('skia/'+meta.wasm,base).href),wasm=await import(new URL('wasm/laymesh_wasm.js',base));await wasm.default();
    const source=`style { @keyframes move { from { transform: translateX(0mm); } to { transform: translateX(30mm); } } }
p=canvas(size=(160mm,90mm),background="#ffffff")
p.add(rect(size=(180mm,110mm),fill="#ee3344",border_width=0pt),offset=(-10mm,-10mm))
g=group()
for i in range(40) { g.add(line(length=40mm,line_color="#0099aa",line_width=0.8mm),offset=(0mm,i*0.5mm)) }
a=p.add(g,offset=(135mm,70mm))
p.add(ellipse(size=(24mm,24mm),fill="#0099aa",effects=[shadow(blur=4mm,offset=(4mm,4mm))]),offset=(150mm,-10mm))
s=p.step()
s.animate(a,name="move",duration=1s,easing="linear",fill="both")`;
    const job=wasm.prepare_render(source,'/clip.lay','{}'),draw=job.draw_job(),list=JSON.parse(draw.display_list()),out=[];
    for(const software of [false,true]){
     const canvas=document.createElement('canvas');document.body.replaceChildren(canvas);
     let checkTransitions=false,transitionChecks=0;
     const r=new PresentationRenderer(ck,canvas,{software,onPresent:()=>{if(checkTransitions){checkPixels();transitionChecks++;}}});
     const checkPixels=()=>{
      const w=canvas.width,h=canvas.height,scale=Math.min(w/160,h/90),left=(w-160*scale)/2,top=(h-90*scale)/2,right=w-left,bottom=h-top,rgba=r.skia.readPixels();
      let leaked=0;
      for(let y=0;y<h;y++)for(let x=0;x<w;x++)if((x+.5<left-1||x+.5>right+1||y+.5<top-1||y+.5>bottom+1)&&rgba[(y*w+x)*4+3])leaked++;
      if(leaked)throw Error(`${r.skia.backend} ${w}x${h}: ${leaked} pixels leaked outside the page`);
      const center=((Math.floor(h/2)*w)+Math.floor(w/2))*4;
      if(!checkTransitions&&rgba[center+3]!==255)throw Error('Page content was clipped away');
     };
     await r.install('a',list,id=>draw.resource_bytes(id));
     for(const [w,h]of [[800,600],[1000,450],[800,450]]){
      await r.prepare('a',w,h,t=>JSON.parse(draw.frame_state(t,false)),true);
      for(const t of [null,0,.5,1,.5]){
       let result;
       for(let attempt=0;attempt<100;attempt++){
        result=r.render('a',w,h,t===null?{}:JSON.parse(draw.frame_state(t,false)),t!==null);
        if(!result.skipped)break;
        await new Promise(resolve=>setTimeout(resolve,10));
       }
       if(result.skipped)throw Error('Renderer stayed busy');checkPixels();
      }
     }
     await r.prepareEntry('a',800,450,0,false,t=>JSON.parse(draw.frame_state(t,false)));
     if(!r.show('a',800,450,0,false)?.frameCacheHit)throw Error('Prepared entry was not reused');
     const entry=r.skia.readPixels();r.render('a',800,450,JSON.parse(draw.frame_state(0,false)),false);
     const full=r.skia.readPixels();let delta=0;for(let i=0;i<full.length;i++)delta+=Math.abs(full[i]-entry[i]);
     if(delta/full.length>.6)throw Error('Prepared entry changed the frame');
     const effects=await import(new URL('presentation-transitions-data.mjs',base));
     const names=Object.keys(effects.transitions).filter(n=>!effects.transitions[n].textures?.length).slice(0,12);
     const handles=names.map(name=>r.skia.transitions.effect(name));
     if(names.some((name,i)=>r.skia.transitions.effect(name)!==handles[i]))throw Error('Transition effects were evicted');
     const other=structuredClone(list);other.nodes[1].opacity=.4;await r.install('b',other,id=>draw.resource_bytes(id));
     for(const [w,h]of [[800,600],[1000,450]])for(const name of ['fade','Directional','InvertedPageCurl'])for(const reverse of [false,true]){
      r.render('a',w,h,{},false);r.skia.transitions.capture({name,durationMs:100,easing:{Bezier:[0,0,1,1]},params:{direction:[1,0]}},reverse);
      r.render('b',w,h,{},false);checkTransitions=true;
      const result=await r.skia.transitions.play();checkTransitions=false;
      if(result.transitionRasterSize?.[0]!==w||result.transitionRasterSize?.[1]!==h)throw Error('Transition reduced display resolution');
      if(result.transitionError)throw Error(result.transitionError);
      if(result.transitionFrames<1)throw Error('No intermediate transition frame');checkPixels();
     }
     r.retain(new Set(['a']));if(r.skia.transitions.effects.size)throw Error('Unused transition effects remained');
     out.push({retainedTransitionEffects:12,entryFrameMatches:true,backend:r.skia.backend,viewports:3,animationPoses:5,transitions:12,transitionPixelChecks:transitionChecks});r.dispose();
    }
    // General editor/export views may intentionally include off-page geometry.
    const raw=new SkiaRenderer(ck,new OffscreenCanvas(800,600),{software:true});await raw.install(list,id=>draw.resource_bytes(id),1);raw.render([0,-15,160,105],800,600);
    if(raw.readPixels()[(40*800+400)*4+3]===0)throw Error('Unclipped editor viewport changed');raw.dispose();draw.free();job.free();return out;
   });
   assert.deepEqual(errors,[]);results.push({engine,checks});console.log(engine,'page clipping passed');
   fs.mkdirSync(path.dirname(output),{recursive:true});fs.writeFileSync(output,JSON.stringify({status:'passed',results},null,2)+'\n');
  }finally{await browser.close();}
 }
})().catch(e=>{console.error(e);process.exitCode=1;});
