import { transitions } from './presentation-transitions-data.mjs';

// Shared decoder also handles the native preview's uncompressed RGBA assets.
export function decodeSkiaImage(ck, bytes, mime) {
  if (mime !== 'image/x-laymesh-rgba') return ck.MakeImageFromEncoded(bytes);
  if (!bytes || bytes.length < 8) throw Error('Invalid image header');
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const width = view.getUint32(0, true), height = view.getUint32(4, true);
  if (!width || !height || bytes.length !== 8 + width * height * 4) throw Error('Invalid image dimensions');
  return ck.MakeImage({width, height, colorType: ck.ColorType.RGBA_8888, alphaType: ck.AlphaType.Unpremul, colorSpace: ck.ColorSpace.SRGB}, bytes.subarray(8), width * 4);
}

export function transitionProgress(easing, x) {
  x = Math.max(0, Math.min(1, x));
  if (x === 0 || x === 1) return x;
  if (easing?.Steps) {
    const [n, kind] = easing.Steps;
    const start = ['start', 'jump-start', 'jump-both'].includes(kind);
    const count = n + (kind === 'jump-both' ? 1 : kind === 'jump-none' ? -1 : 0);
    return Math.max(0, Math.min(1, (Math.floor(x * n) + Number(start)) / count));
  }
  const [x1, y1, x2, y2] = easing?.Bezier || [0.42, 0, 0.58, 1];
  const at = (t, a, b) => 3 * (1-t)**2 * t * a + 3 * (1-t) * t*t*b + t**3;
  let lo=0, hi=1;
  for(let i=0;i<24;i++){const t=(lo+hi)/2;if(at(t,x1,x2)<x)lo=t;else hi=t;}
  return Math.max(0,Math.min(1,at((lo+hi)/2,y1,y2)));
}

// Snapshots and effects stay in the existing Skia context. No DOM or readback.
export class PageTransitions {
  constructor(skia) { this.skia=skia; this.effects=new Map(); this.warmed=new Set(); }
  capture(spec, reverse=false) {
    this.cancel();this.error=undefined;
    const s=this.skia;
    if(!spec||spec.name==='none'||!spec.durationMs||!s.surface||s.lost)return;
    try {
      this.pending={spec,reverse,from:s.surface.makeImageSnapshot(),extra:reverse?Object.fromEntries(Object.entries(spec.textures||{}).map(([name,id])=>[name,{bytes:s.imageBytes.get(id),mime:s.list?.resources?.[id]?.mime}])):null};
    } catch(error) { this.error=String(error); }
  }
  captureBitmap(spec,reverse,frame) {
    this.cancel();this.error=undefined;
    const s=this.skia,ck=s.ck,{bitmap}=frame;let from;
    try {
      if(!spec||spec.name==='none'||!spec.durationMs)return;
      if(s.gr)from=s.surface.makeImageFromTextureSource(bitmap);
      else {
        const canvas=new OffscreenCanvas(bitmap.width,bitmap.height),context=canvas.getContext('2d');context.drawImage(bitmap,0,0);
        from=ck.MakeImage({width:bitmap.width,height:bitmap.height,colorType:ck.ColorType.RGBA_8888,alphaType:ck.AlphaType.Unpremul,colorSpace:ck.ColorSpace.SRGB},context.getImageData(0,0,bitmap.width,bitmap.height).data,bitmap.width*4);
      }
      if(!from)throw Error('Presentation frame transfer failed');
      this.pending={spec,reverse,from,extra:frame.extra};
    } catch(error) {from?.delete();this.error=String(error);}
    finally {bitmap.close();}
  }
  navigate(list, navigation) {
    const previous=this.navigation;
    this.navigation=navigation;
    const changed=previous?.pageId&&navigation?.pageId&&previous.pageId!==navigation.pageId;
    const reverse=changed&&navigation.pageNumber<previous.pageNumber;
    const spec=reverse?this.skia.list?.pageTransition:list.pageTransition;
    this.capture(changed&&!navigation.reduced?spec:null,reverse);
  }
  cancel() {
    this.active?.finish(true);
    if(this.pending){this.pending.from?.delete();this.pending=null;}
  }
  effect(name) {
    if(this.effects.has(name))return this.effects.get(name);
    const entry=transitions[name];if(!entry)throw Error('Unknown page transition: '+name);
    let error='';
    const effect=this.skia.ck.RuntimeEffect.Make(entry.sksl,message=>error+=message);
    if(!effect)throw Error(error||'Page transition compilation failed');
    this.effects.set(name,effect);
    return effect;
  }
  retain(names) {
    for(const [name,effect]of this.effects)if(!names.has(name)){effect.delete();this.effects.delete(name);this.warmed.delete(name);}
  }
  warm(spec,image) {
    if(!spec||spec.name==='none'||!spec.durationMs||this.warmed.has(spec.name))return;
    const s=this.skia,ck=s.ck,entry=transitions[spec.name],effect=this.effect(spec.name),owned=[];
    const scratch=s.surface.makeSurface({width:1,height:1,colorType:ck.ColorType.RGBA_8888,alphaType:ck.AlphaType.Premul,colorSpace:ck.ColorSpace.SRGB});
    if(!scratch)return;
    try {
      const shaderFor=image=>{const shader=image.makeShaderOptions(ck.TileMode.Clamp,ck.TileMode.Clamp,ck.FilterMode.Linear,ck.MipmapMode.None);owned.push(shader);return shader;};
      const input=shaderFor(image),children=[input,input];
      for(const name of entry.textures||[]){const texture=s.images.get(spec.textures?.[name]);if(!texture)throw Error('Missing transition texture: '+name);children.push(shaderFor(texture));}
      const uniforms=new Float32Array(effect.getUniformFloatCount()),values={...entry.defaultParams,...spec.params,_resolution:[image.width(),image.height()],ratio:image.width()/image.height(),progress:.5};
      for(let i=0;i<effect.getUniformCount();i++){const u=effect.getUniform(i);[].concat(values[effect.getUniformName(i)]??0).forEach((v,j)=>uniforms[u.slot+j]=Number(v));}
      const shader=effect.makeShaderWithChildren(uniforms,children);if(!shader)throw Error('Transition shader creation failed');owned.push(shader);
      const paint=new ck.Paint();owned.push(paint);paint.setShader(shader);scratch.getCanvas().drawPaint(paint);scratch.flush();this.warmed.add(spec.name);
    }finally{for(const object of owned.reverse())object.delete();scratch.dispose();}
  }
  async play(reduced=false,onVisible=()=>{}) {
    if(reduced){this.cancel();onVisible();return {};}
    const pending=this.pending;this.pending=null;
    if(!pending){onVisible();return this.error?{transitionError:this.error}:{};}
    const s=this.skia,ck=s.ck,{spec,reverse,from}=pending,owned=[from];
    let timer,finish;
    const request=globalThis.requestAnimationFrame||((fn)=>setTimeout(()=>fn(performance.now()),16));
    const cancel=globalThis.cancelAnimationFrame||clearTimeout;
    try {
      if(s.lost||!s.surface)throw Error('Page transition surface unavailable');
      const surface=s.surface,displayWidth=s.canvas.width,displayHeight=s.canvas.height;
      const to=surface.makeImageSnapshot();owned.push(to);
      // Transitions use the same pixel dimensions as the displayed artwork.
      const width=displayWidth,height=displayHeight,output=surface;
      const shaderFor=(image,contain=false)=>{
        const sx=width/image.width(),sy=height/image.height(),scale=Math.min(sx,sy);
        const matrix=contain?[scale,0,(width-image.width()*scale)/2,0,scale,(height-image.height()*scale)/2,0,0,1]:[sx,0,0,0,sy,0,0,0,1];
        const tile=contain&&Math.abs(sx-sy)>1e-6?ck.TileMode.Decal:ck.TileMode.Clamp;
        const shader=image.makeShaderOptions(tile,tile,ck.FilterMode.Linear,ck.MipmapMode.None,matrix);
        owned.push(shader);return shader;
      };
      const a=shaderFor(reverse?to:from,true),b=shaderFor(reverse?from:to,true);
      const entry=transitions[spec.name],effect=this.effect(spec.name),children=[a,b];
      for(const name of entry.textures||[]){
        let image=reverse?null:s.images.get(spec.textures?.[name]);
        if(reverse){image=decodeSkiaImage(ck,pending.extra[name].bytes,pending.extra[name].mime);if(image)owned.push(image);}
        if(!image)throw Error('Missing transition texture: '+name);
        children.push(shaderFor(image));
      }
      // CanvasKit casts numeric float slots to integer uniforms in its binding.
      const uniforms=new Float32Array(effect.getUniformFloatCount());
      const values={...entry.defaultParams,...spec.params,_resolution:[width,height],ratio:width/height,progress:0};
      let progressSlot;
      for(let i=0;i<effect.getUniformCount();i++){
        const name=effect.getUniformName(i),u=effect.getUniform(i);
        if(name==='progress')progressSlot=u.slot;
        const dest=uniforms;
        [].concat(values[name]??0).forEach((v,j)=>dest[u.slot+j]=Number(v));
      }
      const paint=new ck.Paint();owned.push(paint);
      const canvas=output.getCanvas();
      // Effects may synthesize pixels outside both input snapshots (page-curl
      // shadows, for example). The transition shares the page's output clip.
      const clip=(c,sx=1,sy=1)=>{if(s.pageBounds)c.clipRect(s.pageBounds.map((v,i)=>v*(i%2?sy:sx)),ck.ClipOp.Intersect,false);};
      const draw=(p)=>{
        uniforms[progressSlot]=p;
        // Guarantee exact endpoint frames, including transparent artwork.
        const shader=p<=0?a:p>=1?b:effect.makeShaderWithChildren(uniforms,children);
        if(!shader)throw Error('Transition shader creation failed');
        try {canvas.clear(ck.TRANSPARENT);canvas.save();try{clip(canvas,width/displayWidth,height/displayHeight);paint.setShader(shader);canvas.drawPaint(paint);}finally{canvas.restore();}output.flush();s.onPresent?.();}
        finally {if(shader!==a&&shader!==b)shader.delete();}
      };
      // Compile the first shader pipeline offscreen before starting the clock.
      // Software Skia can spend hundreds of milliseconds on this once per effect.
      if(!this.warmed.has(spec.name)) {
        const scratch=surface.makeSurface({width:1,height:1,colorType:ck.ColorType.RGBA_8888,alphaType:ck.AlphaType.Premul,colorSpace:ck.ColorSpace.SRGB});
        if(scratch){
          uniforms[progressSlot]=.5;
          let shader;
          try{shader=effect.makeShaderWithChildren(uniforms,children);if(!shader)throw Error('Transition shader creation failed');paint.setShader(shader);scratch.getCanvas().drawPaint(paint);scratch.flush();this.warmed.add(spec.name);}
          finally{shader?.delete();scratch.dispose();}
        }
      }
      let frames=0,visible=false,cancelled=false,started;
      await new Promise(resolve=>{
        finish=(wasCancelled=false)=>{if(!finish)return;finish=null;cancel(timer);cancelled=wasCancelled;this.active=null;
          if(!this.disposed&&!s.lost&&s.surface===surface){try{draw(reverse?0:1);}catch(error){this.error=String(error);}}
          resolve();};
        this.active={finish:(value)=>finish?.(value)};
        const tick=()=>{
          // Worker RAF timestamps can use the document clock. Use one clock
          // for both ends of the elapsed-time calculation in every browser.
          const now=performance.now();
          if(!finish)return;
          try{
            if(s.lost||s.surface!==surface){finish(true);return;}
            const t=Math.min(1,(now-started)/spec.durationMs),p=transitionProgress(spec.easing,t);
            draw(reverse?1-p:p);if(!visible&&t>0){visible=true;onVisible();}frames++;
            if(t>=1)finish();else timer=request(tick);
          }catch(error){this.error=String(error);finish(true);}
        };
        draw(reverse?1:0);started=performance.now();timer=request(tick);
      });
      if(this.error)console.warn('LayMesh page transition:',this.error);
      return {transitionName:spec.name,transitionMs:performance.now()-started,transitionFrames:frames,transitionRasterSize:[width,height],transitionCancelled:cancelled,transitionError:this.error};
    } catch(error) {
      this.error=String(error);console.warn('LayMesh page transition:',this.error);
      return {transitionError:this.error};
    } finally {
      finish?.(true);
      for(const object of owned.reverse())object?.delete();
    }
  }
  dispose(){this.disposed=true;this.cancel();for(const effect of this.effects.values())effect.delete();this.effects.clear();this.warmed.clear();}
}
