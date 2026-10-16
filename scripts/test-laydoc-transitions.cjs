// Same-source overrides through the real VS Code native pool and website workers.
const fs = require("fs"),
  os = require("os"),
  path = require("path"),
  assert = require("assert/strict"),
  Module = require("module");
const original = Module._load;
Module._load = function (id, ...args) {
  return id === "vscode" ? {} : original.call(this, id, ...args);
};
const { PreviewWorker } = require("../extensions/vscode/src/preview-host.cjs");
Module._load = original;
const {
  NativePresentation,
} = require("../extensions/vscode/src/presentation-host.cjs");
const source =
  'd=document(transition=transition(name="cube",duration=250ms))\nd.add(page(src="a.lay",id="default"))\nd.add(page(src="a.lay",id="override",transition="fade"))\nd.add(page(src="a.lay",id="off",transition="none"))\nd.add(page(src="a.lay",id="reuse"))';
const pageSource =
  'p=canvas(size=(100,60),background="#fff")\np.add(rect(size=(20,20),fill="#123456"))\nstyle { canvas { -laymesh-transition:luma; -laymesh-transition-texture-luma:url("missing.png"); } }';
const expected = ["cube", "fade", null, "cube"],
  root = path.resolve(__dirname, ".."),
  out = process.argv[2] || "/tmp/laymesh-laydoc-overrides";
fs.mkdirSync(out, { recursive: true });
(async () => {
  const fixture = fs.mkdtempSync(path.join(os.tmpdir(), "laymesh-overrides-")),
    file = path.join(fixture, "d.laydoc");
  fs.writeFileSync(file, source);
  fs.writeFileSync(path.join(fixture, "a.lay"), pageSource);
  let deck;
  const requests = [], messages = [];
  const workers = [],
    create = () => {
      const w = new PreviewWorker(root + "/target/release/laymesh", {
          append() {},
        }),
        enqueue = w.enqueue.bind(w);
      w.enqueue = (job) => {
        requests.push({type: job.request.type || "page", page: job.request.page});
        enqueue(job);
      };
      workers.push(w);
      return w;
    };
  const results = {};
  try {
    deck = new NativePresentation({
      worker: create(),
      createWorker: create,
      dist: root + "/extensions/vscode/dist",
      uri: (f) => f,
      notify: (m) => {
        messages.push(m.type);
        if (["deck-page", "deck-page-reuse"].includes(m.type))
          queueMicrotask(() => deck.ack(m.page.id));
      },
    });
    let timer;
    try {
      await Promise.race([
        deck.prepare(file, source, {}, {}),
        new Promise((_, reject) => {
          timer = setTimeout(() => reject(Error("Native transition preparation stalled: " +
            JSON.stringify({requests, messages, pages: deck.pages}))), 30000);
        }),
      ]);
    } finally {
      clearTimeout(timer);
    }
    assert(
      deck.pages.every((p) => p.status === "ready"),
      JSON.stringify(deck.pages),
    );
    const names = deck.pages.map(
      (p) =>
        JSON.parse(fs.readFileSync(p.file)).list.pageTransition?.name || null,
    );
    assert.deepEqual(names, expected);
    assert.equal(requests.filter(r => r.type === "plan").length, 1);
    assert.equal(requests.filter(r => r.type === "page").length, 3);
    assert.equal(messages.filter(type => type === "deck-page").length, 3);
    assert.equal(messages.filter(type => type === "deck-page-reuse").length, 1);
    results.native = { names, compiled: 3, requests, messages, passed: true };
  } finally {
    deck?.dispose();
    fs.rmSync(fixture, { recursive: true, force: true });
  }
  if (process.argv[3]) {
    const { chromium } = require(process.env.PLAYWRIGHT_MODULE || "playwright"),
      browser = await chromium.launch({ headless: true });
    try {
      const p = await browser.newPage(),
        errors = [];
      p.on("pageerror", (e) => errors.push(e.message));
      await p.goto(process.argv[3]);
      const web = await p.evaluate(
        async ({ source, pageSource }) => {
          const { webPresentationProvider } = await import(
            "/site/live/presentation-web.mjs"
          );
          const editor = {
            entry: "d.laydoc",
            loadedFonts: [],
            module: { dataset: {} },
            files: { "d.laydoc": source, "a.lay": pageSource },
          };
          const deck = await webPresentationProvider(
            editor,
            new URL("/site/live/", location),
            {},
          )({ signal: new AbortController().signal, onProgress() {} });
          try {
            await deck.complete;
            const names = [];
            for (let i = 0; i < deck.pages.length; i++) {
              await deck.wait(i);
              names.push(
                (await deck.store.page(deck.pages[i].id)).list.pageTransition
                  ?.name || null,
              );
            }
            return { names, compiled: deck.preparation.pages.length };
          } finally {
            await deck.store.dispose();
          }
        },
        { source, pageSource },
      );
      assert.deepEqual(web.names, expected);
      assert.equal(web.compiled, 3);
      assert.deepEqual(errors, []);
      results.web = { ...web, errors, passed: true };
    } finally {
      await browser.close();
    }
  }
  fs.writeFileSync(out + "/results.json", JSON.stringify(results, null, 2));
  console.log(JSON.stringify(results));
})().catch((e) => {
  console.error(e);
  process.exitCode = 1;
});
