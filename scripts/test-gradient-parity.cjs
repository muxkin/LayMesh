// Reproducible CSS reference images. Run after building the native engine.
// Browser plugin execution was unavailable in the implementation environment;
// this uses the project's existing Playwright test mechanism.
const fs=require('node:fs'),path=require('node:path'),cp=require('node:child_process'),crypto=require('node:crypto');
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const root=path.resolve(__dirname,'..'),out=path.resolve(process.argv[2]||'target/gradient-compatibility/parity');
const engine=path.resolve(process.env.LAYMESH_TEST_BINARY||'target/debug/laymesh');
const native=process.env.GRADIENT_NATIVE!=='0',runtime=process.env.GRADIENT_RUNTIME_URL;
const exportDpi=Number(process.env.GRADIENT_EXPORT_DPI||96);
const prior=!native&&fs.existsSync(path.join(out,'manifest.json'))?JSON.parse(fs.readFileSync(path.join(out,'manifest.json'),'utf8')):null;
fs.mkdirSync(out,{recursive:true});
const css=fs.readFileSync(path.join(root,'crates/laymesh-core/assets/styles/open-props.css'),'utf8');
const extras=[
 ['hint','background:linear-gradient(to right,red,25%,blue)'],
 ['hint-auto','background:linear-gradient(to right,red,10%,blue,lime)'],
 ['transparent','background:linear-gradient(to right,red,transparent)'],
 ['alpha','background:linear-gradient(to right,rgba(255,0,0,.2),rgba(0,0,255,.8))'],
 ['hard','background:linear-gradient(45deg,red 30% 50%,blue 50% 70%,lime 70%)'],
 ['outside-stops','background:linear-gradient(to right,red -20%,blue 120%)'],
 ['physical-stops','background:linear-gradient(to right,red 10px,blue calc(80% + 10px))'],
 ['repeat-linear','background:repeating-linear-gradient(35deg,red 10px 20px,blue 20px 30px)'],
 ['repeat-linear-offset','background:repeating-linear-gradient(to right,red -20px,blue 20px)'],
 ['repeat-radial','background:repeating-radial-gradient(circle at 35% 70%,red 10px,blue 30px)'],
 ['repeat-conic','background:repeating-conic-gradient(from 17deg,red 0deg 60deg,blue 60deg 120deg)'],
 ['repeat-conic-offset','background:repeating-conic-gradient(red -30deg,blue 30deg)'],
 ['radial-near','background:radial-gradient(ellipse closest-side at 25% 40%,red,blue)'],
 ['radial-near-corner','background:radial-gradient(ellipse closest-corner at 25% 40%,red,blue)'],
 ['radial-far','background:radial-gradient(circle farthest-side at 10% 80%,red,blue)'],
 ['radial-size','background:radial-gradient(80px 35% at right -10px bottom 5px,red,blue)'],
 ['srgb-linear','background:linear-gradient(to right in srgb-linear,black,white)'],
 ['hsl-long','background:linear-gradient(to right in hsl longer hue,red,blue)'],
 ['hsl-increasing','background:linear-gradient(to right in hsl increasing hue,blue,red)'],
 ['hsl-decreasing','background:linear-gradient(to right in hsl decreasing hue,red,blue)'],
 ['lab','background:linear-gradient(to right in lab,lab(40% 50 30),lab(70% -30 -20))'],
 ['oklch','background:linear-gradient(to right in oklch longer hue,oklch(60% .1 20),oklch(70% .1 250))'],
 ['missing','background:linear-gradient(to right in srgb,rgb(none 0 0),rgb(255 0 0))'],
 ['p3','background:linear-gradient(to right in display-p3,color(display-p3 .6 .2 .1),color(display-p3 .1 .3 .6))'],
 ['bottom-color','background-color:lime;background-image:linear-gradient(90deg,rgba(255,0,0,.5),transparent)'],
 ['layer-order','background:linear-gradient(90deg,rgba(255,0,0,.5),transparent),linear-gradient(0deg,rgba(0,0,255,.5),transparent) white'],
 ['size-position','background:conic-gradient(red,blue) right 10px bottom 20px / 80px 60px no-repeat #eee'],
 ['repeat-x','background:linear-gradient(90deg,red,blue) 15px 30px / 50px 60px repeat-x #eee'],
 ['space','background:linear-gradient(90deg,red,blue) 20% 40% / 60px 45px space #eee'],
 ['round','background:linear-gradient(90deg,red,blue) 20% 40% / 65px 45px round #eee'],
 ['size-list','background-image:linear-gradient(90deg,red,transparent),linear-gradient(0deg,blue,transparent);background-color:yellow;background-size:50px 35px;background-position:left top,right bottom;background-repeat:no-repeat'],
 ['cascade-reset','background:conic-gradient(red,blue) 20px 30px / 50px 40px no-repeat;background:linear-gradient(red,blue)'],
 ['rounded','background:conic-gradient(from 20deg,red,yellow,lime,blue,red);border-radius:20px'],
];
extras.push(
 ['conic-calc','background:conic-gradient(from 100grad at 120% -10%,red calc(25% - 45deg),blue 450deg)'],
 ['radial-zero-width','background:radial-gradient(0px 30px,red 0px,blue 80px)'],
 ['radial-zero-height','background:radial-gradient(30px 0px,red,blue)'],
 ['zero-repeat','background:repeating-linear-gradient(red 0px,white 0px,blue 0px)'],
 ['hint-zero','background:linear-gradient(to right,red,0%,blue)'],
 ['hint-one','background:linear-gradient(to right,red,100%,blue)'],
 ['calc-size','background:radial-gradient(ellipse calc(50% - 10px) calc(40% + 5px),red,blue)'],
 ['round-zero','background:linear-gradient(90deg,red,blue) 0 0 / 65px 45px round #eee'],
 ['cover','background:linear-gradient(to right,transparent,blue),url("tile.svg") center / cover no-repeat yellow'],
 ['contain','background:url("tile.svg") right bottom / contain no-repeat yellow'],
 ['image-size','background:url("tile.svg") 10px 15px / 70px auto repeat-y yellow'],
 ['opacity','background:linear-gradient(90deg,red,transparent),conic-gradient(blue,transparent) yellow;opacity:.5']
);
for(const space of ['srgb','srgb-linear','display-p3','a98-rgb','prophoto-rgb','rec2020','xyz-d50','xyz-d65','lab','lch','oklab','oklch','hsl','hwb'])extras.push(['space-'+space,`background:linear-gradient(to right in ${space},#23568c,#cc6633)`]);
const tile='<svg xmlns="http://www.w3.org/2000/svg" width="120" height="60"><rect width="120" height="60" fill="#12ab56"/><rect x="60" width="60" height="60" fill="#8934cd"/></svg>';
fs.writeFileSync(path.join(out,'tile.svg'),tile);
const cases=[];
for(const space of ['default','srgb'])for(let i=1;i<=30;i++)cases.push({name:`open-${space}-${i}`,css:`background:var(--gradient-${i})`,space,w:240,h:160});
for(const [name,value] of extras)cases.push({name,css:value,space:'default',w:240,h:160});
function run(args){const r=cp.spawnSync(engine,args,{cwd:root,encoding:'utf8'});if(r.status!==0)throw Error(r.stderr||r.stdout);return r.stderr;}
(async()=>{
 const browser=await chromium.launch({headless:true,args:['--no-sandbox','--force-color-profile=srgb']});
 const results=[];
 try {
  const page=await browser.newPage({viewport:{width:640,height:480},deviceScaleFactor:1});
  let wasmPage;
  if(runtime){wasmPage=await browser.newPage();await wasmPage.goto(runtime);await wasmPage.evaluate(gpu=>window.gradientGPU=gpu,process.env.GRADIENT_GPU==='1');}
  for(const c of cases){
   if(process.env.GRADIENT_CASES&&!c.name.match(new RegExp(process.env.GRADIENT_CASES)))continue;
   const override=c.space==='srgb'?':where(html){--gradient-space:in srgb!important}':'';
   const declarations=c.css+';border-width:0';
   const browserDeclarations=declarations.replaceAll('url("tile.svg")',`url("data:image/svg+xml;base64,${Buffer.from(tile).toString('base64')}")`);
   await page.setContent(`<style>${css}${override}html,body{margin:0;background:white}#sample{box-sizing:border-box;width:${c.w}px;height:${c.h}px;${browserDeclarations}}</style><div id="sample"></div>`);
   const reference=await page.locator('#sample').evaluate(el=>({background:getComputedStyle(el).background,space:getComputedStyle(document.documentElement).getPropertyValue('--gradient-space')}));
   await page.locator('#sample').screenshot({path:path.join(out,c.name+'-css.png')});
   if(c.name==='calc-size'){
    // Keep the rejected original as the primary reference. This additional
    // image checks the same geometry with layout-resolved radii.
    await page.locator('#sample').evaluate((el,{w,h})=>{el.style.background=`radial-gradient(ellipse ${w*.5-10}px ${h*.4+5}px,red,blue)`;},c);
    await page.locator('#sample').screenshot({path:path.join(out,c.name+'-resolved-css.png')});
   }
   const source=`p=canvas(size=(${c.w*25.4/96}mm,${c.h*25.4/96}mm),stylesheet="@laymesh/open-props.css",background="#fff")\nstyle{${c.space==='srgb'?':root{--gradient-space:in srgb!important}':''}.sample{${declarations}}}\np.add(rect(size=(${c.w}px,${c.h}px),class="sample",border_width=0))\n`;
   const input=path.join(out,c.name+'.lay');fs.writeFileSync(input,source);
   const r={...(prior?.cases.find(p=>p.name===c.name&&p.css===c.css)||{}),...c,reference};
   if(native){
    r.diagnostics=run(['render',input,'-o',path.join(out,c.name+'-native.png'),'--dpi','96']);
    if(/Ignored (background|fill)/.test(r.diagnostics))throw Error(c.name+': '+r.diagnostics);
    if(process.env.GRADIENT_VECTORS==='1')for(const fmt of ['svg','pdf','pptx'])r[fmt+'Diagnostics']=run(['render',input,'-o',path.join(out,c.name+'.'+fmt),'--dpi',String(exportDpi)]);
   }
   if(wasmPage){
    const result=await wasmPage.evaluate(async ({source,w,h,tile})=>{
     if(!window.gradientHarness){
      const wasm=await import('./wasm/laymesh_wasm.js');await wasm.default();
      const skia=await import('./skia-renderer.mjs');const manifest=await(await fetch('./skia/manifest.json')).json();const ck=await skia.initializeSkia(new URL('./skia/'+manifest.wasm,location.href).href);
      window.gradientHarness={wasm,skia,ck};
     }
     const {wasm,skia,ck}=window.gradientHarness;
     const job=wasm.prepare_render(source,'/gradient.lay',JSON.stringify({'/tile.svg':tile})),draw=job.draw_job(),list=JSON.parse(draw.display_list());
     const canvas=document.createElement('canvas'),renderer=new skia.SkiaRenderer(ck,canvas,{software:!window.gradientGPU});document.body.replaceChildren(canvas);
     await renderer.install(list,id=>draw.resource_bytes(id),1);renderer.render([0,0,list.width,list.height],w,h);
     const pixels=Array.from(renderer.readPixels());
     const owned=renderer.owned.length;for(let n=0;n<30;n++)renderer.render([0,0,list.width,list.height],w,h);
     if(renderer.owned.length!==owned)throw Error('Repeated gradient draws leaked objects');
     const backend=renderer.backend,device=renderer.device;
     if(window.gradientGPU&&!backend.startsWith("Skia WebGL2"))throw Error("GPU test fell back to "+backend);
     renderer.dispose();draw.free();job.free();return {pixels,version:list.schemaVersion,backend,device};
    },{source,w:c.w,h:c.h,tile});
    fs.writeFileSync(path.join(out,c.name+'-canvaskit.rgba'),Buffer.from(result.pixels));r.displayVersion=result.version;r.backend=result.backend;r.device=result.device;
   }
   results.push(r);console.log(c.name);
  }
  const wasmProvenance=wasmPage?await wasmPage.evaluate(async()=>await(await fetch('./wasm/laymesh-build.json')).json()):undefined;
  fs.writeFileSync(path.join(out,'manifest.json'),JSON.stringify({...prior,browser:browser.version(),exportDpi:prior?.exportDpi??exportDpi,readbackDpi:96,librarySha256:crypto.createHash('sha256').update(css).digest('hex'),engineSha256:prior?.engineSha256??crypto.createHash('sha256').update(fs.readFileSync(engine)).digest('hex'),...(wasmProvenance?{wasmProvenance,wasmSha256:wasmProvenance.wasm_sha256}:{}),cases:results},null,2)+'\n');
 }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
