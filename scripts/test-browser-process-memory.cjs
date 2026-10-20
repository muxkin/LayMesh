// A child forked by a worker thread must be counted, as must its descendants.
const fs=require('node:fs'),os=require('node:os'),path=require('node:path'),assert=require('node:assert/strict');
const memory=require('./browser-process-memory.cjs');
const root=fs.mkdtempSync(path.join(os.tmpdir(),'laymesh-proc-'));
function proc(pid,parent,pss,{mainChildren='',threadChildren='',gpu=false,drm=false,unreadable=false}={}){
 const dir=path.join(root,String(pid));fs.mkdirSync(dir,{recursive:true});
 fs.writeFileSync(path.join(dir,'status'),`Tgid:\t${pid}\nPPid:\t${parent}\n`);
 fs.writeFileSync(path.join(dir,'cmdline'),gpu?'browser\0--type=gpu-process\0':'browser\0');
 if(!unreadable)fs.writeFileSync(path.join(dir,'smaps_rollup'),`Rss: ${pss*2} kB\nPss: ${pss} kB\n`);
 for(const [tid,children]of[[pid,mainChildren],[pid+1000,threadChildren]]){const d=path.join(dir,'task',String(tid));fs.mkdirSync(d,{recursive:true});fs.writeFileSync(path.join(d,'children'),children);}
 if(drm){const d=path.join(dir,'fdinfo');fs.mkdirSync(d);for(const fd of['3','4'])fs.writeFileSync(path.join(d,fd),'drm-client-id: 7\ndrm-pdev: 0000:03:00.0\ndrm-memory-vram: 4096 KiB\n');}
}
try{
 proc(10,1,100,{mainChildren:'11',threadChildren:'12'});proc(11,10,20);proc(12,10,30,{mainChildren:'13',gpu:true,drm:true});proc(13,12,40,{drm:true});
 proc(14,12,0,{unreadable:true,mainChildren:'15'});proc(15,14,50);proc(90,1,999);
 const m=memory(10,root);assert.equal(m.pss,240*1024);assert.equal(m.rss,480*1024);assert.equal(m.gpuProcessPss,30*1024);
 assert.equal(m.processes,5);assert.equal(m.discoveredProcesses,6);assert.deepEqual(m.unreadable,[14]);assert.equal(m.drm.length,1);assert.equal(m.protocol,'proc-ppid-v2');assert.equal(memory(90,root).gpuProcessPss,null);
 console.log('Whole process tree, non-main-thread children, exit races and DRM deduplication passed');
}finally{fs.rmSync(root,{recursive:true,force:true});}
