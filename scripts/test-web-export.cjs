// Real file:// playback, editing/download round trips, codecs and ZIP extraction.
const fs = require("fs"),
  path = require("path"),
  os = require("os"),
  assert = require("assert/strict"),
  cp = require("child_process"),
  http = require("http");
const { createHash } = require("crypto");
const { chromium, firefox, webkit } = require(
  process.env.PLAYWRIGHT_MODULE || "playwright",
);
const root = path.resolve(__dirname, ".."),
  out = path.resolve(
    process.argv[2] || path.join(os.tmpdir(), "laymesh-web-export-qa"),
  ),
  binary =
    process.env.LAYMESH_TEST_BINARY ||
    path.join(root, "target/release/laymesh");
fs.mkdirSync(out, { recursive: true });
const run = (args) =>
  JSON.parse(
    cp.execFileSync(binary, args, {
      cwd: root,
      encoding: "utf8",
      maxBuffer: 8 * 1048576,
    }),
  );
(async () => {
  const archive = await import(path.join(root, "site/live/web-archive.mjs"));
  const bootstrap = fs.readFileSync(
    path.join(root, "target/web-export/laymesh-wasm/laymesh_wasm.js"),
  ); // build must exist
  assert(bootstrap.length > 0);
  const rt = archive.webUnzip(
      fs.readFileSync(path.join(root, "target/web-export/runtime.zip")),
    ),
    bootText = new TextDecoder().decode(rt["runtime/bootstrap.js"]);
  const codecBoot = bootText.replace(
    /export async function webBoot\(files\)\s*\{[\s\S]*$/,
    'export async function webBoot(files){window.codec=files;document.getElementById("app").textContent="Codec ready";}',
  );
  const sample = Uint8Array.from({ length: 1024 * 1024 }, (_, i) => i % 256),
    attack = new TextEncoder().encode(
      "<script>window.injected=true</script>\r\n\0<!-- -->",
    ),
    codecZip = archive.webZip({
      "全部字节.bin": [sample, { level: 0 }],
      "结束标记.bin": [attack, { level: 0 }],
    });
  fs.writeFileSync(
    path.join(out, "codec.zip.html"),
    Buffer.from(await archive.webUniversal(codecZip, codecBoot).arrayBuffer()),
  );
  assert.equal(
    archive.webCrc32(archive.webUnzip(codecZip)["全部字节.bin"]),
    archive.webCrc32(sample),
  );
  const exports = [
    run([
      "render",
      "examples/animation/presentation.laydoc",
      "-o",
      path.join(out, "play.zip.html"),
    ]),
    run([
      "render",
      "examples/animation/presentation.laydoc",
      "-o",
      path.join(out, "edit.zip.html"),
      "--html-editable",
      "--html-embed-fonts",
    ]),
    run([
      "render",
      "examples/animation/presentation.laydoc",
      "-o",
      path.join(out, "plain.zip"),
    ]),
  ];
  for (const file of ["play.zip.html", "edit.zip.html", "plain.zip"]) {
    const bytes = fs.readFileSync(path.join(out, file)),
      entries = archive.webUnzip(archive.webPayload(bytes)),
      manifest = JSON.parse(new TextDecoder().decode(entries["manifest.json"]));
    assert.equal(manifest.pages.length, 2);
    for (const [name, data] of Object.entries(entries))
      if (name.startsWith("assets/"))
        assert.equal(
          name.slice(7),
          createHash("sha256").update(data).digest("hex"),
        );
    assert.equal(manifest.fonts.length > 0, manifest.editable);
    assert.equal(
      Object.keys(entries).some((n) => n.startsWith("runtime/compiler/")),
      manifest.editable,
    );
    for (const p of manifest.pages) {
      const scene = JSON.parse(new TextDecoder().decode(entries[p.file]));
      assert(!JSON.stringify(scene.list).includes('"kind":"glyphs"'));
      assert(
        Object.values(scene.list.resources).every((r) =>
          r.mime.startsWith("image/"),
        ),
      );
    }
  }
  const plain = archive.webUnzip(fs.readFileSync(path.join(out, "plain.zip")));
  const host = process.env.LAYMESH_TEST_HOST || "127.0.0.1",
    server = http.createServer((req, res) => {
      const name =
          decodeURIComponent(req.url.split("?")[0]).replace(/^\//, "") ||
          "index.html",
        b =
          plain[name] ||
          (name === "play.zip.html"
            ? fs.readFileSync(path.join(out, name))
            : undefined);
      if (!b) {
        res.writeHead(404);
        res.end();
        return;
      }
      res.setHeader(
        "Content-Type",
        name.endsWith(".html")
          ? "text/html; charset=utf-8"
          : name.endsWith(".mjs") || name.endsWith(".js")
            ? "text/javascript"
            : name.endsWith(".wasm")
              ? "application/wasm"
              : "application/octet-stream",
      );
      res.end(b);
    });
  await new Promise((r) => server.listen(0, host, r));
  const results = [];
  try {
    for (const [name, type] of Object.entries({ chromium, firefox, webkit })) {
      let browser;
      try {
        browser = await type.launch({
          headless: true,
          ...(name === "chromium" ? { args: ["--no-sandbox"] } : {}),
        });
        const context = await browser.newContext({
          viewport: { width: 1280, height: 800 },
          deviceScaleFactor: 2,
          acceptDownloads: true,
        });
        await context.route(/^https?:\/\//, (route) => route.abort());
        const page = await context.newPage(),
          errors = [];
        page.on("pageerror", (e) => errors.push(String(e)));
        await page.goto("file://" + path.join(out, "codec.zip.html"));
        await page.waitForFunction(() => window.codec);
        assert(
          await page.evaluate(() =>
            window.codec[Object.keys(window.codec)[0]].every(
              (b, i) => b === i % 256,
            ),
          ),
        );
        assert.equal(await page.evaluate(() => !!window.injected), false);
        const started = Date.now();
        await page.goto("file://" + path.join(out, "play.zip.html"));
        await page.waitForFunction(
          () => document.getElementById("app")?.dataset.painted === "true",
          null,
          { timeout: 60000 },
        );
        const readyMs = Date.now() - started;
        assert.equal(await page.title(), "LCSS animation presentation");
        await page.locator("#preview-more summary").click();
        await page.locator("#static").click();
        await page.locator("#preview-more summary").click();
        await page.waitForTimeout(150);
        await page.screenshot({ path: path.join(out, name + "-play.png") });
        await page.locator("#play").click();
        await page.waitForFunction(() => window.__LAYMESH_WEB.clock.time > 0.2);
        await page.locator("#play").click();
        assert.equal(
          await page.evaluate(() => window.__LAYMESH_WEB.metrics.compiles),
          0,
        );
        await page.locator("#page").selectOption("1");
        await page.waitForFunction(
          () =>
            document.getElementById("app").dataset.painted === "true" &&
            document.querySelector("#page").value === "1",
        );
        assert.equal(
          await page.evaluate(() => window.__LAYMESH_WEB.metrics.compiles),
          0,
        );
        await page.locator("#seek").evaluate((el) => {
          el.value = "1.1";
          el.dispatchEvent(new Event("input", { bubbles: true }));
        });
        await page.waitForTimeout(150);
        const metrics = await page.evaluate(() => window.__LAYMESH_WEB.metrics);
        const canvasBox=await page.locator('.preview-figure > canvas:not(.preview-overview)').boundingBox();
        assert(Math.abs(metrics.width-canvasBox.width*2)<1.1);
        assert(metrics.width<=Math.round((await page.locator('.stage').boundingBox()).width*2));
        await page.goto("file://" + path.join(out, "edit.zip.html"));
        await page.waitForFunction(
          () => document.getElementById("app")?.dataset.painted === "true",
          null,
          { timeout: 60000 },
        );
        assert.equal(
          await page.evaluate(() => window.__LAYMESH_WEB.metrics.compiles),
          0,
        );

        await page.waitForFunction(
          () => document.getElementById("app").dataset.editorReady === "true",
          null,
          { timeout: 60000 },
        );
        await page
          .locator('.tree-file[title="/project/presentation.lay"]')
          .click();
        const code = page.locator(".source-panel:not([hidden]) .cm-content");
        await code.click();
        await code.press("ControlOrMeta+a");
        await code.press("Backspace");
        await code.pressSequentially(
          'page = canvas(size=(160mm,90mm), background="#ffffff")\npage.add(text("新增中文 龘 Results",font_size=12pt),offset=(10mm,20mm))',
          { delay: 0 },
        );
        await page.waitForFunction(
          () =>
            ["ready", "error"].includes(
              document.getElementById("app").dataset.compileState,
            ),
          null,
          { timeout: 60000 },
        );
        assert.equal(
          await page.locator("#app").getAttribute("data-compile-state"),
          "ready",
          await page.locator("#status").innerText(),
        );
        assert(!(await page.locator("#status").innerText()).includes("错误"));
        if (name === "chromium") {
          const count = await page.evaluate(
            () => window.__LAYMESH_WEB.metrics.compiles,
          );
          await code.dispatchEvent("compositionstart");
          await page.evaluate(() => {
            const e = window.__LAYMESH_WEB.editor;
            const view = e.views.get("/project/presentation.lay").view;
            view.dispatch({
              changes: { from: view.state.doc.length, insert: " " },
            });
          });
          await page.waitForTimeout(350);
          assert.equal(
            await page.evaluate(() => window.__LAYMESH_WEB.metrics.compiles),
            count,
          );
          assert.equal(
            await page.evaluate(() => window.__LAYMESH_WEB.clock.running),
            false,
          );
          await code.dispatchEvent("compositionend");
          await page.waitForFunction(
            () =>
              document.getElementById("app").dataset.compileState === "ready",
          );
        }
        const editorMetrics = await page.evaluate(() => ({
          ...window.__LAYMESH_WEB.metrics,
        }));
        assert(
          editorMetrics.compileMs > 0 && editorMetrics.compilerMemoryBytes > 0,
        );
        await page.screenshot({ path: path.join(out, name + "-editor.png") });
        const downloadPromise = page.waitForEvent("download");
        await page.locator("#export").click();
        await page.locator("#save").click();
        const download = await downloadPromise;
        const saved = path.join(out, name + "-edited.zip.html");
        await download.saveAs(saved);
        const edited = archive.webUnzip(
          archive.webPayload(fs.readFileSync(saved)),
        );
        const m = JSON.parse(new TextDecoder().decode(edited["manifest.json"]));
        assert(
          new TextDecoder()
            .decode(edited[m.project.files["/project/presentation.lay"]])
            .includes("新增中文"),
        );
        assert(edited["entries.json"]);
        assert(Object.keys(edited).some((p) => p.startsWith("licenses/")));
        await page.goto("file://" + saved);
        await page.waitForFunction(
          () => document.getElementById("app").dataset.painted === "true",
          null,
          { timeout: 60000 },
        );
        assert.equal(
          await page.evaluate(() => window.__LAYMESH_WEB.metrics.compiles),
          0,
        );
        if (name === "chromium") {
          await page.waitForFunction(
            () => document.getElementById("app").dataset.editorReady === "true",
          );
          await page
            .locator('.tree-file[title="/project/presentation.lay"]')
            .click();
          const input = page.locator(".source-panel:not([hidden]) .cm-content");
          await input.press("Escape");
          await page.waitForTimeout(250);
          const previous = await page.locator(".preview-figure > canvas:not(.preview-overview)").screenshot();
          await input.fill(
            "page=canvas(size=(160mm,90mm))\npage.add(unknown_function())",
          );
          await page.waitForFunction(
            () =>
              document.getElementById("app").dataset.compileState === "error",
          );
          await input.press("Escape");
          await page.waitForTimeout(250);
          assert.equal(
            Buffer.compare(
              previous,
              await page.locator(".preview-figure > canvas:not(.preview-overview)").screenshot(),
            ),
            0,
          );
          await page
            .locator("#import")
            .setInputFiles(path.join(root, "tests/assets/photo.jpg"));
          await page
            .locator('.tree-file[title="/project/presentation.lay"]')
            .click();
          await input.fill(
            'page=canvas(size=(160mm,90mm),background="#ffffff")\npage.add(image(src="photo.jpg"),size=(80mm,50mm),offset=(10mm,10mm))',
          );
          await page.waitForFunction(
            () =>
              document.getElementById("app").dataset.compileState === "ready",
          );
          assert(
            await page.evaluate(
              () => "/project/photo.jpg" in window.__LAYMESH_WEB.editor.assets,
            ),
          );
          await page
            .locator("#import")
            .setInputFiles(path.join(root, "tests/fonts/DejaVuSans.ttf"));
          await page
            .locator('.tree-file[title="/project/presentation.lay"]')
            .click();
          await input.fill(
            'page=canvas(size=(160mm,90mm),background="#ffffff")\npage.add(image(src="photo.jpg"),size=(80mm,50mm),offset=(10mm,10mm))\npage.add(text("Imported font",font_family="./DejaVuSans.ttf",font_size=12pt),offset=(10mm,70mm))',
          );
          await page.waitForFunction(
            () =>
              document.getElementById("app").dataset.compileState === "ready",
          );
          await page
            .locator('.tree-file[title="/project/motion.lcss"]')
            .click();
          await page.evaluate(() => {
            const view = window.__LAYMESH_WEB.editor.views.get(
              "/project/motion.lcss",
            ).view;
            view.dispatch({
              selection: {
                anchor:
                  view.state.doc.toString().indexOf("@laymesh/animate.css") + 8,
              },
            });
            view.focus();
          });
          await page
            .locator(".source-panel:not([hidden]) .cm-content")
            .press("F12");
          await page
            .getByRole("tab", { name: "animate.css", exact: true })
            .waitFor();
          assert(
            await page.evaluate(
              () =>
                window.__LAYMESH_WEB.editor.files[
                  "laymesh-library:/animate.css"
                ].includes("@keyframes") &&
                window.__LAYMESH_WEB.editor.readonly.has(
                  "laymesh-library:/animate.css",
                ),
            ),
          );

          assert(
            await page.evaluate(() => {
              const view = window.__LAYMESH_WEB.editor.views.get(
                  "laymesh-library:/animate.css",
                ).view,
                original = view.state.doc.toString();
              view.dispatch({ changes: { from: 0, insert: "Edited" } });
              return view.state.doc.toString() === original;
            }),
          );
          await page.setViewportSize({ width: 450, height: 850 });
          await page.waitForTimeout(200);
          assert(
            await page.evaluate(
              () => document.documentElement.scrollWidth <= innerWidth,
            ),
          );
          await page.screenshot({
            path: path.join(out, "chromium-import-mobile.png"),
          });
          const readonly = page.waitForEvent("download");
          await page.locator("#export").click();
          await page.locator("#download").click();
          const imported = path.join(out, "imported-play.zip.html");
          await (await readonly).saveAs(imported);
          await page.goto("file://" + imported);
          await page.waitForFunction(
            () => document.getElementById("app").dataset.painted === "true",
          );
          assert.equal(
            await page.evaluate(() => window.__LAYMESH_WEB.metrics.compiles),
            0,
          );
          assert.equal(await page.locator("#fonts").isVisible(), false);
        }
        await context.unroute(/^https?:\/\//);
        await page.goto("http://" + host + ":" + server.address().port + "/");
        await page.waitForFunction(
          () => document.getElementById("app").dataset.painted === "true",
          null,
          { timeout: 60000 },
        );
        assert.equal(
          await page.evaluate(() => window.__LAYMESH_WEB.pages.length),
          2,
        );
        await page.goto(
          "http://" + host + ":" + server.address().port + "/play.zip.html",
        );
        await page.waitForFunction(
          () => document.getElementById("app").dataset.painted === "true",
          null,
          { timeout: 60000 },
        );
        if (name === "chromium") {
          for (const example of [
            "keyframes",
            "art-direction",
            "complex-dashboard",
          ]) {
            const exported = path.join(out, example + ".zip.html");
            run([
              "render",
              "examples/animation/" + example + ".lay",
              "-o",
              exported,
              ...(example === "art-direction"
                ? ["--html-editable", "--html-embed-fonts"]
                : []),
            ]);
            await page.goto("file://" + exported);
            await page.waitForFunction(
              () => document.getElementById("app").dataset.painted === "true",
              null,
              { timeout: 60000 },
            );
            await page.locator("#preview-more summary").click();
            await page.locator("#static").click();
            await page.locator("#preview-more summary").click();
            await page.waitForFunction(
              () => document.getElementById("app").dataset.painting === "false",
            );
            await page.waitForTimeout(300);
            assert.equal(
              await page.evaluate(() => window.__LAYMESH_WEB.viewStatic),
              true,
            );
            const canvasShot = path.join(out, example + "-canvas.png");
            await page
              .locator(".preview-figure > canvas:not(.preview-overview)")
              .screenshot({ path: canvasShot });
            cp.execFileSync("python", [
              "-c",
              "from PIL import Image; import sys; e=Image.open(sys.argv[1]).convert('RGB').getextrema(); assert max(b-a for a,b in e)>30, 'Static canvas is blank'",
              canvasShot,
            ]);
            await page.screenshot({
              path: path.join(out, example + "-static.png"),
            });
            await page.locator("#seek").evaluate((input) => {
              input.value = String(Number(input.max) / 2);
              input.dispatchEvent(new Event("input", { bubbles: true }));
            });
            await page.waitForTimeout(150);
            assert.equal(
              await page.evaluate(() => window.__LAYMESH_WEB.metrics.compiles),
              0,
            );
            if (example === "art-direction") {
              await page.waitForFunction(
                () =>
                  document.getElementById("app").dataset.editorReady === "true",
              );
              await page
                .getByRole("tab", { name: "art-direction.lay", exact: true })
                .click();
              const source = await page.evaluate(
                () =>
                  window.__LAYMESH_WEB.editor.files[
                    window.__LAYMESH_WEB.editor.entry
                  ],
              );
              await page
                .locator(".source-panel:not([hidden]) .cm-content")
                .fill(
                  source +
                    "\n# Verify packaged image paths after recompilation\n",
                );
              await page.waitForFunction(
                () =>
                  ["ready", "error"].includes(
                    document.getElementById("app").dataset.compileState,
                  ),
                null,
                { timeout: 60000 },
              );
              assert.equal(
                await page.locator("#app").getAttribute("data-compile-state"),
                "ready",
                await page.locator("#status").innerText(),
              );
            }
          }
          await page.addInitScript(() => {
            delete HTMLCanvasElement.prototype.transferControlToOffscreen;
          });
          await page.emulateMedia({ reducedMotion: "reduce" });
          await page.goto("file://" + path.join(out, "play.zip.html"));
          await page.waitForFunction(
            () => document.getElementById("app").dataset.painted === "true",
          );
          assert.equal(
            await page.evaluate(() => window.__LAYMESH_WEB.metrics.compiles),
            0,
          );
          assert.equal(
            await page.evaluate(() => window.__LAYMESH_WEB.clock.duration),
            await page.evaluate(
              () =>
                window.__LAYMESH_WEB.pages[0].inspection.animation
                  .reducedDuration,
            ),
          );
        }
        assert.deepEqual(errors, []);
        results.push({
          browser: name,
          readyMs,
          metrics,
          editorMetrics,
          checks: [
            "all bytes and HTML markers",
            "offline playback without compilation",
            "DPR 2 and seek",
            "offline editor and complete CJK fonts",
            "editable download/reopen",
            "standard ZIP HTTP",
            "Universal HTTP with UTF-8 headers",
            ...(name === "chromium"
              ? [
                  "error preserves pixels",
                  "image import",
                  "font import",
                  "bundled-library definition",
                  "mobile viewport",
                  "readonly download/reopen",
                  "main-thread renderer fallback",
                  "reduced motion",
                  "keyframes, artistic text, alpha images and dashboard",
                  "editable packaged image paths",
                  "IME composition defers compilation",
                ]
              : []),
          ],
        });
      } finally {
        await browser?.close();
      }
    }
  } finally {
    server.close();
  }
  cp.execFileSync("python", [
    "-c",
    "import sys,zipfile; [zipfile.ZipFile(p).testzip() is None or sys.exit(1) for p in sys.argv[1:]]",
    ...["play.zip.html", "edit.zip.html", "plain.zip"].map((f) =>
      path.join(out, f),
    ),
  ]);
  fs.writeFileSync(
    path.join(out, "results.json"),
    JSON.stringify({ exports, results }, null, 2),
  );
  console.log(JSON.stringify({ exports, results }, null, 2));
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
