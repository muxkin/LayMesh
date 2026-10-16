// Build the normal web runtime first. The 256 MiB maximum is ONLY a test engine
// constraint to exercise minimum-growth fallback, never a production limit.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {execFileSync} from 'node:child_process';
const deps='target/wasm32-unknown-unknown/release/deps';
const library=fs.readdirSync(deps).filter(n=>/^libdlmalloc-.*\.rlib$/.test(n)).map(n=>deps+'/'+n).sort((a,b)=>fs.statSync(b).mtimeMs-fs.statSync(a).mtimeMs)[0];
assert(library,'Build the WASM runtime first');fs.mkdirSync('target/wasm-allocation',{recursive:true});
const output='target/wasm-allocation/probe.wasm';
execFileSync('rustc',['--edition=2024','--crate-type=cdylib','--target=wasm32-unknown-unknown','-O','-C','target-feature=+simd128','-C','link-arg=--max-memory=268435456','-L','dependency='+deps,'--extern','dlmalloc='+library,'scripts/fixtures/wasm-allocation.rs','-o',output],{stdio:'inherit'});
const {instance:{exports:e}}=await WebAssembly.instantiate(fs.readFileSync(output));
assert.equal(e.exercise(),1);assert.equal(e.hold(160*1024*1024),1);
const before=e.memory.buffer.byteLength;assert(before>128*1024*1024);
assert.equal(e.hold(4*1024*1024),1,'Retry minimum when geometric growth exceeds the engine maximum');
assert(e.memory.buffer.byteLength>before);assert.equal(e.release(),1);
const retained=e.memory.buffer.byteLength;
for(let i=0;i<20;i++){assert.equal(e.exercise(),1);assert.equal(e.hold(160*1024*1024),1);assert.equal(e.hold(4*1024*1024),1);assert.equal(e.release(),1);assert.equal(e.memory.buffer.byteLength,retained,'Freed blocks must be reused');}
console.log('WASM allocator: alignment, realloc data, zeroing, growth fallback and repeated block reuse passed');
