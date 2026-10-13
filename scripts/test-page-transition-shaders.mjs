// Deterministic CanvasKit CPU compatibility sweep. No claim of hardware GPU QA.
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {fileURLToPath} from 'node:url';
import {PageTransitions, transitionProgress} from '../site/live/presentation-transitions.mjs';
import {transitions} from '../site/live/presentation-transitions-data.mjs';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const out=path.resolve(process.argv[2]||'/tmp/laymesh-transition-qa');fs.mkdirSync(out,{recursive:true});
const require=createRequire(import.meta.url),ck=await require('../vendor/canvaskit/canvaskit.js')({locateFile:()=>root+'/vendor/canvaskit/canvaskit.wasm'});
const realNow=performance.now.bind(performance);let clock=0,nextFrame;
globalThis.performance={now:()=>clock};
globalThis.requestAnimationFrame=callback=>{nextFrame=callback;return 1;};
globalThis.cancelAnimationFrame=()=>{nextFrame=null;};
const rgba=(surface)=>{const image=surface.makeImageSnapshot();try{return image.readPixels(0,0,{width:image.width(),height:image.height(),colorType:ck.ColorType.RGBA_8888,alphaType:ck.AlphaType.Unpremul,colorSpace:ck.ColorSpace.SRGB});}finally{image.delete();}};
function artwork(surface,width,height,target,transparent){
  const c=surface.getCanvas(),p=new ck.Paint();
  c.clear(transparent?ck.TRANSPARENT:target?ck.Color(242,223,200):ck.Color(207,224,246));
  for(let y=0;y<4;y++)for(let x=0;x<6;x++){
    p.setColor(target?ck.Color(150+x*15,45+y*25,40,transparent?.7:1):ck.Color(30+x*10,65+y*25,180,transparent?.8:1));
    c.drawRect([x*width/6+2,y*height/4+2,(x+1)*width/6-2,(y+1)*height/4-2],p);
  }
  p.delete();surface.flush();
}
const sheet=ck.MakeSurface(1000,Math.ceil(Object.keys(transitions).length/8)*91),sheetCanvas=sheet.getCanvas();
sheetCanvas.clear(ck.WHITE);const labelPaint=new ck.Paint();labelPaint.setColor(ck.BLACK);
const face=ck.Typeface.MakeFreeTypeFaceFromData(fs.readFileSync(root+'/tests/fonts/DejaVuSans.ttf')),font=new ck.Font(face,8);
const rows=[];let index=0;
for(const [name,entry]of Object.entries(transitions)){
  const started=realNow(),row={name,variants:0,frames:0,endpoints:true,intermediateChanges:true,error:null};
  try{
    for(const [width,height,transparent,variant]of [[128,72,false,false],[72,128,false,false],[96,96,true,false],[128,72,false,true]]){
      const surface=ck.MakeSurface(width,height),skia={ck,surface,canvas:{width,height},budget:256*1048576,images:new Map(),imageBytes:new Map()},player=new PageTransitions(skia);
      const texture=ck.MakeSurface(width,height);artwork(texture,width,height,true,false);const tex=texture.makeImageSnapshot();
      const encoded=tex.encodeToBytes();for(const t of entry.textures||[]){skia.images.set(t,tex);skia.imageBytes.set(t,encoded);}
      const spec={name,durationMs:100,easing:{Bezier:[0,0,1,1]},params:{...entry.defaultParams},textures:Object.fromEntries((entry.textures||[]).map(t=>[t,t]))};
      if(variant)for(const [key,value]of Object.entries(spec.params)){
        const kind=entry.paramsTypes[key];spec.params[key]=typeof value==='boolean'?!value:Array.isArray(value)?value.map(v=>kind.startsWith('ivec')?v+1:v*.85):kind==='int'?value+1:value*.85;
      }
      artwork(surface,width,height,false,transparent);const from=rgba(surface);player.capture(spec);
      artwork(surface,width,height,true,transparent);const to=rgba(surface);clock=0;const promise=player.play();
      assert.deepEqual(rgba(surface),from,name+' start');
      const frames=[];
      for(const t of [10,25,50,75,90,100]){clock=t;const callback=nextFrame;nextFrame=null;assert.ok(callback,name+' animation callback');callback(clock);frames.push(rgba(surface));}
      const result=await promise;assert.ok(!result.transitionError,result.transitionError);assert.deepEqual(rgba(surface),to,name+' end');
      assert.ok(frames.some(frame=>!Buffer.from(frame).equals(Buffer.from(from))&&!Buffer.from(frame).equals(Buffer.from(to))),name+' has intermediate frames');
      if(!variant&&!transparent&&width===128){
        // Re-render the same effect at its midpoint for the human review sheet.
        artwork(surface,width,height,false,false);player.capture(spec);artwork(surface,width,height,true,false);clock=0;const preview=player.play();clock=50;nextFrame(clock);
        const image=surface.makeImageSnapshot(),x=index%8*125,y=Math.floor(index/8)*91;
        sheetCanvas.drawImageRect(image,[0,0,width,height],[x,y,x+123,y+70],null);image.delete();
        sheetCanvas.drawText(name,x+2,y+83,labelPaint,font);player.cancel();await preview;
      }
      // The same boundary plays backward with the exact opposite endpoints.
      player.capture(spec,true);artwork(surface,width,height,false,transparent);clock=0;const backwards=player.play();assert.deepEqual(rgba(surface),to,name+' reverse start');clock=100;nextFrame(clock);await backwards;assert.deepEqual(rgba(surface),from,name+' reverse end');
      assert.equal(player.pending,null);assert.equal(player.active,null);
      row.variants++;row.frames+=frames.length+4;player.dispose();tex.delete();texture.dispose();surface.dispose();
    }
  }catch(error){row.error=String(error);}
  row.cpuMs=realNow()-started;rows.push(row);index++;
}
sheet.flush();const image=sheet.makeImageSnapshot();fs.writeFileSync(out+'/midpoints.png',image.encodeToBytes());image.delete();font.delete();face.delete();labelPaint.delete();sheet.dispose();
assert.equal(transitionProgress({Steps:[4,'jump-end']},.3),.25);
const report={backend:'CanvasKit 0.42.0 CPU (WASM), synthetic 128×72 / 72×128 / transparent 96×96; not hardware GPU',version:'1.74.0',count:rows.length,passed:rows.filter(r=>!r.error).length,rows};
fs.writeFileSync(out+'/compatibility.json',JSON.stringify(report,null,2)+'\n');
console.log(JSON.stringify({count:report.count,passed:report.passed,failures:rows.filter(r=>r.error),output:out},null,2));
assert.equal(report.passed,report.count);
