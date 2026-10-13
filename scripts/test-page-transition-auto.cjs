// Exercise automatic page advancement and the animation clock across transitions.
const fs=require('fs'),path=require('path'),cp=require('child_process'),assert=require('assert/strict');
const engines=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const root=path.resolve(__dirname,'..'),out=path.resolve(process.argv[2]||'/tmp/laymesh-transition-auto');
fs.mkdirSync(out,{recursive:true});
cp.execFileSync(process.env.LAYMESH_TEST_BINARY||root+'/target/release/laymesh',['render',root+'/examples/animation/transitions/demo.laydoc','-o',out+'/demo.html','--warnings','hide']);
(async()=>{
  const results=[];
  for(const engine of (process.env.LAYMESH_TEST_ENGINES||'chromium,firefox,webkit').split(',')){
    const browser=await engines[engine].launch({headless:true});
    try{
      const page=await browser.newPage({viewport:{width:960,height:640}}),errors=[];
      page.on('pageerror',e=>errors.push(String(e)));page.setDefaultTimeout(60000);
      await page.goto('file://'+out+'/demo.html');
      await page.waitForFunction(()=>window.__LAYMESH_WEB&&document.querySelector('#app').dataset.painted==='true');
      await page.evaluate(async()=>{
        const p=__LAYMESH_WEB.presentation;p.fullscreen=false;await p.open();
        const call=p.call.bind(p),go=p.go.bind(p);window.autoTransitions=[];window.transitionClocks=[];
        p.call=async(data,...rest)=>{
          const before=p.clock.time,result=await call(data,...rest);
          if(data.type==='activate'&&data.transition)transitionClocks.push({before,after:p.clock.time,effect:result.transitionName,frames:result.transitionFrames,error:result.transitionError});
          return result;
        };
        p.go=async(index,...rest)=>{await go(index,...rest);if(p.index===index&&!p.switching)autoTransitions.push({index,time:p.clock.time,running:p.clock.running});};
        p.clock.mode='auto';p.clock.speed=4;p.action('pause');
      });
      await page.waitForFunction(()=>__LAYMESH_WEB.presentation.ended);
      const state=await page.evaluate(()=>({visits:autoTransitions,clocks:transitionClocks,index:__LAYMESH_WEB.presentation.index}));
      assert.deepEqual(state.visits.map(v=>v.index),[1,2,3]);
      assert.deepEqual(state.clocks.map(v=>v.effect),['InvertedPageCurl','cube','luma']);
      for(const row of state.clocks){assert.equal(row.before,row.after,'Timeline must pause while the transition renders');assert.ok(row.frames>0);assert.ok(!row.error,row.error);}
      assert.equal(state.visits[0].time,0,'Title animation starts at zero after the page transition');
      assert.equal(state.visits[0].running,true);assert.equal(state.index,3);assert.deepEqual(errors,[]);
      await page.evaluate(()=>__LAYMESH_WEB.presentation.close());
      results.push({engine,...state});console.log(engine+' automatic transitions and animation sequencing passed');
      fs.writeFileSync(out+'/auto.json',JSON.stringify(results,null,2));
    }finally{await browser.close();}
  }
})().catch(error=>{console.error(error);process.exitCode=1;});
