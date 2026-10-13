import {
  canonicalFont,
  fontFamilyKey,
  fontWeight,
  fontItalic,
  fontAliases,
} from "./font-aliases.mjs";
import { fontNames, displayFont } from "./font-names.mjs";
import { fontPicker } from "./font-picker.mjs";
const fontFile = /\.(ttf|otf|ttc|otc)$/i;
const generic = new Set([
  "sans-serif",
  "serif",
  "monospace",
  "system-ui",
  "ui-sans-serif",
  "ui-serif",
  "ui-monospace",
]);
const key = (f) => f.postscriptName || f.path + "#" + (f.fontIndex || 0);
const canonical = canonicalFont;
const named = (name, f) =>
  [
    f.family,
    f.postscriptName,
    ...(f.names || [])
      .filter((n) => [1, 16].includes(n.nameID))
      .map((n) => n.value),
  ].some((n) => fontFamilyKey(n) === fontFamilyKey(name));
export function createFontManager({
  root,
  manifest,
  files,
  editor,
  compilerCall,
  resetCompiler,
  compileNow,
}) {
  const panel = root.querySelector("#font-panel"),
    notice = panel.querySelector(".font-notice"),
    footer = panel.querySelector(".font-summary"),
    apply = root.querySelector("#font-apply");
  const bindings = { ...(manifest.fontBindings || {}) },
    drafts = new Map(),
    liveFonts = (manifest.fonts || [])
      .filter((f) => f.file && files[f.file])
      .map((f) => ({
        ...f,
        bytes: files[f.file],
        source: f.source || "legacy",
      }));
  let localFonts = [],
    pickers = [],
    rows = new Map(),
    applying = false,
    disposed = false,
    dynamic = [];
  let catalog = null;
  const loading = new Map();
  let connected = false;
  const collator = new Intl.Collator("zh-CN");
  const requirements = () => {
    const requested = (manifest.requestedFonts || []).filter(
      (r) => !generic.has(r.family),
    );
    const base = manifest.fontRequirements || manifest.fonts || [];
    const replacements = (
      manifest.fontReplacementRequirements ||
      Object.keys(bindings).map((family) => ({ family, inherited: true }))
    ).filter((r) =>
      r.inherited
        ? (manifest.requestedFonts || []).some((q) => generic.has(q.family)) ||
          !manifest.requestedFonts
        : requested.some((q) => q.family === r.family),
    );
    const combined = [
      ...replacements,
      ...requested,
      ...base.filter(
        (r) =>
          (!requested.length ||
            requested.some((q) => named(bindings[q.family] || q.family, r))) &&
          !replacements.some((q) => named(bindings[q.family] || q.family, r)),
      ),
      ...dynamic,
    ];
    const seen = new Set();
    return combined.filter((r) => {
      const id = r.family + "|" + (r.weight || 400) + "|" + !!r.italic;
      if (seen.has(id)) return false;
      seen.add(id);
      return (
        !r.dynamic ||
        Object.values(editor.files).some((t) => t.includes(r.family))
      );
    });
  };
  const matches = (r, f) =>
    (r.postscriptName &&
      f.postscriptName &&
      r.postscriptName === f.postscriptName) ||
    (named(r.family, f) &&
      fontWeight(f) === (r.weight || 400) &&
      fontItalic(f) === !!r.italic);
  const prepared = (r, map = bindings) =>
    liveFonts.some((f) =>
      map[r.family]
        ? named(map[r.family], f) &&
          fontWeight(f) === (r.weight || 400) &&
          fontItalic(f) === !!r.italic
        : matches(r, f) ||
          (fontFile.test(r.family) &&
            f.path?.endsWith("/" + r.family.split("/").at(-1))),
    );
  const missing = (map = bindings) =>
    requirements().filter((r) => !prepared(r, map));
  const choices = () => {
    if (catalog) return catalog;
    const merged = new Map(localFonts.map((f) => [key(f), f]));
    for (const f of liveFonts) merged.set(key(f), f);
    return (catalog = [...merged.values()].sort(
      (a, b) =>
        collator.compare(displayFont(a).family, displayFont(b).family) ||
        (a.weight || 400) - (b.weight || 400),
    ));
  };
  const setNotice = (message, error = false) => {
    notice.textContent = message;
    notice.dataset.error = String(error);
  };
  const update = () => {
    const absent = missing();
    root.querySelector("#font-status").textContent = absent.length
      ? "待补齐 " + absent.length + " 个字体"
      : "作品字体已准备";
    footer.textContent = applying
      ? "正在加载并应用…"
      : drafts.size
        ? drafts.size + " 项选择尚未应用"
        : absent.length
          ? absent.length + " 项仍需补齐"
          : "字体已准备";
    const available = choices();
    for (const picker of pickers) picker.update(available);
    const needed = requirements();
    for (const [family, row] of rows) {
      const r = needed.find((r) => r.family === family);
      row.querySelector("small").textContent = drafts.has(family)
        ? "尚未应用"
        : r && prepared(r)
          ? bindings[family]
            ? "使用替代字体"
            : "已准备"
          : "待补齐";
    }
  };
  async function describe(path, data, source) {
    const result = await compilerCall({ type: "fontInfo", bytes: data });
    if (!result.faces.length) throw Error("字体文件无法读取");
    const metadata = await fontNames(new Blob([data])).catch(() => []);
    return result.faces.map((face) => ({
      ...face,
      names:
        metadata.find((f) => f.postscriptName === face.postscriptName)?.names ||
        [],
      path,
      bytes: data,
      source,
    }));
  }
  async function install(path, data, source, { reset = true } = {}) {
    const faces = await describe(path, data, source);
    for (let i = liveFonts.length - 1; i >= 0; i--)
      if (liveFonts[i].path === path) liveFonts.splice(i, 1);
    liveFonts.push(...faces);
    catalog = null;
    if (reset) resetCompiler();
    render();
    return faces;
  }
  function render() {
    if (disposed) return;
    for (const p of pickers) p.dispose();
    pickers = [];
    rows.clear();
    const list = panel.querySelector(".font-list");
    list.replaceChildren();
    const seen = new Set();
    for (const r of requirements()) {
      if (seen.has(r.family)) continue;
      seen.add(r.family);
      const row = document.createElement("div");
      row.className = "font-row";
      row.dataset.family = r.family;
      const info = document.createElement("div"),
        title = document.createElement("strong"),
        small = document.createElement("small");
      info.className = "font-requirement";
      title.textContent = displayFont(r).family;
      title.title = r.family;
      info.append(title, small);
      row.append(info);
      rows.set(r.family, row);
      const choice = choices().find(
        (f) =>
          drafts.get(r.family) === key(f) ||
          (!drafts.has(r.family) &&
            bindings[r.family] &&
            named(bindings[r.family], f)),
      );
      const picker = fontPicker({
        dialog: panel,
        label: "替换 " + r.family,
        placeholder: "搜索替代字体（中文或英文）…",
        value: choice ? key(choice) : "",
        choices: choices(),
        onSelect: (id) => {
          if (id) drafts.set(r.family, id);
          else if (bindings[r.family]) drafts.set(r.family, "");
          else drafts.delete(r.family);
          if (notice.dataset.error === "true")
            setNotice("替代字体选择已更新，点击“确认并应用”重试。");
          update();
        },
      });
      pickers.push(picker);
      row.append(picker.box);
      list.append(row);
    }
    if (localFonts.length) {
      const row = document.createElement("div");
      row.className = "font-catalog";
      const label = document.createElement("label");
      label.textContent = "加载其他本机字体";
      let selected;
      const picker = fontPicker({
          dialog: panel,
          label: "选择其他本机字体",
          choices: choices(),
          onSelect: (id) => {
            selected = id;
          },
        }),
        button = document.createElement("button");
      button.textContent = "加载选中字体";
      button.onclick = async () => {
        const f = choices().find((f) => key(f) === selected);
        if (!f) return;
        button.disabled = true;
        try {
          await load(f);
          resetCompiler();
          if (!missing().length) await compileNow();
          setNotice("已加载 " + displayFont(f).family);
        } catch (e) {
          setNotice(e.message, true);
        } finally {
          button.disabled = false;
        }
      };
      pickers.push(picker);
      row.append(label, picker.box, button);
      list.append(row);
    }
    update();
  }
  async function load(f) {
    if (liveFonts.some((v) => key(v) === key(f) && v.bytes?.length)) return;
    if (!f.blob) throw Error("字体不可读取，请重新连接或导入");
    if (!loading.has(key(f)))
      loading.set(
        key(f),
        (async () => {
          const blob = await f.blob();
          if (disposed) return;
          await install(
            "/@local/" + f.postscriptName + ".ttf",
            new Uint8Array(await blob.arrayBuffer()),
            "local",
            { reset: false },
          );
        })().finally(() => loading.delete(key(f))),
      );
    return loading.get(key(f));
  }
  async function loadRequired() {
    const errors = [];
    for (const r of missing()) {
      const target = bindings[r.family]
        ? { ...r, family: bindings[r.family], postscriptName: undefined }
        : r;
      const f =
        (target.postscriptName &&
          localFonts.find((f) => f.postscriptName === target.postscriptName)) ||
        localFonts.find((f) => matches(target, f));
      if (f)
        try {
          await load(f);
        } catch (e) {
          errors.push(displayFont(f).family + "：" + e.message);
        }
    }
    return errors;
  }
  // Automatic same-family aliases are compiler input, never user replacements.
  function compilerBindings() {
    const result = {};
    for (const f of liveFonts) {
      for (const name of [
        f.family,
        ...(f.names || [])
          .filter((n) => [1, 16].includes(n.nameID))
          .map((n) => n.value),
      ]) {
        for (const alias of fontAliases(name))
          if (alias !== f.family) result[alias] = f.family;
      }
    }
    return { ...result, ...bindings };
  }
  async function applyFonts() {
    if (applying) return;
    applying = true;
    apply.disabled = true;
    root.querySelector("#font-later").disabled = true;
    panel.querySelector(".dialog-close").disabled = true;
    update();
    try {
      const next = { ...bindings },
        available = choices(),
        chosen = [];
      for (const [family, id] of drafts) {
        if (!id) {
          delete next[family];
          continue;
        }
        const f = available.find((f) => key(f) === id);
        if (!f) throw Error("所选字体已不可用：" + family);
        next[family] = f.family;
        chosen.push(f);
      }
      for (const f of chosen) {
        await load(f);
        for (const r of requirements().filter(
          (r) => bindings[r.family] === f.family || next[r.family] === f.family,
        )) {
          const weight = (v) =>
            v.weight ||
            (/bold|粗/i.test(v.style)
              ? 700
              : /light|细/i.test(v.style)
                ? 300
                : 400);
          const extra = localFonts
            .filter((v) => v.family === f.family)
            .sort(
              (a, b) =>
                Math.abs(weight(a) - (r.weight || 400)) -
                  Math.abs(weight(b) - (r.weight || 400)) ||
                Number(/italic|斜/i.test(a.style) !== !!r.italic) -
                  Number(/italic|斜/i.test(b.style) !== !!r.italic),
            )[0];
          if (extra) await load(extra);
        }
      }
      const absent = missing(next);
      if (absent.length)
        throw Error(
          "仍需为以下字体选择替代或导入：" +
            [...new Set(absent.map((r) => displayFont(r).family))].join("、"),
        );
      const changed = drafts.size > 0;
      const previousRequirements = requirements();
      for (const name of Object.keys(bindings)) delete bindings[name];
      Object.assign(bindings, next);
      manifest.fontReplacementRequirements = previousRequirements
        .filter((r) => bindings[r.family])
        .map((r) => ({
          ...r,
          inherited:
            r.inherited ??
            !(manifest.requestedFonts || []).some((q) => q.family === r.family),
        }));
      manifest.fontBindings = { ...bindings };
      drafts.clear();
      resetCompiler();
      if (
        changed ||
        root.dataset.compileState === "fonts" ||
        ["pending", "error"].includes(root.dataset.compileState)
      )
        await compileNow();
      setNotice("字体已应用");
      render();
      // Keep the non-blocking guide's geometry stable across success/error.
      if (missing().length) root.querySelector("#font-guide").hidden = false;
      panel.close();
    } catch (e) {
      setNotice(e.message, true);
      render();
      notice.focus();
    } finally {
      applying = false;
      apply.disabled = false;
      root.querySelector("#font-later").disabled = false;
      panel.querySelector(".dialog-close").disabled = false;
      update();
    }
  }
  apply.onclick = applyFonts;
  const close = () => {
    if (!applying) panel.close();
  };
  panel.querySelector(".dialog-close").onclick = close;
  root.querySelector("#font-later").onclick = close;
  panel.addEventListener("cancel", (e) => {
    if (applying) e.preventDefault();
  });
  root.querySelector("#fonts").onclick = () => {
    render();
    panel.showModal();
  };
  root.querySelector("#connect-fonts").onclick = async () => {
    const button = root.querySelector("#connect-fonts");
    button.disabled = true;
    apply.disabled = true;
    try {
      if (typeof window.queryLocalFonts !== "function")
        throw Error("此浏览器不支持访问本机字体，请导入字体文件");
      const request = window.queryLocalFonts();
      localFonts = await request;
      if (disposed) return;
      catalog = null;
      connected = true;
      render();
      setNotice("已连接，可搜索中文或英文字体名。正在准备作品需要的字体…");
      const errors = await loadRequired();
      if (disposed) return;
      resetCompiler();
      render();
      setNotice(
        errors.length
          ? errors.join("；")
          : missing().length
            ? "已连接，部分作品字体未找到，可导入原字体或选择替代字体。"
            : "作品字体已自动匹配。",
        !!errors.length,
      );
      if (!missing().length) await compileNow();
    } catch (e) {
      setNotice(
        e.name === "NotAllowedError"
          ? "未获得字体授权，请重试连接或导入字体文件。"
          : e.message,
        true,
      );
    } finally {
      button.disabled = false;
      apply.disabled = false;
    }
  };
  root.querySelector("#font-import").onclick = () =>
    root.querySelector("#font-file").click();
  root.querySelector("#font-file").onchange = (e) =>
    editor
      .importFiles(e.target.files)
      .catch((e) => setNotice(e.message, true))
      .finally(() => {
        e.target.value = "";
      });
  root.querySelector("#dismiss-fonts").onclick = () =>
    (root.querySelector("#font-guide").hidden = true);
  async function assertFonts() {
    const errors = [];
    for (const [path, data] of Object.entries(editor.assets))
      if (fontFile.test(path) && !liveFonts.some((f) => f.path === path))
        try {
          await install(path, data, "project");
        } catch (error) {
          errors.push(path + "：" + error.message);
        }
    if (connected) errors.push(...(await loadRequired()));
    if (errors.length)
      setNotice("字体文件损坏或无法读取：" + errors.join("；"), true);
    const absent = missing();
    if (absent.length) {
      root.dataset.compileState = "fonts";
      root.querySelector("#font-guide").hidden = false;
      render();
      throw Error(
        (errors.length
          ? "字体文件损坏或无法读取：" + errors.join("；") + "。仍需补齐："
          : "缺少字体：") +
          [...new Set(absent.map((f) => displayFont(f).family))].join("、") +
          (connected
            ? "。请导入原字体或选择替代字体。"
            : "。请连接本机字体或导入字体文件。"),
      );
    }
  }
  function checkFontWarnings(warnings) {
    const absent = [];
    for (const w of warnings || []) {
      const match =
        w.code === "W_FONT" && /^字体 (.+?) 不可用(?:[：:]|$)/.exec(w.message);
      if (!match) continue;
      if (
        w.message.includes("不可用：") &&
        !Object.values(bindings).some((target) =>
          liveFonts.some((f) => named(target, f) && named(match[1], f)),
        ) &&
        !requirements().some((r) => named(r.family, { family: match[1] }))
      )
        continue;
      if (
        /不可用$/.test(w.message) &&
        bindings[match[1]] &&
        liveFonts.some((f) => named(bindings[match[1]], f))
      )
        continue;
      const family = match[1];
      if (!requirements().some((r) => r.family === family))
        dynamic.push({ family, dynamic: true });
      absent.push(family);
    }
    if (absent.length) {
      root.dataset.compileState = "fonts";
      root.querySelector("#font-guide").hidden = false;
      render();
      throw Error(
        "字体不可用：" + absent.join("、") + "。请导入或确认替代字体。",
      );
    }
  }
  render();
  return {
    bindings,
    compilerBindings,
    liveFonts,
    install,
    describe,
    assertFonts,
    checkFontWarnings,
    refresh() {
      catalog = null;
      dynamic = [];
      render();
    },
    dispose() {
      disposed = true;

      for (const p of pickers) p.dispose();
    },
  };
}
