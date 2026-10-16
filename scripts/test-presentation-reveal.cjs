// Compare compact retained reveals with the existing full-geometry sampler.
const fs=require('node:fs'),assert=require('node:assert/strict');
const pw=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const [url,out]=process.argv.slice(2);
(async()=>{const results=[];for(const engine of (process.env.ENGINES||'chromium,firefox,webkit').split(',')){
 const browser=await pw[engine].launch({headless:true});try{
  const page=await browser.newPage(),errors=[];page.on('pageerror',e=>errors.push(e.message));
  await page.goto(new URL('site/live/manifest.json',url).href);
  const checks=await page.evaluate(async()=>{
   const base=new URL('./',location),wasm=await import(new URL('wasm/laymesh_wasm.js',base));await wasm.default();
   const {SkiaRenderer,initializeSkia}=await import(new URL('skia-renderer.mjs',base)),meta=await(await fetch(new URL('skia/manifest.json',base))).json(),ck=await initializeSkia(new URL('skia/'+meta.wasm,base).href);
   const sources=[
    `style { @keyframes move { from {transform:rotateZ(-12deg);opacity:0.3} to {transform:rotateZ(12deg);opacity:1} } }
p=canvas(size=(100,80))
g=group()
for i in range(80) { g.add(line(length=30mm,line_color="#0796aa"),offset=(0mm,i*0.35mm)) }
a=p.add(g,offset=(15,12),rotation=17deg)
s=p.step()
s.animate(a,name="reveal",duration=1s,fill="both",easing="linear")
s.animate(a,name="move",duration=3s,fill="both",easing="linear")`,
    `p=canvas(size=(100,80))
a=p.add(rect(size=(35,25),fill="#137ea3",effects=[shadow(blur=2mm,offset=(3mm,2mm))]),offset=(25,25),rotation=15deg)
s=p.step()
s.animate(a,name="reveal",duration=1s,fill="both",easing="linear")`,
    `p=canvas(size=(100,80))
a=p.add(path(commands=[move_to(0,0),cubic_to(c1x=8,c1y=30,c2x=30,c2y=-5,x=45,y=10)],fill="none",border_color="#123456",end_head=head(size=(4mm,3mm))),offset=(20,20))
s=p.step()
s.animate(a,name="draw",duration=2s,fill="both",easing="linear")
s.animate(a,name="reveal",duration=1s,fill="both",easing="linear")`,
    `p=canvas(size=(100,80))
c=plot(size=(75,55),plot_area=box(offset=(10,5),size=(60,40)),x=axis(range=(0,2)),y=axis(range=(0,3)))
c.line(x=[0,1,2],y=[1,2,1],color="#ff0000")
a=p.add(c,offset=(5,5))
s=p.step()
s.animate(a,name="reveal",duration=1s,fill="both",easing="linear")`,
    `style { @keyframes fade {from {opacity:0}to {opacity:1}} }
p=canvas(size=(100,80))
a=p.add(rect(size=(40,35),fill="#ff0000"),offset=(15,12))
b=p.add(rect(size=(8,20),fill="#00aaee"),offset=(8,5))
s=p.step()
s.animate(a,name="reveal",duration=1s,fill="both",easing="linear")
s.animate(b,name="fade",duration=1s,fill="both",easing="linear")`
   ],checks=[];
   for(let index=0;index<sources.length;index++){
    const job=wasm.prepare_render(sources[index],'/reveal.lay','{}');let draw=job.draw_job();const list=JSON.parse(draw.display_list());
    if(index===4){
     const [parentName,childName]=list.animation.normal.steps[0].clips.map(c=>c.target),parent=list.nodes.find(n=>n.name===parentName),child=list.nodes.find(n=>n.name===childName);
     const sibling={...child,name:'',transform:[1,0,0,1,22,5],kind:{...child.kind}};
     parent.kind={kind:'group',children:[child,sibling],clip:null,mask:null};parent.hint={...parent.hint,animationSize:[40,35]};
     list.nodes=list.nodes.filter(n=>n!==child);draw.free();draw=wasm.prepare_display_list(JSON.stringify(list));
    }
    for(const software of [true,false]){
     const r=new SkiaRenderer(ck,new OffscreenCanvas(500,400),{software});await r.install(list,id=>draw.resource_bytes(id),1);
     let maxMeanError=0,maxDelta=0,reuseHits=0;
     for(const reduced of [false,true])for(const time of [0,.1,.3,.7,.99,1,1.5,2,3,.3]){
      const old=JSON.parse(draw.frame_state(time,reduced)),compact=JSON.parse(draw.presentation_state(time,reduced));
      r.render([0,0,100,80],500,400,old,1,false);await r.completeResources();const expected=r.readPixels();
      r.render([0,0,100,80],500,400,compact,1,false);await r.completeResources();const actual=r.readPixels();
      let sum=0,max=0;for(let i=0;i<actual.length;i++){const d=Math.abs(actual[i]-expected[i]);sum+=d;max=Math.max(max,d);}maxMeanError=Math.max(maxMeanError,sum/actual.length);maxDelta=Math.max(maxDelta,max);
      if(sum/actual.length>.01)throw Error(`Reveal ${index} ${r.backend} t=${time} changed pixels: ${sum/actual.length}`);
     }
     if(r.gr&&index===0){
      for(const time of [.2,.3,.4,.5]){await r.completeResources();const result=r.render([0,0,100,80],500,400,JSON.parse(draw.presentation_state(time,false)),1,true);if(result.skipped)throw Error('Prepared frame skipped');reuseHits+=result.layerCacheHits;}
      if(reuseHits<2)throw Error('Reveal content did not reuse the layer');
     }
     checks.push({index,backend:r.backend,maxMeanError,maxDelta,reuseHits});r.dispose();
    }
    draw.free();job.free();
   }return checks;
  });assert.deepEqual(errors,[]);results.push({engine,version:browser.version(),checks,errors});fs.writeFileSync(out,JSON.stringify(results,null,2));console.log(engine,'retained reveal oracle passed');
 }finally{await browser.close();}
}})().catch(e=>{console.error(e);process.exitCode=1;});
