// Diagnostic only: pixel oracle and timings before any production change.
const fs=require('node:fs'),path=require('node:path');
const pw=require(process.env.PLAYWRIGHT_MODULE||'playwright'),[base,out]=process.argv.slice(2);
(async()=>{const browser=await pw.firefox.launch({headless:true});try{
 const page=await browser.newPage();await page.goto(base+'/site/live/manifest.json');
 const result=await page.evaluate(async base=>{
  const root=base+'/site/live/',module=await import(root+'skia-renderer-blit-probe.mjs'),wasm=await import(root+'wasm/laymesh_wasm.js');await wasm.default();const ck=await module.initializeSkia(root+'skia/canvaskit.wasm');
  const results=[];
  for(const index of[11,16]){
   const doc=await(await fetch(base+'/fixtures/pages/'+index+'.json')).json(),job=wasm.prepare_display_list(JSON.stringify(doc.list)),width=3840,height=1840,scale=Math.min(width/doc.list.width,height/doc.list.height),dx=(width/scale-doc.list.width)/2,dy=(height/scale-doc.list.height)/2,view=[-dx,-dy,doc.list.width+dx,doc.list.height+dy],rs=[];
   for(let variant=0;variant<2;variant++){
    const r=new module.SkiaRenderer(ck,new OffscreenCanvas(width,height),{software:true,premultipliedRaster:true,clipToPage:true});rs.push(r);
    if(variant){const original=r.drawLayer.bind(r);r.drawLayer=function(c,node,id,p,opacity){
     const m=c.getTotalMatrix();if(!/^\d+$/.test(id)||node.hint?.animationRevealContent||m[1]||m[3]||m[6]||m[7]||m[0]<=0||m[4]<=0)return original(c,node,id,p,opacity);
     if(c.quickReject(p.bounds))return true;const required=module.projectedDensity(m,p.bounds),entry=this.buildLayer(p,required);if(!entry)return false;
     const sx=m[0]/entry.density,sy=m[4]/entry.density,tx=m[2],ty=m[5],clip=this.pageBounds||[0,0,width,height],alpha=Math.fround(Math.max(0,Math.min(1,opacity)));
     const prepared=entry.tiles.map(tile=>{if(!tile.probePixels)tile.probePixels=tile.image.readPixels(0,0,{width:tile.w+4,height:tile.h+4,colorType:ck.ColorType.RGBA_8888,alphaType:ck.AlphaType.Premul,colorSpace:ck.ColorSpace.SRGB});return tile;});
     // Acquire after readPixels: a native allocation may have grown the heap.
     const dst=this.surface.premulPixels();
     for(const {x,y,w,h,probePixels:src}of prepared){
      const sw=w+4,left=Math.max(0,Math.ceil(clip[0]-.5),Math.ceil(tx+x*sx-.5)),right=Math.min(width,Math.ceil(clip[2]-.5),Math.ceil(tx+(x+w)*sx-.5)),top=Math.max(0,Math.ceil(clip[1]-.5),Math.ceil(ty+y*sy-.5)),bottom=Math.min(height,Math.ceil(clip[3]-.5),Math.ceil(ty+(y+h)*sy-.5));
      for(let py=top;py<bottom;py++){
       const v=(py+.5-ty)/sy-y+1.5,iy=Math.floor(v),fy=v-iy;let di=(py*width+left)*4;
       for(let px=left;px<right;px++,di+=4){
        const u=(px+.5-tx)/sx-x+1.5,ix=Math.floor(u),fx=u-ix,a0=(iy*sw+ix)*4,a1=a0+sw*4;
        const topA=src[a0+3]+(src[a0+7]-src[a0+3])*fx,bottomA=src[a1+3]+(src[a1+7]-src[a1+3])*fx,a=(topA+(bottomA-topA)*fy)*alpha;if(a<=0)continue;const inv=1-a/255;
        for(let k=0;k<3;k++){const topC=src[a0+k]+(src[a0+k+4]-src[a0+k])*fx,bottomC=src[a1+k]+(src[a1+k+4]-src[a1+k])*fx;dst[di+k]=Math.min(255,Math.floor((topC+(bottomC-topC)*fy)*alpha+dst[di+k]*inv+.5));}
        dst[di+3]=Math.min(255,Math.floor(a+dst[di+3]*inv+.5));
       }
      }
     }
     return true;
    };}
    await r.install(doc.list,()=>{throw Error('Unexpected image');},1);r.ensureSurface(width,height);r.preparationTileSize=256;
    const states=JSON.parse(job.presentation_state(.65,false)),work=r.sceneTasks(view,width,height,states),fixed=function*(){for(;;){r.preparationTileSize=256;const step=work.next();if(step.done)return;yield step.value;}};await r.prepareTasks(fixed());r.render(view,width,height,states,1,false,true,true);
   }
   const checks=[],timings=[];
   for(const time of[.1,.65,doc.inspection.animation.duration]){
    const s=JSON.parse(job.presentation_state(time,false)),pixels=rs.map(r=>{r.render(view,width,height,s,1,false,true,true);return r.readPixels();});
    let max=0,sum=0,large=0;for(let i=0;i<pixels[0].length;i++){const d=Math.abs(pixels[0][i]-pixels[1][i]);max=Math.max(max,d);sum+=d;large+=d>8;}const objects=[];doc.list.nodes.forEach((node,id)=>{if(!/^c\d+$/.test(node.name))return;const p=rs[0].prepare(node),m=s[id]?.matrix||p.matrix,b=p.bounds,points=[[b[0],b[1]],[b[2],b[1]],[b[0],b[3]],[b[2],b[3]]].map(([x,y])=>[(m[0]*x+m[1]*y+m[2]-view[0])*scale,(m[3]*x+m[4]*y+m[5]-view[1])*scale]),x0=Math.max(0,Math.floor(Math.min(...points.map(p=>p[0])))),x1=Math.min(width,Math.ceil(Math.max(...points.map(p=>p[0])))),y0=Math.max(0,Math.floor(Math.min(...points.map(p=>p[1])))),y1=Math.min(height,Math.ceil(Math.max(...points.map(p=>p[1]))));let total=0,sum=0,max=0,large=0;for(let y=y0;y<y1;y++)for(let x=x0;x<x1;x++)for(let k=0;k<4;k++){const i=(y*width+x)*4+k,d=Math.abs(pixels[0][i]-pixels[1][i]);total++;sum+=d;max=Math.max(max,d);large+=d>8;}objects.push({name:node.name,total,max,mean:total?sum/total:0,over8:total?large/total:0});});checks.push({time,max,mean:sum/pixels[0].length,over8:large/pixels[0].length,objects});
    for(let round=0;round<4;round++)for(const variant of round%2?[1,0]:[0,1]){const v=rs[variant].render(view,width,height,s,1,false,true,true);timings.push({time,round,variant,rasterMs:v.rasterMs,layerBuilds:v.layerCacheBuilds,staticBuilds:v.staticCacheBuilds});}
   }
   results.push({page:doc.id,checks,timings});rs.forEach(r=>r.dispose());job.free();
  }
  return results;
 },base);fs.mkdirSync(path.dirname(out),{recursive:true});fs.writeFileSync(out,JSON.stringify(result,null,2));for(const d of result){console.log(d.page,JSON.stringify(d.checks));for(let variant=0;variant<2;variant++){const rows=d.timings.filter(x=>x.variant===variant&&x.round>0);console.log(variant,rows.reduce((s,x)=>s+x.rasterMs,0)/rows.length);}}
 }finally{await browser.close();}})().catch(e=>{console.error(e);process.exitCode=1;});
