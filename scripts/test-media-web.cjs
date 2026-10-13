// Real browser media decoding, painting, manual waits and cross-page audio.
const assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path');
const {pathToFileURL}=require('node:url');
const playwright=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const file=path.resolve(process.argv[2]||'target/media-trial/deck.zip.html');
const out=path.resolve(process.argv[3]||'/tmp/laymesh-media-browser');fs.mkdirSync(out,{recursive:true});
const state=()=>{const p=window.__LAYMESH_WEB.presentation,m=p.media;return {page:p.index,time:p.clock.time,running:p.clock.running,tracks:m.tracks.map(t=>({scope:t.cue.scope,target:t.cue.target,started:t.started,paused:t.element.paused,time:t.element.currentTime,soundTime:t.sound?.currentTime})),frames:p.metrics.frames};};
(async()=>{
 const results=[];
 for(const name of ['chromium','firefox','webkit']){
  console.log("Testing",name);
  const browser=await playwright[name].launch({headless:true,env:{...process.env,XDG_RUNTIME_DIR:process.env.XDG_RUNTIME_DIR||'/run/user/'+process.getuid(),PULSE_SERVER:process.env.PULSE_SERVER||'unix:/run/user/'+process.getuid()+'/pulse/native'},...(name==='chromium'?{args:['--no-sandbox']}: {})});
  try{
   const page=await browser.newPage({viewport:{width:1280,height:900}}),errors=[];
   page.on('pageerror',e=>errors.push(e.message));page.on('console',m=>{if(m.type()==='error')errors.push(m.text());});
   await page.goto(pathToFileURL(file).href);await page.waitForFunction(()=>window.__LAYMESH_WEB?.media?.tracks.length===3);
   assert.equal(await page.title(),'Media experiment');assert((await page.locator('body').innerText()).includes('LayMesh'));
   assert.equal(await page.evaluate(()=>window.__LAYMESH_WEB.clock.running),false,'Static layout must wait for explicit playback');
   await page.getByRole('button',{name:'播放',exact:true}).click();await page.waitForFunction(()=>window.__LAYMESH_WEB.media.tracks.every(t=>t.started&&!t.element.paused));
   await page.waitForTimeout(800);assert(await page.evaluate(()=>window.__LAYMESH_WEB.metrics.frames>=3),JSON.stringify({browser:name,errors,metrics:await page.evaluate(()=>window.__LAYMESH_WEB.metrics),text:(await page.locator("body").innerText()).slice(-500)}));
   await page.getByRole('button',{name:'暂停',exact:true}).click();assert(await page.evaluate(()=>window.__LAYMESH_WEB.media.tracks.every(t=>t.element.paused)));
   await page.evaluate(()=>window.__LAYMESH_WEB.presentation.open({fullscreen:false}));await page.waitForFunction(()=>window.__LAYMESH_WEB.presentation.ready);await page.waitForFunction(()=>window.__LAYMESH_WEB.presentation.media.tracks.every(t=>t.started&&!t.element.paused));
   await page.waitForTimeout(1000);let playing=await page.evaluate(state);assert(playing.frames>=3,JSON.stringify({name,playing,errors,metrics:await page.evaluate(()=>window.__LAYMESH_WEB.presentation.metrics)}));assert(playing.time>.2,JSON.stringify({name,playing,debug:await page.evaluate(()=>({hidden:document.hidden,menu:window.__LAYMESH_WEB.presentation.menu.hidden,clock:window.__LAYMESH_WEB.presentation.clock,reduced:window.__LAYMESH_WEB.presentation.reduced.matches,switching:window.__LAYMESH_WEB.presentation.switching,external:!!window.__LAYMESH_WEB.presentation.externalMenu,ready:window.__LAYMESH_WEB.presentation.ready,message:window.__LAYMESH_WEB.presentation.message.textContent,tracks:window.__LAYMESH_WEB.presentation.media.tracks.map(t=>({mime:t.asset.mime,error:t.element.error?.message,soundError:t.sound?.error?.message}))}))}));
   const video=playing.tracks.find(t=>t.target);assert(video.soundTime==null||Math.abs(video.time-video.soundTime)<.06,'Video and its compatible audio stay synchronized');
   await page.screenshot({path:path.join(out,name+'-playing.png')});
   // Step clock can wait while the independent media clock keeps playing.
   await page.evaluate(()=>{const p=window.__LAYMESH_WEB.presentation;p.clock.running=false;p.media.seek(.2);p.media.resume();});
   await page.waitForFunction(()=>{const t=window.__LAYMESH_WEB.presentation.media.tracks.find(t=>t.cue.target);return !t.element.seeking&&t.element.currentTime<.5;});
   const before=await page.evaluate(state);await page.waitForTimeout(250);const after=await page.evaluate(state);assert.equal(after.time,before.time);assert(after.tracks.find(t=>t.target).time>before.tracks.find(t=>t.target).time+.1,JSON.stringify({name,before,after}));
   await page.keyboard.press('s');assert((await page.evaluate(state)).tracks.every(t=>t.paused));
   await page.keyboard.press('s');await page.waitForTimeout(80);
   await page.evaluate(()=>{const p=window.__LAYMESH_WEB.presentation;p.media.oldBackground=p.media.tracks.find(t=>t.cue.scope==='document');return p.go(1);});await page.waitForFunction(()=>window.__LAYMESH_WEB.presentation.index===1&&!window.__LAYMESH_WEB.presentation.switching);
   const continuous=await page.evaluate(()=>{const m=window.__LAYMESH_WEB.presentation.media;return {count:m.tracks.length,same:m.tracks[0]===m.oldBackground,paused:m.tracks[0].element.paused};});assert.equal(continuous.count,1);assert(continuous.same&&!continuous.paused);
   await page.keyboard.press('Escape');assert.equal(await page.evaluate(()=>window.__LAYMESH_WEB.presentation.opened),false);
   const clickFile=path.join(path.dirname(file),'click.zip.html');
   await page.goto(pathToFileURL(clickFile).href);await page.waitForFunction(()=>window.__LAYMESH_WEB?.media?.tracks[0]?.element.readyState>=2&&window.__LAYMESH_WEB.preview.presented);
   const point=await page.evaluate(()=>{const r=window.__LAYMESH_WEB.preview.canvas.getBoundingClientRect(),v=window.__LAYMESH_WEB.preview.request().viewport;return {x:r.x+(50-v[0])/(v[2]-v[0])*r.width,y:r.y+(27-v[1])/(v[3]-v[1])*r.height};});
   await page.mouse.click(point.x,point.y);await page.waitForFunction(()=>{const t=window.__LAYMESH_WEB.media.tracks[0];return t.started&&!t.element.paused&&t.element.currentTime>.6;});
   await page.mouse.click(point.x,point.y);await page.waitForFunction(()=>{const t=window.__LAYMESH_WEB.media.tracks[0];return t.localPaused&&t.element.paused;});
   const editableFile=path.join(path.dirname(file),'editable.zip.html');let editable=false;
   if(name==='chromium'&&fs.existsSync(editableFile)){
    await page.goto(pathToFileURL(editableFile).href);await page.waitForFunction(()=>window.__LAYMESH_WEB?.editor&&window.__LAYMESH_WEB.workspace);
    await page.evaluate(async()=>{const w=window.__LAYMESH_WEB,file=Object.keys(w.editor.files).find(p=>p.endsWith('/second.lay'));await w.workspace.open(file);const view=w.editor.views.get(file).view;view.dispatch({changes:{from:0,to:view.state.doc.length,insert:w.editor.files[file]+'\nv=p.add(video(src="demo.mp4"),size=(80mm,45mm),offset=(10mm,5mm))\np.play(v,trim_start=500ms,trim_end=1500ms)\np.play(audio(src="music.wav"),volume=0.1)\n'}});});
    await page.waitForFunction(()=>window.__LAYMESH_WEB.pages[1].list.media?.cues.length===3,{},{timeout:60000});
    await page.locator('#page').selectOption('1');await page.waitForFunction(()=>window.__LAYMESH_WEB.media.tracks.length===3);
    await page.locator('#play').click();await page.waitForFunction(()=>window.__LAYMESH_WEB.media.tracks.every(t=>t.started&&!t.element.paused));
    assert(await page.evaluate(()=>window.__LAYMESH_WEB.media.tracks.find(t=>t.cue.target).cue.trimStart===.5));editable=true;
   }
   assert.deepEqual(errors,[]);results.push({browser:name,checks:'static, play, pause, frames, wait, A/V, cross-page music, exit, video click',editable,passed:true});
  }finally{await browser.close();}
 }
 fs.writeFileSync(path.join(out,'results.json'),JSON.stringify(results,null,2));console.log(JSON.stringify(results));
})().catch(e=>{console.error(e);process.exitCode=1;});
