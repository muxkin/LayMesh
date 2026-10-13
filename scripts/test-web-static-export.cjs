// Export actual downloadable documents from file:// with all networking blocked.
const fs = require("fs"),
  path = require("path"),
  assert = require("assert/strict"),
  cp = require("child_process");
const engines = require(process.env.PLAYWRIGHT_MODULE || "playwright"),
  root = path.resolve(__dirname, ".."),
  out = path.resolve(process.argv[2] || "/tmp/laymesh-static-export");
fs.mkdirSync(out, { recursive: true });
const binary =
  process.env.LAYMESH_TEST_BINARY || root + "/target/release/laymesh";
fs.writeFileSync(
  out + "/one.lay",
  'p=canvas(size=(40mm,30mm),background="none")\np.add(rect(size=(10mm,8mm),fill="#e04020",border_color="none"),offset=(2mm,2mm))\np.add(text("Export 0.6.3",font_size=8pt),offset=(2mm,14mm))\np.add(rect(size=(6mm,6mm),fill="#2080ff",effects=[shadow(blur=0.5mm,offset=(0.5mm,0.5mm))]),offset=(25mm,3mm))\n',
);
fs.writeFileSync(
  out + "/two.lay",
  'p=canvas(size=(60mm,40mm),background="#ffffff")\np.add(rect(size=(10mm,10mm),fill="#20a060",border_color="none"))',
);
fs.writeFileSync(
  out + "/deck.laydoc",
  'd=document(name="Export test")\nd.add(page(src="one.lay",id="one"))\nd.add(page(src="two.lay",id="two"))',
);
const source = out + "/deck.zip.html";
cp.execFileSync(binary, [
  "render",
  out + "/deck.laydoc",
  "-o",
  source,
  "--html-editable",
]);
(async () => {
  const results = [];
  for (const engine of process.env.LAYMESH_TEST_ENGINES?.split(",") || [
    "chromium",
    "firefox",
    "webkit",
  ]) {
    const browser = await engines[engine].launch({ headless: true });
    let page;
    try {
      const context = await browser.newContext({
        acceptDownloads: true,
        viewport: { width: 1440, height: 1000 },
      });
      // Headless Chromium dismisses the native OS save picker. Exercise the
      // download fallback shared by all three engines; mock picker writes below.
      await context.addInitScript(() => {
        Object.defineProperty(window, "showSaveFilePicker", {
          value: undefined,
          configurable: true,
        });
      });
      const requests = [],
        errors = [];
      await context.route(/^https?:\/\//, (r) => {
        requests.push(r.request().url());
        return r.abort();
      });
      page = await context.newPage();
      page.on("pageerror", (e) => errors.push(e.message));
      page.setDefaultTimeout(120000);
      await page.goto("file://" + source);
      await page.waitForFunction(
        () =>
          window.__LAYMESH_WEB?.workspace &&
          document.querySelector("#app").dataset.painted === "true",
      );
      assert.equal(await page.title(), "Export test");
      const save = async (format, suffix = format, scope) => {
        await page.locator("#export").click();
        await page.locator("#export-format").selectOption(format);
        if (scope) await page.locator("#export-scope").selectOption(scope);
        const download = page.waitForEvent("download");
        await page.locator("#export-file").click();
        const file = await download;
        await file.saveAs(out + "/" + engine + "-" + suffix);
        await page.waitForFunction(
          () => !document.querySelector("#export-file").disabled,
        );
      };
      for (const format of [
        "pdf",
        "svg",
        "pptx",
        "png",
        "jpg",
        "tif",
        "webp",
        "bmp",
        "gif",
        "pam",
        "ppm",
        "pgm",
        "pbm",
        "tga",
      ])
        await save(format);
      assert.equal(
        await page.evaluate(() => __LAYMESH_WEB.metrics.compiles),
        0,
        "Static exports must not require fonts or compile",
      );
      await page.locator("#export").click();
      await page.locator("#export-format").selectOption("ico");
      await page.locator("#export-file").click();
      await page.waitForFunction(() =>
        document.querySelector("#export-progress").textContent.includes("256"),
      );
      assert(await page.locator("#export-panel").isVisible());
      await page.locator("#export-settings").click();
      assert.equal(await page.locator("#export-dpi").inputValue(), "1200");
      assert.equal(
        await page.locator("#export-tiff-compression").inputValue(),
        "lzw",
      );
      await page.locator("#export-dpi").fill("120");
      await page.locator("#export-settings-panel button[type=submit]").click();
      await page.locator("#export-panel .dialog-close").click();
      await save("ico");
      await save("svg", "pages.zip", "all");
      await page.locator("#page").selectOption({ index: 1 });
      await save("png", "second.png");
      await save("pdf", "single.pdf", "current");
      await page.locator("#export").click();
      await page.locator("#export-format").selectOption("png");
      await page.locator("#export-file").click();
      await page.locator("#export-cancel").click();
      await page.waitForFunction(
        () =>
          document.querySelector("#export-progress").textContent ===
          "已取消导出",
      );
      await page.locator("#export-panel .dialog-close").click();
      await page.locator("#export").click();
      await page.screenshot({ path: out + "/" + engine + "-dialog.png" });
      const dl = page.waitForEvent("download");
      await page.locator("#save").click();
      const updated = out + "/" + engine + "-updated.zip.html";
      await (await dl).saveAs(updated);
      await page.goto("file://" + updated);
      await page.waitForFunction(
        () =>
          window.__LAYMESH_WEB?.workspace &&
          document.querySelector("#app").dataset.painted === "true",
      );
      await page.locator("#export").click();
      await page.locator("#export-settings").click();
      assert.equal(await page.locator("#export-dpi").inputValue(), "120");
      await page.locator("#export-settings-panel .dialog-close").click();
      await page.locator("#export-panel .dialog-close").click();
      await save("png", "reopened.png");
      // The actual file-handle branch commits only after a successful write.
      await page.evaluate(() => {
        Object.defineProperty(window, "showSaveFilePicker", {
          configurable: true,
          value: async (options) => ({
            createWritable: async () => ({
              write: async (blob) => {
                window.savedExport = {
                  name: options.suggestedName,
                  text: await blob.text(),
                };
              },
              close: async () => {
                window.savedExport.closed = true;
              },
              abort: async () => {},
            }),
          }),
        });
      });
      await page.locator("#export").click();
      await page.locator("#export-format").selectOption("svg");
      await page.locator("#export-file").click();
      await page.waitForFunction(() => window.savedExport?.closed);
      assert((await page.evaluate(() => savedExport.text)).startsWith("<svg"));
      await page.evaluate(() => {
        Object.defineProperty(window, "showSaveFilePicker", {
          configurable: true,
          value: async () => ({
            createWritable: async () => ({
              write: async () => {
                throw Error("Test disk full");
              },
              close: async () => {},
              abort: async () => {
                window.saveAborted = true;
              },
            }),
          }),
        });
      });
      await page.locator("#export").click();
      await page.locator("#export-file").click();
      await page.waitForFunction(
        () =>
          window.saveAborted &&
          document
            .querySelector("#export-progress")
            .textContent.includes("Test disk full"),
      );
      assert(await page.locator("#export-panel").isVisible());
      await page.locator("#export-panel .dialog-close").click();
      await page.evaluate(() =>
        Object.defineProperty(window, "showSaveFilePicker", {
          value: undefined,
          configurable: true,
        }),
      );
      // Playback copies retain the codecs and vector snapshots as well.
      await page.locator("#export").click();
      const playbackDownload = page.waitForEvent("download");
      await page.locator("#download").click();
      const playback = out + "/" + engine + "-playback.zip.html";
      await (await playbackDownload).saveAs(playback);
      await page.goto("file://" + playback);
      await page.waitForFunction(
        () =>
          window.__LAYMESH_WEB &&
          document.querySelector("#app").dataset.painted === "true",
      );
      await save("pdf", "playback.pdf");
      // Phone-width dialog remains inside the visible content area.
      await page.setViewportSize({ width: 390, height: 844 });
      await page.locator("#export").click();
      const box = await page.locator("#export-panel").boundingBox();
      assert(box.x >= 0 && box.x + box.width <= 391);
      await page.screenshot({ path: out + "/" + engine + "-mobile.png" });
      assert.deepEqual(requests, []);
      assert.deepEqual(errors, []);
      results.push({
        engine,
        formats: 15,
        defaultDPI: 1200,
        defaultTiff: "lzw",
        zeroCompiles: true,
        settingsRoundTrip: true,
        playbackExport: true,
        fileHandleWriteAndFailure: true,
        cancelled: true,
        networkRequests: requests,
        errors,
        passed: true,
      });
    } catch (e) {
      if (page)
        await page.screenshot({ path: out + "/" + engine + "-failure.png" });
      throw e;
    } finally {
      await browser.close();
    }
  }
  fs.writeFileSync(out + "/results.json", JSON.stringify(results, null, 2));
  console.log(JSON.stringify(results));
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
