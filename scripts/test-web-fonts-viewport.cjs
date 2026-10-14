// Foreign font catalogs, explicit font application and retained viewport motion.
const fs = require("fs"),
  path = require("path"),
  cp = require("child_process"),
  assert = require("assert/strict");
const engines = require(process.env.PLAYWRIGHT_MODULE || "playwright"),
  root = path.resolve(__dirname, ".."),
  out = path.resolve(process.argv[2] || "/tmp/laymesh-fonts-viewport-qa");
fs.mkdirSync(out, { recursive: true });
const binary =
  process.env.LAYMESH_TEST_BINARY || path.join(root, "target/release/laymesh");
function namedFont(style) {
  const original = fs.readFileSync(
      path.join(root, "tests/fonts/DejaVuSans.ttf"),
    ),
    entries = [];
  for (const [lang, family] of [
    [0x409, "Microsoft YaHei"],
    [0x804, "微软雅黑"],
  ])
    for (const [id, value] of [
      [1, family],
      [2, style],
      [4, family + " " + style],
      [6, "MicrosoftYaHei" + (style === "Bold" ? "-Bold" : "")],
      [16, family],
      [17, style],
    ]) {
      const bytes = Buffer.from(value, "utf16le").swap16();
      entries.push({ lang, id, bytes });
    }
  const table = Buffer.alloc(
    6 + entries.length * 12 + entries.reduce((n, e) => n + e.bytes.length, 0),
  );
  table.writeUInt16BE(entries.length, 2);
  table.writeUInt16BE(6 + entries.length * 12, 4);
  let offset = 0;
  entries.forEach((e, i) => {
    const at = 6 + i * 12;
    table.writeUInt16BE(3, at);
    table.writeUInt16BE(1, at + 2);
    table.writeUInt16BE(e.lang, at + 4);
    table.writeUInt16BE(e.id, at + 6);
    table.writeUInt16BE(e.bytes.length, at + 8);
    table.writeUInt16BE(offset, at + 10);
    e.bytes.copy(table, 6 + entries.length * 12 + offset);
    offset += e.bytes.length;
  });
  const start = Math.ceil(original.length / 4) * 4,
    result = Buffer.alloc(start + Math.ceil(table.length / 4) * 4);
  original.copy(result);
  table.copy(result, start);
  let head;
  for (let i = 0; i < result.readUInt16BE(4); i++) {
    const at = 12 + i * 16,
      tag = result.toString("ascii", at, at + 4);
    if (tag === "name") {
      result.writeUInt32BE(start, at + 8);
      result.writeUInt32BE(table.length, at + 12);
      let sum = 0;
      for (let p = start; p < result.length; p += 4)
        sum = (sum + result.readUInt32BE(p)) >>> 0;
      result.writeUInt32BE(sum, at + 4);
    }
    if (tag === "head") head = result.readUInt32BE(at + 8);
    if (tag === "OS/2" && style === "Bold")
      result.writeUInt16BE(700, result.readUInt32BE(at + 8) + 4);
  }
  result.writeUInt32BE(0, head + 8);
  let sum = 0;
  for (let p = 0; p < result.length; p += 4)
    sum = (sum + result.readUInt32BE(p)) >>> 0;
  result.writeUInt32BE((0xb1b0afba - sum) >>> 0, head + 8);
  return result;
}
(async () => {
  const sample = path.join(out, "font-test.zip.html");
  const sameSource=path.join(out,'same-font.lay'),sameSample=path.join(out,'same-font.zip.html');
  fs.writeFileSync(sameSource,'p=canvas(size=(100mm,60mm))\np.add(text("Automatic font matching",font_family="DejaVu Sans"))');
  cp.execFileSync(binary,['render',sameSource,'-o',sameSample,'--html-editable'],{cwd:root,stdio:'pipe'});
  cp.execFileSync(
    binary,
    [
      "render",
      "examples/animation/presentation.laydoc",
      "-o",
      sample,
      "--html-editable",
    ],
    { cwd: root, stdio: "pipe" },
  );
  const bytes = [...namedFont("Regular")],
    bold = [...namedFont("Bold")],
    results = [];
  for (const name of ["chromium", "firefox", "webkit"]) {
    const browser = await engines[name].launch({ headless: true });
    try {
      const context = await browser.newContext({
        viewport: { width: 1586, height: 992 },
        deviceScaleFactor: 2,
        acceptDownloads: true,
      });
      await context.route(/^https?:\/\//, (r) => r.abort());
      await context.addInitScript(
        ({ bytes, bold }) => {
          const Base = window.Worker;
          window.__probe = {
            requests: [],
            replies: [],
            fontReads: [],
            fontBlobs: [],
          };
          window.Worker = class extends Base {
            constructor(...a) {
              super(...a);
              this.addEventListener("message", (e) =>
                __probe.replies.push({
                  at: performance.now(),
                  id: e.data.id,
                  type: e.data.type,
                  skipped: e.data.skipped,
                  raster: e.data.rasterMs,
                }),
              );
            }
            postMessage(d, ...a) {
              __probe.requests.push({
                at: performance.now(),
                type: d.type,
                id: d.id,
                serial: d.serial,
              });
              super.postMessage(d, ...a);
            }
          };
          const font = (postscriptName, style, data) => ({
            family: "Microsoft YaHei",
            fullName: "Microsoft YaHei " + style,
            postscriptName,
            style,
            async blob() {
              __probe.fontBlobs.push(postscriptName);
              const b = new Blob([new Uint8Array(data)]),
                original = b.arrayBuffer.bind(b);
              b.arrayBuffer = () => {
                __probe.fontReads.push(postscriptName);
                return original();
              };
              return b;
            },
          });
          window.queryLocalFonts = async () => [
            font("MicrosoftYaHei", "Regular", bytes),
            font("MicrosoftYaHei-Bold", "Bold", bold),
            {
              family: "BrokenFont",
              fullName: "BrokenFont",
              style: "Regular",
              postscriptName: "BrokenFont",
              blob: async () => {
                throw Error("模拟字体读取失败");
              },
            },
            ...Array.from({ length: 200 }, (_, i) => ({
              family: "Catalog " + String(i).padStart(3, "0"),
              fullName: "Catalog " + i,
              style: "Regular",
              postscriptName: "Catalog" + i,
              async blob() {
                const value = "Catalog " + i,
                  data = new Uint8Array(46 + value.length * 2),
                  v = new DataView(data.buffer);
                v.setUint32(0, 0x10000);
                v.setUint16(4, 1);
                v.setUint32(12, 0x6e616d65);
                v.setUint32(20, 28);
                v.setUint32(24, data.length - 28);
                v.setUint16(30, 1);
                v.setUint16(32, 18);
                v.setUint16(34, 3);
                v.setUint16(36, 1);
                v.setUint16(38, 0x409);
                v.setUint16(40, 1);
                v.setUint16(42, value.length * 2);
                for (let n = 0; n < value.length; n++)
                  v.setUint16(46 + n * 2, value.charCodeAt(n));
                return new Blob([data]);
              },
            })),
          ];
        },
        { bytes, bold },
      );
      const page = await context.newPage(),
        errors = [];
      page.on("pageerror", (e) => errors.push(e.message));
      await page.goto("file://" + sample);
      await page.waitForFunction(
        () =>
          window.__LAYMESH_WEB?.editor &&
          document.querySelector("#app").dataset.painted === "true",
      );
      await page.locator("#fonts").click();
      assert(await page.locator("#font-apply").isVisible());
      assert(
        await page
          .locator(".font-replacement-heading")
          .getByText("替换为")
          .isVisible(),
      );
      await page.locator("#connect-fonts").click();
      await page.waitForFunction(() => !document.querySelector('#connect-fonts').disabled);
      assert.equal(await page.locator('#font-index-progress').count(),0);
      assert.deepEqual(await page.evaluate(()=>__probe.fontBlobs),[]);
      assert.equal(
        (await page.evaluate(() => __LAYMESH_WEB.workspace.liveFonts)).length,
        0,
      );
      assert.equal(await page.evaluate(() => __probe.fontReads.length), 0);
      const input = page.locator('.font-row [role="combobox"]').first();
      await input.click();
      assert(
        (await page
          .locator('.font-options:not([hidden]) [role="option"]')
          .count()) < 30,
      );
      await page
        .locator(".font-options:not([hidden])")
        .evaluate((n) => (n.scrollTop = n.scrollHeight));
      // Virtual rows update on the browser's next rendering opportunity, which
      // can exceed 30 ms on the software-rendered CI runner.
      await page.waitForFunction(() => {
        const options = document.querySelectorAll('.font-options:not([hidden]) [role="option"]');
        return Number(options[0]?.getAttribute('aria-posinset')) > 180 &&
          options[options.length - 1]?.getAttribute('aria-posinset') === '203';
      });
      assert(
        Number(
          await page
            .locator('.font-options:not([hidden]) [role="option"]')
            .first()
            .getAttribute("aria-posinset"),
        ) > 180,
      );
      assert.equal(
        await page
          .locator('.font-options:not([hidden]) [role="option"]')
          .last()
          .getAttribute("aria-posinset"),
        "203",
      );
      await input.fill("不存在的名字");
      assert(await page.getByText("没有匹配字体").isVisible());
      await input.fill("微软雅黑");
      assert.equal(
        await page
          .locator('.font-options:not([hidden]) [role="option"]')
          .count(),
        2,
      );
      await input.press("ArrowDown");
      await input.press("Enter");
      assert((await input.inputValue()).includes("微软雅黑"));
      await page.locator("#font-later").click();
      assert.equal(
        await page.evaluate(() => __LAYMESH_WEB.metrics.compiles),
        0,
      );
      await page.locator("#fonts").click();
      assert(
        (
          await page.locator('.font-row [role="combobox"]').first().inputValue()
        ).includes("微软雅黑"),
      );
      assert(
        (await page.locator(".font-summary").innerText()).includes("尚未应用"),
      );
      // Failed selections remain visible, and no mapping is silently committed.
      await page
        .locator('.font-row [role="combobox"]')
        .last()
        .fill("BrokenFont");
      await page
        .locator('.font-options:not([hidden]) [role="option"]')
        .first()
        .click();
      await page.locator("#font-apply").click();
      await page.waitForFunction(() =>
        document
          .querySelector(".font-notice")
          .textContent.includes("模拟字体读取失败"),
      );
      assert(await page.locator("#font-panel").isVisible());
      assert.equal(
        await page.evaluate(
          () => Object.keys(__LAYMESH_WEB.workspace.bindings).length,
        ),
        0,
      );
      for (const field of await page
        .locator('.font-row [role="combobox"]')
        .all()) {
        await field.fill("微软雅黑");
        await page
          .locator('.font-options:not([hidden]) [role="option"]')
          .filter({ hasText: /MicrosoftYaHei$/ })
          .first()
          .click();
      }
      // Unresolved imported-font fallback requirements must all be explicitly handled.
      await page.screenshot({
        path: path.join(out, name + "-font-choices.png"),
      });
      await page.locator("#font-apply").click();
      await page.waitForFunction(
        () =>
          document.querySelector("#app").dataset.compileState === "ready" &&
          !document.querySelector("#font-panel").open,
        null,
        { timeout: 60000 },
      );
      assert.equal(
        await page.evaluate(() => __LAYMESH_WEB.metrics.compiles),
        1,
      );
      assert(
        (await page.evaluate(() => __probe.fontReads)).includes(
          "MicrosoftYaHei",
        ),
      );
      assert.equal(
        await page.evaluate(
          () =>
            __LAYMESH_WEB.workspace.liveFonts.filter(
              (f) => f.family === "Microsoft YaHei",
            ).length,
        ),
        1,
      );
      await page.locator("#fonts").click();
      const extra = page.getByLabel("选择其他本机字体", { exact: true });
      await extra.fill("MicrosoftYaHei-Bold");
      await page
        .locator('.font-options:not([hidden]) [role="option"]')
        .first()
        .click();
      await page.getByRole("button", { name: "加载选中字体" }).click();
      await page.waitForFunction(() =>
        __LAYMESH_WEB.workspace.liveFonts.some(
          (f) => f.postscriptName === "MicrosoftYaHei-Bold",
        ),
      );
      assert.equal(
        await page.evaluate(
          () =>
            __LAYMESH_WEB.workspace.liveFonts.filter(
              (f) => f.family === "Microsoft YaHei",
            ).length,
        ),
        2,
      );
      await page.locator("#font-apply").click();
      await page.waitForFunction(
        () => !document.querySelector("#font-panel").open,
      );
      await page.locator("#zoom").selectOption("4");
      await page.waitForFunction(
        () => document.querySelector("#app").dataset.painted === "true",
      );
      const before = await page.evaluate(() => ({
        installs: __LAYMESH_WEB.metrics.pageInstalls,
        compiles: __LAYMESH_WEB.metrics.compiles,
      }));
      const motion = await page.evaluate(async () => {
        const { preview, view } = __LAYMESH_WEB,
          stage = preview.stage,
          s = preview.scroll,
          raf = () => new Promise((r) => requestAnimationFrame(r));
        __probe.requests = [];
        __probe.replies = [];
        const latency = [],
          positions = [];
        let previous = s.scrollLeft;
        for (let i = 0; i < 60; i++) {
          const at = performance.now();
          stage.dispatchEvent(
            new WheelEvent("wheel", {
              bubbles: true,
              cancelable: true,
              deltaX: i < 30 ? 4 : -4,
            }),
          );
          await raf();
          latency.push(performance.now() - at);
          positions.push(preview.figure.getBoundingClientRect().left);
        }
        await new Promise((r) => setTimeout(r, 150));
        const sorted = latency.sort((a, b) => a - b),
          request = preview.request(),
          rect = preview.rect(),
          point = preview.coordinates(
            s.getBoundingClientRect().left + rect.left + view.pixelsPerMM * 80,
            s.getBoundingClientRect().top + rect.top + view.pixelsPerMM * 45,
          );
        return {
          p95Ms: sorted[56],
          positionsChanged: new Set(positions).size,
          types: __probe.requests.map((r) => r.type),
          scrollbars: [
            s.scrollWidth > s.clientWidth,
            s.scrollHeight > s.clientHeight,
          ],
          point,
          canvas: [preview.canvas.width, preview.canvas.height],
          request,
          view,
          installs: __LAYMESH_WEB.metrics.pageInstalls,
          compiles: __LAYMESH_WEB.metrics.compiles,
          overviewPixels: preview.overview.width * preview.overview.height,
        };
      });
      fs.writeFileSync(
        path.join(out, name + "-motion.json"),
        JSON.stringify(motion, null, 2),
      );
      assert(motion.positionsChanged > 20);
      assert(motion.scrollbars.every(Boolean));
      assert(
        !motion.types.includes("load") && !motion.types.includes("prepare"),
      );
      assert.equal(motion.installs, before.installs);
      assert.equal(motion.compiles, before.compiles);
      assert(Math.abs(motion.point.mm[0] - 80) < 0.01);
      assert(motion.overviewPixels <= 500000);
      if (name === "chromium" && process.env.LAYMESH_ASSERT_PERFORMANCE === "1")
        assert(motion.p95Ms < 33, motion.p95Ms);
      const zoomMotion = await page.evaluate(async () => {
        const { preview } = __LAYMESH_WEB,
          b = preview.scroll.getBoundingClientRect(),
          latency = [];
        __probe.requests = [];
        for (let i = 0; i < 60; i++) {
          const at = performance.now();
          preview.stage.dispatchEvent(
            new WheelEvent("wheel", {
              bubbles: true,
              cancelable: true,
              ctrlKey: true,
              deltaY: i % 2 ? 3 : -3,
              clientX: b.x + b.width / 2,
              clientY: b.y + b.height / 2,
            }),
          );
          await new Promise((r) => requestAnimationFrame(r));
          latency.push(performance.now() - at);
        }
        await new Promise((r) => setTimeout(r, 250));
        latency.sort((a, b) => a - b);
        return {
          p95Ms: latency[56],
          types: __probe.requests.map((r) => r.type),
          installs: __LAYMESH_WEB.metrics.pageInstalls,
          compiles: __LAYMESH_WEB.metrics.compiles,
        };
      });
      assert(
        !zoomMotion.types.includes("load") &&
          !zoomMotion.types.includes("prepare"),
      );
      assert.equal(zoomMotion.installs, before.installs);
      assert.equal(zoomMotion.compiles, before.compiles);
      if (name === "chromium" && process.env.LAYMESH_ASSERT_PERFORMANCE === "1")
        assert(zoomMotion.p95Ms < 33, zoomMotion);
      await page.locator("#fonts").click();
      await page.screenshot({ path: path.join(out, name + "-fonts.png") });
      await page.locator("#font-later").click();
      await page.screenshot({ path: path.join(out, name + "-viewport.png") });
      await page.locator("#export").click();
      const dl = page.waitForEvent("download");
      await page.locator("#save").click();
      const saved = path.join(out, name + "-updated.zip.html");
      await (await dl).saveAs(saved);
      await page.goto("file://" + saved);
      await page.waitForFunction(
        () =>
          window.__LAYMESH_WEB?.workspace &&
          document.querySelector("#app").dataset.painted === "true",
      );
      assert.equal(
        await page.evaluate(() => __LAYMESH_WEB.metrics.compiles),
        0,
      );
      assert.equal(
        await page.evaluate(() => __LAYMESH_WEB.workspace.liveFonts.length),
        0,
      );
      await page.locator("#fonts").click();
      assert(
        (await page.locator("#font-status").innerText()).includes("待补齐"),
      );
      // Real font metadata and real WASM compilation, with no replacement choice.
      await page.goto('file://'+sameSample);
      await page.waitForFunction(()=>window.__LAYMESH_WEB?.workspace&&document.querySelector('#app').dataset.editorReady==='true');
      await page.locator('#fonts').click();await page.locator('#font-file').setInputFiles(path.join(root,'tests/fonts/DejaVuSans.ttf'));
      await page.waitForFunction(()=>document.querySelector('#app').dataset.compileState==='ready');
      assert.deepEqual(await page.evaluate(()=>__LAYMESH_WEB.workspace.bindings),{});
      assert.equal(await page.locator('#font-status').innerText(),'作品字体已准备');
      assert((await page.evaluate(()=>__LAYMESH_WEB.metrics.compiles))>0);
      assert.deepEqual(errors, []);
      results.push({ browser: name, motion, zoomMotion, automaticSameFontImport:true, errors });
    } finally {
      await browser.close();
    }
  }
  fs.writeFileSync(
    path.join(out, "results.json"),
    JSON.stringify(results, null, 2),
  );
  console.log(JSON.stringify(results));
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
