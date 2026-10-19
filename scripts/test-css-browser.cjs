// Pinned offline CSS, project units and effect parity across native SVG/PNG and WASM/CanvasKit.
const fs=require('fs'),path=require('path'),cp=require('child_process'),assert=require('assert/strict');
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const [url,output]=process.argv.slice(2);if(!url||!output)throw Error('Usage: test-css-browser.cjs SITE_URL OUTPUT_DIRECTORY');
const root=path.resolve(__dirname,'..'),entry=path.join(root,'tests/css/libraries.lay'),source=fs.readFileSync(entry,'utf8'),binary=process.env.LAYMESH_CLI||path.join(root,'target/release/laymesh');
fs.mkdirSync(output,{recursive:true});
for(const ext of ['svg','png','pdf'])cp.execFileSync(binary,['render',entry,'-o',path.join(output,'native.'+ext),...(ext==='svg'?[]:['--dpi','254']),'--warnings','hide']);
(async()=>{const browser=await chromium.launch({headless:true,...(process.env.CHROMIUM_PATH?{executablePath:process.env.CHROMIUM_PATH}:{}),args:['--no-sandbox','--no-proxy-server']});try{
 const page=await browser.newPage({viewport:{width:1200,height:850}}),errors=[];page.on('pageerror',e=>errors.push(String(e)));await page.goto(new URL('site/live/manifest.json',url).href);
 const report=await page.evaluate(async ({base,entry,source,font,fontPath,nativeSVG,nativePNG})=>{
  const wasm=await import(new URL('wasm/laymesh_wasm.js',base));await wasm.default();
  wasm.register_preview_font(fontPath,Uint8Array.from(atob(font),c=>c.charCodeAt(0)));
  const fontStack=wasm.prepare_render('p=canvas(size=(80mm,50mm),stylesheet="@laymesh/bootstrap.css")\np.add(text("Offline CSS libraries",class="text-primary"))','/font-stack.lay','{}');
  if(JSON.parse(fontStack.inspection()).warnings.some(w=>w.code==='W_FONT'))throw Error('Bootstrap font stack failed with registered WASM fonts');fontStack.free();
  const {initializeSkia,SkiaRenderer}=await import(new URL('skia-renderer.mjs',base)),meta=await(await fetch(new URL('skia/manifest.json',base))).json(),ck=await initializeSkia(new URL('skia/'+meta.wasm,base).href);
  const job=wasm.prepare_render(source,entry,'{}'),inspection=JSON.parse(job.inspection()),svg=job.svg();
  // Float color conversion can differ by a few ULPs between native and WASM.
  // Preserve exact structure and bound numeric serialization, then compare
  // actual pixels below; do not quantize source colors to make strings equal.
  const numbers=/[-+]?\d*\.?\d+(?:e[-+]?\d+)?/gi;
  const a=Array.from(svg.matchAll(numbers),m=>Number(m[0])),b=Array.from(nativeSVG.matchAll(numbers),m=>Number(m[0]));
  const svgNumericError=a.length===b.length?a.reduce((max,x,i)=>Math.max(max,x===b[i]?0:Math.abs(x-b[i])),0):Infinity;
  if(svg.replace(numbers,'#')!==nativeSVG.replace(numbers,'#')||svgNumericError>0.0001)throw Error('Native/WASM SVG structure or numeric values differ: '+svgNumericError);
  const warnings=inspection.warnings.filter(w=>w.code==='W_CSS');if(warnings.length!==2)throw Error('Expected one warning per pinned CSS file');
  const draw=job.draw_job(),list=JSON.parse(draw.display_list()),canvas=document.createElement('canvas');document.body.replaceChildren(canvas);document.body.style.margin='16px';canvas.style.width='1000px';canvas.style.height='700px';
  const renderer=new SkiaRenderer(ck,canvas,{software:true});await renderer.install(list,id=>draw.resource_bytes(id),1);renderer.render([0,0,100,70],1000,700);const pixels=renderer.readPixels();
  const bitmap=await createImageBitmap(await(await fetch('data:image/png;base64,'+nativePNG)).blob()),reference=document.createElement('canvas');reference.width=1000;reference.height=700;const context=reference.getContext('2d');context.drawImage(bitmap,0,0);bitmap.close();const expected=context.getImageData(0,0,1000,700).data;
  let sum=0,max=0;for(let i=0;i<pixels.length;i++){const d=Math.abs(pixels[i]-expected[i]);sum+=d;max=Math.max(max,d);}const mean=sum/pixels.length;if(mean>1)throw Error('Native/WASM PNG mean error '+mean+' exceeds 1/255');
  const effects=[];function collect(nodes){for(const n of nodes){effects.push(...n.effects);if(n.kind.kind==='group')collect(n.kind.children);}}collect(list.nodes);
  if(!effects.some(e=>e.knockout&&e.sourcePath)||!effects.some(e=>e.inner)||!effects.some(e=>e.spread<0))throw Error('Missing standard shadow geometry');
  const updated=wasm.prepare_render(source.replace('box-shadow-blur:2mm','box-shadow-blur:4mm'),entry,'{}'),next=updated.draw_job(),nextList=JSON.parse(next.display_list());const modified=[];function shadows(nodes){for(const n of nodes){modified.push(...n.effects.filter(e=>e.knockout));if(n.kind.kind==='group')shadows(n.kind.children);}}shadows(nextList.nodes);
  const old=effects.filter(e=>e.knockout)[0],changed=modified[0];for(const key of ['rgba','offset','spread','inner'])if(JSON.stringify(old[key])!==JSON.stringify(changed[key]))throw Error('Partial blur changed '+key);if(changed.blur!==2||old.blur!==1)throw Error('CSS blur radius was not halved');
  await renderer.install(nextList,id=>next.resource_bytes(id),2);renderer.render([0,0,100,70],1000,700);const after=renderer.readPixels();if(!pixels.some((v,i)=>v!==after[i]))throw Error('Local blur override did not repaint');
  renderer.dispose();next.free();updated.free();draw.free();job.free();
  return {nativeWasmSVG:{structure:'identical',maxNumericError:svgNumericError},nativeWasmPNG:{mean,max},offlineLibraries:['bootstrap@5.3.8','open-props@1.7.23'],fontStack:'registered WASM fonts; no font warnings',warnings: warnings.length,blurOverride:{beforeRadius:2,afterRadius:4,otherFields:'preserved'},effects:effects.length};
 },{base:new URL('site/live/',url).href,entry,source,font:fs.readFileSync(path.join(root,'tests/fonts/DejaVuSans.ttf')).toString('base64'),fontPath:path.join(root,'tests/fonts/DejaVuSans.ttf'),nativeSVG:fs.readFileSync(path.join(output,'native.svg'),'utf8'),nativePNG:fs.readFileSync(path.join(output,'native.png')).toString('base64')});
 assert.deepEqual(errors,[]);report.consoleErrors=errors;await page.screenshot({path:path.join(output,'wasm-libraries.png')});
 fs.writeFileSync(path.join(output,'verification.json'),JSON.stringify(report,null,2));console.log(JSON.stringify(report));
}finally{await browser.close();}})().catch(e=>{console.error(e);process.exitCode=1});
