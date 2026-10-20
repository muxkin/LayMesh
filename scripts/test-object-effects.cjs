// Object effects: native/CanvasKit parity, scale-aware caching, and local updates.
// Uses the repository's Playwright harness when the in-app Browser is unavailable.
const fs=require('node:fs'),path=require('node:path'),cp=require('node:child_process'),assert=require('node:assert/strict');
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const [url,folder]=process.argv.slice(2);if(!url||!folder)throw Error('Usage: test-object-effects.cjs RUNTIME_ROOT OUTPUT');
const root=path.resolve(__dirname,'..'),out=path.resolve(folder),binary=process.env.LAYMESH_TEST_BINARY||path.join(root,'target/debug/laymesh');fs.mkdirSync(out,{recursive:true});
const assets=['tests/fonts/DejaVuSans.ttf','tests/fonts/text/CJK-Regular.otf','examples/gallery/assets/sample.png'].map(p=>({path:path.join(root,p),bytes:Array.from(fs.readFileSync(path.join(root,p)))}));
const cases={
 reflection:'p.add(rect(size=(24mm,14mm),fill="#43a6b5",effects=[reflection(distance=2mm,length=0.8,blur=0.3mm)]),offset=(12mm,8mm))',
 soft:'p.add(ellipse(size=(25mm,20mm),fill="#e45286",effects=[soft_edge(radius=2mm),glow(color="#8844ff",blur=1mm),reflection(length=0.6)]),offset=(12mm,8mm))',
 bevel:'p.add(rect(size=(25mm,18mm),fill="#e29b42",three_d=three_d(depth=5mm,rotation=(20deg,-30deg,0deg),bevel_top=(2mm,1mm),bevel_bottom=(1mm,1mm),material="metal"),effects=[shadow(blur=0.8mm)]),offset=(12mm,10mm))',
 perspective:'p.add(ellipse(size=(25mm,18mm),fill=linear_gradient(stops=[(0,"#42b8c2"),(1,"#4258c2")]),three_d=three_d(depth=4mm,rotation=(25deg,-35deg,5deg),projection="perspective",distance=60mm)),offset=(12mm,10mm))',
 curved_gradient:'p.add(rect(size=(35mm,22mm),border_radius=4mm,fill=linear_gradient(stops=[(0,"#4ec9b0"),(1,"#23889c")]),three_d=three_d(depth=5mm,rotation=(20deg,-28deg,-5deg),bevel_top=(1.5mm,1mm),bevel_bottom=(0.7mm,0.7mm),material="metal")),offset=(12mm,10mm))',
 curved_color:'p.add(rect(size=(35mm,22mm),border_radius=5mm,fill="#48a9a6",three_d=three_d(depth=5mm,rotation=(20deg,-30deg,0deg),material="flat")),offset=(10mm,10mm))',
 back:'p.add(rect(size=(25mm,18mm),fill="#bb7ac3",three_d=three_d(depth=5mm,rotation=(20deg,145deg,0deg),bevel_bottom=(1mm,1mm))),offset=(12mm,10mm))'
};
(async()=>{
cases.curved_color_alpha=cases.curved_color.replace("#48a9a6","#48a9a61a");
cases.holes=`p.add(text("OB8",font_family=${JSON.stringify(assets[0].path)},font_size=9mm,text_stroke_width=0.2mm,text_stroke_color="#173a51",color="#56a8bc",three_d=three_d(depth=2mm,rotation=(20deg,-30deg,0deg))),offset=(12mm,10mm))`;
cases.chinese=`p.add(text("中文",font_family=${JSON.stringify(assets[1].path)},font_size=10mm,color="#b49756",three_d=three_d(depth=2mm,rotation=(15deg,145deg,0deg))),offset=(15mm,12mm))`;
cases.formula='p.add(formula(r"\\frac{x^2+1}{\\sqrt{y}}",font_size=22pt,color="#6778b2",three_d=three_d(depth=1mm,rotation=(20deg,-25deg,0deg))),offset=(15mm,10mm))';
cases.rounded=`p.add(image(src=${JSON.stringify(assets[2].path)},size=(28mm,20mm),border_radius=3mm,three_d=three_d(depth=2mm,rotation=(20deg,145deg,0deg))),crop=box(offset=(0.2,0.1),size=(0.6,0.8)),offset=(15mm,10mm))`;
const browser=await chromium.launch({headless:true,...(process.env.CHROMIUM_PATH?{executablePath:process.env.CHROMIUM_PATH}:{}),args:['--no-sandbox','--no-proxy-server',...(process.env.HARDWARE_GPU==='1'?['--enable-gpu','--use-gl=angle','--use-angle=vulkan','--enable-features=Vulkan']:[])]});const results={cases:[],browser:browser.version(),checks:[],errors:[]};
try{
 const page=await browser.newPage({viewport:{width:1100,height:700}});page.on('pageerror',e=>results.errors.push(String(e)));
 await page.goto(url);await page.evaluate(()=>{document.title='LayMesh object effect verification';document.body.innerHTML='<h1>Object effects</h1><button id="zoom">Zoom ×2</button><canvas id="surface"></canvas>';});
 await page.evaluate(async ({base,assets,gpu})=>{
  const runtime=new URL('site/live/',base),wasm=await import(new URL('wasm/laymesh_wasm.js',runtime));await wasm.default();
  const mod=await import(new URL('skia-renderer.mjs',runtime)),meta=await(await fetch(new URL('skia/manifest.json',runtime))).json(),ck=await mod.initializeSkia(new URL('skia/'+meta.wasm,runtime).href);
  for(const asset of assets){if(/\.(ttf|otf)$/.test(asset.path))wasm.register_preview_font(asset.path,new Uint8Array(asset.bytes));else wasm.register_preview_asset(asset.path,new Uint8Array(asset.bytes));}
  mod.setSurfaceRasterizer(wasm.raster_surface3d,wasm.surface_texture_region,wasm.refine_surface3d,wasm.surface_material_texture);window.test={wasm,ck,renderer:new mod.SkiaRenderer(ck,document.querySelector('canvas'),{software:!gpu}),generation:0};
 },{base:url,assets,gpu:process.env.LAYMESH_BROWSER_GPU==='1'});
 for(const [name,body] of Object.entries(cases)){
  const source=`p=canvas(size=(60mm,45mm),background="${name.startsWith('curved_color')?'#00000000':'#f6f8fc'}")\n`+body,src=path.join(out,name+'.lay'),png=path.join(out,name+'-native.png');fs.writeFileSync(src,source);cp.execFileSync(binary,['render',src,'-o',png,'--dpi','152.4'],{cwd:root,stdio:'pipe'});
  const actual=await page.evaluate(async ({source,native,name})=>{
   const {wasm,renderer,ck}=test,job=wasm.prepare_render(source,'/effect.lay','{}'),draw=job.draw_job(),list=JSON.parse(draw.display_list());test.generation++;await renderer.install(list,id=>draw.resource_bytes(id),test.generation);const first=renderer.render([0,0,60,45],360,270);
   const before=renderer.readPixels(),image=ck.MakeImageFromEncoded(Uint8Array.from(atob(native),c=>c.charCodeAt(0))),ref=image.readPixels(0,0,{width:360,height:270,colorType:ck.ColorType.RGBA_8888,alphaType:ck.AlphaType.Unpremul,colorSpace:ck.ColorSpace.SRGB});image.delete();
   let sum=0,large=0;for(let i=0;i<before.length;i++){const d=Math.abs(before[i]-ref[i]);sum+=d;if(d>8)large++;}
   let colorCheck;
   if(name.startsWith('curved_color')){
    let interior=0,worst=0;const expected=[72,169,166],alpha=name==='curved_color_alpha'?26:255;
    for(let y=1;y<269;y++)for(let x=1;x<359;x++){
     let opaque=true;for(let dy=-1;dy<=1;dy++)for(let dx=-1;dx<=1;dx++)if(before[((y+dy)*360+x+dx)*4+3]!==alpha)opaque=false;
     if(opaque){interior++;for(let c=0;c<3;c++)worst=Math.max(worst,Math.abs(before[(y*360+x)*4+c]-expected[c])*alpha/255);}
    }
    colorCheck={interiorPixels:interior,alpha,maxPremultipliedChannelError:worst};
    if(interior<1000||worst>2)throw Error(`Rounded side color stripe: ${worst}/255 in ${interior} interior pixels`);
   }
   const all=()=>[...renderer.entries].filter(e=>e.surfaceTexture).map(e=>e.surfaceTexture),cached=all();const repeat=renderer.render([0,0,60,45],360,270);if(cached.some((c,i)=>all()[i]!==c))throw Error('Repeated render rebuilt a 3D texture');
   document.querySelector('#zoom').onclick=()=>renderer.render([0,0,60,45],720,540);test.oldTextures=cached;
   if(name==='bevel')test.wasmSvg=job.svg();
   draw.free();job.free();return {meanDelta:sum/before.length,largeFraction:large/before.length,surfaceTextures:cached.length,backend:renderer.backend,device:renderer.device,firstMs:first.rasterMs,repeatMs:repeat.rasterMs,colorCheck,svg:name==='bevel'?test.wasmSvg:undefined};
  },{source,native:fs.readFileSync(png).toString('base64'),name});
  await page.locator('canvas').screenshot({path:path.join(out,name+'-browser.png')});await page.locator('#zoom').click();
  const zoom=await page.evaluate(()=>({width:document.querySelector('canvas').width,newTextures:[...test.renderer.entries].filter(e=>e.surfaceTexture).every(e=>!test.oldTextures.includes(e.surfaceTexture))}));assert.equal(zoom.width,720);if(actual.surfaceTextures)assert(zoom.newTextures,'Zoom must replace lower-density 3D textures');
  const svg=path.join(out,name+'.svg');cp.execFileSync(binary,['render',src,'-o',svg,'--dpi','152.4'],{cwd:root,stdio:'pipe'});
  const vector=await page.evaluate(async ({svg,wasmSvg,native})=>{
   async function pixels(url){const im=new Image();im.src=url;await im.decode();const c=new OffscreenCanvas(360,270),ctx=c.getContext('2d');ctx.drawImage(im,0,0,360,270);return ctx.getImageData(0,0,360,270).data;}
   const ref=await pixels('data:image/png;base64,'+native),compare=async svg=>{const url=URL.createObjectURL(new Blob([svg],{type:'image/svg+xml'}));try{const data=await pixels(url);return data.reduce((sum,v,i)=>sum+Math.abs(v-ref[i]),0)/data.length;}finally{URL.revokeObjectURL(url);}};
   return {nativeSVG:await compare(svg),wasmSVG:wasmSvg?await compare(wasmSvg):null};
  },{svg:fs.readFileSync(svg,'utf8'),wasmSvg:actual.svg,native:fs.readFileSync(png).toString('base64')});delete actual.svg;
  assert(vector.nativeSVG<2,`${name}: SVG mismatch ${vector.nativeSVG}`);if(vector.wasmSVG!==null)assert(vector.wasmSVG<2,`WASM SVG mismatch ${vector.wasmSVG}`);
  results.cases.push({name,...actual,zoom,vector});assert(actual.meanDelta<2,`${name}: native/browser mean delta ${actual.meanDelta}`);assert(actual.largeFraction<.04,`${name}: native/browser mismatch area ${actual.largeFraction}`);
  if(process.env.HARDWARE_GPU==='1')assert(actual.backend==='Skia WebGL2'&&actual.device&&!/SwiftShader|llvmpipe|softpipe/i.test(actual.device),'Requested hardware GPU was unavailable');
 }
 const update=await page.evaluate(async()=>{
  const {wasm,renderer}=test,source='p=canvas(size=(60mm,45mm))\np.add(rect(size=(15mm,15mm),fill="#56aabb",three_d=three_d(rotation=(20deg,ANGLE,0deg))),offset=(8mm,10mm))\np.add(rect(size=(15mm,15mm),fill="#bb9956",three_d=three_d(rotation=(20deg,-30deg,0deg))),offset=(35mm,10mm))';
  const install=async angle=>{const job=wasm.prepare_render(source.replace('ANGLE',angle),'/update.lay','{}'),draw=job.draw_job();await renderer.install(JSON.parse(draw.display_list()),id=>draw.resource_bytes(id),++test.generation);renderer.render([0,0,60,45],360,270);draw.free();job.free();};
  await install('20deg');const before=[...renderer.entries].filter(e=>e.surfaceTexture).map(e=>e.surfaceTexture);await install('45deg');const after=[...renderer.entries].filter(e=>e.surfaceTexture).map(e=>e.surfaceTexture);return {before:before.length,after:after.length,reused:after.filter(e=>before.includes(e)).length};
 });assert.deepEqual(update,{before:2,after:2,reused:1});results.update=update;
 results.checks.push('single-object update retains unrelated texture');
 results.checks.push('nonblank canvas','repeat render cache','zoom regenerates 3D textures','native/browser pixel comparison');
 assert.equal(await page.title(),'LayMesh object effect verification');await page.screenshot({path:path.join(out,'interaction.png')});assert.deepEqual(results.errors,[]);
 const html=path.join(out,'offline.zip.html');cp.execFileSync(binary,['render',path.join(out,'bevel.lay'),'-o',html],{cwd:root,stdio:'pipe'});
 await page.goto('file://'+html);await page.waitForFunction(()=>document.getElementById('app')?.dataset.painted==='true',{},{timeout:60000});await page.locator('.preview-figure > canvas:not(.preview-overview)').screenshot({path:path.join(out,'offline.png')});
 assert(!/error|错误/i.test(await page.locator('#status').innerText()));results.checks.push('real file:// offline playback');assert.deepEqual(results.errors,[]);
 const showcase=path.join(out,'composition.zip.html');cp.execFileSync(binary,['render','examples/effects/object-3d.lay','-o',showcase],{cwd:root,stdio:'pipe'});await page.goto('file://'+showcase);await page.waitForFunction(()=>document.getElementById('app')?.dataset.painted==='true',{},{timeout:60000});await page.locator('.preview-figure > canvas:not(.preview-overview)').screenshot({path:path.join(out,'composition-offline.png')});assert.deepEqual(results.errors,[]);results.checks.push('shared object-3d composition in offline HTML');
 results.status='passed';
}catch(e){results.status='failed';results.error=String(e);throw e;}finally{fs.writeFileSync(path.join(out,'verification.json'),JSON.stringify(results,null,2)+'\n');await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
