// Retained-content ownership and compositing oracle; never a speed benchmark.
const fs=require('node:fs'),path=require('node:path'),pw=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const [base,out]=process.argv.slice(2);
(async()=>{for(const engine of(process.env.ENGINES||'chromium,firefox').split(',')){
 const browser=await pw[engine].launch({headless:true});try{
  const page=await browser.newPage();await page.goto(base+'/site/live/manifest.json');
  const result=await page.evaluate(async base=>{
   const root=base+'/site/live/',{initializeSkia,SkiaRenderer}=await import(root+'skia-renderer.mjs'),{drawNativeFade}=await import(root+'presentation-transitions.mjs'),{premultiplyPixels}=await import(root+'skia-cpu-surface.mjs'),wasm=await import(root+'wasm/laymesh_wasm.js');await wasm.default();const meta=await(await fetch(root+'skia/manifest.json')).json(),ck=await initializeSkia(root+'skia/'+meta.wasm);
   const job=wasm.prepare_render(`p=canvas(size=(120mm,80mm),background="#00000000")
a=p.add(rect(size=(55mm,32mm),border_radius=4mm,fill="#27b9aa77",effects=[shadow(blur=1mm,offset=(2mm,3mm),opacity=0.17)]),offset=(15.17mm,22.13mm))
b=p.add(ellipse(size=(42mm,35mm),fill="#d57e5688",effects=[glow(blur=1mm,color="#4477bb",opacity=0.23)]),offset=(49.23mm,29.11mm))
style { @keyframes move { from { transform:translateX(-3mm); opacity:0.23; } to { transform:translateX(3mm); opacity:0.91; } } }
s=p.step(hold=2s)
s.animate(a,name="move",duration=1s,easing="linear",fill="both")
s.animate(b,name="move",duration=1s,easing="linear",fill="both")`,'/native.lay','{}'),draw=job.draw_job(),list=JSON.parse(draw.display_list()),renderers=[],width=960,height=640,checks=[];
   const compare=(label,a,b,limit=3)=>{a=premultiplyPixels(a.slice());b=premultiplyPixels(b.slice());let max=0,sum=0,large=0;for(let i=0;i<a.length;i++){const d=Math.abs(a[i]-b[i]);max=Math.max(max,d);sum+=d;large+=d>limit;}const result={label,max,mean:sum/a.length,overLimit:large/a.length};if(result.mean>.35||result.overLimit>.005)throw Error(JSON.stringify(result));checks.push(result);};
   for(const nativeComposition of[false,true]){
    const r=new SkiaRenderer(ck,new OffscreenCanvas(width,height),{software:true,premultipliedRaster:true,clipToPage:true,nativeComposition});await r.install(list,id=>draw.resource_bytes(id),1);r.ensureSurface(width,height);r.preparationTileSize=256;
    for(const node of list.nodes)if(r.animatedNames.has(node.name)){const p=r.prepare(node);await r.prepareTasks(r.layerTasks(p,12));}
    renderers.push(r);
   }
   const [reference,native]=renderers;
   for(const time of[0,.3,.7,1])for(const angle of[0,.23,Math.PI]){
    const states=JSON.parse(draw.presentation_state(time,false));for(const state of Object.values(states)){const m=state.matrix,c=Math.cos(angle),s=Math.sin(angle);if(m)state.matrix=[c*m[0]-s*m[3],c*m[1]-s*m[4],m[2],s*m[0]+c*m[3],s*m[1]+c*m[4],m[5],0,0,1];}
    const pixels=renderers.map(r=>{r.render([0,0,120,80],width,height,states,1,false,true,true);return r.readPixels();});compare('rotation-opacity-'+time+'-'+angle,...pixels);
   }
   if(!native.nativeCompositions||native.surface.rasterBufferBytes)throw Error('Native composition allocated a full WASM framebuffer');
   const retained=native.textureAllocated,promotions=native.nativePromotions;
   for(let i=0;i<4;i++)native.render([0,0,120,80],width,height,JSON.parse(draw.presentation_state(.65,false)),1,false,true,true);
   if(native.textureAllocated!==retained||native.nativePromotions!==promotions)throw Error('Stable content rebuilt or changed representation');
   for(const entry of [...native.staticTextures.values(),...native.layerTextures.values()])if(entry.bitmap&&entry.tiles.some(t=>t.image||t.pixels))throw Error('Duplicate persistent representations');
   // Deferred target preparation must leave the displayed frame untouched.
   const before=native.canvas.getContext('2d').getImageData(0,0,width,height).data;
   native.render([0,0,120,80],width,height,JSON.parse(draw.presentation_state(1,false)),1,false,true,true,false,null,false);
   const during=native.canvas.getContext('2d').getImageData(0,0,width,height).data;
   if(before.some((v,i)=>v!==during[i]))throw Error('Deferred composition changed displayed frame');
   const bitmap=native.surface.nativeSnapshot();native.surface.flush();const after=native.canvas.getContext('2d').getImageData(0,0,width,height).data;compare('deferred-endpoint',after,native.readPixels());bitmap.close();
   const node=list.nodes.find(n=>native.animatedNames.has(n.name)),clip=ck.Path.MakeFromSVGString('M0 0 L960 420 L0 640 Z');
   const clipped=renderers.map(r=>{const c=r.surface.getCanvas();c.clear(ck.TRANSPARENT);c.save();c.clipPath(clip,ck.ClipOp.Intersect,true);c.scale(8,8);c.concat(r.prepare(node).matrix);r.drawLayer(c,node,'parent/child',r.prepare(node),.57);c.restore();r.surface.flush();return r.readPixels();});clip.delete();compare('nested-clip-fallback',...clipped);
   if(!native.nativeDemotions)throw Error('Nested clip did not restore Skia content');
   const states=JSON.parse(draw.presentation_state(.65,false));native.render([0,0,120,80],width,height,states,1,false,true,true);const demotions=native.nativeDemotions,allocated=native.textureAllocated;
   for(let i=0;i<3;i++)native.render([0,0,120,80],width,height,states,1,false,true,true);
   if(native.nativeDemotions!==demotions||native.textureAllocated!==allocated)throw Error('Fallback caused representation churn');
   const resource={bitmapBytes:native.nativeBitmapBytes,allocations:native.textureAllocated,promotions:native.nativePromotions,demotions:native.nativeDemotions};
   renderers.forEach(r=>{r.dispose();if(r.textureAllocated!==r.textureReleased||r.textureImages||(r.nativeBitmapBytes||0))throw Error('Retained resources leaked');});draw.free();job.free();
   // Deterministic premultiplied fade reference, including low-alpha colors.
   const w=64,h=64,inputs=[0,1].map(n=>{const c=new OffscreenCanvas(w,h),g=c.getContext('2d'),data=new ImageData(w,h);for(let i=0;i<data.data.length;i+=4){data.data[i]=(i*7+n*31)%256;data.data[i+1]=(i*3+n*79)%256;data.data[i+2]=(i+n*17)%256;data.data[i+3]=(i/4+n*53)%256;}g.putImageData(data,0,0);const retained=g.getImageData(0,0,w,h);return {bitmap:c.transferToImageBitmap(),data:retained};}),output=new OffscreenCanvas(w,h),c=output.getContext('2d');
   for(const progress of[0,.01,.25,.5,.75,.99,1]){
    drawNativeFade(c,inputs[0].bitmap,inputs[1].bitmap,w,h,null,progress);const actual=premultiplyPixels(c.getImageData(0,0,w,h).data),a=premultiplyPixels(inputs[0].data.data.slice()),b=premultiplyPixels(inputs[1].data.data.slice());let max=0;for(let i=0;i<a.length;i++)max=Math.max(max,Math.abs(actual[i]-Math.round(a[i]*(1-progress)+b[i]*progress)));if(max>3){let worst;for(let i=0;i<a.length;i++)if(Math.abs(actual[i]-Math.round(a[i]*(1-progress)+b[i]*progress))===max){worst={i,actual:actual[i],a:a[i],b:b[i],expected:Math.round(a[i]*(1-progress)+b[i]*progress)};break;}throw Error(JSON.stringify({label:'fade',progress,max,worst}));}checks.push({label:'fade-'+progress,max});
   }inputs.forEach(i=>i.bitmap.close());
   const opaque=[0,1].map(n=>{const canvas=new OffscreenCanvas(w,h),g=canvas.getContext('2d');g.fillStyle=n?'#a6c8ef':'#f7e4c1';g.fillRect(0,0,w,h);return canvas.transferToImageBitmap();});
   for(const progress of[.01,.5,.99]){drawNativeFade(c,...opaque,w,h,null,progress,[0,0,w,h]);const rgba=c.getImageData(0,0,w,h).data;for(let i=3;i<rgba.length;i+=4)if(rgba[i]!==255)throw Error('Opaque fade lost coverage');checks.push({label:'opaque-fade-'+progress,alpha:255});}opaque.forEach(b=>b.close());return {checks,resource};
  },base);fs.mkdirSync(out,{recursive:true});fs.writeFileSync(path.join(out,engine+'.json'),JSON.stringify({status:'passed',engine,result},null,2));console.log(engine,'native composition, transparency, rotation, deferred frame and ownership passed');
 }finally{await browser.close();}
}})().catch(e=>{console.error(e);process.exitCode=1;});
