// Verify displayed cached pixels against direct drawing, including CPU-prepared
// images subsequently shown by a GPU renderer. This is correctness, not timing.
const fs=require('node:fs'),assert=require('node:assert/strict'),path=require('node:path'),{pathToFileURL}=require('node:url');
const pw=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const [file,out]=process.argv.slice(2);
(async()=>{const results=[];for(const engine of (process.env.ENGINES||'chromium,firefox,webkit').split(',')){
 const browser=await pw[engine].launch({headless:true});try{
  const p=await browser.newPage({viewport:{width:1280,height:800}}),errors=[];p.setDefaultTimeout(180000);p.on('pageerror',e=>errors.push(e.message));
  await p.route(/^https?:\/\//,r=>r.abort());await p.goto(pathToFileURL(path.resolve(file)).href);
  await p.waitForFunction(()=>window.__LAYMESH_WEB?.presentation&&document.querySelector('#app').dataset.painted==='true');
  const checks=await p.evaluate(async()=>{
   const v=__LAYMESH_WEB.presentation;await v.open({fullscreen:false});if(!v.ready)throw Error(v.root.dataset.error||'Presentation failed to open');v.clock.running=false;await v.paint();await v.preload;
   const checks=[];window.preparationImages={};
   for(let index=0;index<v.deck.pages.length;index++){
    const page=await v.deck.store.page(v.deck.pages[index].id),r=v.worker.owners.get(page.id),base={serial:v.serial,...v.dimensions(),reduced:false,priority:true};
    const ids=[page.id+'/cpu-oracle',page.id+'/gpu-oracle'];
    for(const [i,id]of ids.entries()){
     await v.worker.request(r,{...base,type:'load',page:{...page,id},stage:'entry',cpuPreparation:i===0});
     await v.worker.request(r,{...base,type:'prepare',pageId:id,stage:'entry',entryTime:v.deck.pages[index].inspection.animation?.duration||0,cpuPreparation:i===0});
     await v.worker.request(r,{...base,type:'prepare',pageId:id,stage:'warm',cpuPreparation:i===0});
    }
    for(const time of [0,.25,1,3,6.5]){
     const pixels=[];let draw;
     for(const pageId of ids){
      draw=await v.worker.request(r,{...base,type:'paint',pageId,time,running:true});
      pixels.push((await v.worker.request(r,{type:'capture',cached:true})).rgba);
     }
     const [actual,expected]=pixels;
     if(!expected.some(c=>c>0)||!actual.some(c=>c>0))throw Error('Empty pixels invalidate the comparison');
     let sum=0,max=0,large=0;for(let i=0;i<actual.length;i++){const delta=Math.abs(actual[i]-expected[i]);sum+=delta;max=Math.max(max,delta);large+=delta>8;}
     if(index===1&&time===3){for(const [name,pixels]of [['cpu',actual],['gpu',expected]]){const canvas=new OffscreenCanvas(base.width,base.height);canvas.getContext('2d').putImageData(new ImageData(new Uint8ClampedArray(pixels),base.width,base.height),0,0);const raw=new Uint8Array(await(await canvas.convertToBlob()).arrayBuffer());window.preparationImages[name]=Array.from(raw);}}
     checks.push({page:index+1,time,meanChannelError:sum/actual.length,maxChannelError:max,channelsOver8:large/actual.length,layerHits:draw.layerCacheHits,staticHits:draw.staticCacheHits,backend:draw.backend});
    }
    const frame=await v.worker.request(r,{type:'snapshot'}),surface=new OffscreenCanvas(frame.bitmap.width,frame.bitmap.height),ctx=surface.getContext('2d');ctx.drawImage(frame.bitmap,0,0);frame.bitmap.close();
    if(!ctx.getImageData(0,0,surface.width,surface.height).data.some(v=>v>0))throw Error('Transferred transition bitmap was empty');
   }
   await v.close();if(document.querySelector('.presentation-buffer'))throw Error('Retained presentation canvas after close');
   await v.open({fullscreen:false});if(!v.ready)throw Error(v.root.dataset.error||'Presentation failed to open');v.clock.running=false;await v.close();return checks;
  });
  for(const [name,data]of Object.entries(await p.evaluate(()=>window.preparationImages)))fs.writeFileSync(out+'.'+engine+'.'+name+'.png',Buffer.from(data));
  assert.deepEqual(errors,[]);results.push({engine,checks,errors});fs.writeFileSync(out,JSON.stringify(results,null,2));
  // This compares different Skia raster backends, not bitwise-identical images.
  // Preserve and report all differences; require sub-code-value mean error and
  // a small affected area for this dense-curve fixture. Same-backend scene and
  // animation fidelity is checked separately against the original HTML.
  assert(checks.every(c=>c.meanChannelError<1&&c.channelsOver8<.03),JSON.stringify(checks.filter(c=>c.meanChannelError>=1||c.channelsOver8>=.03)));
  console.log(engine,'CPU/GPU full-density cache comparison and close/reopen passed',checks.length);
 }finally{await browser.close();}
}})().catch(e=>{console.error(e);process.exitCode=1;});
