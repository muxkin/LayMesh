// Collect every WASM evaluation attempt, including interrupted attempts.
// This is a reuse oracle, not a timing benchmark. The delivered runtime is not
// changed: a test-only Blob wrapper adds read-only metrics calls to its worker.
// Usage: PLAYWRIGHT_MODULE=/path/to/playwright node scripts/test-web-semantic-reuse.cjs HTML OUTPUT
const fs = require('node:fs'), path = require('node:path'), assert = require('node:assert/strict');
const {pathToFileURL} = require('node:url'), crypto = require('node:crypto');
const pw = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const cases = require('./semantic-document-cases.cjs');
const html = path.resolve(process.argv[2]), out = path.resolve(process.argv[3]);
fs.mkdirSync(out, {recursive:true});

function instrumentCompiler() {
  const NativeBlob = window.Blob;
  window.Blob = new Proxy(NativeBlob, {
    construct(target, [parts = [], options], newTarget) {
      parts = Array.from(parts).map(part => {
        if (typeof part !== 'string' || !part.includes('async function evaluateSession(data,run)')) return part;
        const replacements = [
          ['let milliseconds=200,yields=0;', 'let milliseconds=200,yields=0,attemptMetrics=[];'],
          ['try{return {value:run(),yields};}', 'try{const value=run();attemptMetrics.push(JSON.parse(session.metrics()));return {value,yields,attemptMetrics};}'],
          ["if(code!=='E_YIELD')throw error;", "if(code!=='E_YIELD')throw error;attemptMetrics.push(JSON.parse(session.metrics()));"],
          ['const {value:job,yields}=await evaluateSession', 'const {value:job,yields,attemptMetrics}=await evaluateSession'],
          ['compileMetrics: {...JSON.parse(session.metrics()),', 'compileMetrics: {...JSON.parse(session.metrics()),allAttemptMetrics:attemptMetrics,'],
        ];
        for (const [before, after] of replacements) {
          if (part.split(before).length !== 2) throw Error('Reuse instrumentation no longer matches worker: '+before);
          part = part.replace(before, after);
        }
        window.__instrumentedCompilerWorkers = (window.__instrumentedCompilerWorkers || 0) + 1;
        return part;
      });
      return Reflect.construct(target, [parts, options], newTarget);
    }
  });
}

(async () => {
  const results = [];
  for (const engine of (process.env.LAYMESH_TEST_ENGINES || 'chromium,firefox,webkit').split(',')) {
    console.log(engine+' reuse verification starting');
    const browser = await pw[engine].launch({headless:true});
    try {
      const page = await browser.newPage({viewport:{width:1600,height:1000}}), errors = [], network = [];
      page.setDefaultTimeout(240000);
      page.on('pageerror', error => errors.push(error.message));
      await page.route(/^https?:\/\//, route => { network.push(route.request().url()); return route.abort(); });
      await page.addInitScript(instrumentCompiler);
      await page.goto(pathToFileURL(html).href);
      await page.waitForFunction(() => window.__LAYMESH_WEB?.editor && document.querySelector('#app').dataset.painted === 'true');
      await page.evaluate(() => __LAYMESH_WEB.previewCompiler.invalidate());
      await page.waitForFunction(() => document.querySelector('#app').dataset.documentState === 'ready');
      await page.locator('#page').selectOption('5');
      await page.waitForFunction(() => __LAYMESH_WEB.metrics.pageId === __LAYMESH_WEB.pages[5].id && document.querySelector('#app').dataset.painted === 'true');
      assert.equal(await page.evaluate(() => window.__instrumentedCompilerWorkers), 1);
      const name = '/project/pages/06-editable.lay';
      const original = await page.evaluate(name => ({lay:__LAYMESH_WEB.editor.files[name], css:__LAYMESH_WEB.editor.files['/project/theme.lcss'], doc:__LAYMESH_WEB.editor.files['/project/storyboard.laydoc']}), name);
      const edits = {
        color: original.lay.replace('#fbfaf5', '#faf7e1'),
        position: original.lay.replace(/offset=\(([-\d.]+)mm,/, (_,x) => `offset=(${Number(x)+1.23}mm,`),
        text: original.lay.replace(/chrome\(page,("[^"\n]*"),("[^"\n]*")/, (_,number,title) => `chrome(page,${number},${JSON.stringify(JSON.parse(title)+' reuse')}`),
        size: original.lay.replace(/rect\(size=\(([-\d.]+)mm,/, (_,x) => `rect(size=(${Number(x)-1.23}mm,`),
      };
      const apply = async (file, source) => {
        const previous = await page.evaluate(async ({file,source}) => {
          const w = __LAYMESH_WEB; await w.workspace.open(file);
          const v = w.editor.views.get(file).view, id = w.metrics.pageId;
          v.dispatch({changes:{from:0,to:v.state.doc.length,insert:source}}); return id;
        }, {file,source});
        await page.waitForFunction(id => {
          const w = __LAYMESH_WEB, a = document.querySelector('#app');
          return a.dataset.documentState === 'ready' && a.dataset.painted === 'true' && w.metrics.pageId !== id;
        }, previous);
        return page.evaluate(() => __LAYMESH_WEB.pages[5].compileMetrics);
      };
      await apply(name, original.lay+'\n# reuse warmup\n');
      await apply(name, original.lay);
      for (const [kind,source] of Object.entries(edits)) {
        console.log(engine+' checking '+kind);
        assert.notEqual(source, original.lay);
        const metrics = await apply(name, source), attempts = metrics.allAttemptMetrics;
        assert(attempts?.length, 'Missing per-attempt metrics');
        const pathCalls = attempts.reduce((n,m) => n + (m.evaluatedBuiltinCalls.path || 0), 0);
        const geometry = attempts.reduce((n,m) => n + m.rebuiltGeometry, 0);
        assert.equal(pathCalls, 0, `${engine} ${kind}: curve constructors across all attempts`);
        assert(geometry <= (kind === 'size' ? 1 : 0), `${engine} ${kind}: geometry across all attempts: ${geometry}`);
        results.push({engine,version:browser.version(),kind,attemptCount:attempts.length,pathCalls,geometry,attempts});
        await apply(name, original.lay);
      }
      for (const kind of ['lcss-heading','lcss-curves']) {
        console.log(engine+' checking '+kind);
        const change = cases.change(kind, 7, original.css, original.doc);
        const metrics = await apply('/project/'+change.file, change.source), attempts = metrics.allAttemptMetrics;
        assert(attempts?.length);
        const pathCalls = attempts.reduce((n,m) => n + (m.evaluatedBuiltinCalls.path || 0), 0);
        const geometry = attempts.reduce((n,m) => n + m.rebuiltGeometry, 0);
        assert.equal(pathCalls, 0, `${engine} ${kind}: unrelated curve constructors across all attempts`);
        assert.equal(geometry, 0, `${engine} ${kind}: appearance edits must retain geometry in every attempt`);
        results.push({engine,version:browser.version(),kind,attemptCount:attempts.length,pathCalls,geometry,attempts});
      }
      assert.deepEqual(errors, []); assert.deepEqual(network, []);
      fs.writeFileSync(path.join(out, 'verification.json'), JSON.stringify({
        html,htmlSha256:crypto.createHash('sha256').update(fs.readFileSync(html)).digest('hex'),
        method:'Test-only read-only metrics after every evaluation attempt; no timings used for acceptance',results,
      }, null, 2)+'\n');
      console.log(engine+' all-attempt reuse checks passed');
    } finally { await browser.close(); }
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
