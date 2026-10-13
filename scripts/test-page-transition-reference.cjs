// Compare upstream GLSL with the exact vendored SkSL, on the same browser GPU.
const fs=require('fs'),path=require('path'),http=require('http'),assert=require('assert/strict');
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const root=path.resolve(__dirname,'..'),out=process.argv[2]||'/tmp/laymesh-transition-qa';fs.mkdirSync(out,{recursive:true});
const host=process.env.LAYMESH_TEST_HOST||'100.64.0.6';
const server=http.createServer((req,res)=>{let p=path.join(root,new URL(req.url,'http://local').pathname);if(p===root+'/'){res.end('<body></body>');return;}try{res.setHeader('Content-Type',p.endsWith('.wasm')?'application/wasm':(/\.m?js$/).test(p)?'text/javascript':'application/json');res.end(fs.readFileSync(p));}catch{res.statusCode=404;res.end();}});
(async()=>{await new Promise(r=>server.listen(0,host,r));const browser=await chromium.launch({headless:true,args:['--enable-unsafe-swiftshader']});try{
const page=await browser.newPage();await page.goto(`http://${host}:${server.address().port}/`);
const report=await page.evaluate(async()=>{
  const script=document.createElement('script');script.src='/vendor/canvaskit/canvaskit.js';document.head.append(script);await new Promise((r,j)=>{script.onload=r;script.onerror=j;});
  const ck=await CanvasKitInit({locateFile:()=>'/vendor/canvaskit/canvaskit.wasm'}),{transitions}=await import('/site/live/presentation-transitions-data.mjs'),catalog=await(await fetch('/vendor/gl-transitions/gl-transitions.json')).json();
  const w=128,h=72,skCanvas=document.createElement('canvas');skCanvas.width=w;skCanvas.height=h;document.body.append(skCanvas);
  const surface=ck.MakeWebGLCanvasSurface(skCanvas);if(!surface)throw Error('CanvasKit WebGL unavailable');
  const glCanvas=document.createElement('canvas');glCanvas.width=w;glCanvas.height=h;const gl=glCanvas.getContext('webgl2',{premultipliedAlpha:false,preserveDrawingBuffer:true});
  const debug=gl.getExtension('WEBGL_debug_renderer_info'),device=debug?gl.getParameter(debug.UNMASKED_RENDERER_WEBGL):'unknown';
  const pixels=target=>{const a=new Uint8Array(w*h*4);for(let y=0;y<h;y++)for(let x=0;x<w;x++){const i=(y*w+x)*4;a[i]=target?160+x*.6:40+x*.1;a[i+1]=60+y*1.6;a[i+2]=target?40:200;a[i+3]=255;}return a;};
  const from=pixels(false),to=pixels(true),info={width:w,height:h,colorType:ck.ColorType.RGBA_8888,alphaType:ck.AlphaType.Unpremul,colorSpace:ck.ColorSpace.SRGB};
  const images=[from,to,to].map(p=>ck.MakeImage(info,p,w*4)),children=images.map(im=>im.makeShaderOptions(ck.TileMode.Clamp,ck.TileMode.Clamp,ck.FilterMode.Linear,ck.MipmapMode.None));
  for(let n=0;n<3;n++){gl.activeTexture(gl.TEXTURE0+n);const tex=gl.createTexture();gl.bindTexture(gl.TEXTURE_2D,tex);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MIN_FILTER,gl.LINEAR);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MAG_FILTER,gl.LINEAR);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_WRAP_S,gl.CLAMP_TO_EDGE);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_WRAP_T,gl.CLAMP_TO_EDGE);const bytes=[from,to,to][n],flipped=new Uint8Array(bytes.length);for(let y=0;y<h;y++)flipped.set(bytes.subarray(y*w*4,(y+1)*w*4),(h-1-y)*w*4);gl.texImage2D(gl.TEXTURE_2D,0,gl.RGBA,w,h,0,gl.RGBA,gl.UNSIGNED_BYTE,flipped);}
  const shader=(type,code)=>{const s=gl.createShader(type);gl.shaderSource(s,code);gl.compileShader(s);if(!gl.getShaderParameter(s,gl.COMPILE_STATUS))throw Error(gl.getShaderInfoLog(s));return s;};
  const vertex=shader(gl.VERTEX_SHADER,'#version 300 es\nin vec2 position;out vec2 uv;void main(){uv=position*.5+.5;gl_Position=vec4(position,0.,1.);}');
  const buffer=gl.createBuffer();gl.bindBuffer(gl.ARRAY_BUFFER,buffer);gl.bufferData(gl.ARRAY_BUFFER,new Float32Array([-1,-1,1,-1,-1,1,-1,1,1,-1,1,1]),gl.STATIC_DRAW);
  const rows=[];
  for(const item of catalog){const row={name:item.name};try{
    const code='#version 300 es\nprecision highp float;precision highp int;in vec2 uv;out vec4 _result;uniform sampler2D fromTex,toTex;uniform float progress,ratio;vec4 getFromColor(vec2 p){return texture(fromTex,p);}vec4 getToColor(vec2 p){return texture(toTex,p);}\n'+item.glsl.replaceAll('texture2D(','texture(')+'\nvoid main(){_result=transition(uv);}';
    const frag=shader(gl.FRAGMENT_SHADER,code),program=gl.createProgram();gl.attachShader(program,vertex);gl.attachShader(program,frag);gl.linkProgram(program);if(!gl.getProgramParameter(program,gl.LINK_STATUS))throw Error(gl.getProgramInfoLog(program));gl.useProgram(program);
    const position=gl.getAttribLocation(program,'position');gl.enableVertexAttribArray(position);gl.vertexAttribPointer(position,2,gl.FLOAT,false,0,0);gl.uniform1i(gl.getUniformLocation(program,'fromTex'),0);gl.uniform1i(gl.getUniformLocation(program,'toTex'),1);gl.uniform1f(gl.getUniformLocation(program,'ratio'),w/h);
    for(const t of item.textures||[])gl.uniform1i(gl.getUniformLocation(program,t),2);
    for(const [name,kind]of Object.entries(item.paramsTypes)){const value=item.defaultParams[name],location=gl.getUniformLocation(program,name);if(location===null)continue;const n=kind.includes('vec')?Number(kind.at(-1)):1,integer=kind==='bool'||kind==='int'||kind.startsWith('ivec');gl[`uniform${n}${integer?'i':'f'}v`](location,[].concat(value).map(Number));}
    const effect=ck.RuntimeEffect.Make(transitions[item.name].sksl),uniforms=new Float32Array(effect.getUniformFloatCount()),ints=new Int32Array(uniforms.buffer),values={...item.defaultParams,_resolution:[w,h],ratio:w/h};let progress;
    for(let i=0;i<effect.getUniformCount();i++){const name=effect.getUniformName(i),u=effect.getUniform(i);if(name==='progress')progress=u.slot;else [].concat(values[name]??0).forEach((v,j)=>uniforms[u.slot+j]=Number(v));}
    const paint=new ck.Paint(),frames=[];
    for(const t of [.1,.25,.5,.75,.9]){uniforms[progress]=t;const shader=effect.makeShaderWithChildren(uniforms,children.slice(0,2+(item.textures||[]).length));paint.setShader(shader);surface.getCanvas().clear(ck.TRANSPARENT);surface.getCanvas().drawPaint(paint);surface.flush();const snapshot=surface.makeImageSnapshot(),sk=snapshot.readPixels(0,0,info);snapshot.delete();shader.delete();
      gl.uniform1f(gl.getUniformLocation(program,'progress'),t);gl.drawArrays(gl.TRIANGLES,0,6);const ref=new Uint8Array(w*h*4);gl.readPixels(0,0,w,h,gl.RGBA,gl.UNSIGNED_BYTE,ref);let sum=0,bad=0;for(let y=0;y<h;y++)for(let x=0;x<w;x++)for(let k=0;k<4;k++){const d=Math.abs(sk[(y*w+x)*4+k]-ref[((h-1-y)*w+x)*4+k]);sum+=d;if(d>16)bad++;}frames.push({t,mae:sum/ref.length,differentFraction:bad/ref.length});
    }
    row.frames=frames;row.maxMAE=Math.max(...frames.map(f=>f.mae));paint.delete();effect.delete();gl.deleteProgram(program);gl.deleteShader(frag);
  }catch(e){row.error=String(e);}rows.push(row);}
  children.forEach(x=>x.delete());images.forEach(x=>x.delete());surface.dispose();return {device,environment:'Headless Chromium WebGL2; upstream GLSL reference and vendored SkSL, opaque gradients, clamp sampling',rows};
});fs.writeFileSync(out+'/reference.json',JSON.stringify(report,null,2));console.log(JSON.stringify({device:report.device,errors:report.rows.filter(r=>r.error),differences:report.rows.filter(r=>r.maxMAE>8).map(r=>({name:r.name,maxMAE:r.maxMAE}))},null,2));
assert.equal(report.rows.filter(r=>r.error||r.maxMAE>8).length,0,'Upstream visual compatibility failed; do not publish failed effects');
}finally{await browser.close();server.close();}})().catch(e=>{console.error(e);server.close();process.exitCode=1;});
