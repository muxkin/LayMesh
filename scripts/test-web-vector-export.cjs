// Pixel parity for the two offline export profiles, including local PDF fallbacks.
const fs = require("fs"),
  path = require("path"),
  cp = require("child_process"),
  assert = require("assert/strict");
const engines = require(process.env.PLAYWRIGHT_MODULE || "playwright");
const root = path.resolve(__dirname, ".."),
  out = path.resolve(process.argv[2] || "/tmp/laymesh-vector-export");
fs.mkdirSync(out, { recursive: true });
const binary =
  process.env.LAYMESH_TEST_BINARY || root + "/target/release/laymesh";
fs.writeFileSync(
  out + "/mask.svg",
  '<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><defs><linearGradient id="m"><stop stop-color="white"/><stop offset="1" stop-color="black"/></linearGradient><mask id="a"><rect width="100" height="100" fill="url(#m)"/></mask><clipPath id="c"><circle cx="50" cy="50" r="45"/></clipPath></defs><g mask="url(#a)" clip-path="url(#c)"><rect width="100" height="100" fill="#18a098"/><circle cx="50" cy="50" r="30" fill="#e03868"/></g></svg>',
);
fs.writeFileSync(
  out + "/page.lay",
  `p=canvas(size=(80mm,60mm),background="none")
g=group(effects=[glow(color="#9346e0",blur=0.7mm)])
g.add(rect(size=(25mm,16mm),border_radius=2mm,fill=linear_gradient(stops=[(0,"#ed342880"),(1,"#184ae0")]),effects=[shadow(mode="inner",blur=0.7mm,offset=(0.8mm,0.9mm))]))
g.add(text("Vector",font_size=11pt,color="#ffffff"),offset=(3mm,5mm))
p.add(g,size=(34mm,22mm),rotation=-12deg,offset=(7mm,7mm),opacity=0.7)
p.add(image(src="mask.svg"),size=(25mm,25mm),offset=(48mm,3mm))
p.add(rect(size=(25mm,16mm),fill=hatch(pattern=cross,color="#137f8c",spacing=2mm,line_width=0.3pt)),offset=(6mm,35mm))
p.add(ellipse(size=(20mm,15mm),fill=radial_gradient(stops=[(0,"#ffed00"),(1,"#ed008f80")]),effects=[shadow(blur=0.5mm,offset=(1mm,1mm))]),offset=(45mm,35mm))
`,
);
cp.execFileSync(binary, [
  "render",
  out + "/page.lay",
  "-o",
  out + "/native.png",
  "--dpi",
  "300",
]);
for (const mode of ["editable", "playback"])
  cp.execFileSync(binary, [
    "render",
    out + "/page.lay",
    "-o",
    out + "/" + mode + ".zip.html",
    ...(mode === "editable" ? ["--html-editable"] : []),
  ]);
(async () => {
  const results = [];
  for (const engine of process.env.LAYMESH_TEST_ENGINES?.split(",") || [
    "chromium",
    "firefox",
    "webkit",
  ]) {
    const browser = await engines[engine].launch({ headless: true });
    try {
      const context = await browser.newContext({ acceptDownloads: true });
      const errors = [],
        requests = [];
      await context.addInitScript(() =>
        Object.defineProperty(window, "showSaveFilePicker", {
          value: undefined,
          configurable: true,
        }),
      );
      await context.route(/^https?:\/\//, (r) => {
        requests.push(r.request().url());
        return r.abort();
      });
      const page = await context.newPage();
      page.on("pageerror", (e) => errors.push(e.message));
      page.setDefaultTimeout(120000);
      for (const mode of ["editable", "playback"]) {
        await page.goto("file://" + out + "/" + mode + ".zip.html");
        await page.waitForFunction(
          () => document.querySelector("#app").dataset.painted === "true",
        );
        await page.locator("#export").click();
        await page.locator("#export-settings").click();
        await page.locator("#export-dpi").fill("300");
        await page
          .locator("#export-settings-panel button[type=submit]")
          .click();
        await page.locator("#export-panel .dialog-close").click();
        for (const format of ["pdf", "svg", "png"]) {
          await page.locator("#export").click();
          await page.locator("#export-format").selectOption(format);
          const download = page.waitForEvent("download");
          await page.locator("#export-file").click();
          const file = await download;
          await file.saveAs(`${out}/${engine}-${mode}.${format}`);
          await page.waitForFunction(
            () => !document.querySelector("#export-file").disabled,
          );
        }
        const svg = fs.readFileSync(`${out}/${engine}-${mode}.svg`, "utf8");
        const png = await page.evaluate(async (svg) => {
          const blob = new Blob([svg], { type: "image/svg+xml" }),
            url = URL.createObjectURL(blob),
            image = new Image();
          try {
            image.src = url;
            await image.decode();
            const c = document.createElement("canvas");
            c.width = Math.round((80 * 300) / 25.4);
            c.height = Math.round((60 * 300) / 25.4);
            c.getContext("2d").drawImage(image, 0, 0, c.width, c.height);
            return c.toDataURL("image/png").split(",")[1];
          } finally {
            URL.revokeObjectURL(url);
          }
        }, svg);
        fs.writeFileSync(
          `${out}/${engine}-${mode}-svg.png`,
          Buffer.from(png, "base64"),
        );
        results.push({
          engine,
          mode,
          errors: [...errors],
          requests: [...requests],
        });
      }
      assert.deepEqual(errors, []);
      assert.deepEqual(requests, []);
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
