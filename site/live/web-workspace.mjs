import { workspaceTree } from "./workspace-tree.mjs";
import {
  parentPath,
  fileName,
  within,
  topPaths,
  validateName,
  projectPath,
  relocate,
  droppedFiles,
} from "./workspace-files.mjs";
import {
  createContextMenu,
  inputMenu,
  writeClipboard,
} from "./workspace-menu.mjs";
import { editorMenu } from "./workspace-editor-menu.mjs";
import { query } from "./language.mjs";
import { webZip } from "./web-archive.mjs";
import {
  editor as sourceEditor,
  restoreEditorState,
} from "./source-editor.mjs";
import { sha256 } from "./web-sha256.mjs";
import { uiIcon } from "./presentation-icons.mjs";
import { createFontManager } from "./web-fonts.mjs";
const textFile = /\.(lay|laydoc|lcss|css|csv|json|txt|md)$/i;
const fontFile = /\.(ttf|otf|ttc|otc)$/i;
const imageFile = /\.(png|jpe?g|webp|gif|bmp|svg|tiff?|ico|tga|p[bnpg]m|pam)$/i;
const basename = (p) => p.split("/").at(-1);
const size = (b) =>
  b < 1024
    ? b + " B"
    : b < 1048576
      ? (b / 1024).toFixed(1) + " KiB"
      : (b / 1048576).toFixed(2) + " MiB";
const node = (tag, cls, text) => {
  const n = document.createElement(tag);
  if (cls) n.className = cls;
  if (text != null) n.textContent = text;
  return n;
};
export async function createWorkspace({
  root,
  manifest,
  files,
  pages,
  compilerCall,
  resetCompiler,
  compileNow,
  makeLanguageWorker,
  schedule,
  onStatus,
  previewMenu = () => [],
  presentationRoot = () => null,
}) {
  const decoder = new TextDecoder(),
    encoder = new TextEncoder(),
    tabs = root.querySelector(".file-tabs"),
    panels = root.querySelector("#panels"),
    tree = root.querySelector("#tree");
  const entries = new Map(),
    openTabs = new Map(),
    urls = new Map(),
    observers = new Map();
  const directories = new Set(manifest.project.directories || []);
  let fileTree,
    menus,
    busyOperation = false,
    restoring = false;
  const undoFiles = [],
    redoFiles = [];
  const idleWaiters = [];
  let importChain = Promise.resolve();
  const waitForOperation = () =>
    busyOperation
      ? new Promise((resolve) => idleWaiters.push(resolve))
      : Promise.resolve();
  function operationBusy(value) {
    busyOperation = value;
    root.dataset.fileOperation = value ? "pending" : "ready";
    root.querySelector("#export").disabled = value;
    root.querySelector('main').inert=value;
    if (!value) idleWaiters.splice(0).forEach((resolve) => resolve());
  }
  let clipboardFiles;
  let active,
    selectedDirectory = manifest.project.entry.slice(
      0,
      manifest.project.entry.lastIndexOf("/"),
    ),
    localFonts = [];
  const editor = {
    module: root,
    files: {},
    assets: {},
    views: new Map(),
    maps: new Map(),
    readonly: new Set(),
    light: true,
    seq: 0,
    entry: manifest.project.entry,
    composing: false,
    editor: sourceEditor,
    createLanguageWorker: makeLanguageWorker,
    changed() {
      if (restoring) return;
      schedule.changed();
      refreshTabs();
    },
    schedule(immediate = false) {
      if (restoring) return;
      schedule(immediate);
      refreshTabs();
    },
    async openDefinition(target, query) {
      if (!entries.has(target.uri)) {
        const content = await query(target.uri);
        if (!content) return;
        add(target.uri, encoder.encode(content.text), true, "library");
      }
      await open(target.uri);
      const v = editor.views.get(target.uri)?.view;
      if (v) {
        v.dispatch({
          selection: { anchor: Math.min(target.from, v.state.doc.length) },
        });
        v.focus();
      }
    },
  };
  function add(path, data, readonly = false, type = null) {
    const old = entries.get(path);
    if (!readonly) {
      let dir = parentPath(path);
      while (dir.startsWith("/project/")) {
        directories.add(dir);
        dir = parentPath(dir);
      }
    }
    entries.set(path, {
      path,
      data,
      readonly,
      type,
      original:
        old?.original ?? (textFile.test(path) ? decoder.decode(data) : null),
    });
    if (readonly) editor.readonly.add(path);
    if (textFile.test(path)) editor.files[path] = decoder.decode(data);
    else if (!readonly) editor.assets[path] = data;
  }
  for (const [p, f] of Object.entries(manifest.project.files)) add(p, files[f]);
  for (const info of manifest.resourceInfo || [])
    if (info.readonly && files[info.file])
      add(info.path, files[info.file], true, info.type);
  const used = new Set();
  for (const p of pages)
    for (const [id, r] of Object.entries(p.list.resources)) {
      if (used.has(id)) continue;
      used.add(id);
      const b = p.resources?.[id] || files[manifest.resources[id]];
      if (!b || [...entries.values()].some((e) => sha256(e.data) === sha256(b)))
        continue;
      const ext = r.mime.split("/")[1]?.replace("svg+xml", "svg") || "png";
      add("/@generated/" + id.slice(-12) + "." + ext, b, true, "generated");
    }
  function refreshTabs() {
    for (const [path, t] of openTabs) {
      const changed =
        !entries.get(path).readonly &&
        editor.files[path] != null &&
        editor.files[path] !== entries.get(path).original;
      t.label.textContent = basename(path);
      t.button.dataset.dirty = String(changed);
      t.button.title = path + (changed ? " · 已修改" : "");
    }
  }
  function select(path) {
    active = path;
    for (const [p, t] of openTabs) {
      t.panel.hidden = p !== path;
      t.button.setAttribute("aria-selected", String(p === path));
    }
    root.dispatchEvent(new Event("laymesh:tab"));
  }
  async function open(path, { fromTree = false } = {}) {
    if (!entries.has(path)) return;
    let t = openTabs.get(path);
    if (!t) {
      const button = node("div", "file-tab");
      button.tabIndex = 0;
      button.dataset.path = path;
      button.setAttribute("role", "tab");
      button.setAttribute("aria-label", basename(path));
      const panel = node("div", "source-panel");
      panel.dataset.editFile = path;
      panel.id = "file-" + openTabs.size + "-" + panels.children.length;
      button.setAttribute("aria-controls", panel.id);
      button.append(
        uiIcon(
          imageFile.test(path)
            ? "image"
            : fontFile.test(path)
              ? "font"
              : "page",
        ),
      );
      const label = node("span", null, basename(path));
      button.append(label);
      const close = node("button", "tab-close");
      close.title = "关闭 " + basename(path);
      close.setAttribute("aria-label", close.title);
      close.append(uiIcon("close"));
      button.append(close);
      tabs.append(button);
      panels.append(panel);
      t = { button, panel, label };
      openTabs.set(path, t);
      button.onclick = () => open(path);
      button.onkeydown = (e) => {
        if (["Enter", " "].includes(e.key)) {
          e.preventDefault();
          open(path);
        }
        if (e.key === "Delete") close.click();
        if (["ArrowLeft", "ArrowRight", "Home", "End"].includes(e.key)) {
          e.preventDefault();
          const visible = [...openTabs.values()].filter(
              (t) => !t.button.hidden,
            ),
            index = visible.findIndex((t) => t.button === button),
            next =
              visible[
                e.key === "Home"
                  ? 0
                  : e.key === "End"
                    ? visible.length - 1
                    : (index +
                        (e.key === "ArrowLeft" ? -1 : 1) +
                        visible.length) %
                      visible.length
              ];
          next?.button.click();
          next?.button.focus();
        }
      };
      close.onclick = (e) => {
        e.stopPropagation();
        button.hidden = true;
        panel.hidden = true;
        if (active === path) {
          const next = [...openTabs].find(
            ([p, v]) => p !== path && !v.button.hidden,
          );
          if (next) select(next[0]);
          else {
            active = null;
            root.querySelector("#empty-editor").hidden = false;
          }
        }
      };
    }
    t.button.hidden = false;
    root.querySelector("#empty-editor").hidden = true;
    select(path);
    if (textFile.test(path)) {
      if (!t.panel.querySelector(".code-block")) {
        t.panel.innerHTML =
          '<div class="code-block"><pre></pre><button class="wrap-code">换行</button></div>';
        editor.editor(t.panel);
      }
    } else if (!t.panel.dataset.loaded) {
      await resourceView(t.panel, entries.get(path));
      t.panel.dataset.loaded = "true";
    }
    refreshTabs();
    if (!fromTree) fileTree?.select(path);
    tree
      .querySelector(`[data-path="${CSS.escape(path)}"]`)
      ?.scrollIntoView({ block: "nearest" });
  }
  function buildTree() {
    manifest.project.directories = [...directories].sort();
    fileTree?.refresh();
  }
  async function resourceView(panel, entry) {
    observers.get(entry.path)?.disconnect();
    if (urls.has(entry.path)) URL.revokeObjectURL(urls.get(entry.path));
    panel.replaceChildren();
    const bar = node("div", "resource-toolbar"),
      viewer = node("div", "resource-viewer"),
      foot = node("div", "resource-info");
    panel.append(bar, viewer, foot);
    foot.textContent =
      entry.path +
      " · " +
      size(entry.data.byteLength) +
      (entry.readonly ? " · 只读" : "");
    const action = (label, fn) => {
      const b = node("button", null, label);
      b.onclick = fn;
      bar.append(b);
      return b;
    };
    action("复制路径", () => copy(entry.path));
    action("复制引用", () =>
      copy(
        fontFile.test(entry.path)
          ? 'text("文字",font_family=' + JSON.stringify(entry.path) + ")"
          : imageFile.test(entry.path)
            ? "image(src=" + JSON.stringify(entry.path) + ")"
            : JSON.stringify(entry.path),
      ),
    );
    action("下载", () => {
      const a = node("a");
      a.href = URL.createObjectURL(new Blob([entry.data]));
      a.download = basename(entry.path);
      a.click();
      setTimeout(() => URL.revokeObjectURL(a.href), 3000);
    });
    if (!imageFile.test(entry.path)) {
      const note = node(
        "div",
        "resource-note",
        fontFile.test(entry.path)
          ? entry.error
            ? "无法使用此字体：" + entry.error
            : "字体文件 · 可用于重新排版"
          : "此文件暂无预览",
      );
      viewer.append(note);
      return;
    }
    let data = entry.data,
      mime =
        { jpg: "image/jpeg", jpeg: "image/jpeg", svg: "image/svg+xml" }[
          entry.path.split(".").at(-1).toLowerCase()
        ] || "image/" + entry.path.split(".").at(-1).toLowerCase();
    if (/\.(tiff?|tga|p[bnpg]m|pam|ico)$/i.test(entry.path)) {
      try {
        const r = await compilerCall({
          type: "decodeImage",
          bytes: data,
          path: entry.path,
        });
        data = Uint8Array.from(atob(r.image.data), (c) => c.charCodeAt(0));
        mime = r.image.mime;
      } catch (e) {
        viewer.append(
          node("div", "resource-note", "无法解码图片：" + e.message),
        );
        return;
      }
    }
    const url = URL.createObjectURL(new Blob([data], { type: mime }));
    urls.set(entry.path, url);
    const img = node("img");
    img.alt = basename(entry.path);
    img.draggable = false;
    viewer.append(img);
    let scale = 1,
      pan = [0, 0],
      natural,
      fitMode = true;
    const zoom = node("span", "resource-zoom");
    bar.prepend(zoom);
    const apply = () => {
      img.style.width = natural.width * scale + "px";
      img.style.height = natural.height * scale + "px";
      img.style.transform = `translate(${pan[0]}px,${pan[1]}px)`;
      zoom.textContent = Math.round(scale * 100) + "%";
    };
    const fit = () => {
      if (!natural || viewer.clientWidth < 1 || viewer.clientHeight < 1) return;
      scale = Math.min(
        1,
        (viewer.clientWidth - 48) / natural.width,
        (viewer.clientHeight - 48) / natural.height,
      );
      pan = [0, 0];
      apply();
    };
    action("适应", () => {
      fitMode = true;
      fit();
    });
    const observer = new ResizeObserver(() => {
      if (fitMode) requestAnimationFrame(fit);
    });
    observer.observe(viewer);
    observers.set(entry.path, observer);
    action("100%", () => {
      if (!natural) return;
      fitMode = false;
      scale = 1;
      pan = [0, 0];
      apply();
    });
    action("−", () => {
      fitMode = false;
      scale = Math.max(0.05, scale / 1.2);
      apply();
    });
    action("+", () => {
      fitMode = false;
      scale = Math.min(16, scale * 1.2);
      apply();
    });
    img.onload = () => {
      natural = { width: img.naturalWidth, height: img.naturalHeight };
      foot.textContent =
        entry.path +
        " · " +
        mime.replace("image/", "").toUpperCase() +
        " · " +
        natural.width +
        " × " +
        natural.height +
        " px · " +
        size(entry.data.byteLength) +
        (entry.readonly ? " · 只读" : "");
      fit();
    };
    img.onerror = () => {
      img.remove();
      viewer.append(
        node("div", "resource-note", "浏览器无法预览此图片，可下载原文件"),
      );
    };
    img.src = url;
    viewer.onwheel = (e) => {
      e.preventDefault();
      if (e.ctrlKey || e.metaKey) {
        fitMode = false;
        scale = Math.min(
          16,
          Math.max(0.05, scale * Math.exp(-e.deltaY * 0.002)),
        );
        apply();
      } else {
        pan[0] -= e.deltaX;
        pan[1] -= e.deltaY;
        apply();
      }
    };
    let drag;
    viewer.onpointerdown = (e) => {
      drag = [e.clientX, e.clientY, ...pan];
      viewer.setPointerCapture(e.pointerId);
    };
    viewer.onpointermove = (e) => {
      if (drag) {
        pan = [drag[2] + e.clientX - drag[0], drag[3] + e.clientY - drag[1]];
        apply();
      }
    };
    viewer.onpointerup = () => (drag = null);
  }
  async function copy(text) {
    try {
      await navigator.clipboard.writeText(text);
      onStatus("已复制：" + text);
    } catch {
      const d = node("dialog");
      d.append(node("p", null, "复制以下内容"), node("pre", null, text));
      const b = node("button", null, "关闭");
      b.onclick = () => {
        d.close();
        d.remove();
      };
      d.append(b);
      document.body.append(d);
      d.showModal();
    }
  }
  function conflict(path) {
    const dialog = node("dialog", "conflict-dialog");
    dialog.append(node("h3", null, "资源已存在"), node("p", null, path));
    document.body.append(dialog);
    return new Promise((resolve) => {
      for (const [label, value] of [
        ["替换", "replace"],
        ["保留两份", "keep"],
        ["取消", "cancel"],
      ]) {
        const b = node("button", null, label);
        b.onclick = () => {
          dialog.close();
          dialog.remove();
          resolve(value);
        };
        dialog.append(b);
      }
      dialog.addEventListener(
        "cancel",
        () => {
          dialog.remove();
          resolve("cancel");
        },
        { once: true },
      );
      dialog.showModal();
    });
  }
  function importFiles(list, target = selectedDirectory) {
    const batch = Array.from(list);
    const next = importChain
      .catch(() => {})
      .then(async () => {
        await waitForOperation();
        return performImport(batch, target);
      });
    importChain = next;
    return next;
  }
  async function performImport(list, target) {
    let imported = 0;
    await transaction("导入", async (state) => {
      // Read and validate the entire batch before mutating the workspace. The
      // transaction also rejects an intervening text edit while dialogs are open.
      for (const item of list) {
        const file = item.file || item;
        let path = projectPath(
          target + "/" + (item.path || file.webkitRelativePath || file.name),
        );
        let parent = parentPath(path);
        while (parent.startsWith("/project/")) {
          if (state.entries.has(parent)) throw Error("父路径是文件：" + parent);
          state.directories.add(parent);
          parent = parentPath(parent);
        }
        if (item.directory) {
          if (state.entries.has(path)) throw Error("文件与目录同名：" + path);
          state.directories.add(path);
          continue;
        }
        if (state.directories.has(path)) throw Error("目录已存在：" + path);
        if (state.entries.has(path)) {
          const choice = await conflict(path);
          if (choice === "cancel") return;
          if (choice === "keep") {
            const stem = path.replace(/(\.[^/.]+)?$/, ""),
              ext = path.slice(stem.length);
            let n = 2;
            while (state.entries.has(path) || state.directories.has(path))
              path = stem + " (" + n++ + ")" + ext;
          }
        }
        const data = new Uint8Array(await file.arrayBuffer());
        const entry = {
          path,
          data,
          readonly: false,
          type: null,
          original:
            state.entries.get(path)?.original ??
            (textFile.test(path) ? decoder.decode(data) : null),
        };
        state.entries.set(path, entry);
        if (textFile.test(path)) state.files[path] = decoder.decode(data);
        if (fontFile.test(path)) {
          state.fonts = state.fonts.filter((f) => f.path !== path);
          try {
            state.fonts.push(
              ...(await fontManager.describe(path, data, "project")),
            );
          } catch (error) {
            entry.error = "字体损坏或无法读取：" + error.message;
          }
        }
        if (!state.tabs.includes(path)) state.tabs.push(path);
        state.active = path;
        imported++;
      }
      state.project.directories = [...state.directories];
      return state;
    });
    root.querySelector("#import").value = "";
    if (imported) {
      const errors = [...entries.values()]
        .filter((e) => e.error)
        .map((e) => e.path + "：" + e.error);
      onStatus(
        "已导入 " +
          imported +
          " 个文件" +
          (errors.length ? " · " + errors.join("；") : ""),
      );
    }
  }
  root.querySelector("#import").onchange = (e) =>
    importFiles(e.target.files).catch(report);
  const folderInput = document.createElement("input");
  folderInput.type = "file";
  folderInput.multiple = true;
  folderInput.webkitdirectory = true;
  folderInput.hidden = true;
  root.append(folderInput);
  folderInput.onchange = (e) =>
    importFiles(e.target.files)
      .catch(report)
      .finally(() => (folderInput.value = ""));
  root.querySelector("#import-button").onclick = (e) => {
    const r = e.currentTarget.getBoundingClientRect();
    menus.show(
      { target: e.currentTarget, clientX: r.left, clientY: r.bottom },
      creationMenu(selectedDirectory),
    );
  };
  const drop = root.querySelector("main");
  drop.ondragover = (e) => {
    if (e.target.closest("#tree")) return;
    if ([...e.dataTransfer.types].includes("Files")) {
      e.preventDefault();
      drop.classList.add("dragging");
    }
  };
  drop.ondragleave = () => drop.classList.remove("dragging");
  drop.ondrop = (e) => {
    drop.classList.remove("dragging");
    if (e.defaultPrevented || e.target.closest("#tree")) return;
    if ([...e.dataTransfer.types].includes("Files")) {
      e.preventDefault();
      importDrop(e.dataTransfer, selectedDirectory).catch(report);
    }
  };
  function report(error) {
    onStatus(error.message || String(error));
  }
  function capture() {
    return {
      entries: new Map([...entries].map(([p, e]) => [p, { ...e }])),
      files: { ...editor.files },
      directories: new Set(directories),
      project: structuredClone(manifest.project),
      fonts: fontManager.liveFonts.map((f) => ({ ...f })),
      requirements: Object.fromEntries(
        [
          "fontRequirements",
          "requestedFonts",
          "fontReplacementRequirements",
        ].map((k) => [k, structuredClone(manifest[k])]),
      ),
      bindings: { ...fontManager.bindings },
      tabs: [...openTabs].filter(([, t]) => !t.button.hidden).map(([p]) => p),
      active,
      tree: fileTree?.snapshot(),
      states: new Map(
        [...editor.views].map(([p, { view }]) => [
          p,
          {
            state: view.state,
            scrollTop: view.scrollDOM.scrollTop,
            scrollLeft: view.scrollDOM.scrollLeft,
          },
        ]),
      ),
    };
  }
  async function restore(state, { compile = true } = {}) {
    restoring = true;
    if (compile) {
      editor.seq++;
      clearTimeout(editor.debounce);
      resetCompiler();
    }
    try {
      for (const { view } of editor.views.values()) view.destroy();
      editor.views.clear();
      editor.maps.clear();
      for (const o of observers.values()) o.disconnect();
      observers.clear();
      for (const url of urls.values()) URL.revokeObjectURL(url);
      urls.clear();
      openTabs.clear();
      tabs.replaceChildren();
      panels.querySelectorAll(".source-panel").forEach((p) => p.remove());
      entries.clear();
      for (const [p, e] of state.entries) entries.set(p, { ...e });
      editor.files = { ...state.files };
      editor.assets = Object.fromEntries(
        [...entries]
          .filter(([p, e]) => !e.readonly && !textFile.test(p))
          .map(([p, e]) => [p, e.data]),
      );
      directories.clear();
      for (const p of state.directories) directories.add(p);
      Object.assign(manifest.project, state.project);
      editor.entry = manifest.project.entry;
      fontManager.liveFonts.splice(
        0,
        fontManager.liveFonts.length,
        ...state.fonts.map((f) => ({ ...f })),
      );
      Object.assign(manifest, state.requirements);
      for (const k of Object.keys(fontManager.bindings))
        delete fontManager.bindings[k];
      Object.assign(fontManager.bindings, state.bindings);
      active = null;
      root.querySelector("#empty-editor").hidden = false;
      buildTree();
      for (const p of state.tabs || [])
        if (entries.has(p)) {
          await open(p, { fromTree: true });
          restoreEditorState.call(editor, p, state.states?.get(p));
        }
      if (state.active && entries.has(state.active)) {
        await open(state.active, { fromTree: true });
      }
      if (state.tree) fileTree?.restore(state.tree);
      else if (active) fileTree?.select(active);
      if (!directories.has(selectedDirectory)) selectedDirectory = "/project";
      fontManager.refresh();
      refreshTabs();
    } finally {
      restoring = false;
    }
    // An incomplete dependency graph may not know about a newly added file
    // yet. Reconcile the latest snapshot after a transaction in that state.
    if (compile || ["pending", "error"].includes(root.dataset.documentState))
      editor.schedule(true);
  }
  async function choose(title, message, options, { value } = {}) {
    const d = node("dialog", "workspace-dialog");
    d.append(node("h3", null, title), node("p", null, message));
    let input;
    if (value != null) {
      input = node("input");
      input.value = value;
      input.setAttribute("aria-label", title);
      d.append(input);
    }
    const bar = node("div", "workspace-dialog-actions");
    d.append(bar);
    document.body.append(d);
    return new Promise((resolve) => {
      const done = (result) => {
        d.close();
        d.remove();
        resolve(result);
      };
      for (const [label, result] of options) {
        const b = node("button", null, label);
        b.onclick = () => done(input && result === "ok" ? input.value : result);
        bar.append(b);
      }
      d.addEventListener("cancel", () => done(null), { once: true });
      if (input)
        input.onkeydown = (e) => {
          if (e.key === "Enter" && !e.isComposing) {
            e.preventDefault();
            done(input.value);
          }
        };
      d.showModal();
      if (input) {
        input.focus();
        input.select();
      }
    });
  }
  async function references() {
    const all = [];
    for (const path of Object.keys(editor.files).filter(
      (p) => /\.(lay|laydoc|lcss|css)$/i.test(p) && !editor.readonly.has(p),
    )) {
      const result = await query(editor, path, "fileReferences");
      if (!Array.isArray(result)) throw Error("无法检查文件引用，请稍后重试");
      all.push(...result);
    }
    return all;
  }
  async function transaction(label, prepare) {
    if (busyOperation) throw Error("文件操作正在进行");
    operationBusy(true);
    const before = capture(),
      revision = editor.seq;
    try {
      const result = await prepare(capture());
      if (!result) return;
      if (editor.seq !== revision) throw Error("操作期间文本已改变，请重试");
      const dependencies = new Set(
        before.project.dependencies || Object.keys(before.project.files),
      );
      const compile =
        before.project.entry !== result.project.entry ||
        before.fonts.length !== result.fonts.length ||
        before.fonts.some(
          (f, i) =>
            f.bytes !== result.fonts[i]?.bytes ||
            f.path !== result.fonts[i]?.path,
        ) ||
        [...dependencies].some(
          (p) =>
            before.files[p] !== result.files[p] ||
            before.entries.get(p)?.data !== result.entries.get(p)?.data,
        );
      try {
        await restore(result, { compile });
      } catch (error) {
        await restore(before);
        throw error;
      }
      undoFiles.push({ before, after: capture(), label, compile });
      redoFiles.length = 0;
      onStatus("已" + label + " · 可在文件菜单撤销");
    } finally {
      operationBusy(false);
    }
  }
  async function undoOperation(redo = false) {
    const stack = redo ? redoFiles : undoFiles,
      other = redo ? undoFiles : redoFiles,
      item = stack.at(-1);
    if (!item) return;
    const expected = redo ? item.before : item.after;
    if (JSON.stringify(editor.files) !== JSON.stringify(expected.files))
      throw Error("请先撤销后续文本编辑，再撤销／重做文件操作");
    if (busyOperation) return;
    operationBusy(true);
    try {
      await restore(redo ? item.after : item.before, { compile: item.compile });
      stack.pop();
      other.push(item);
      onStatus((redo ? "已重做：" : "已撤销：") + item.label);
    } finally {
      operationBusy(false);
    }
  }
  const writable = (path) =>
    path?.startsWith("/project/") && !entries.get(path)?.readonly;
  const exists = (path) => entries.has(path) || directories.has(path);
  const selection = (path) => {
    const selected = fileTree?.selected() || [];
    return topPaths(
      path && selected.includes(path) ? selected : path ? [path] : selected,
    );
  };
  function remapState(state, map, copy = false) {
    state.tabs = state.tabs.map((p) => (copy ? p : map(p)));
    state.active = copy ? state.active : map(state.active || "");
    state.states = new Map(
      [...state.states].map(([p, s]) => [copy ? p : map(p), s]),
    );
    if (!copy && state.tree) {
      state.tree = {
        ...state.tree,
        selectedItems: state.tree.selectedItems.map(map),
        expandedItems: state.tree.expandedItems.map(map),
        focusedItem: state.tree.focusedItem
          ? map(state.tree.focusedItem)
          : null,
      };
      for (const path of state.tree.selectedItems) {
        let parent = parentPath(path);
        while (parent.startsWith("/project")) {
          if (!state.tree.expandedItems.includes(parent))
            state.tree.expandedItems.push(parent);
          parent = parentPath(parent);
        }
      }
    }
    if (!copy)
      state.bindings = Object.fromEntries(
        Object.entries(state.bindings).map(([k, v]) => [k, map(v)]),
      );
    return state;
  }
  async function changePaths(paths, target, { name, copy = false } = {}) {
    paths = topPaths(paths);
    if (!paths.length) return;
    if (paths.some((p) => !writable(p))) throw Error("只读资源不能修改");
    if (target !== "/project" && !directories.has(target))
      throw Error("请选择目标文件夹");
    return transaction(
      copy ? "复制" : name ? "重命名" : "移动",
      async (before) => {
        const mapping = {};
        for (const from of paths) {
          let to = projectPath(target + "/" + (name || fileName(from)));
          if (to === from && !copy) continue;
          if (within(to, from) && to !== from)
            throw Error("不能移入自身或子目录");
          if (exists(to) || Object.values(mapping).includes(to)) {
            const decision = await choose("目标已存在", to, [
              ["保留两份", "keep"],
              ["取消", null],
            ]);
            if (!decision) return;
            const ext = entries.has(from)
                ? /\.[^/.]+$/.exec(to)?.[0] || ""
                : "",
              stem = ext ? to.slice(0, -ext.length) : to;
            let n = 2;
            while (exists(to) || Object.values(mapping).includes(to))
              to = stem + " (" + n++ + ")" + ext;
          }
          mapping[from] = to;
        }
        if (!Object.keys(mapping).length) return;
        const result = relocate(before, mapping, await references(), { copy });
        if (result.issues.length) {
          const proceed = await choose(
            "部分引用需要手动检查",
            result.issues.map((r) => r.uri + "：" + r.message).join("\n"),
            [
              ["取消", null],
              [copy ? "仅复制" : "仅移动", "continue"],
            ],
          );
          if (!proceed) return;
        }
        return remapState(result.state, result.map, copy);
      },
    );
  }
  const moveFiles = (paths, target) => changePaths(paths, target);
  const renameFile = (path, name) =>
    changePaths([path], parentPath(path), { name: validateName(name) });
  async function createFile(target, folder) {
    const name = await choose(
      folder ? "新建文件夹" : "新建文件",
      "创建到 " + target,
      [
        ["创建", "ok"],
        ["取消", null],
      ],
      { value: folder ? "新建文件夹" : "untitled.lay" },
    );
    if (name == null) return;
    const path = projectPath(target + "/" + validateName(name));
    if (exists(path)) throw Error("名称已存在：" + path);
    await transaction(folder ? "新建文件夹" : "新建文件", async (before) => {
      if (folder) before.directories.add(path);
      else {
        const text = /\.laydoc$/i.test(path)
          ? 'doc = document(name="新文档")\n'
          : /\.lay$/i.test(path)
            ? "p = canvas(size=(160mm, 90mm))\n"
            : "";
        before.entries.set(path, {
          path,
          data: encoder.encode(text),
          readonly: false,
          type: null,
          original: "",
        });
        if (textFile.test(path)) before.files[path] = text;
        before.tabs.push(path);
        before.active = path;
      }
      before.project.directories = [...before.directories];
      return before;
    });
  }
  async function deleteFiles(paths) {
    paths = topPaths(paths);
    if (!paths.length) return;
    if (paths.some((p) => !writable(p) || within(editor.entry, p)))
      throw Error("只读资源和当前入口不能删除");
    await transaction("删除", async (before) => {
      const refs = (await references()).filter(
        (r) =>
          r.target &&
          paths.some((p) => within(r.target, p)) &&
          !paths.some((p) => within(r.uri, p)),
      );
      if (
        !(await choose(
          "删除项目文件",
          paths.join("\n") +
            (refs.length
              ? "\n以下文件仍有引用：\n" +
                [...new Set(refs.map((r) => r.uri))].join("\n")
              : "") +
            "\n删除后可在文件菜单撤销。",
          [
            ["取消", null],
            ["删除", "delete"],
          ],
        ))
      )
        return;
      for (const p of before.entries.keys())
        if (paths.some((root) => within(p, root))) {
          before.entries.delete(p);
          delete before.files[p];
        }
      for (const p of before.directories)
        if (paths.some((root) => within(p, root))) before.directories.delete(p);
      before.fonts = before.fonts.filter(
        (f) => !f.path || !paths.some((p) => within(f.path, p)),
      );
      before.tabs = before.tabs.filter((p) => before.entries.has(p));
      before.active = before.tabs.includes(before.active)
        ? before.active
        : before.tabs.at(-1);
      before.project.directories = [...before.directories];
      return before;
    });
  }
  function downloadFiles(paths) {
    const selected = topPaths(paths),
      zip = {};
    if (!selected.length) return;
    const download = (data, name) => {
      const a = node("a");
      a.href = URL.createObjectURL(new Blob([data]));
      a.download = name;
      a.click();
      setTimeout(() => URL.revokeObjectURL(a.href), 3000);
    };
    if (selected.length === 1 && entries.has(selected[0])) {
      const path = selected[0];
      download(
        editor.files[path] != null
          ? encoder.encode(editor.files[path])
          : entries.get(path).data,
        fileName(path),
      );
      return;
    }
    for (const path of selected) {
      const prefix = parentPath(path) + "/";
      for (const [p, e] of entries)
        if (within(p, path))
          zip[p.slice(prefix.length)] =
            editor.files[p] != null ? encoder.encode(editor.files[p]) : e.data;
      for (const p of directories)
        if (within(p, path))
          zip[p.slice(prefix.length) + "/"] = new Uint8Array();
    }
    download(
      webZip(zip),
      selected.length === 1 ? fileName(selected[0]) + ".zip" : "files.zip",
    );
  }
  async function importDrop(transfer, target) {
    if (!directories.has(target) && target !== "/project")
      target = parentPath(target);
    const list = await droppedFiles(transfer);
    await importFiles(list, target);
  }
  function creationMenu(target) {
    return [
      { name: "新建文件", run: () => createFile(target, false) },
      { name: "新建文件夹", run: () => createFile(target, true) },
      "separator",
      {
        name: "导入文件…",
        run: () => {
          selectedDirectory = target;
          root.querySelector("#import").click();
        },
      },
      {
        name: "导入文件夹…",
        run: () => {
          selectedDirectory = target;
          folderInput.click();
        },
      },
    ];
  }
  function fileMenu(path) {
    const paths = selection(path),
      readonly = paths.some((p) => !writable(p)),
      target =
        directories.has(path) || path === "/project"
          ? path
          : path
            ? parentPath(path)
            : selectedDirectory;
    return [
      ...(target.startsWith("/project") ? creationMenu(target) : []),
      "separator",
      {
        name: "打开",
        disabled: paths.length !== 1 || !entries.has(path),
        run: () => open(path),
      },
      {
        name: "重命名",
        disabled: readonly || paths.length !== 1,
        run: () => fileTree.rename(paths[0]),
      },
      {
        name: "剪切",
        disabled: readonly || !paths.length,
        run: () => {
          clipboardFiles = { paths, copy: false };
          onStatus("已剪切 " + paths.length + " 项");
        },
      },
      {
        name: "复制",
        disabled: readonly || !paths.length,
        run: () => {
          clipboardFiles = { paths, copy: true };
          onStatus("已复制 " + paths.length + " 项");
        },
      },
      {
        name: "粘贴",
        disabled: !clipboardFiles || !target.startsWith("/project"),
        run: async () => {
          await changePaths(clipboardFiles.paths, target, {
            copy: clipboardFiles.copy,
          });
        },
      },
      {
        name: "删除",
        disabled:
          readonly ||
          !paths.length ||
          paths.some((p) => within(editor.entry, p)),
        run: () => deleteFiles(paths),
      },
      "separator",
      {
        name: "下载",
        disabled: !paths.length,
        run: () => downloadFiles(paths),
      },
      { name: "复制路径", disabled: !path, run: () => writeClipboard(path) },
      "separator",
      {
        name: "撤销文件操作",
        disabled: !undoFiles.length,
        run: () => undoOperation(),
      },
      {
        name: "重做文件操作",
        disabled: !redoFiles.length,
        run: () => undoOperation(true),
      },
    ];
  }
  function fileShortcut(e, paths) {
    const mod = e.ctrlKey || e.metaKey;
    let action;
    if (e.key === "Delete") action = () => deleteFiles(paths);
    else if (mod && e.key.toLowerCase() === "z")
      action = () => undoOperation(e.shiftKey);
    else if (mod && ["c", "x"].includes(e.key.toLowerCase()))
      action = () => {
        if (paths.every(writable))
          clipboardFiles = { paths, copy: e.key.toLowerCase() === "c" };
      };
    else if (mod && e.key.toLowerCase() === "v" && clipboardFiles)
      action = () =>
        changePaths(clipboardFiles.paths, selectedDirectory, {
          copy: clipboardFiles.copy,
        });
    if (action) {
      e.preventDefault();
      e.stopPropagation();
      Promise.resolve().then(action).catch(report);
    }
  }
  function contextMenu(target, event) {
    const input = target.closest(
      "input:not([type=range]):not([type=file]),textarea",
    );
    if (input) return inputMenu(input);
    const source = target.closest(".live-editor");
    if (source) {
      const path = source.closest("[data-edit-file]").dataset.editFile;
      const view = editor.views.get(path)?.view;
      if (view) return editorMenu(view, event, editor.readonly.has(path));
    }
    const tab = target.closest(".file-tab");
    if (tab) {
      const path = tab.dataset.path;
      return [
        {
          name: "关闭",
          run: () =>
            openTabs.get(path).button.querySelector(".tab-close").click(),
        },
        {
          name: "关闭其他",
          run: () => {
            for (const [p, t] of openTabs)
              if (p !== path) t.button.querySelector(".tab-close").click();
          },
        },
        {
          name: "关闭全部",
          run: () => {
            for (const t of openTabs.values())
              t.button.querySelector(".tab-close").click();
          },
        },
        { name: "在文件树定位", run: () => fileTree.select(path) },
      ];
    }
    if (target.closest("#tree")) {
      const path = target.closest("[data-path]")?.dataset.path;
      if (path) {
        if (!fileTree.selected().includes(path))
          fileTree.tree.setSelectedItems([path]);
        fileTree.tree.getItemInstance(path).setFocused();
        if (path.startsWith("/project"))
          selectedDirectory =
            directories.has(path) || path === "/project"
              ? path
              : parentPath(path);
      }
      return fileMenu(path);
    }
    if (target.closest(".preview-pane") || presentationRoot()?.contains(target))
      return previewMenu(target);
    if (target.closest("dialog[open],.laymesh-color-popover"))
      return [
        {
          name: "关闭面板",
          run: () => {
            const d = target.closest("dialog");
            if (d) {
              const close = d.querySelector(".dialog-close");
              if (close) close.click();
              else if (
                d.dispatchEvent(new Event("cancel", { cancelable: true }))
              )
                d.close();
            } else
              target.closest(".laymesh-color-popover")?.dispatchEvent(
                new KeyboardEvent("keydown", {
                  key: "Escape",
                  bubbles: true,
                  cancelable: true,
                }),
              );
          },
        },
      ];
    return [
      ...creationMenu(selectedDirectory),
      "separator",
      { name: "字体管理", run: () => root.querySelector("#fonts").click() },
      { name: "导出作品", run: () => root.querySelector("#export").click() },
      {
        name: "显示／隐藏资源管理器",
        run: () => root.querySelector("#workspace-tree").click(),
      },
      ...previewMenu(target),
    ];
  }
  const fontManager = createFontManager({
    root,
    manifest,
    files,
    editor,
    compilerCall,
    resetCompiler,
    compileNow,
  });
  const { liveFonts, bindings, assertFonts, checkFontWarnings } = fontManager;
  editor.addTab = (path) => open(path);
  editor.openFile = open;
  editor.importFiles = importFiles;
  editor.entries = entries;
  fileTree = workspaceTree(tree, {
    entries,
    directories: () => directories,
    open: (p) => open(p, { fromTree: true }),
    selectDirectory: (p) => (selectedDirectory = p),
    move: moveFiles,
    rename: renameFile,
    importDrop,
    onError: report,
    menu: (e, p) => menus.show(e, fileMenu(p)),
    shortcut: fileShortcut,
  });
  menus = createContextMenu({
    root,
    onError: report,
    contains: (target) =>
      root.contains(target) ||
      !!target.closest("dialog[open],.laymesh-color-popover") ||
      !!presentationRoot()?.contains(target),
    resolve: contextMenu,
  });
  buildTree();
  await open(editor.entry);
  return {
    editor,
    whenIdle: async () => {
      await importChain.catch(() => {});
      await waitForOperation();
    },
    liveFonts,
    bindings,
    compilerBindings: fontManager.compilerBindings,
    refreshFonts: fontManager.refresh,
    assertFonts,
    checkFontWarnings,
    entries,
    open,
    markSaved() {
      for (const e of entries.values())
        if (editor.files[e.path] != null) e.original = editor.files[e.path];
      refreshTabs();
    },
    dispose() {
      fileTree?.dispose();
      menus?.dispose();
      folderInput.remove();
      fontManager.dispose();
      for (const o of observers.values()) o.disconnect();
      for (const { view } of editor.views.values()) view.destroy();
      for (const u of urls.values()) URL.revokeObjectURL(u);
    },
  };
}
