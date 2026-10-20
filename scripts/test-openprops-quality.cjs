// Actual-document per-object pixel oracle. Separate from performance runs:
// captures at fixed times intentionally read pixels and do not measure speed.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const pw=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const [base,out]=process.argv.slice(2);fs.mkdirSync(out,{recursive:true});
(async()=>{
 const engine=process.env.ENGINE||'chromium',software=process.env.SOFTWARE==='1';
 const browser=await pw[engine].launch({headless:true,args:engine==='chromium'&&process.env.HARDWARE_GPU==='1'?['--enable-gpu','--use-gl=angle','--use-angle=vulkan','--enable-features=Vulkan']:[]});
 try{
  const page=await browser.newPage();await page.goto(base+'/site/live/manifest.json');
  await page.evaluate(async({base,reference})=>{
   const url=new URL(base+'/site/live/'),{initializeSkia,SkiaRenderer}=await import(new URL('skia-renderer.mjs',url)),{SkiaRenderer:Before}=await import(new URL(reference,url)),w=await import(new URL('wasm/laymesh_wasm.js',url));await w.default();
   const ck=await initializeSkia(new URL('skia/canvaskit.wasm',url).href);window.oracle={ck,w,SkiaRenderer,Before,base};
  },{base,reference:process.env.REFERENCE_MODULE||'baseline/skia-renderer.mjs'});
  const results=[];
  const first=Number(process.env.FIRST||1)-1,count=Number(process.env.PAGES||26);
  for(let index=first;index<Math.min(26,first+count);index++){
   const result=await page.evaluate(async({index,software,width,height,referenceCached})=>{
    const {ck,w,SkiaRenderer,Before,base}=oracle,doc=await(await fetch(base+'/fixtures/pages/'+index+'.json')).json(),list=doc.list,job=w.prepare_display_list(JSON.stringify(list));
    const scale=Math.min(width/list.width,height/list.height),dx=(width/scale-list.width)/2,dy=(height/scale-list.height)/2,view=[-dx,-dy,list.width+dx,list.height+dy];
    const before=new Before(ck,new OffscreenCanvas(width,height),{software,clipToPage:true,premultipliedRaster:referenceCached}),after=new SkiaRenderer(ck,new OffscreenCanvas(width,height),{software,clipToPage:true,premultipliedRaster:true}),checks=[],images={};
    await before.install(list,()=>{throw Error('Unexpected image resource');},1);await after.install(list,()=>{},1);
    const duration=doc.inspection.animation?.duration||0;
    for(const time of [...new Set([0,Math.min(.65,duration),duration])]){
     const states=duration?JSON.parse(job.presentation_state(time,false)):{};
     before.render(view,width,height,states,1,false,referenceCached,referenceCached);const expected=before.readPixels();
     after.render(view,width,height,states,1,false,true,true);const actual=after.readPixels();
     const objects=list.nodes.map((node,i)=>{
      const p=after.prepare(node),b=p.bounds,m=states[i]?.matrix||p.matrix;
      const corners=[[b[0],b[1]],[b[2],b[1]],[b[0],b[3]],[b[2],b[3]]].map(([x,y])=>{const d=m[6]*x+m[7]*y+m[8];return [(m[0]*x+m[1]*y+m[2])/d,(m[3]*x+m[4]*y+m[5])/d];});
      const region=[Math.max(0,Math.floor((Math.min(...corners.map(p=>p[0]))+dx)*scale)-2),Math.max(0,Math.floor((Math.min(...corners.map(p=>p[1]))+dy)*scale)-2),Math.min(width,Math.ceil((Math.max(...corners.map(p=>p[0]))+dx)*scale)+2),Math.min(height,Math.ceil((Math.max(...corners.map(p=>p[1]))+dy)*scale)+2)];
      let sum=0,max=0,large=0,count=0;for(let y=region[1];y<region[3];y++)for(let x=region[0];x<region[2];x++)for(let k=0;k<4;k++){const delta=Math.abs(expected[(y*width+x)*4+k]-actual[(y*width+x)*4+k]);sum+=delta;max=Math.max(max,delta);large+=delta>8;count++;}
      // Object-wide means can hide a broken low-opacity shadow beneath an
      // unchanged opaque subject. Also check local patches inside each object.
      let patchMean=0,patchOver8=0;
      for(let y0=region[1];y0<region[3];y0+=64)for(let x0=region[0];x0<region[2];x0+=64){let ps=0,pl=0,pc=0;for(let y=y0;y<Math.min(region[3],y0+64);y++)for(let x=x0;x<Math.min(region[2],x0+64);x++)for(let k=0;k<4;k++){const d=Math.abs(expected[(y*width+x)*4+k]-actual[(y*width+x)*4+k]);ps+=d;pl+=d>8;pc++;}if(pc){patchMean=Math.max(patchMean,ps/pc);patchOver8=Math.max(patchOver8,pl/pc);}}
      return {id:node.name||String(i),region,mean:count?sum/count:0,max,over8:count?large/count:0,patchMean,patchOver8};
     });
     checks.push({time,objects});
     if(index===15||index===16&&time>0&&time<1)for(const [name,pixels]of [['before',expected],['after',actual]]){const canvas=new OffscreenCanvas(width,height);canvas.getContext('2d').putImageData(new ImageData(new Uint8ClampedArray(pixels),width,height),0,0);images['P'+(index+1)+'-'+name]=Array.from(new Uint8Array(await(await canvas.convertToBlob()).arrayBuffer()));}
    }
    const result={id:doc.id,backend:after.backend,device:after.device,referenceBackend:before.backend,referenceDevice:before.device,nativeCompositions:after.nativeCompositions||0,checks,images};before.dispose();after.dispose();job.free();return result;
   },{index,software,width:Number(process.env.WIDTH||1920),height:Number(process.env.HEIGHT||920),referenceCached:process.env.REFERENCE_CACHED==='1'});
   for(const [name,bytes]of Object.entries(result.images))fs.writeFileSync(path.join(out,name+'.png'),Buffer.from(bytes));delete result.images;
   results.push(result);fs.writeFileSync(path.join(out,'objects.json'),JSON.stringify(results,null,2));console.log(result.id,'captured');
   if(process.env.EXPECT_BACKEND_SWITCH==='1'){
    assert.match(result.referenceBackend,/WebGL/);assert.match(result.referenceDevice,/swiftshader|llvmpipe|softpipe/i);
    assert.match(result.backend,/CPU/);assert(result.nativeCompositions>0,'Native composition not exercised after software-GPU bypass');
   }
  }
  const failures=results.flatMap(p=>p.checks.flatMap(c=>c.objects.filter(o=>o.mean>1||o.over8>.03||o.patchMean>3||o.patchOver8>.12).map(o=>({page:p.id,time:c.time,...o}))));
  fs.writeFileSync(path.join(out,'result.json'),JSON.stringify({status:failures.length?'failed':'passed',pages:results.length,engine,width:Number(process.env.WIDTH||1920),height:Number(process.env.HEIGHT||920),reference:process.env.REFERENCE_MODULE||'baseline/skia-renderer.mjs',referenceCached:process.env.REFERENCE_CACHED==='1',referenceBackend:results[0].referenceBackend,referenceDevice:results[0].referenceDevice,backendSwitchRequired:process.env.EXPECT_BACKEND_SWITCH==='1',backend:results[0].backend,device:results[0].device,failures},null,2));assert.deepEqual(failures,[]);
 }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
