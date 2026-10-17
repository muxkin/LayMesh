// Real overflow diagnostics must survive export without covering the artwork.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const http = require('node:http');
const {spawnSync} = require('node:child_process');
const {pathToFileURL} = require('node:url');
const pw = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const root = path.resolve(__dirname, '..');
const out = path.resolve(process.argv[2] || '/tmp/laymesh-presentation-warnings');
const binary = process.env.LAYMESH_TEST_BINARY || path.join(root, 'target/release/laymesh');
const source = `style { @keyframes move { from { transform: translateX(0mm); } to { transform: translateX(5mm); } } }
p=canvas(size=(160mm,90mm),background="#ffffff")
p.add(rect(size=(180mm,110mm),fill="#80b5dc",border_width=0pt),offset=(-10mm,-10mm))
p.add(ellipse(size=(24mm,24mm),fill="#0099aa",effects=[shadow(blur=4mm,offset=(4mm,4mm))]),offset=(150mm,-10mm))
a=p.add(text("Warning regression",font_size=20pt),offset=(20mm,35mm))
s=p.step()
s.animate(a,name="move",duration=0.3s,easing="linear",fill="both")`;

(async () => {
  fs.mkdirSync(out, {recursive:true});
  const archive = await import(pathToFileURL(path.join(root, 'site/live/web-archive.mjs')));
  const input = path.join(out, 'overflow.lay');
  fs.writeFileSync(input, source);
  const variants = [], served = new Map(), results = [];
  for (const editable of [false, true]) for (const zip of [false, true]) {
    const name = `${editable ? 'editable' : 'playback'}-${zip ? 'zip' : 'html'}`;
    const file = path.join(out, name + (zip ? '.zip' : '.zip.html'));
    const render = spawnSync(binary, ['render', input, '-o', file, '--warnings', 'show', ...(editable ? ['--html-editable', '--html-embed-fonts'] : [])], {encoding:'utf8'});
    assert.equal(render.status, 0, render.stderr);
    assert.match(render.stderr, /W_EFFECT_OVERFLOW/, name + ': command-line diagnostic');
    const entries = archive.webUnzip(archive.webPayload(fs.readFileSync(file)));
    const manifest = JSON.parse(new TextDecoder().decode(entries['manifest.json']));
    const page = JSON.parse(new TextDecoder().decode(entries[manifest.pages[0].file]));
    assert(page.inspection.warnings.some(w => w.code === 'W_EFFECT_OVERFLOW'), name + ': real overflow diagnostic');
    for (const [entry, bytes] of Object.entries(entries)) served.set('/' + name + '/' + entry, bytes);
    variants.push({name, file, editable, zip});
  }
  const host = process.env.LAYMESH_TEST_HOST || '127.0.0.1';
  const server = http.createServer((req, res) => {
    const name = decodeURIComponent(req.url.split('?')[0]);
    const data = served.get(name);
    if (!data) { res.writeHead(404); res.end(); return; }
    res.setHeader('Content-Type', /\.m?js$/.test(name) ? 'text/javascript' : name.endsWith('.wasm') ? 'application/wasm' : name.endsWith('.html') ? 'text/html' : 'application/octet-stream');
    res.end(data);
  });
  await new Promise(resolve => server.listen(0, host, resolve));
  try {
    for (const engine of (process.env.LAYMESH_TEST_ENGINES || 'chromium,firefox,webkit').split(',')) {
      const browser = await pw[engine].launch({headless:true, ...(engine === 'chromium' ? {args:['--enable-unsafe-swiftshader']} : {})});
      try {
        const context = await browser.newContext({acceptDownloads:true, viewport:{width:1280,height:800}});
        const errors = [];
        context.on('page', page => page.on('pageerror', error => errors.push(error.message)));
        const page = await context.newPage();
        page.setDefaultTimeout(90000);
        const ready = () => page.waitForFunction(() => document.querySelector('#app')?.dataset.painted === 'true');
        const cleanFrame = async () => {
          await page.waitForFunction(() => window.__LAYMESH_WEB.presentation.ready && !__LAYMESH_WEB.presentation.switching);
          assert.equal(await page.locator('.presentation-warning').count(), 0);
          assert.doesNotMatch(await page.locator('.presentation').innerText(), /Decorated content|项警告/);
          assert(await page.evaluate(() => __LAYMESH_WEB.presentation.deck.pages[0].inspection.warnings.some(w => w.code === 'W_EFFECT_OVERFLOW')));
          assert(await page.locator('.presentation-canvas').isVisible());
        };
        const windowed = async () => {
          await page.locator('#preview-more summary').click();
          await page.locator('#windowed').click();
          await cleanFrame();
        };
        const close = async () => {
          await page.evaluate(() => __LAYMESH_WEB.presentation.close());
          await page.waitForFunction(() => !__LAYMESH_WEB.presentation.closing);
        };
        for (const variant of variants) {
          await page.goto(variant.zip ? `http://${host}:${server.address().port}/${variant.name}/index.html` : pathToFileURL(variant.file).href);
          await ready();
          await windowed();
          await page.locator('.presentation').focus();
          await page.keyboard.press('ArrowRight');
          await page.waitForFunction(() => __LAYMESH_WEB.presentation.clock.time > 0.1);
          await cleanFrame();
          await page.screenshot({path:path.join(out, `${engine}-${variant.name}.png`)});
          await close();
          if (engine === 'chromium') {
            await page.locator('#full').click();
            await cleanFrame();
            await page.waitForFunction(() => document.fullscreenElement === __LAYMESH_WEB.presentation.root);
            await close();
            await page.locator('#preview-more summary').click();
            const popup = context.waitForEvent('page');
            await page.locator('#presenter').click();
            const consolePage = await popup;
            await consolePage.waitForFunction(() => window.laymeshPresenterConsole?.state?.ready);
            await cleanFrame();
            await close();
            await windowed();
            await page.evaluate(() => __LAYMESH_WEB.presentation.fail(Error('Recovery regression')));
            assert.match(await page.locator('.presentation-progress').innerText(), /Recovery regression/);
            await page.getByRole('button', {name:'重试',exact:true}).click();
            await cleanFrame();
            await close();
          }
          if (variant.editable) {
            await page.waitForFunction(() => document.querySelector('#app').dataset.editorReady === 'true');
            const code = page.locator('.source-panel:not([hidden]) .cm-content');
            await code.fill(source.replace('Warning regression', 'Edited regression'));
            await page.waitForFunction(() => document.querySelector('#app').dataset.compileState === 'ready' && __LAYMESH_WEB.metrics.compiles > 0);
            const downloading = page.waitForEvent('download');
            await page.locator('#export').click();
            await page.locator('#save').click();
            const saved = path.join(out, `${engine}-${variant.name}-saved.zip.html`);
            await (await downloading).saveAs(saved);
            const entries = archive.webUnzip(archive.webPayload(fs.readFileSync(saved)));
            const manifest = JSON.parse(new TextDecoder().decode(entries['manifest.json']));
            assert(new TextDecoder().decode(entries[manifest.project.files[manifest.project.entry]]).includes('Edited regression'));
            await page.goto(pathToFileURL(saved).href);
            await ready();
            await windowed();
            await cleanFrame();
            await close();
          }
          results.push({engine, variant:variant.name, warningRetained:true, noOverlay:true, savedReopened:variant.editable, fullscreenAndPresenter:engine === 'chromium', recovery:engine === 'chromium'});
          console.log(engine, variant.name, 'passed');
        }
        assert.deepEqual(errors, []);
        await context.close();
      } finally { await browser.close(); }
    }
  } finally { await new Promise(resolve => server.close(resolve)); }
  fs.writeFileSync(path.join(out, 'results.json'), JSON.stringify({results}, null, 2) + '\n');
})().catch(error => { console.error(error); process.exitCode = 1; });
