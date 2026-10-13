// Real Tailscale HTTP service: media MIME, HEAD and byte-range delivery.
const assert=require('node:assert/strict'),fs=require('node:fs'),os=require('node:os'),path=require('node:path');
const {NativePresentation}=require('../extensions/vscode/src/presentation-host.cjs');
(async()=>{
 const directory=fs.mkdtempSync(path.join(os.tmpdir(),'laymesh-media-http-'));
 const file=path.resolve(process.argv[2]||'examples/media/demo.mp4'),bytes=fs.readFileSync(file);
 const deck=new NativePresentation({worker:{resourceDir:directory,dispose(){}},dist:directory,uri:f=>f,notify(){}});
 try{
  deck.ready=true;deck.resources.set('video-test',file);deck.resourceMimes.set('video-test','video/mp4');
  const base=await deck.browser(),url=base+'resources/video-test';
  const head=await fetch(url,{method:'HEAD'});assert.equal(head.status,200);assert.equal(head.headers.get('content-type'),'video/mp4');assert.equal(Number(head.headers.get('content-length')),bytes.length);assert.equal((await head.arrayBuffer()).byteLength,0);
  const part=await fetch(url,{headers:{Range:'bytes=10-99'}});assert.equal(part.status,206);assert.equal(part.headers.get('content-range'),`bytes 10-99/${bytes.length}`);assert.deepEqual(Buffer.from(await part.arrayBuffer()),bytes.subarray(10,100));
  const suffix=await fetch(url,{headers:{Range:'bytes=-20'}});assert.equal(suffix.status,206);assert.deepEqual(Buffer.from(await suffix.arrayBuffer()),bytes.subarray(-20));
  assert.equal((await fetch(url,{headers:{Range:`bytes=${bytes.length}-`}})).status,416);
  console.log('Tailscale media MIME, HEAD, byte ranges and invalid range checks passed');
 }finally{deck.dispose();fs.rmSync(directory,{recursive:true,force:true});}
})().catch(error=>{console.error(error);process.exitCode=1;});
