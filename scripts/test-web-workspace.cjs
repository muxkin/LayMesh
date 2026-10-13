// Portable workbench, fonts and the same Presentation used by site/VS Code.
const fs = require("fs"),
  path = require("path"),
  assert = require("assert/strict"),
  cp = require("child_process");
const engines = require(process.env.PLAYWRIGHT_MODULE || "playwright");
const root = path.resolve(__dirname, ".."),
  out = path.resolve(process.argv[2] || "/tmp/laymesh-workspace-qa"),
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
  const exports = {};
  for (const [name, flags] of [
    ["editable", []],
    ["embedded", ["--html-embed-fonts"]],
    ["play", []],
  ]) {
    exports[name] = run([
      "render",
      "examples/animation/presentation.laydoc",
      "-o",
      path.join(out, name + ".zip.html"),
      ...(name === "play" ? [] : ["--html-editable"]),
      ...flags,
    ]);
  }
  const unpack = (file) =>
    archive.webUnzip(archive.webPayload(fs.readFileSync(file)));
  const defaultFiles = unpack(path.join(out, "editable.zip.html")),
    m = JSON.parse(new TextDecoder().decode(defaultFiles["manifest.json"]));
  assert(m.fontRequirements.length);
  assert(m.fonts.every((f) => !f.file));
  assert(!m.embedFonts);
  // Font sizes vary across machines, and runtime codecs are shared by both
  // packages. Verify the actual embedding contract instead of a size ratio.
  const embeddedFiles = unpack(path.join(out, "embedded.zip.html")),
    embedded = JSON.parse(new TextDecoder().decode(embeddedFiles["manifest.json"]));
  assert(embedded.embedFonts);
  assert(embedded.fonts.length);
  for (const font of embedded.fonts) {
    assert(font.file && embeddedFiles[font.file]?.length, "Requested font bytes must be bundled");
    assert(!defaultFiles[font.file], "Default export must omit system font bytes");
  }
  assert(exports.editable.bytes < exports.embedded.bytes);
  const results = [];
  for (const name of ["chromium", "firefox", "webkit"]) {
    const browser = await engines[name].launch({
      headless: true,
      ...(name === "chromium" ? { args: ["--no-sandbox"] } : {}),
    });
    const context = await browser.newContext({
      viewport: { width: 1586, height: 992 },
      acceptDownloads: true,
    });
    await context.route(/^https?:\/\//, (r) => r.abort());
    const page = await context.newPage(),
      errors = [];
    page.on("pageerror", (e) => errors.push(e.message));
    const ready = () =>
      page.waitForFunction(
        () =>
          document.querySelector("#app")?.dataset.editorReady === "true" &&
          document.querySelector("#app")?.dataset.painted === "true",
        null,
        { timeout: 60000 },
      );
    const started = Date.now();
    await page.goto("file://" + path.join(out, "editable.zip.html"));
    await ready();
    const startMs = Date.now() - started;
    assert.equal(await page.title(), "LCSS animation presentation");
    assert.equal(
      await page.evaluate(() => window.__LAYMESH_WEB.metrics.compiles),
      0,
    );
    assert(await page.locator(".workspace-tree").isVisible());
    assert(await page.locator(".cm-content").isVisible());
    assert.equal(
      await page.evaluate(() => window.__LAYMESH_WEB.viewStatic),
      true,
    );
    await page.locator("#page").selectOption("1");
    await page.waitForFunction(
      () => document.querySelector("#app").dataset.painted === "true",
    );
    // Fit, actual viewport zoom and pan, without compiling.
    await page.locator("#zoom").selectOption("2");
    await page.waitForTimeout(200);
    await page.locator(".stage").hover();
    await page.mouse.wheel(0, 80);
    await page.waitForTimeout(200);
    const saved = await page.evaluate(() => ({
      index: document.querySelector("#page").value,
      scale: window.__LAYMESH_WEB.view.scale,
      pan: window.__LAYMESH_WEB.view.pan.slice(),
    }));
    for (let i = 0; i < 2; i++) {
      await page.locator("#full").click();
      await page.waitForFunction(
        () => window.__LAYMESH_WEB.presentation.ready,
        null,
        { timeout: 60000 },
      );
      assert.equal(
        await page.evaluate(() => window.__LAYMESH_WEB.presentation.index),
        0,
      );
      assert(await page.locator(".presentation-canvas").isVisible());
      if (i === 0) {
        assert(
          await page
            .locator(".presentation-bar")
            .evaluate((e) => e.classList.contains("idle")),
        );
        await page.keyboard.press("Tab");
        assert(await page.locator(".presentation-bar").isVisible());
        await page.locator(".presentation").focus();
      }
      await page.keyboard.press("ArrowRight");
      await page.waitForTimeout(80);
      await page.keyboard.press("Escape");
      await page.waitForFunction(
        () => !window.__LAYMESH_WEB.presentation.opened,
      );
      assert.equal(await page.locator(".presentation").count(), 0);
      assert.equal(
        await page.evaluate(
          () => window.__LAYMESH_WEB.presentation.worker == null,
        ),
        true,
      );
      assert.deepEqual(
        await page.evaluate(() => ({
          index: document.querySelector("#page").value,
          scale: window.__LAYMESH_WEB.view.scale,
          pan: window.__LAYMESH_WEB.view.pan.slice(),
        })),
        saved,
      );
    }
    await page.locator("#page").selectOption("0");
    await page.waitForFunction(
      () => document.querySelector("#app").dataset.painted === "true",
    );
    await page.locator("#play").click();
    await page.waitForFunction(() => window.__LAYMESH_WEB.clock.time > 0.1);
    await page.locator("#preview-more summary").click();
    await page.locator("#windowed").click();
    await page.waitForFunction(() => window.__LAYMESH_WEB.presentation.ready);
    await page.setViewportSize({ width: 1480, height: 940 });
    await page.waitForTimeout(150);
    await page.keyboard.press("Escape");
    await page.waitForFunction(
      () =>
        !window.__LAYMESH_WEB.presentation.opened &&
        window.__LAYMESH_WEB.clock.running,
    );
    await page.locator("#play").click();
    await page.setViewportSize({ width: 1586, height: 992 });
    await page.locator("#preview-more summary").click();
    await page.locator("#static").click();
    await page.locator("#preview-more summary").click();
    await page.locator("#fit").click();
    await page.waitForTimeout(200);
    // Imported image is visible immediately and closing does not remove resource.
    await page
      .locator("#import")
      .setInputFiles(path.join(root, "examples/effects/alpha-ring.png"));
    await page.locator(".resource-viewer img").waitFor();
    await page.waitForFunction(
      () =>
        document.querySelector(".resource-viewer img").naturalWidth === 1024,
    );
    assert(
      (await page.locator(".resource-info").innerText()).includes(
        "1024 × 1024",
      ),
    );
    assert(
      await page
        .locator('.tree-file[title="/project/alpha-ring.png"]')
        .isVisible(),
    );
    await page.screenshot({ path: path.join(out, name + "-resource.png") });
    await page
      .getByRole("button", { name: "关闭 alpha-ring.png", exact: true })
      .click();
    await page.locator('.tree-file[title="/project/alpha-ring.png"]').click();
    assert(await page.locator(".resource-viewer img").isVisible());
    await page
      .locator("#import")
      .setInputFiles(path.join(root, "examples/effects/alpha-ring.png"));
    await page.getByRole("button", { name: "保留两份", exact: true }).click();
    await page.waitForFunction(
      () => !!window.__LAYMESH_WEB.editor.assets["/project/alpha-ring (2).png"],
    );
    await page
      .locator("#import")
      .setInputFiles(path.join(root, "examples/effects/alpha-ring.png"));
    await page.getByRole("button", { name: "替换", exact: true }).click();
    await page
      .locator("#import")
      .setInputFiles(path.join(root, "tests/assets/gray8.tiff"));
    await page
      .locator(".source-panel:not([hidden]) .resource-viewer img")
      .waitFor();
    assert.equal(
      await page.evaluate(() => window.__LAYMESH_WEB.metrics.compiles),
      0,
    );
    await page.locator("#export").click();
    const untouched = page.waitForEvent("download");
    await page.locator("#save").click();
    const untouchedFile = path.join(out, name + "-resources-only.zip.html");
    await (await untouched).saveAs(untouchedFile);
    const untouchedContents = unpack(untouchedFile),
      untouchedManifest = JSON.parse(
        new TextDecoder().decode(untouchedContents["manifest.json"]),
      );
    assert.deepEqual(
      Buffer.from(
        untouchedContents[
          untouchedManifest.project.files["/project/gray8.tiff"]
        ],
      ),
      fs.readFileSync(path.join(root, "tests/assets/gray8.tiff")),
    );
    assert.equal(
      await page.evaluate(() => window.__LAYMESH_WEB.metrics.compiles),
      0,
    );
    assert(untouchedManifest.fonts.every((f) => !f.file));

    await page.locator('.tree-file[title="/project/presentation.lay"]').click();
    await page.evaluate(() => {
      const v = window.__LAYMESH_WEB.editor.views.get(
        "/project/presentation.lay",
      ).view;
      v.dispatch({
        changes: {
          from: v.state.doc.length,
          insert: "\n# Font readiness verification\n",
        },
      });
    });
    await page.waitForFunction(
      () =>
        ["fonts", "ready", "error"].includes(
          document.querySelector("#app").dataset.compileState,
        ),
      null,
      { timeout: 60000 },
    );
    // A missing font blocks layout but never destroys the already drawn scene.
    assert.equal(
      await page.evaluate(() => window.__LAYMESH_WEB.metrics.compiles),
      0,
    );
    assert.equal(
      await page.locator("#app").getAttribute("data-compile-state"),
      "fonts",
    );
    await page.locator('.tree-file[title="/project/presentation.lay"]').click();
    await page.screenshot({ path: path.join(out, name + "-source.png") });
    if (name === "chromium") {
      await page.evaluate(() => {
        window.originalLocalFonts = window.queryLocalFonts;
        window.queryLocalFonts = async () => {
          throw new DOMException("Denied", "NotAllowedError");
        };
      });
      await page.locator("#fonts").click();
      await page.locator("#connect-fonts").click();
      assert(
        (await page.locator(".font-notice").innerText()).includes("未获得"),
      );
      await page.locator("#font-panel .dialog-close").click();
      assert.equal(
        await page.evaluate(() => window.__LAYMESH_WEB.metrics.compiles),
        0,
      );
      await page.evaluate(
        () => (window.queryLocalFonts = window.originalLocalFonts),
      );
      await context.grantPermissions(["local-fonts"]);
      await page.locator("#fonts").click();
      await page.locator("#connect-fonts").click();
      await page.waitForFunction(() =>
        window.__LAYMESH_WEB.workspace.liveFonts.some(
          (f) => f.source === "local",
        ),
      );
      await page.locator("#font-apply").click();
      await page.waitForFunction(
        () =>
          document.querySelector("#app").dataset.compileState === "ready" &&
          !document.querySelector("#font-panel").open,
        null,
        { timeout: 60000 },
      );
    } else {
      await page.locator("#fonts").click();
      await page.locator("#connect-fonts").click();
      assert(
        (await page.locator(".font-notice").innerText()).includes("不支持"),
      );
      await page.locator("#font-panel .dialog-close").click();
      await page.locator("#fonts").click();
      const chooser = page.waitForEvent("filechooser");
      await page.locator("#font-import").click();
      await (
        await chooser
      ).setFiles(path.join(root, "tests/fonts/DejaVuSans.ttf"));
      await page.locator("#font-panel .dialog-close").click();
      await page.waitForFunction(() =>
        window.__LAYMESH_WEB.workspace.liveFonts.some(
          (f) => f.source === "project",
        ),
      );
      await page.evaluate(()=>__LAYMESH_WEB.workspace.whenIdle());
      await page.waitForFunction(()=>document.querySelector("#app").dataset.compileState==="fonts");
      await page.locator("#fonts").click();
      for (const input of await page
        .locator('.font-row [role="combobox"]')
        .all()) {
        await input.fill("DejaVu Sans");
        await page
          .locator('.font-options:not([hidden]) [role="option"]')
          .first()
          .click();
      }
      await page.locator("#font-apply").click();
      await page.waitForFunction(
        () => document.querySelector("#app").dataset.compileState === "ready",
        null,
        { timeout: 60000 },
      );
    }
    if (name === "chromium") {
      await page
        .locator("#import")
        .setInputFiles(path.join(root, "tests/fonts/DejaVuSans.ttf"));
      await page.locator('.tree-file[title="/project/DejaVuSans.ttf"]').waitFor();
      await page.evaluate(()=>__LAYMESH_WEB.workspace.whenIdle());
      await page
        .locator('.tree-file[title="/project/presentation.lay"]')
        .click();
      const input = page.locator(".source-panel:not([hidden]) .cm-content");
      await input.fill(
        'page=canvas(size=(160mm,90mm),background="#ffffff")\npage.add(text("Font test",font_family="./DejaVuSans.ttf"),offset=(10mm,10mm))',
      );
      await page.waitForFunction(
        () => document.querySelector("#app").dataset.compileState === "ready",
      );
      await page.waitForTimeout(200);
      const success = await page
        .locator(".preview-figure > canvas:not(.preview-overview)")
        .screenshot();
      await input.fill(
        'page=canvas(size=(160mm,90mm),background="#ffffff")\npage.add(text("Font test",font_family="LayMeshMissingFace"),offset=(10mm,10mm))',
      );
      await page.waitForFunction(
        () => document.querySelector("#app").dataset.compileState === "fonts",
      );
      await page.waitForTimeout(200);
      assert.deepEqual(
        await page
          .locator(".preview-figure > canvas:not(.preview-overview)")
          .screenshot(),
        success,
      );
      await page.locator("#fonts").click();
      await page
        .getByLabel("替换 LayMeshMissingFace", { exact: true })
        .fill("DejaVu Sans");
      await page
        .locator('.font-options:not([hidden]) [role="option"]')
        .first()
        .click();
      await page.locator("#font-apply").click();
      await page.waitForFunction(
        () => document.querySelector("#app").dataset.compileState === "ready",
      );
      await input.fill(
        'page=canvas(size=(160mm,90mm),background="#ffffff")\npage.add(image(src="alpha-"+"ring.png"),size=(30mm,30mm))',
      );
      await page.waitForFunction(
        () => document.querySelector("#app").dataset.compileState === "ready",
      );
      assert(
        await page.evaluate(
          () =>
            window.__LAYMESH_WEB.workspace.editor.assets[
              "/project/alpha-ring.png"
            ] && window.__LAYMESH_WEB.metrics.compiles > 0,
        ),
      );
      const count = await page.evaluate(
        () => window.__LAYMESH_WEB.metrics.compiles,
      );
      await page
        .locator("#import")
        .setInputFiles(path.join(root, "examples/effects/alpha-ring.png"));
      await page.getByRole("button", { name: "替换", exact: true }).click();
      await page.waitForFunction(
        (n) =>
          window.__LAYMESH_WEB.metrics.compiles > n &&
          document.querySelector("#app").dataset.compileState === "ready",
        count,
      );
      await page
        .locator('.tree-file[title="/project/presentation.lay"]')
        .click();
      await page.evaluate(
        () =>
          (window.heldView = window.__LAYMESH_WEB.editor.views.get(
            "/project/presentation.lay",
          ).view),
      );
      await page
        .getByRole("button", { name: "关闭 presentation.lay", exact: true })
        .click();
      await page
        .locator('.tree-file[title="/project/presentation.lay"]')
        .click();
      assert(
        await page.evaluate(
          () =>
            window.heldView ===
            window.__LAYMESH_WEB.editor.views.get("/project/presentation.lay")
              .view,
        ),
      );
    }
    const before = await page.evaluate(
      () => window.__LAYMESH_WEB.metrics.compiles,
    );
    await page.locator("#page").selectOption("1");
    await page.waitForTimeout(200);
    await page.locator("#play").click();
    await page.waitForTimeout(200);
    await page.locator("#play").click();
    assert.equal(
      await page.evaluate(() => window.__LAYMESH_WEB.metrics.compiles),
      before,
    );
    // Save new resources without collecting local font bytes, reopen offline.
    await page.locator("#export").click();
    assert.equal(await page.locator("#embed-fonts").isChecked(), false);
    const dl = page.waitForEvent("download");
    await page.locator("#save").click();
    const downloaded = await dl;
    assert(downloaded.suggestedFilename().endsWith(".zip.html"));
    const savedFile = path.join(out, name + "-updated.zip.html");
    await downloaded.saveAs(savedFile);
    const contents = unpack(savedFile),
      next = JSON.parse(new TextDecoder().decode(contents["manifest.json"]));
    assert(next.project.files["/project/alpha-ring.png"]);
    assert(next.project.files["/project/alpha-ring (2).png"]);
    assert(
      next.fonts.every((f) => f.source !== "local" && f.source !== "system"),
    );
    assert(Object.keys(contents).some((k) => k.startsWith("licenses/")));
    await page.goto("file://" + savedFile);
    await ready();
    await page.locator('.tree-file[title="/project/alpha-ring.png"]').click();
    await page.locator(".resource-viewer img").waitFor();
    assert.equal(
      await page.evaluate(() => window.__LAYMESH_WEB.metrics.compiles),
      0,
    );
    await page.setViewportSize({ width: 450, height: 850 });
    await page.waitForTimeout(250);
    assert(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= innerWidth,
      ),
    );
    await page.screenshot({ path: path.join(out, name + "-mobile.png") });
    await page.setViewportSize({ width: 3840, height: 2160 });
    await page.waitForTimeout(200);
    await page.screenshot({ path: path.join(out, name + "-4k.png") });
    // Plain playback first frame is the complete layout; no compiler included.
    await page.goto("file://" + path.join(out, "play.zip.html"));
    await page.waitForFunction(
      () => document.querySelector("#app")?.dataset.painted === "true",
      null,
      { timeout: 60000 },
    );
    assert(await page.evaluate(() => window.__LAYMESH_WEB.viewStatic));
    await page.setViewportSize({ width: 1586, height: 992 });
    await page.waitForTimeout(150);
    await page.screenshot({ path: path.join(out, name + "-play.png") });
    assert.deepEqual(errors, []);
    results.push({
      browser: name,
      startMs,
      metrics: await page.evaluate(() => window.__LAYMESH_WEB.metrics),
      errors,
    });
    await browser.close();
  }
  fs.writeFileSync(
    path.join(out, "workspace-results.json"),
    JSON.stringify({ exports, results }, null, 2),
  );
  console.log(JSON.stringify({ exports, results }, null, 2));
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
