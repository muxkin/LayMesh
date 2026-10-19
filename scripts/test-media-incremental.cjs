// Media invalidation through the real retained native presentation compiler.
const assert=require('node:assert/strict'),fs=require('node:fs'),os=require('node:os'),path=require('node:path'),Module=require('node:module');
const load=Module._load;Module._load=function(id,...args){return id==='vscode'?{}:load.call(this,id,...args);};
const {PreviewWorker}=require('../extensions/vscode/src/preview-host.cjs');Module._load=load;
const {NativePresentation,NativePresentationCache}=require('../extensions/vscode/src/presentation-host.cjs');
// Date.now() loses the fractional millisecond. A file written before prepare
// in that same clock tick must not be mistaken for a concurrent edit.
const statSync=fs.statSync,clock=Object.create(NativePresentation.prototype);
Object.assign(clock,{overlays:{},startedAt:1000,signatures:new Map()});
let stamp=1000.75;fs.statSync=()=>({ctimeMs:stamp,mtimeMs:stamp,size:10});
try{
 assert.doesNotThrow(()=>clock.record('same-tick.mp4'));
 stamp=1000.875;assert.throws(()=>clock.record('same-tick.mp4'),/Source changed/);
 stamp=1001.125;assert.throws(()=>clock.record('later-tick.mp4'),/Source changed/);
}finally{fs.statSync=statSync;}
(async()=>{const dir=fs.mkdtempSync(path.join(os.tmpdir(),'laymesh-media-incremental-')),file=path.join(dir,'deck.laydoc');
 const original='d=document()\nd.play(audio(src="music.wav"),volume=0.2,loop=true)\nd.add(page(src="first.lay",id="first"))\nd.add(page(src="second.lay",id="second"))';
 for(const name of ['first.lay','second.lay','demo.mp4','music.wav'])fs.copyFileSync(path.join(__dirname,'../examples/media',name),path.join(dir,name));fs.writeFileSync(file,original);
 const cache=new NativePresentationCache(()=>new PreviewWorker(path.resolve(process.env.LAYMESH_TEST_BINARY||'target/debug/laymesh'),{append(){},appendLine(){}},dir));let deck;
 async function run(source){deck?.dispose();deck=new NativePresentation({worker:cache.worker(),createWorker:()=>cache.worker(cache.workers.length),cache,dist:dir,uri:f=>f,notify:m=>{if(m.type==='deck-page'||m.type==='deck-page-reuse')queueMicrotask(()=>deck.ack(m.page.id));}});await deck.prepare(file,source,{},{});assert(deck.pages.every(p=>p.status==='ready'),JSON.stringify(deck.pages.map(p=>({id:p.id,status:p.status,error:p.error}))));return JSON.parse(fs.readFileSync(deck.pages[0].file)).list;}
 try{let first=await run(original);assert.equal(first.media.cues[0].volume,.2);await run(original);assert.equal(deck.preparation.compiledPages,0);
  const revised=original.replace('volume=0.2','volume=0.6');let changed=await run(revised);assert.equal(deck.preparation.compiledPages,2);assert.equal(changed.media.cues[0].volume,.6);
  const video=changed.media.cues.find(c=>c.target).asset;fs.copyFileSync(path.join(__dirname,'../examples/media/controls.mp4'),path.join(dir,'demo.mp4'));changed=await run(revised);assert.notEqual(changed.media.cues.find(c=>c.target).asset,video);assert.equal(deck.preparation.compiledPages,1);assert.equal(deck.preparation.reusedPages,1);
  const audio=changed.media.cues[0].asset,data=fs.readFileSync(path.join(dir,'music.wav'));data[data.length-1]^=1;fs.writeFileSync(path.join(dir,'music.wav'),data);changed=await run(revised);assert.notEqual(changed.media.cues[0].asset,audio);assert.equal(deck.preparation.compiledPages,2);
  console.log('Native retained media: zero unchanged compilation; music settings/file and video replacement invalidate the right pages');
 }finally{deck?.dispose();cache.dispose();fs.rmSync(dir,{recursive:true,force:true});}
})().catch(e=>{console.error(e);process.exitCode=1;});
