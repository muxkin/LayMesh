const fs=require('fs'),os=require('os'),path=require('path'),assert=require('assert/strict'),Module=require('module');
const original=Module._load;Module._load=function(id,...a){return id==='vscode'?{}:original.call(this,id,...a);};const {PreviewWorker}=require('../extensions/vscode/src/preview-host.cjs');Module._load=original;
const {NativePresentation}=require('../extensions/vscode/src/presentation-host.cjs');
const root=path.resolve(__dirname,'..'),out=process.argv[2]||'/tmp/laymesh-progressive/native';fs.mkdirSync(out,{recursive:true});
const create=()=>{const w=new PreviewWorker(path.join(root,'target/release/laymesh'),{append(){} });w.background=true;return w;};
const file=path.join(root,'tests/presentation/academic/report.laydoc'),source=fs.readFileSync(file,'utf8');
(async()=>{const rows=[],preview=create();preview.background=false;let previewBusy=false,activeDeck;
 const current=()=>new Promise(resolve=>{previewBusy=true;activeDeck?.interruptForPreview();const start=performance.now();preview.enqueue({file,source,overlays:{},request:{protocol:3,page_id:'p29'},timeout:120000,done:r=>{previewBusy=false;assert(!r.error,JSON.stringify(r.error));resolve(performance.now()-start);}});});
 try{await current();const baseline=[];for(let i=0;i<5;i++)baseline.push(await current());
 for(let round=0;round<5;round++)for(const concurrency of [1,4]){let firstReady,deck,started=performance.now(),firstMs,previewMs;const first=new Promise(r=>firstReady=r),orders=[],workers=[];const factory=()=>{const w=create();workers.push(w);return w;};
  deck=new NativePresentation({worker:factory(),createWorker:factory,concurrency,canPrepare:()=>!previewBusy,dist:path.join(root,'extensions/vscode/dist'),uri:f=>f,notify:m=>{if(m.type==='deck-page'){orders.push(m.page.id);if(m.page.id==='p01'){firstMs=performance.now()-started;firstReady();}queueMicrotask(()=>deck.ack(m.page.id));}if(m.type==='deck-error')throw Error(m.error);}});
  try{activeDeck=deck;const finished=deck.prepare(file,source,{},{});await first;assert(firstMs>0);await new Promise(r=>setTimeout(r,10));previewMs=await current();await finished;assert.equal(deck.pages.length,32);assert(deck.pages.every(p=>p.status==='ready'));assert.equal(orders[0],'p01');assert(workers.every(w=>!w.process),'Completed background processes must exit');rows.push({concurrency,round:round+1,firstMs,totalMs:deck.prepareMs,previewMs,workers:workers.length,interrupted:deck.interrupted||0});console.log(rows.at(-1));}finally{activeDeck=null;deck.dispose();}
 }
 fs.writeFileSync(path.join(out,'measurements.json'),JSON.stringify({baseline,rows,definition:'Native compilation and artifact delivery with immediate test ACK; excludes webview GPU rendering. Same page preview on a separate worker; concurrent deck jobs yield dispatch to preview.',hardwareAcceptance:false},null,2));
 }finally{preview.dispose();}
 console.log('Native multicore preparation, first artifact, preview priority and process cleanup passed.');
})().catch(e=>{console.error(e);process.exitCode=1;});
