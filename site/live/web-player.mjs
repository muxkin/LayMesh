import { installExportDialog } from "./web-export-dialog.mjs";
import { selectWebLicenses } from "./web-licenses.mjs";
import { selectPageTransitions } from "./web-transition-package.mjs";
import { Presentation } from "./presentation.mjs";
import { createContextMenu, inputMenu } from "./workspace-menu.mjs";
import { buildShell } from "./web-shell.mjs";
import { iconLabel } from "./presentation-icons.mjs";
import { PresentationClock } from "./presentation-clock.mjs";
import { sha256 } from "./web-sha256.mjs";
import { WebViewport } from "./web-viewport.mjs";
const textExtensions = /\.(lay|laydoc|lcss|css|csv|json)$/i;
export function outlineList(list) {
  const walk = (n) => {
    const k = n.kind;
    if (k.kind === "glyphs") {
      const { kind, resource, glyphs, outlines, ...text } = k;
      n.hint = { ...n.hint, exportText: text };
      n.kind = {
        kind: "group",
        clip: null,
        mask: null,
        children: k.outlines.map((path) => ({
          name: "",
          transform: [1, 0, 0, 1, 0, 0],
          opacity: 1,
          effects: [],
          semantic: null,
          hint: null,
          kind: {
            kind: "path",
            path,
            fill: { kind: "solid", rgba: k.color },
            stroke: null,
            even_odd: false,
          },
        })),
      };
    } else if (k.kind === "group") {
      k.children.forEach(walk);
      if (k.mask) walk(k.mask.node);
    } else if (k.kind === "path" && k.fill.kind === "pattern")
      k.fill.nodes.forEach(walk);
  };
  list.nodes.forEach(walk);
  for (const [id, r] of Object.entries(list.resources))
    if (!r.mime.startsWith("image/")) delete list.resources[id];
  return list;
}
export async function mountWebPlayer({
  manifest,
  files,
  moduleURL,
  archive,
  transportMetrics = {},
  root,
}) {
  const decoder = new TextDecoder(),
    encoder = new TextEncoder(),
    bytes = (name) => {
      if (!files[name]) throw Error("Missing resource: " + name);
      return files[name];
    },
    json = (name) => JSON.parse(decoder.decode(bytes(name)));
  const style = document.createElement("style");
  style.textContent = decoder.decode(bytes("runtime/web-player.css"));
  document.head.append(style);
  document.documentElement.lang = "zh-CN";
  document.title = manifest.name || "LayMesh";
  const el = buildShell(root, manifest.editable),
    status = (message) => {
      if (el("status")) {
        el("status").textContent = message;
        el("status").title = message;
      }
    },
    stage = root.querySelector(".stage");
  root.querySelector(".project-name").textContent = manifest.name || "LayMesh";
  root.dataset.mode = "source";
  const metrics = {
      compiles: 0,
      frames: 0,
      prepareMs: 0,
      sampleMs: 0,
      rasterMs: 0,
      container: manifest.container,
    },
    clock = new PresentationClock(),
    reduced = matchMedia("(prefers-reduced-motion: reduce)");
  Object.assign(metrics, transportMetrics);
  let pages = manifest.pages.map((p) => ({ ...p, ...json(p.file) })),
    index = 0,
    runtime,
    worker,
    request = 0,
    pending = new Map(),
    mainQueue = Promise.resolve(),
    serial = 0,
    painting = false,
    viewStatic = true,
    requestedPaint = null,
    paintPromise = Promise.resolve(),
    busy = false,
    dirty = false,
    closed = false,
    last = performance.now(),
    raf;
  const view = { mode: "fit", scale: 1, pan: [0, 0], rulers: true };
  const preview = new WebViewport(
    stage,
    view,
    () => pages[index],
    refreshViewport,
  );
  let viewPaintTimer;
  function viewport() {
    preview.layout();
    return preview.request()?.viewport || [0, 0, 1, 1];
  }
  function updateView() {
    if (closed || !el("zoom")) return;
    preview.layout();
    el("zoom").value = view.mode === "fit" ? "fit" : String(view.scale);
    if (!el("zoom").value) {
      let option = el("zoom").querySelector("[data-custom]");
      if (!option) {
        option = new Option();
        option.dataset.custom = "true";
        el("zoom").append(option);
      }
      option.value = view.scale;
      option.textContent = Math.round(view.scale * 100) + "%";
      el("zoom").value = option.value;
    }
  }
  function refreshViewport() {
    if (closed || !el("zoom") || presentation.opened) return;
    if (worker) worker.postMessage({ type: "target", serial });
    else if (runtime) call({ type: "target", serial }).catch(fail);
    updateView();
    clearTimeout(viewPaintTimer);
    if (!preview.presented && !busy) return;
    const time = viewStatic || clock.staticPage ? null : clock.time;
    if (preview.covered(time, pages[index].id)) {
      root.dataset.painted = "true";
      return;
    }
    // CSS updates immediately; only the sharp-density repaint settles briefly.
    root.dataset.painted = "false";
    if (preview.presented && preview.presented.scale !== view.pixelsPerMM)
      viewPaintTimer = setTimeout(() => paint(viewStatic), 80);
    else paint(viewStatic);
  }
  el("fit").onclick = () => {
    view.mode = "fit";
    view.pan = [0, 0];
    refreshViewport();
  };
  el("zoom").onchange = () => {
    view.mode = el("zoom").value === "fit" ? "fit" : "zoom";
    if (view.mode === "zoom") view.scale = Number(el("zoom").value);
    view.pan = [0, 0];
    refreshViewport();
  };
  el("rulers").onclick = () => {
    view.rulers = !view.rulers;
    el("rulers").setAttribute("aria-pressed", String(view.rulers));
    updateView();
  };
  stage.onwheel = (e) => {
    e.preventDefault();
    if (e.ctrlKey || e.metaKey) {
      const rect = preview.scroll.getBoundingClientRect(),
        x = e.clientX - rect.left - rect.width / 2,
        y = e.clientY - rect.top - rect.height / 2,
        old = view.pixelsPerMM;
      view.scale = Math.max(
        0.05,
        Math.min(
          16,
          (old / (pages[index].list.layout_dpi || 96)) *
            25.4 *
            Math.exp(-e.deltaY * 0.002),
        ),
      );
      view.mode = "zoom";
      const next = (view.scale * (pages[index].list.layout_dpi || 96)) / 25.4;
      view.pan = [
        x - ((x - view.pan[0]) * next) / old,
        y - ((y - view.pan[1]) * next) / old,
      ];
    } else {
      preview.panBy(-e.deltaX, -e.deltaY);
    }
    preview.schedule();
  };
  let panKey = false,
    drag;
  document.addEventListener("keydown", (e) => {
    if (
      e.target.closest(
        ".edit-pane,.workspace-tree,button,summary,input,select,dialog",
      ) ||
      presentation.opened
    )
      return;
    if (e.code === "Space") {
      panKey = true;
      e.preventDefault();
    }
    if (e.key === "0") el("fit").click();
    if (["+", "-", "="].includes(e.key)) {
      view.scale =
        (view.mode === "fit"
          ? (view.pixelsPerMM * 25.4) / (pages[index].list.layout_dpi || 96)
          : view.scale) * (e.key === "-" ? 0.8 : 1.25);
      view.mode = "zoom";
      refreshViewport();
    }
  });
  document.addEventListener("keyup", (e) => {
    if (e.code === "Space") panKey = false;
  });
  stage.onpointerdown = (e) => {
    if (panKey || e.button === 1) {
      e.preventDefault();
      drag = [e.clientX, e.clientY, ...view.pan];
      stage.setPointerCapture(e.pointerId);
    }
  };
  stage.onpointermove = (e) => {
    const point = preview.coordinates(e.clientX, e.clientY),
      unit = preview.page?.unit || "mm";
    el("coordinates").textContent = point
      ? point.display.map((v) => v.toFixed(1)).join(" , ") + " " + unit
      : "—";
    if (drag) {
      preview.panTo([
        drag[2] + e.clientX - drag[0],
        drag[3] + e.clientY - drag[1],
      ]);
    }
  };
  stage.onpointerup = () => (drag = null);
  stage.onpointercancel = () => (drag = null);
  const size = () =>
    preview.request() || { width: 1, height: 1, viewport: [0, 0, 1, 1] };
  async function call(data, transfer = []) {
    if (runtime) {
      if (data.type === "target") return runtime(data);
      const q = mainQueue.then(() => runtime(data));
      mainQueue = q.catch(() => {});
      return q;
    }
    return new Promise((resolve, reject) => {
      const id = ++request,
        timer = setTimeout(() => {
          pending.delete(id);
          reject(Error("显示资源准备超时"));
        }, 60000);
      pending.set(id, { resolve, reject, timer });
      worker.postMessage({ ...data, id }, transfer);
    });
  }
  const fail = (error) => {
    clock.running = false;
    let message = error.message || String(error);
    try {
      const diagnostic = JSON.parse(message);
      if (diagnostic.message)
        message =
          (diagnostic.file
            ? diagnostic.file.split("/").at(-1) +
              (diagnostic.loc
                ? " " + diagnostic.loc.line + ":" + diagnostic.loc.column
                : "") +
              " · "
            : "") + diagnostic.message;
    } catch {}
    status(message);
    root.dataset.error = message;
  };
  const workerURLs = [];
  const workerScripts = new Map();
  function makeWorker(name, renderer = false, exporter = false) {
    const key = String(renderer) + String(exporter) + name;
    if (workerScripts.has(key)) return new Worker(workerScripts.get(key));
    // file:// gives each module worker an opaque origin. Inline dependencies
    // rather than asking it to fetch blobs created in the owning window.
    const stripImports = (code) => code.replace(/^import\b[\s\S]*?;\s*/gm, "");
    const binding = decoder.decode(
      bytes(
        "runtime/" +
          (exporter
            ? manifest.editable
              ? "exporter-full"
              : "exporter"
            : renderer
              ? "wasm"
              : "compiler") +
          "/laymesh_wasm.js",
      ),
    );
    const parts =
      renderer || exporter
        ? [
            "skia/canvaskit.mjs",
            "presentation-transitions-data.mjs",
            "presentation-transitions.mjs",
            "skia-renderer.mjs",
            "presentation-resources.mjs",
            "presentation-renderer.mjs",
            ...(exporter && !manifest.editable
              ? [
                  "web-archive.mjs",
                  "web-image-density.mjs",
                  "web-vector-export.mjs",
                ]
              : []),
            name,
          ]
        : [name];
    let source =
      binding +
      "\nconst init=__wbg_init;\n" +
      parts
        .map((p) =>
          stripImports(decoder.decode(bytes("runtime/" + p))).replace(
            /^export default CanvasKitInit;\s*/m,
            "",
          ),
        )
        .join("\n");
    // Chromium refuses module workers at opaque file:// origins, including
    // import-free blobs. A classic worker with inlined bindings works offline.
    source =
      '"use strict";\n' +
      source
        .replace(/^export\s+\{[^}]*\};?\s*/gm, "")
        .replace(
          /^export\s+(?=(?:async\s+)?function|class|const|let|var)/gm,
          "",
        )
        .replace(/import\.meta\.url/g, "self.location.href");
    const url = URL.createObjectURL(
      new Blob([source], { type: "text/javascript" }),
    );
    workerURLs.push(url);
    workerScripts.set(key, url);
    return new Worker(url);
  }
  async function initRenderer() {
    let canvas = preview.canvas;
    const init = {
      type: "init",
      wasmBytes: bytes("runtime/wasm/laymesh.wasm"),
      skiaBytes: bytes("runtime/skia/canvaskit.wasm"),
      budget: 256 * 1048576,
      preserveDrawingBuffer: true,
    };
    if (canvas.transferControlToOffscreen && typeof Worker !== "undefined") {
      worker = makeWorker("presentation-worker.mjs", true);
      worker.onmessage = ({ data }) => {
        if (data.type === "context") {
          clock.running = false;
          if (data.ready) go(index, true).catch(fail);
          return;
        }
        const p = pending.get(data.id);
        if (p) {
          pending.delete(data.id);
          clearTimeout(p.timer);
          data.error ? p.reject(Error(data.error)) : p.resolve(data);
        }
      };
      worker.onerror = () => {
        for (const p of pending.values()) {
          clearTimeout(p.timer);
          p.reject(Error("显示 Worker 失败"));
        }
        pending.clear();
      };
      try {
        const offscreen = canvas.transferControlToOffscreen();
        await call({ ...init, canvas: offscreen }, [offscreen]);
        return;
      } catch (error) {
        worker.terminate();
        worker = null;
        const replacement = canvas.cloneNode();
        canvas.replaceWith(replacement);
        canvas = replacement;
        preview.canvas = replacement;
      }
    }
    const { createPresentationRuntime } = await import(
      moduleURL("runtime/presentation-worker.mjs")
    );
    runtime = await createPresentationRuntime({ ...init, canvas }, (data) => {
      clock.running = false;
      if (data.ready) go(index, true).catch(fail);
    });
  }
  function resources(page) {
    return Object.fromEntries(
      Object.keys(page.list.resources).map((id) => [
        id,
        page.resources?.[id] || bytes(manifest.resources[id]),
      ]),
    );
  }
  function sync() {
    if (closed || !el("play")) return;
    iconLabel(el("play"), clock.running ? "pause" : "play");
    el("play").setAttribute("aria-label", clock.running ? "暂停" : "播放");
    el("preview-state").textContent = viewStatic
      ? "完整排版"
      : clock.running
        ? "正在播放"
        : "动画预览";
    el("full").disabled = busy || dirty;
    el("windowed").disabled = busy || dirty;
    el("seek").max = String(clock.duration);
    el("seek").value = String(clock.time);
    el("time").textContent = clock.time.toFixed(2) + " s";
    el("page").value = String(index);
    root.dataset.running = String(clock.running);
    root.dataset.time = String(clock.time);
    for (const id of [
      "play",
      "next",
      "previous",
      "replay",
      "static",
      "page",
      "seek",
      "mode",
    ])
      el(id).disabled = busy || dirty;
  }
  async function go(n, preserve = false) {
    if (closed) return;
    if (!preserve) {
      viewStatic = true;
      view.mode = "fit";
      view.pan = [0, 0];
      preview.reset();
    }
    const start = performance.now();
    const resume = preserve && clock.running;
    busy = true;
    clock.running = false;
    requestedPaint = null;
    root.dataset.painted = "false";
    const current = ++serial;
    n = Math.max(0, Math.min(n, pages.length - 1));
    const transition = !preserve && n !== index ? { reverse: n < index } : null;
    index = n;
    updateView();
    const p = pages[n],
      time = preserve ? clock.time : 0;
    try {
      if (worker) worker.postMessage({ type: "target", serial: current });
      else await call({ type: "target", serial: current });
      const installed = await call({
        type: "load",
        installOnly: true,
        page: { ...p, resources: resources(p) },
        priority: true,
        ...size(),
      });
      if (current !== serial) return;
      index = n;
      clock.install(p.inspection?.animation, reduced.matches);
      if (preserve) clock.seek(time);
      const drawSize = size();
      const frame = await call({
        type: "activate",
        transition,
        serial: current,
        pageId: p.id,
        time: viewStatic || clock.staticPage ? null : clock.time,
        reduced: reduced.matches,
        ...drawSize,
      });
      if (current !== serial || frame.type === "stale") return;
      preview.present(
        drawSize,
        viewStatic || clock.staticPage ? null : clock.time,
        p.id,
      );
      metrics.pageInstalls =
        (metrics.pageInstalls || 0) + (installed.installed ? 1 : 0);
      metrics.prepareMs = performance.now() - start;
      Object.assign(metrics, {
        width: drawSize.width,
        height: drawSize.height,
        rasterMs: frame.rasterMs,
        backend: frame.backend,
        pageId: p.id,
        transitionName: frame.transitionName,
        transitionMs: frame.transitionMs,
        transitionFrames: frame.transitionFrames,
        transitionError: frame.transitionError,
        wasmMemoryBytes: frame.wasmMemoryBytes,
        skiaMemoryBytes: frame.skiaMemoryBytes,
        residentCacheBytes: frame.residentCacheBytes,
        preserveDrawingBuffer: frame.preserveDrawingBuffer,
      });
      root.dataset.painted = "true";
      if (!dirty) {
        status("可以播放");
        delete root.dataset.error;
      }
    } finally {
      if (current === serial) {
        busy = false;
        clock.running = !!resume || (!preserve && clock.running);
        last = performance.now();
        sync();
        if (requestedPaint !== null) paint(requestedPaint);
      }
    }
  }
  function pageOptions() {
    el("page").replaceChildren(
      ...pages.map((p, i) => {
        const o = document.createElement("option");
        o.value = i;
        o.textContent = i + 1 + " · " + p.name;
        return o;
      }),
    );
  }
  function paint(staticPage = false) {
    if (closed) return Promise.resolve();
    requestedPaint = staticPage;
    if (painting || busy) return paintPromise;
    painting = true;
    root.dataset.painting = "true";
    paintPromise = (async () => {
      try {
        while (requestedPaint !== null && !busy && !closed) {
          const staticFrame = requestedPaint;
          requestedPaint = null;
          const current = serial;
          const drawSize = size(),
            drawTime =
              staticFrame || viewStatic || clock.staticPage ? null : clock.time;
          if (preview.covered(drawTime, pages[index].id)) {
            root.dataset.painted = "true";
            continue;
          }
          const frame = await call({
            type: "paint",
            serial: current,
            pageId: pages[index].id,
            time:
              staticFrame || viewStatic || clock.staticPage ? null : clock.time,
            reduced: reduced.matches,
            running: clock.running,
            ...drawSize,
          });
          if (current === serial) {
            if (frame.skipped || frame.type === "stale") continue;
            preview.present(drawSize, drawTime, pages[index].id);
            root.dataset.painted = "true";
            metrics.frames++;
            Object.assign(metrics, {
              sampleMs: frame.sampleMs,
              rasterMs: frame.rasterMs,
              width: drawSize.width,
              height: drawSize.height,
            });
          }
        }
      } catch (error) {
        fail(error);
      } finally {
        painting = false;
        root.dataset.painting = "false";
      }
    })();
    return paintPromise;
  }
  function tick(now) {
    if (closed) return;
    const seconds = Math.max(0, (now - last) / 1000);
    last = now;
    if (clock.running && !busy && !dirty) {
      if (clock.tick(seconds) && index + 1 < pages.length)
        go(index + 1).catch(fail);
      else paint();
      sync();
    }
    raf = requestAnimationFrame(tick);
  }
  function advance() {
    viewStatic = false;
    if (clock.advance()) {
      if (index + 1 < pages.length) go(index + 1).catch(fail);
    } else paint();
    sync();
  }
  el("play").onclick = () => {
    viewStatic = false;
    if (clock.staticPage) {
      if (index + 1 < pages.length) go(index + 1).catch(fail);
    } else {
      clock.toggle();
      last = performance.now();
      paint();
      sync();
    }
  };
  el("next").onclick = advance;
  el("previous").onclick = () => {
    viewStatic = false;
    if (clock.previous()) {
      if (index > 0) go(index - 1).catch(fail);
    } else paint();
    sync();
  };
  el("replay").onclick = async () => {
    await go(index);
    viewStatic = false;
    paint();
  };
  el("static").onclick = () => {
    viewStatic = true;
    clock.running = false;
    paint(true);
    sync();
  };
  el("page").onchange = () => go(Number(el("page").value)).catch(fail);
  el("seek").oninput = () => {
    viewStatic = false;
    clock.seek(Number(el("seek").value));
    paint();
    sync();
  };
  el("mode").onchange = () => {
    clock.mode = el("mode").value;
    sync();
  };
  let previewSnapshot;
  const presentationAdapter = async (p, generation) => {
    const init = {
      type: "init",
      wasmBytes: bytes("runtime/wasm/laymesh.wasm"),
      skiaBytes: bytes("runtime/skia/canvaskit.wasm"),
      budget: 256 * 1048576,
    };
    const notify = (data) => {
      if (generation !== p.generation) return;
      if (data.type === "context") {
        if (data.ready && p.opened)
          p.go(p.index, { time: p.clock.time, preserve: true });
        else p.clock.running = false;
        return;
      }
      const waiting = p.calls.get(data.id);
      if (waiting) {
        p.calls.delete(data.id);
        clearTimeout(waiting.timer);
        data.error ? waiting.reject(Error(data.error)) : waiting.resolve(data);
      }
    };
    if (p.canvas.transferControlToOffscreen && typeof Worker !== "undefined")
      try {
        p.worker = makeWorker("presentation-worker.mjs", true);
        p.worker.onmessage = ({ data }) => notify(data);
        p.worker.onerror = (e) => {
          for (const c of p.calls.values()) {
            clearTimeout(c.timer);
            c.reject(Error(e.message));
          }
          p.calls.clear();
          p.fail(Error(e.message));
        };
        const canvas = p.canvas.transferControlToOffscreen();
        await p.call({ ...init, canvas }, [canvas]);
        return;
      } catch (e) {
        p.rendererFallback = e.message;
        await p.stopRenderer();
      }
    if (!p.opened || generation !== p.generation) return;
    const { createPresentationRuntime } = await import(
      moduleURL("runtime/presentation-worker.mjs")
    );
    const run = await createPresentationRuntime(
      { ...init, canvas: p.canvas },
      notify,
    );
    if (!p.opened || generation !== p.generation) {
      await run({ type: "dispose" });
      return;
    }
    p.mainRuntime = run;
  };
  presentationAdapter.stylesheet = URL.createObjectURL(
    new Blob([bytes("runtime/presentation.css")], { type: "text/css" }),
  );
  workerURLs.push(presentationAdapter.stylesheet);
  const presentation = new Presentation({
    base: new URL("./", location.href),
    runtimeAdapter: presentationAdapter,
    provider: async () => {
      const snapshot = pages.slice();
      return {
        pages: snapshot,
        budget: 256 * 1048576,
        prepareMs: 0,
        store: {
          async page(id) {
            const p = snapshot.find((p) => p.id === id);
            return { ...p, resources: resources(p) };
          },
          dispose() {},
        },
      };
    },
    onEnter() {
      previewSnapshot = {
        index,
        viewStatic,
        time: clock.time,
        active: clock.active,
        completed: clock.completed,
        running: clock.running,
      };
      clearTimeout(resizeTimer);
      clock.running = false;
      sync();
      root.inert = true;
    },
    onExit() {
      if (closed) return;
      root.inert = false;
      presentation.savedFocus?.focus?.({ preventScroll: true });
      const snapshot = previewSnapshot;
      viewStatic = snapshot.viewStatic;
      Object.assign(clock, {
        time: snapshot.time,
        active: snapshot.active,
        completed: snapshot.completed,
        running: false,
      });
      go(snapshot.index, true)
        .then(() => {
          if (closed || presentation.opened) return;
          Object.assign(clock, {
            time: Math.min(clock.duration, snapshot.time),
            active: snapshot.active,
            completed: snapshot.completed,
            running: snapshot.running && !dirty && !document.hidden,
          });
          last = performance.now();
          sync();
          paint(viewStatic);
        })
        .catch(fail);
    },
  });
  el("full").onclick = () => {
    presentation.invalidate();
    presentation.open({ fullscreen: true });
  };
  el("windowed").onclick = () => {
    el("preview-more").open = false;
    presentation.invalidate();
    presentation.open({ fullscreen: false });
  };
  const key = (e) => {
    if (
      presentation.opened ||
      e.target.closest(".edit-pane,.workspace-tree,input,select,button,dialog")
    )
      return;
    if (e.code === "ArrowRight") {
      e.preventDefault();
      advance();
    } else if (e.code === "ArrowLeft") el("previous").click();
  };
  document.addEventListener("keydown", key);
  let resizeTimer;
  const viewportObserver = new ResizeObserver(() =>
    requestAnimationFrame(refreshViewport),
  );
  viewportObserver.observe(stage);
  window.addEventListener("resize", refreshViewport);
  reduced.addEventListener("change", refreshViewport);
  document.addEventListener("visibilitychange", () => {
    if (document.hidden) {
      clock.running = false;
      sync();
    }
  });
  let workspace,
    editor,
    compiler,
    compilerInit,
    compileRequest = 0,
    compilePending = new Map(),
    compileChain = Promise.resolve();
  async function compilerCall(data) {
    if (!compiler) {
      compiler = makeWorker("web-compiler-worker.mjs");
      compiler.onmessage = ({ data }) => {
        const p = compilePending.get(data.id);
        if (p) {
          compilePending.delete(data.id);
          data.error ? p.reject(Error(data.error)) : p.resolve(data);
        }
      };
      compiler.onerror = (event) => {
        for (const p of compilePending.values())
          p.reject(Error(event.message || "编辑 Worker 失败"));
        compilePending.clear();
        compiler.terminate();
        compiler = null;
        compilerInit = null;
      };
      compilerInit = compilerCall({
        type: "init",
        wasm: bytes("runtime/compiler/laymesh.wasm"),
        fonts: [
          ...new Map(
            (workspace?.liveFonts || manifest.fonts.filter((f) => f.file)).map(
              (f) => [f.path, f],
            ),
          ).values(),
        ].map((f) => ({
          path: f.path,
          bytes: f.bytes || bytes(f.file),
        })),
      });
    }
    if (data.type !== "init") await compilerInit;
    return new Promise((resolve, reject) => {
      const id = ++compileRequest;
      compilePending.set(id, { resolve, reject });
      compiler.postMessage({ ...data, id });
    });
  }
  async function compile() {
    if (!editor || editor.composing) return;
    const generation = editor.seq;
    dirty = true;
    clock.running = false;
    root.dataset.compileState = "preparing";
    sync();
    status("正在准备修改后的场景…");
    const task = compileChain.then(async () => {
      if (generation !== editor.seq) return;
      await workspace?.assertFonts();
      metrics.compiles++;
      root.dataset.compileCount = metrics.compiles;
      const result = await compilerCall({
        type: "compile",
        entry: manifest.project.entry,
        page: manifest.project.page,
        files: Object.fromEntries(
          Object.entries(editor.files).filter(
            ([p]) => !editor.readonly?.has(p),
          ),
        ),
        assets: {
          ...editor.assets,
          "/@laymesh/font-bindings.json": encoder.encode(
            JSON.stringify(workspace?.compilerBindings() || {}),
          ),
        },
      });
      Object.assign(metrics, {
        compileMs: result.compileMs,
        compilerMemoryBytes: result.wasmMemoryBytes,
      });
      if (generation !== editor.seq) return;
      workspace?.checkFontWarnings(result.warnings);
      if (workspace)
        manifest.fontRequirements = result.fonts.map((r) => {
          const f =
            workspace.liveFonts.find(
              (f) => f.postscriptName === r.postscriptName,
            ) || r;
          return { ...r, path: f.path, source: f.source };
        });
      manifest.requestedFonts =
        result.requestedFonts || manifest.requestedFonts;
      workspace?.refreshFonts();
      manifest.project.dependencies = result.dependencies;
      pages = result.pages.map((p) => ({
        ...p,
        id: p.id + "@" + generation,
        list: outlineList(p.list),
      }));
      pageOptions();
      dirty = false;
      await go(Math.min(index, pages.length - 1));
      if (!root.querySelector(".edit-pane").hidden) {
        viewStatic = true;
        clock.running = false;
        sync();
        await paint(true);
      }
      root.dataset.compileState = "ready";
      metrics.warnings = result.warnings || [];
      if (result.warnings?.length)
        status(result.warnings.map((w) => w.message).join("；"));
    });
    compileChain = task.catch((error) => {
      if (generation !== editor.seq) return;
      if (root.dataset.compileState !== "fonts")
        root.dataset.compileState = "error";
      fail(error);
    });
    await compileChain;
  }
  function resetCompiler() {
    compiler?.terminate();
    for (const p of compilePending.values()) p.reject(Error("字体资源已更新"));
    compilePending.clear();
    compiler = null;
    compilerInit = null;
  }
  async function openEditor() {
    const { createWorkspace } = await import(
      moduleURL("runtime/web-workspace.mjs")
    );
    const schedule = (immediate = false) => {
      editor.seq++;
      schedule.changed();
      clearTimeout(editor.debounce);
      editor.debounce = setTimeout(
        () => compile().catch(fail),
        immediate ? 0 : 250,
      );
    };
    schedule.changed = () => {
      clock.running = false;
      dirty = true;
      root.dataset.compileState = "pending";
      sync();
    };
    workspace = await createWorkspace({
      root,
      manifest,
      files,
      pages,
      compilerCall,
      resetCompiler,
      async compileNow() {
        editor.seq++;
        clearTimeout(editor.debounce);
        await compile();
        if (dirty)
          throw Error(root.dataset.error || "修改后的场景尚未准备成功");
      },
      makeLanguageWorker() {
        const w = makeWorker("web-language-worker.mjs");
        w.postMessage({
          type: "init",
          wasm: bytes("runtime/compiler/laymesh.wasm"),
        });
        return w;
      },
      schedule,
      onStatus: status,
      presentationRoot: () => (presentation.opened ? presentation.root : null),
      previewMenu,
    });
    editor = workspace.editor;
    el("font-guide").hidden = false;
    el("embed-fonts").checked =
      manifest.embedFonts ?? manifest.schemaVersion === 1;
    root.dataset.editorReady = "true";
    status("可以编辑 · 导入资源后可在文件树中查看");
  }
  function previewMenu() {
    if (presentation.opened) {
      presentation.hideMenu();
      const paused = { resume: presentation.clock.running },
        generation = presentation.generation;
      presentation.externalMenu = paused;
      presentation.clock.running = false;
      presentation.sync();
      const action = (name, id) => ({
        name,
        disabled: !presentation.ready || presentation.switching,
        run: () => {
          if (id === "pause") presentation.clock.running = paused.resume;
          paused.resume = false;
          presentation.action(id);
        },
      });
      return {
        items: [
          action(paused.resume ? "暂停" : "播放", "pause"),
          action("上一页／步", "previous"),
          action("下一页／步", "next"),
          action("重播当前页", "replay"),
          action("全屏", "fullscreen"),
          {
            name: "退出放映",
            run: () => {
              paused.resume = false;
              presentation.close();
            },
          },
        ],
        onClose: () => {
          presentation.externalMenu = null;
          if (
            presentation.opened &&
            generation === presentation.generation &&
            paused.resume
          ) {
            presentation.clock.running = true;
            presentation.last = performance.now();
            presentation.sync();
          }
        },
      };
    }
    const command = (name, id) => ({
      name,
      disabled: el(id).disabled,
      run: () => el(id).click(),
    });
    return [
      command(clock.running ? "暂停" : "播放", "play"),
      command("上一页／步", "previous"),
      command("下一页／步", "next"),
      command("重播当前页", "replay"),
      "separator",
      command("适应窗口", "fit"),
      {
        name: "100%",
        run: () => {
          el("zoom").value = "1";
          el("zoom").dispatchEvent(new Event("change"));
        },
      },
      command("显示／隐藏标尺", "rulers"),
      command("窗口放映", "windowed"),
      command("全屏放映", "full"),
    ];
  }
  const playbackMenus = manifest.editable
    ? null
    : createContextMenu({
        root,
        onError: (error) => status(error.message),
        contains: (target) =>
          root.contains(target) || !!presentation.root?.contains(target),
        resolve: (target) => {
          const input = target.closest(
            "input:not([type=range]):not([type=file]),textarea",
          );
          return input ? inputMenu(input) : previewMenu();
        },
      });
  const encodedImages = new Map();
  const imagePixelKeys = new Map();
  function imageMime(data) {
    if (data[0] === 137 && data[1] === 80 && data[2] === 78 && data[3] === 71)
      return "image/png";
    if (data[0] === 255 && data[1] === 216) return "image/jpeg";
    if (
      data[0] === 82 &&
      data[1] === 73 &&
      data[2] === 70 &&
      data[3] === 70 &&
      data[8] === 87 &&
      data[9] === 69 &&
      data[10] === 66 &&
      data[11] === 80
    )
      return "image/webp";
    return null;
  }
  async function imagePixelKey(data, mime) {
    const key = sha256(data);
    if (imagePixelKeys.has(key)) return imagePixelKeys.get(key);
    let bitmap;
    let result = null;
    try {
      bitmap = await createImageBitmap(new Blob([data], { type: mime }));
      const canvas = document.createElement("canvas");
      canvas.width = bitmap.width;
      canvas.height = bitmap.height;
      const context = canvas.getContext("2d", { willReadFrequently: true });
      context.drawImage(bitmap, 0, 0);
      result =
        bitmap.width +
        "x" +
        bitmap.height +
        ":" +
        sha256(context.getImageData(0, 0, bitmap.width, bitmap.height).data);
    } catch {
    } finally {
      bitmap?.close();
    }
    imagePixelKeys.set(key, result);
    return result;
  }
  async function sourceImages() {
    const originals = new Map();
    if (!editor) return originals;
    for (const data of Object.values(editor.assets)) {
      const mime = imageMime(data);
      if (!mime) continue;
      const key = await imagePixelKey(data, mime);
      if (
        key &&
        (!originals.has(key) ||
          data.byteLength < originals.get(key).data.byteLength)
      )
        originals.set(key, { data, mime });
    }
    return originals;
  }
  async function optimizeImage(data, mime) {
    if (
      (manifest.imageCompression || "auto") !== "auto" ||
      !/^image\/(png|jpeg|webp|bmp)$/.test(mime)
    )
      return { data, mime };
    const key = sha256(data);
    if (encodedImages.has(key)) return encodedImages.get(key);
    let bitmap;
    try {
      bitmap = await createImageBitmap(new Blob([data], { type: mime }));
      const canvas = document.createElement("canvas");
      canvas.width = bitmap.width;
      canvas.height = bitmap.height;
      canvas.getContext("2d").drawImage(bitmap, 0, 0);
      const candidate = await new Promise((resolve) =>
        canvas.toBlob(
          resolve,
          "image/webp",
          (manifest.imageQuality ?? 85) / 100,
        ),
      );
      if (
        candidate?.type === "image/webp" &&
        candidate.size < data.byteLength
      ) {
        const result = {
          data: new Uint8Array(await candidate.arrayBuffer()),
          mime: "image/webp",
        };
        encodedImages.set(key, result);
        return result;
      }
    } catch {
    } finally {
      bitmap?.close();
    }
    const result = { data, mime };
    encodedImages.set(key, result);
    return result;
  }
  async function download(editable, zip = false) {
    await workspace?.whenIdle();
    clock.running = false;
    sync();
    status("正在生成下载文件…");
    if (editor) {
      clearTimeout(editor.debounce);
      await compileChain;
      if (dirty) await compile();
      if (dirty) throw Error("源码尚未通过编译，请修正后下载");
    }
    if (editable && el("embed-fonts").checked) await workspace?.assertFonts();
    const originals = await sourceImages();
    const out = {},
      next = {
        ...manifest,
        editable,
        pages: [],
        resources: {},
        fonts: editable
          ? workspace
            ? workspace.liveFonts
                .filter(
                  (f) =>
                    (f.source !== "local" && f.source !== "system") ||
                    (el("embed-fonts").checked &&
                      (manifest.fontRequirements || manifest.fonts).some((r) =>
                        r.postscriptName && f.postscriptName
                          ? r.postscriptName === f.postscriptName
                          : r.family === f.family,
                      )),
                )
                .map(({ bytes: b, ...f }) => {
                  const file = "assets/" + sha256(b);
                  out[file] = b;
                  return { ...f, file };
                })
            : manifest.fonts
          : [],
        fontRequirements: editable
          ? manifest.fontRequirements || manifest.fonts
          : [],
        fontBindings: editable
          ? workspace?.bindings || manifest.fontBindings || {}
          : {},
        resourceInfo: editable
          ? (manifest.resourceInfo || []).filter((r) => r.readonly)
          : [],
        embedFonts: editable && el("embed-fonts").checked,
        project: editable
          ? {
              ...manifest.project,
              dependencies:
                manifest.project.dependencies ||
                Object.keys(manifest.project.files),
              files: {},
            }
          : undefined,
      };
    const runtimeFiles = new Set(
      JSON.parse(decoder.decode(bytes("runtime/runtime-files.json")))[
        editable ? "editable" : "playback"
      ],
    );
    for (const [name, data] of Object.entries(files))
      if (name.startsWith("licenses/") || runtimeFiles.has(name))
        out[name] =
          name === "licenses/manifest.json"
            ? selectWebLicenses(data, editable)
            : data;
    for (let i = 0; i < pages.length; i++) {
      const page = pages[i],
        file = "pages/" + i + ".json",
        clean = {
          id: page.id,
          name: page.name,
          list: structuredClone(page.list),
          inspection: page.inspection,
        };

      next.pages.push({ id: page.id, name: page.name, file });
      for (const [id, original] of Object.entries(resources(page))) {
        const preparedMime = clean.list.resources[id].mime;
        const source = originals.size
          ? originals.get(await imagePixelKey(original, preparedMime))
          : undefined;
        const image = await optimizeImage(
            source?.data || original,
            source?.mime || preparedMime,
          ),
          b = image.data;
        clean.list.resources[id].mime = image.mime;
        const resource = "assets/" + sha256(b);
        out[resource] = b;
        next.resources[id] = resource;
      }
      out[file] = encoder.encode(JSON.stringify(clean));
    }
    if (editable) {
      if (editor) {
        for (const [path, text] of Object.entries(editor.files)) {
          if (editor.readonly?.has(path)) continue;
          const b = encoder.encode(text),
            name = "assets/" + sha256(b);
          out[name] = b;
          next.project.files[path] = name;
        }
        for (const [path, b] of Object.entries(editor.assets)) {
          const name = "assets/" + sha256(b);
          out[name] = b;
          next.project.files[path] = name;
        }
      } else
        for (const [path, name] of Object.entries(manifest.project.files)) {
          out[name] = bytes(name);
          next.project.files[path] = name;
        }
      for (const f of next.fonts)
        if (f.file && !out[f.file]) out[f.file] = bytes(f.file);
      for (const r of next.resourceInfo) out[r.file] = bytes(r.file);
    }
    if (!editable) {
      const path = "runtime/presentation-transitions-data.mjs";
      out[path] = selectPageTransitions(out[path], pages);
    }
    next.container = zip ? "zip" : "universal";
    out["manifest.json"] = encoder.encode(JSON.stringify(next));
    out["index.html"] = files["index.html"];
    out["README.txt"] = files["README.txt"];
    out["entries.json"] = encoder.encode(
      JSON.stringify([...Object.keys(out), "entries.json"]),
    );
    const packed = archive.zip(out),
      blob = zip
        ? new Blob([packed], { type: "application/zip" })
        : archive.universal(
            packed,
            decoder.decode(bytes("runtime/bootstrap.js")),
          ),
      a = document.createElement("a"),
      url = URL.createObjectURL(blob);
    a.href = url;
    a.download =
      (manifest.name || "LayMesh").replace(/[\\/:*?"<>|]/g, "_") +
      (zip ? ".zip" : ".zip.html");
    document.body.append(a);
    a.click();
    a.remove();
    setTimeout(() => URL.revokeObjectURL(url), 60000);
    el("export-panel").close();
    status("已生成下载文件");
  }
  const disposeExport = installExportDialog({
    root,
    manifest,
    pages: () => pages,
    index: () => index,
    resources,
    bytes,
    archive,
    status,
    onError: fail,
    makeWorker: () =>
      makeWorker(
        manifest.editable
          ? "web-full-export-worker.mjs"
          : "web-static-export-worker.mjs",
        false,
        true,
      ),
    async prepare() {
      await workspace?.whenIdle();
      clock.running = false;
      sync();
      if (editor) {
        clearTimeout(editor.debounce);
        await compileChain;
        if (dirty) await compile();
        if (dirty) throw Error("源码尚未通过编译，请修正后导出");
      }
    },
  });
  el("download").onclick = () => download(false).catch(fail);
  el("save").onclick = () => download(true).catch(fail);
  el("zip").onclick = () => download(manifest.editable, true).catch(fail);
  window.__LAYMESH_WEB = {
    metrics,
    get pages() {
      return pages;
    },
    get clock() {
      return clock;
    },
    get viewStatic() {
      return viewStatic;
    },
    presentation,
    view,
    preview,
    get workspace() {
      return workspace;
    },
    get editor() {
      return editor;
    },
  };
  window.addEventListener(
    "pagehide",
    () => {
      closed = true;
      disposeExport();
      clearTimeout(resizeTimer);
      viewportObserver.disconnect();
      preview.dispose();
      clearTimeout(viewPaintTimer);
      for (const url of workerURLs) URL.revokeObjectURL(url);
      cancelAnimationFrame(raf);
      clock.running = false;
      worker?.terminate();
      compiler?.terminate();
      runtime?.({ type: "dispose" });
      for (const p of pending.values()) {
        clearTimeout(p.timer);
        p.reject(Error("Page closed"));
      }
      document.removeEventListener("keydown", key);
      presentation.dispose();
      workspace?.dispose();
      playbackMenus?.dispose();
    },
    { once: true },
  );
  root.addEventListener("laymesh:tab", () =>
    requestAnimationFrame(() => paint(viewStatic)),
  );
  pageOptions();
  const rendererStarted = performance.now();
  await initRenderer();
  metrics.rendererInitMs = performance.now() - rendererStarted;
  await go(0);
  if (manifest.editable) await openEditor();
  raf = requestAnimationFrame(tick);
}
