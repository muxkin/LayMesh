// Real offline HTML: Headless Tree, contextual menus, semantic moves and round trips.
const fs = require("fs"),
  path = require("path"),
  assert = require("assert/strict"),
  cp = require("child_process");
const engines = require(process.env.PLAYWRIGHT_MODULE || "playwright"),
  root = path.resolve(__dirname, ".."),
  out = path.resolve(process.argv[2] || "/tmp/laymesh-workspace-interactions");
fs.mkdirSync(out, { recursive: true });
const binary =
    process.env.LAYMESH_TEST_BINARY || root + "/target/release/laymesh",
  file = out + "/editable.zip.html";
cp.execFileSync(
  binary,
  [
    "render",
    "examples/animation/transitions/demo.laydoc",
    "-o",
    file,
    "--html-editable",
    "--html-embed-fonts",
  ],
  { cwd: root, stdio: "pipe" },
);
(async () => {
  const results = [];
  for (const engine of process.env.LAYMESH_TEST_ENGINES?.split(",") || [
    "chromium",
    "firefox",
    "webkit",
  ]) {
    const browser = await engines[engine].launch({ headless: true });
    let page;
    const errors = [];
    try {
      const context = await browser.newContext({
        viewport: { width: 1600, height: 1000 },
        acceptDownloads: true,
      });
      await context.route(/^https?:\/\//, (r) => r.abort());
      page = await context.newPage();
      page.on("pageerror", (e) => errors.push(e.message));
      page.setDefaultTimeout(20000);
      const ready = () =>
        page.waitForFunction(
          () =>
            document.querySelector("#app")?.dataset.editorReady === "true" &&
            document.querySelector("#app")?.dataset.painted === "true",
        );
      const row = (p) => page.locator('#tree [data-path="' + p + '"]');
      const menu = async (name) =>
        page
          .locator(".context-menu-list:visible .context-menu-item")
          .filter({ hasText: new RegExp("^" + name + "$") })
          .click();
      const contextOn = async (p) => {
        await row(p).click({ button: "right" });
      };
      await page.goto("file://" + file);
      await ready();
      assert.equal(
        await page
          .locator("#tree .workspace-tree-row[data-group=true]")
          .first()
          .innerText(),
        "▾\n项目文件",
      );
      const transitions = await page.evaluate(() =>
        __LAYMESH_WEB.pages.map((p) => p.list.pageTransition?.name),
      );
      assert.deepEqual(transitions, [
        "InvertedPageCurl",
        "InvertedPageCurl",
        "cube",
        "luma",
      ]);
      await row("/project/pages").click();
      await row("/project/pages/cube.lay").click();
      await page
        .locator(".source-panel:not([hidden]) .laymesh-color-swatch")
        .first()
        .waitFor();
      const swatches = await page
        .locator(".source-panel:not([hidden]) .laymesh-color-swatch")
        .evaluateAll((ns) => ns.map((n) => [n.offsetWidth, n.offsetHeight]));
      assert(swatches.every(([w, h]) => w === 12 && h === 12));
      assert(await page.locator("#export").isVisible());
      const original = await page.evaluate(
        () => __LAYMESH_WEB.editor.files["/project/pages/cube.lay"],
      );
      const swatch = page
        .locator(".source-panel:not([hidden]) .laymesh-color-swatch")
        .first();
      await swatch.focus();
      await page.keyboard.press("Enter");
      await page
        .locator(".laymesh-color-popover input[aria-label=HEX]")
        .fill("#123456");
      await page
        .locator(".laymesh-color-popover input[aria-label=HEX]")
        .click({ button: "right" });
      await menu("全选");
      assert(await page.locator(".laymesh-color-popover").isVisible());
      await page.keyboard.press("Escape");
      await page
        .locator(".laymesh-color-popover")
        .waitFor({ state: "detached" });
      await page.waitForFunction(() =>
        __LAYMESH_WEB.editor.files["/project/pages/cube.lay"].includes(
          "#123456",
        ),
      );
      await page
        .locator(".source-panel:not([hidden]) .cm-content")
        .click({ button: "right" });
      await menu("撤销");
      assert.equal(
        await page.evaluate(
          () => __LAYMESH_WEB.editor.files["/project/pages/cube.lay"],
        ),
        original,
      );
      // A denied clipboard request leaves both content and its selection intact.
      await page.evaluate(() => {
        Object.defineProperty(navigator, "clipboard", {
          configurable: true,
          value: {
            readText: async () => {
              throw Error("denied");
            },
            writeText: async () => {
              throw Error("denied");
            },
          },
        });
      });
      await page
        .locator(".source-panel:not([hidden]) .cm-content")
        .click({ button: "right" });
      await menu("粘贴");
      await page.waitForFunction(() =>
        document.querySelector("#status").textContent.includes("Ctrl / ⌘ + V"),
      );
      assert.equal(
        await page.evaluate(
          () => __LAYMESH_WEB.editor.files["/project/pages/cube.lay"],
        ),
        original,
      );
      // Preserve edited content and editor history across rename/move.
      await page.evaluate(() => {
        const v = __LAYMESH_WEB.editor.views.get(
          "/project/pages/cube.lay",
        ).view;
        v.dispatch({
          changes: { from: v.state.doc.length, insert: "\n# unsaved note\n" },
        });
      });
      await page.waitForFunction(
        () => document.querySelector("#app").dataset.compileState === "ready",
      );
      await contextOn("/project/pages/cube.lay");
      await menu("重命名");
      await page.locator("#tree input").fill("box.lay");
      await page.locator("#tree input").press("Enter");
      await row("/project/pages/box.lay").waitFor();
      assert(
        (
          await page.evaluate(
            () => __LAYMESH_WEB.editor.files["/project/demo.laydoc"],
          )
        ).includes("pages/box.lay"),
      );
      assert(
        (
          await page.evaluate(
            () => __LAYMESH_WEB.editor.files["/project/pages/box.lay"],
          )
        ).includes("# unsaved note"),
      );
      await contextOn("/project");
      await menu("新建文件夹");
      await page.locator(".workspace-dialog input").fill("moved");
      await page
        .locator(".workspace-dialog button")
        .getByText("创建", { exact: true })
        .click();
      await row("/project/moved").waitFor();
      await row("/project/pages/box.lay").dragTo(row("/project/moved"));
      await row("/project/moved/box.lay").waitFor();
      assert(
        (
          await page.evaluate(
            () => __LAYMESH_WEB.editor.files["/project/demo.laydoc"],
          )
        ).includes("moved/box.lay"),
      );
      await contextOn("/project/moved/box.lay");
      await menu("撤销文件操作");
      await row("/project/pages/box.lay").waitFor();
      await contextOn("/project/pages/box.lay");
      await menu("重做文件操作");
      await row("/project/moved/box.lay").waitFor();
      // Copy, collision handling and delete/undo operate only on this workbench.
      await contextOn("/project/moved/box.lay");
      await menu("复制");
      await contextOn("/project/pages");
      await menu("粘贴");
      await row("/project/pages/box.lay").waitFor();
      await contextOn("/project/pages/box.lay");
      await menu("删除");
      await page
        .locator(".workspace-dialog button")
        .getByText("删除", { exact: true })
        .click();
      await row("/project/pages/box.lay").waitFor({ state: "detached" });
      await contextOn("/project/pages");
      await menu("撤销文件操作");
      await row("/project/pages/box.lay").waitFor();
      // Keyboard menu uses the same commands; all menus dismiss with Escape.
      await row("/project/moved/box.lay").focus();
      await page.keyboard.press("Shift+F10");
      await page.locator(".context-menu-list:visible").waitFor();
      await page.keyboard.press("Escape");
      await page
        .locator(".context-menu-list:visible")
        .waitFor({ state: "hidden" });
      await page
        .locator(".source-panel:not([hidden]) .cm-content")
        .click({ button: "right" });
      assert(
        await page
          .locator(".context-menu-list:visible")
          .getByText("跳转定义", { exact: true })
          .isVisible(),
      );
      await page.keyboard.press("Escape");
      await page.locator(".stage").click({ button: "right" });
      await menu("100%");
      assert.equal(await page.locator("#zoom").inputValue(), "1");
      await page.locator("#fonts").click();
      await page.locator(".font-row input").first().click({ button: "right" });
      assert(
        await page
          .locator(".context-menu-list:visible")
          .getByText("全选", { exact: true })
          .isVisible(),
      );
      await page.keyboard.press("Escape");
      await page.locator("#font-later").click();
      // Imported nested paths, including an empty folder, survive saving and reopening.
      await page.evaluate(async () => {
        await __LAYMESH_WEB.editor.importFiles(
          [
            { path: "nested/empty", directory: true },
            {
              path: "nested/readme.txt",
              file: new File(["hello"], "readme.txt"),
            },
          ],
          "/project",
        );
      });
      await page.waitForFunction(
        () => document.querySelector("#app").dataset.compileState === "ready",
      );
      // A failed batch never leaves the earlier files behind.
      assert.equal(
        await page.evaluate(async () => {
          try {
            await __LAYMESH_WEB.editor.importFiles(
              [
                {
                  path: "partial.txt",
                  file: new File(["partial"], "partial.txt"),
                },
                {
                  path: "broken.txt",
                  file: {
                    arrayBuffer: async () => {
                      throw Error("read failure");
                    },
                  },
                },
              ],
              "/project",
            );
            return false;
          } catch {
            return !__LAYMESH_WEB.editor.entries.has("/project/partial.txt");
          }
        }),
        true,
      );
      await page.evaluate(async () =>
        __LAYMESH_WEB.editor.importFiles(
          [
            { path: "nested/a.txt", file: new File(["a"], "a.txt") },
            { path: "nested/b.txt", file: new File(["b"], "b.txt") },
          ],
          "/project",
        ),
      );
      if (!(await row("/project/nested/a.txt").isVisible()))
        await row("/project/nested").click();
      await row("/project/nested/a.txt").click();
      await row("/project/nested/b.txt").click({ modifiers: ["Control"] });
      await row("/project/nested/a.txt").dragTo(row("/project/nested/empty"));
      await page.waitForFunction(
        () =>
          __LAYMESH_WEB.editor.files["/project/nested/empty/a.txt"] === "a" &&
          __LAYMESH_WEB.editor.files["/project/nested/empty/b.txt"] === "b",
      );
      await contextOn("/project/nested/empty");
      await menu("撤销文件操作");
      await row("/project/nested/a.txt").waitFor();
      await row("/project/nested/b.txt").waitFor();
      await row("/project/nested/a.txt").click();
      await contextOn("/project/nested/a.txt");
      await menu("复制");
      await contextOn("/project/nested");
      await menu("粘贴");
      await page
        .locator(".workspace-dialog")
        .getByText("保留两份", { exact: true })
        .click();
      await row("/project/nested/a (2).txt").waitFor();
      // Directory ZIPs include empty directories as explicit entries.
      await contextOn("/project/nested");
      const zipDownload = page.waitForEvent("download");
      await menu("下载");
      const zipPath = out + "/" + engine + "-folder.zip";
      await (await zipDownload).saveAs(zipPath);
      const { webUnzip } = await import(root + "/site/live/web-archive.mjs");
      const zipped = webUnzip(fs.readFileSync(zipPath));
      assert(zipped["nested/empty/"]);
      assert.equal(
        new TextDecoder().decode(zipped["nested/readme.txt"]),
        "hello",
      );
      await contextOn("/project/demo.laydoc");
      assert(
        await page
          .locator(".context-menu-list:visible")
          .getByText("删除", { exact: true })
          .evaluate((n) =>
            n.closest("li").classList.contains("context-menu-disabled"),
          ),
      );
      await page.keyboard.press("Escape");
      await row("laymesh-library:/gl-transitions.css").click();
      await page
        .locator(".source-panel:not([hidden]) .cm-content")
        .click({ button: "right" });
      assert(
        await page
          .locator(".context-menu-list:visible")
          .getByText("粘贴", { exact: true })
          .evaluate((n) =>
            n.closest("li").classList.contains("context-menu-disabled"),
          ),
      );
      await page.keyboard.press("Escape");
      await page.locator("#export").click();
      const downloadPromise = page.waitForEvent("download");
      await page.locator("#save").click();
      const download = await downloadPromise;
      const saved = out + "/" + engine + "-saved.zip.html";
      await download.saveAs(saved);
      await page.goto("file://" + saved);
      await ready();
      await row("/project/nested").click();
      assert(await row("/project/nested/empty").isVisible());
      assert.equal(
        await page.evaluate(
          () => __LAYMESH_WEB.editor.files["/project/nested/readme.txt"],
        ),
        "hello",
      );
      assert(
        (
          await page.evaluate(
            () => __LAYMESH_WEB.editor.files["/project/demo.laydoc"],
          )
        ).includes("moved/box.lay"),
      );
      await page.locator("#windowed").evaluate((b) => b.click());
      await page.waitForFunction(() => __LAYMESH_WEB.presentation.ready);
      await page.locator(".presentation-canvas").click({ button: "right" });
      await page.keyboard.press("ArrowDown");
      await page.keyboard.press("Escape");
      assert.equal(
        await page.evaluate(() => __LAYMESH_WEB.presentation.opened),
        true,
      );
      await page.locator(".presentation-canvas").click({ button: "right" });
      assert(
        await page
          .locator(".context-menu-list:visible")
          .getByText("退出放映", { exact: true })
          .isVisible(),
      );
      await menu("退出放映");
      await page.waitForFunction(() => !__LAYMESH_WEB.presentation.opened);
      await page.locator("#full").click();
      await page.waitForFunction(() => __LAYMESH_WEB.presentation.ready);
      await page.locator(".presentation-canvas").click({ button: "right" });
      await menu("退出放映");
      await page.waitForFunction(() => !__LAYMESH_WEB.presentation.opened);
      await page.locator("#export").click();
      const playDownload = page.waitForEvent("download");
      await page.locator("#download").click();
      const playback = out + "/" + engine + "-play.zip.html";
      await (await playDownload).saveAs(playback);
      await page.goto("file://" + playback);
      await page.waitForFunction(
        () => document.querySelector("#app")?.dataset.painted === "true",
      );
      await page.locator(".stage").click({ button: "right" });
      await menu("100%");
      assert.equal(await page.locator("#zoom").inputValue(), "1");
      assert.deepEqual(errors, []);
      await page.screenshot({ path: out + "/" + engine + ".png" });
      results.push({ engine, swatches, transitions, errors, passed: true });
      console.log(engine + " workspace interactions passed");
    } catch (error) {
      if (page) {
        console.error(
          await page.evaluate(() => ({
            app: { ...document.querySelector("#app")?.dataset },
            status: document.querySelector("#status")?.textContent,
            tree: document.querySelector("#tree")?.innerText,
            paths: Object.keys(window.__LAYMESH_WEB?.editor?.files || {}),
          })),
          errors,
        );
        await page.screenshot({ path: out + "/" + engine + "-failure.png" });
      }
      throw error;
    } finally {
      await browser.close();
    }
  }
  fs.writeFileSync(out + "/results.json", JSON.stringify(results, null, 2));
})().catch((e) => {
  console.error(e);
  process.exitCode = 1;
});
