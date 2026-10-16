import fs from 'node:fs';
import assert from 'node:assert/strict';
const source=fs.readFileSync(new URL('../site/live/web-compiler-worker.mjs',import.meta.url),'utf8');
const body=source.slice(source.indexOf('async function evaluateSession'),source.indexOf('self.onmessage'));
let now=0,calls=0;const slices=[],events=[],diagnostics={attempts:[]};
const engine=new Function('session','progress','performance',`let latestVersion=0;${body};return {run:evaluateSession,cancel:v=>latestVersion=v};`)(
 {set_time_slice:n=>slices.push(n),metrics:()=> '{}' },(data,phase,detail)=>events.push({phase,...detail}),{timeOrigin:0,now:()=>now});
const result=await engine.run({mode:'preview',version:1},()=>{if(!calls++){now+=70000;throw JSON.stringify({code:'E_YIELD'});}now+=100;return 'done';},diagnostics,'evaluate');
assert.equal(result.value,'done');assert.equal(result.yields,1);assert.deepEqual(slices,[200,0,140000,0],'A long atomic unit must not be trapped behind a 60 second retry ceiling');
assert.equal(diagnostics.attempts.reduce((n,a)=>n+a.computeMs,0),70100,'Include the failed attempt in cumulative computation time');
engine.cancel(2);await assert.rejects(engine.run({mode:'preview',version:1},()=>assert.fail('stale work ran'),{attempts:[]},'evaluate'),/superseded/);
assert.equal(events.filter(e=>e.phase==='attempt-end').length,2);
console.log('Compile slices: measured retry allowance, no fixed ceiling, attempt accounting and cancellation passed');
