// Four real VS Code web-export choices preserve dirty buffers and export all pages.
const assert = require("node:assert/strict"),
  fs = require("node:fs"),
  os = require("node:os"),
  path = require("node:path"),
  vm = require("node:vm");
const root = path.resolve(__dirname, ".."),
  binary = path.resolve(
    process.argv[2] ||
      path.join(
        root,
        "target/release",
        process.platform === "win32" ? "laymesh.exe" : "laymesh",
      ),
  ),
  directory = fs.mkdtempSync(path.join(os.tmpdir(), "laymesh-editor-web-"));
const uri = (fsPath) => ({ scheme: "file", fsPath }),
  main = uri(path.join(directory, "main.laydoc")),
  page = uri(path.join(directory, "page.lay"));
fs.writeFileSync(
  main.fsPath,
  'doc=document(name="Web export")\ndoc.add(page(src="page.lay",id="first"))\ndoc.add(page(src="page.lay",id="second"))',
);
fs.writeFileSync(page.fsPath, "page=canvas(size=(64mm,36mm))");
const docs = [
    {
      uri: main,
      isDirty: false,
      getText: () => fs.readFileSync(main.fsPath, "utf8"),
    },
    {
      uri: page,
      isDirty: true,
      getText: () =>
        'page=canvas(size=(64mm,36mm))\npage.add(text("Unsaved editor text"))',
    },
  ],
  commands = new Map(),
  dialogs = [],
  errors = [],
  context = { subscriptions: [] },
  disposable = () => ({ dispose() {} });
let choiceIndex = 0,
  destination;
const vscode = {
  env: { language: "en" },
  Uri: { file: uri },
  ProgressLocation: { Notification: 1 },
  commands: {
    registerCommand: (n, f) => {
      commands.set(n, f);
      return disposable();
    },
  },
  workspace: {
    isTrusted: true,
    textDocuments: docs,
    openTextDocument: async () => docs[0],
    getConfiguration: () => ({
      get: (_k, f) => f,
      inspect: (key) =>
        key === "export.html.embedFonts" && choiceIndex % 2 === 1
          ? { workspaceValue: true }
          : undefined,
    }),
    onDidChangeTextDocument: disposable,
    onDidSaveTextDocument: disposable,
    onDidCloseTextDocument: disposable,
    onDidChangeConfiguration: disposable,
  },
  window: {
    activeTextEditor: { document: docs[0] },
    showQuickPick: async (choices) => {
      dialogs.push("format");
      const web = choices.filter((c) => ["html", "zip"].includes(c.extension));
      assert.equal(web.length, 4);
      assert(web.every((c) => !c.animated));
      return web[choiceIndex];
    },
    showSaveDialog: async (options) => {
      dialogs.push("destination");
      assert(
        options.defaultUri.fsPath.endsWith(
          choiceIndex < 2 ? ".zip.html" : ".zip",
        ),
      );
      return destination;
    },
    showInputBox: () => {
      throw Error("Unexpected encoding prompt");
    },
    showInformationMessage: async () => {},
    showErrorMessage: (m) => errors.push(m),
    withProgress: async (_o, f) =>
      f({}, { onCancellationRequested: disposable }),
  },
};
const hostRequire = require("node:module").createRequire(
    path.join(root, "extensions/vscode/src/preview-host.cjs"),
  ),
  sandbox = {
    require: (n) => (n === "vscode" ? vscode : hostRequire(n)),
    module: { exports: {} },
    console,
    process,
    setTimeout,
    clearTimeout,
  };
vm.runInNewContext(
  fs.readFileSync(
    path.join(root, "extensions/vscode/src/preview-host.cjs"),
    "utf8",
  ),
  sandbox,
);
(async () => {
  try {
    sandbox.module.exports.activatePreview(context, binary, {
      appendLine() {},
    });
    for (choiceIndex = 0; choiceIndex < 4; choiceIndex++) {
      dialogs.length = 0;
      destination = uri(
        path.join(
          directory,
          "作品 #" + choiceIndex + (choiceIndex < 2 ? ".zip.html" : ".ZIP"),
        ),
      );
      const result = await commands.get("laymesh.exportFigure")(
        main,
        undefined,
        "current-page",
      );
      assert.deepEqual(dialogs, ["format", "destination"]);
      assert.deepEqual(errors, []);
      assert.equal(result.pages, 2);
      assert.equal(result.editable, choiceIndex % 2 === 1);
      assert.equal(result.exported, destination.fsPath.replace(/\\/g, "/"));
      assert(fs.statSync(destination.fsPath).size > 1000);
      const archive = await import(
        path.join(root, "site/live/web-archive.mjs")
      );
      const entries = archive.webUnzip(
        archive.webPayload(fs.readFileSync(destination.fsPath)),
      );
      const manifest = JSON.parse(
        new TextDecoder().decode(entries["manifest.json"]),
      );
      assert.equal(manifest.embedFonts, choiceIndex % 2 === 1);
      if (manifest.editable)
        assert(manifest.fonts.every((f) => !!entries[f.file]));
    }
    console.log(
      "Four HTML/ZIP modes, two-dialog flow, dirty sources and full-document defaults passed.",
    );
  } finally {
    for (const s of context.subscriptions) s.dispose();
    fs.rmSync(directory, { recursive: true, force: true });
  }
})().catch((e) => {
  console.error(e);
  process.exitCode = 1;
});
