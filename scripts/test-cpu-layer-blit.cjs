// Compare identical retained pixels with CanvasKit, including a later change
// from translation to rotation or a nested clip. No performance inference here.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const pw=require(process.env.PLAYWRIGHT_MODULE||'playwright'),[base,out]=process.argv.slice(2);fs.mkdirSync(out,{recursive:true});
(async()=>{for(const engine of(process.env.ENGINES||'chromium,firefox,webkit').split(',')){
 const browser=await pw[engine].launch({headless:true});try{
  const page=await browser.newPage();await page.goto(base+'/site/live/manifest.json');
  const result=await page.evaluate(async base=>{
   const root=base+'/site/live/',before=await import(root+'skia-renderer-v23.mjs'),after=await import(root+'skia-renderer.mjs'),wasm=await import(root+'wasm/laymesh_wasm.js');await wasm.default();const ck=await after.initializeSkia(root+'skia/canvaskit.wasm'),results=[];
   const diff=(a,b)=>{let sum=0,max=0,large=0;for(let i=0;i<a.length;i++){const d=Math.abs(a[i]-b[i]);sum+=d;max=Math.max(max,d);large+=d>1;}return {max,mean:sum/a.length,over1:large/a.length};};
   for(const index of[11,16]){
    const doc=await(await fetch(base+'/fixtures/pages/'+index+'.json')).json(),job=wasm.prepare_display_list(JSON.stringify(doc.list)),width=3840,height=1840,scale=Math.min(width/doc.list.width,height/doc.list.height),dx=(width/scale-doc.list.width)/2,dy=(height/scale-doc.list.height)/2,view=[-dx,-dy,doc.list.width+dx,doc.list.height+dy],rs=[];
    for(const module of[before,after]){
     const r=new module.SkiaRenderer(ck,new OffscreenCanvas(width,height),{software:true,premultipliedRaster:true,clipToPage:true,nativeComposition:false});rs.push(r);await r.install(doc.list,()=>{throw Error('Unexpected image');},1);r.ensureSurface(width,height);
     const states=JSON.parse(job.presentation_state(.65,false)),work=r.sceneTasks(view,width,height,states),fixed=function*(){for(;;){r.preparationTileSize=256;const step=work.next();if(step.done)return;yield step.value;}};await r.prepareTasks(fixed());
    }
    const checks=[],r=rs[1];
    const compare=(label,states)=>{const pixels=rs.map(r=>{r.render(view,width,height,states,1,false,true,true);return r.readPixels();});const difference=diff(...pixels);if(difference.max>1)throw Error(JSON.stringify({label,...difference}));checks.push({label,...difference,blits:r.cpuLayerBlits,conversions:r.cpuTileConversions});};
    for(const time of[.1,.65,doc.inspection.animation.duration])compare(String(time),JSON.parse(job.presentation_state(time,false)));
    if(!r.cpuLayerBlits||!r.cpuTileBytes)throw Error('CPU pixel path was not exercised');
    const retained=r.textureAllocated,states=JSON.parse(job.presentation_state(.65,false));for(let i=0;i<3;i++)compare('repeat-'+i,states);
    if(r.textureAllocated!==retained||r.cpuTileConversions)throw Error('Stable frames changed retained representations');
    const id=doc.list.nodes.findIndex(node=>{const p=r.prepare(node);return /^c\d+$/.test(node.name)&&r.layerTextures.get(p.textureKey??p.uid)?.tiles.some(t=>t.pixels);}),node=doc.list.nodes[id],p=r.prepare(node),b=p.bounds,baseMatrix=JSON.parse(job.presentation_state(doc.inspection.animation.duration,false))[id]?.matrix||p.matrix,cx=(b[0]+b[2])/2,cy=(b[1]+b[3])/2,angle=Math.PI,cos=Math.cos(angle),sin=Math.sin(angle),local=[cos,-sin,cx-cos*cx+sin*cy,sin,cos,cy-sin*cx-cos*cy,0,0,1],multiply=(a,b)=>Array.from({length:9},(_,i)=>{const row=Math.floor(i/3)*3,col=i%3;return a[row]*b[col]+a[row+1]*b[col+3]+a[row+2]*b[col+6];});
    const rotated={...states,[id]:{...states[id],matrix:multiply(baseMatrix,local),opacity:.57}};compare('rotation-fallback',rotated);
    if(!r.cpuTileDemotions)throw Error(JSON.stringify({problem:'Rotation did not restore the general image representation',page:doc.id,id,name:node.name,baseMatrix,rotated:rotated[id],blits:r.cpuLayerBlits,builds:r.layerBuilds,tiles:[...r.layerTextures].map(([key,e])=>({key,density:e.density,pixels:e.tiles.filter(t=>t.pixels).length,disabled:e.cpuBlitDisabled})),selected:p.textureKey??p.uid}));
    const restored=r.textureAllocated;compare('translation-after-rotation',states);compare('rotation-again',rotated);if(r.textureAllocated!==restored)throw Error('Alternating transforms caused representation churn');
    // The same content with an enclosing diagonal clip must respect that clip.
    const clipId=doc.list.nodes.findIndex((node,i)=>{const p=r.prepare(node);return i!==id&&/^c\d+$/.test(node.name)&&r.layerTextures.get(p.textureKey??p.uid)?.tiles.some(t=>t.pixels);}),clipNode=doc.list.nodes[clipId],clipMatrix=JSON.parse(job.presentation_state(doc.inspection.animation.duration,false))[clipId]?.matrix||r.prepare(clipNode).matrix,clip=ck.Path.MakeFromSVGString('M0 0 L3840 1600 L0 1840 Z');
    const clipped=rs.map(renderer=>{const c=renderer.surface.getCanvas();c.clear(ck.TRANSPARENT);c.save();c.clipPath(clip,ck.ClipOp.Intersect,true);c.scale(scale,scale);c.translate(-view[0],-view[1]);c.concat(clipMatrix);renderer.drawLayer(c,clipNode,'parent/child',renderer.prepare(clipNode),.63);c.restore();renderer.surface.flush();return renderer.readPixels();});clip.delete();const clipDiff=diff(...clipped);if(clipDiff.max>1)throw Error(JSON.stringify({clipDiff}));checks.push({label:'nested-clip',...clipDiff});
    for(const entry of r.layerTextures.values())for(const tile of entry.tiles)if(!!tile.image===!!tile.pixels)throw Error('Tile must retain exactly one representation');
    const resources={retainedBytes:r.textureAllocated-r.textureReleased,cpuTileBytes:r.cpuTileBytes,promotions:r.cpuTilePromotions,demotions:r.cpuTileDemotions,workingPeakBytes:r.cpuTileWorkingPeakBytes};
    for(const renderer of rs){renderer.dispose();if(renderer.textureAllocated!==renderer.textureReleased||renderer.textureImages||(renderer.cpuTileBytes||0))throw Error('Retained tile resources leaked');}job.free();results.push({page:doc.id,checks,resources});
   }
   const source=`p=canvas(size=(120mm,80mm),background="#00000000")
a=p.add(rect(size=(55mm,32mm),border_radius=4mm,fill="#27b9aa77",effects=[shadow(blur=1mm,offset=(2mm,3mm),opacity=0.17)]),offset=(15.17mm,22.13mm))
b=p.add(ellipse(size=(42mm,35mm),fill="#d57e5688",effects=[glow(blur=1mm,color="#4477bb",opacity=0.23)]),offset=(49.23mm,29.11mm))
style { @keyframes move { from { transform:translateX(-3mm); opacity:0.23; } to { transform:translateX(3mm); opacity:0.91; } } }
s=p.step(hold=2s)
s.animate(a,name="move",duration=1s,easing="linear",fill="both")
s.animate(b,name="move",duration=1s,easing="linear",fill="both")`;
   const job=wasm.prepare_render(source,'/transparent.lay','{}'),draw=job.draw_job(),list=JSON.parse(draw.display_list()),renderers=[];
   for(const module of[before,after]){const renderer=new module.SkiaRenderer(ck,new OffscreenCanvas(960,640),{software:true,premultipliedRaster:true,clipToPage:true,nativeComposition:false});await renderer.install(list,id=>draw.resource_bytes(id),1);renderers.push(renderer);}
   const checks=[];
   for(const time of[0,.3,.7,1]){const states=JSON.parse(draw.presentation_state(time,false)),pixels=renderers.map(renderer=>{renderer.render([0,0,120,80],960,640,states,1,false,true,true);return renderer.surface.premultipliedPixels().slice();}),difference=diff(...pixels);if(difference.max>1)throw Error(JSON.stringify({transparent:time,...difference}));checks.push({time,premultiplied:difference,blits:renderers[1].cpuLayerBlits});}
   if(!checks.some(c=>c.blits))throw Error('Transparent composite did not exercise CPU pixel path');
   renderers.forEach(r=>r.dispose());draw.free();job.free();results.push({page:'transparent-overlap',checks});
   return results;
  },base);
  assert.equal(result.length,3);fs.writeFileSync(path.join(out,engine+'.json'),JSON.stringify({status:'passed',engine,result},null,2));console.log(engine,'translation, opacity, rotation, clipping, ownership passed');
 }finally{await browser.close();}
}})().catch(e=>{console.error(e);process.exitCode=1;});
