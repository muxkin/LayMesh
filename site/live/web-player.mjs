import {WebCompilerPool} from './web-compiler-pool.mjs';
import {createWebPresentationSession} from './web-presentation-session.mjs';
import {PresentationWorkers} from './presentation-workers.mjs';
import {PresenterView,startPresentation} from "./presenter.mjs";
import {PagePreviewRenderer,PageThumbnails} from "./page-previews.mjs";
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
import { PreviewCompiler } from "./web-preview-compiler.mjs";
import {makePreviewState,restorePreviewState} from "./web-preview-state.mjs";
import { applyDisplayUpdate } from "./display-update.mjs";
import { waitForPresentation } from "./playback-stats.mjs";
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
  let thumbnails,sidebarResize,previewCompiler;
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
      if (!busy && !painting) root.dataset.painted = "true";
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
            ...(exporter ? ["fflate.mjs", "web-archive.mjs", "web-export-raster.mjs"] : []),
            "display-update.mjs",
            "skia/canvaskit.mjs",
            "presentation-transitions-data.mjs",
            "presentation-transitions.mjs",
            "skia-renderer.mjs",
            "presentation-resources.mjs",
            "presentation-renderer.mjs",
            ...(exporter && !manifest.editable
              ? [
                  "web-pdf-resources.mjs",
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
      preserveDrawingBuffer: true,
    };
    if (typeof OffscreenCanvas !== "undefined" && typeof Worker !== "undefined") {
      worker = makeWorker("presentation-worker.mjs", true);
      let frameSequence = 0;
      worker.onmessage = ({ data }) => {
        if (data.type === "frame") {
          try {
            if (!closed && data.serial === serial && data.sequence > frameSequence) {
              frameSequence = data.sequence;
              const bitmap = data.bitmap;
              if (canvas.width !== bitmap.width) canvas.width = bitmap.width;
              if (canvas.height !== bitmap.height) canvas.height = bitmap.height;
              const context = canvas.getContext("2d");
              context.clearRect(0, 0, canvas.width, canvas.height);
              context.drawImage(bitmap, 0, 0);
            }
          } finally { data.bitmap.close(); }
          return;
        }
        if (data.type === "context") {
          clock.running = false;
          if (data.ready) {
            if (previewCompiler && !previewCompiler.current?.valid) previewCompiler.schedule(0);
            else go(index, true).catch(fail);
          }
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
        const offscreen = new OffscreenCanvas(1, 1);
        await call({ ...init, canvas: offscreen, bitmapFrames: true }, [offscreen]);
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
      if (data.ready) {
        if (previewCompiler && !previewCompiler.current?.valid) previewCompiler.schedule(0);
        else go(index, true).catch(fail);
      }
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
    thumbnails?.select(index);
    if (closed || !el("play")) return;
    iconLabel(el("play"), clock.running ? "pause" : "play");
    el("play").setAttribute("aria-label", clock.running ? "暂停" : "播放");
    el("preview-state").textContent = viewStatic
      ? "完整排版"
      : clock.running
        ? "正在播放"
        : "动画预览";
    const currentDirty = previewCompiler ? !previewCompiler.currentReady : dirty;
    el("full").disabled = busy;
    el("windowed").disabled = busy;
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
      el(id).disabled = busy || currentDirty;
    // Page selection stays available while the compiler is busy. Selecting a
    // pending page changes the next job instead of queueing another document.
    el("page").disabled = false;
  }
  async function go(n, preserve = false, compiledVersion) {
    if (closed || (compiledVersion!=null && compiledVersion!==previewCompiler?.version)) return;
    n = Math.max(0, Math.min(n, pages.length - 1));
    if (compiledVersion==null && previewCompiler && (!previewCompiler.records[n]?.valid || previewCompiler.pending)) {
      // Cancel an earlier page activation before waiting for the new target.
      // Otherwise its late renderer reply can restore the previous index.
      serial++;
      if (worker) worker.postMessage({ type: "target", serial });
      else if (runtime) call({ type: "target", serial }).catch(fail);
      busy = false;
      requestedPaint = null;
      index = n;
      const key = previewCompiler.records[n]?.key;
      sync();
      await previewCompiler.ensure(key);
      if (closed || previewCompiler.current?.key !== key) return;
      n = Math.min(index, pages.length - 1);
    }
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
      const partial=p.listUpdate&&previewResidents.has(p.listUpdate.base);
      const request={type:"load",installOnly:true,page:partial?{id:p.id,listUpdate:p.listUpdate,resources:resources(p)}:{...p,listUpdate:undefined,resources:resources(p)},priority:true,...size()};
      let installed;
      try{installed=await call(request);}catch(error){if(!partial)throw error;installed=await call({...request,page:{...p,listUpdate:undefined,resources:resources(p)}});}
      previewResidents=new Set(installed.resident||[]);
      await call({type:"retain",pages:pages.map(p=>p.id)});
      if(installed.patched)metrics.pagePatches=(metrics.pagePatches||0)+1;
      if (installed.warnings?.length) status(installed.warnings.map(w=>w.message).join("\n"));
      if (current !== serial) return;
      index = n;
      clock.install(p.inspection?.animation, reduced.matches);
      if (preserve) clock.seek(time);
      let drawSize, frame, attempt = 0;
      do {
      drawSize = size();
      frame = await call({
        type: "activate",
        visible: document.visibilityState === "visible",
        transition: attempt++ ? null : transition,
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
      frame.presentationConfirmed = frame.presentationConfirmed && await waitForPresentation();
      if (current !== serial) return;
      // The overview also reads the placeholder canvas. Copy after the owning
      // window has received its bitmap, rather than caching a transient blank.
      preview.present(drawSize, viewStatic || clock.staticPage ? null : clock.time, p.id);
      } while (preview.request(0) && !preview.covered(viewStatic || clock.staticPage ? null : clock.time, p.id));
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
        presentationConfirmed: frame.presentationConfirmed,
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
        clock.running = !viewStatic && (!!resume || (!preserve && clock.running));
        last = performance.now();
        if (previewCompiler) compileState(previewCompiler);
        else sync();
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
            if (!clock.running) {
              frame.presentationConfirmed = frame.presentationConfirmed && await waitForPresentation();
              if (current !== serial) continue;
              preview.present(drawSize, drawTime, pages[index].id);
            }
            root.dataset.painted = String(drawTime !== null || preview.covered(drawTime, pages[index].id));
            metrics.frames++;
            Object.assign(metrics, {
              sampleMs: frame.sampleMs,
              rasterMs: frame.rasterMs,
              width: drawSize.width,
              height: drawSize.height,
              presentationConfirmed: frame.presentationConfirmed,
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
    if (clock.running && !busy && !(previewCompiler ? !previewCompiler.currentReady : dirty)) {
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
    if (!clock.staticPage && !clock.running) clock.toggle();
    paint();
    sync();
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
    };
    const notify = (data) => {
      if (generation !== p.generation) return;
      if (data.type === "navigation-visible") {p.navigationVisible(data);return;}
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
        p.worker = new PresentationWorkers(p, () => makeWorker("presentation-worker.mjs", true));
        p.worker.onmessage = ({ data }) => notify(data);
        p.worker.onerror = (e) => {
          for (const c of p.calls.values()) {
            clearTimeout(c.timer);
            c.reject(Error(e.message));
          }
          p.calls.clear();
          p.fail(Error(e.message));
        };
        await p.call(init);
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
    getStartPageId: () => pages[index]?.sourceId || pages[index]?.id,
    provider: async ({startPageId,signal,onProgress} = {}) => {
      const load=async(p,id,{packed=false}={})=>{
        if(packed&&p.file)return {id,name:p.name,packedPage:bytes(p.file).slice(),resources:resources(p)};
        return {...p,id,listUpdate:undefined,resources:resources(p)};
      };
      if(previewCompiler)return createWebPresentationSession({compiler:previewCompiler,startPageId,signal,load,
        onProgress(progress){onProgress?.(progress);if(presentation.ready){presentation.preloadAgain=true;presentation.schedulePreload();}}});
      const snapshot=pages.slice();
      return {pages:snapshot,prepareMs:0,store:{page:(id,options)=>load(snapshot.find(p=>p.id===id),id,options),dispose(){}}};
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
  const rendererRuntime=async()=>({wasmBytes:bytes("runtime/wasm/laymesh.wasm").slice().buffer,skiaBytes:bytes("runtime/skia/canvaskit.wasm").slice().buffer});
  const presenter=new PresenterView(presentation,{runtime:rendererRuntime,title:()=>manifest.name||document.title,css:async()=>decoder.decode(bytes("runtime/presentation.css"))});
  el("presenter").onclick=()=>{el("preview-more").open=false;presentation.invalidate();startPresentation(presentation,{mode:"presenter"});};
  el("from-start").onclick=()=>{el("preview-more").open=false;presentation.open({fullscreen:false,startPageId:null});};
  el("from-start-full").onclick=()=>{el("preview-more").open=false;presentation.open({fullscreen:true,startPageId:null});};
  if(!manifest.editable){
    const main=root.querySelector('main'),pane=document.createElement('aside'),splitter=document.createElement('div');pane.className='playback-pages';pane.innerHTML='<h2>页面预览</h2>';splitter.className='playback-page-splitter';splitter.tabIndex=0;splitter.setAttribute('role','separator');splitter.setAttribute('aria-label','调整页面预览宽度');splitter.setAttribute('aria-orientation','vertical');main.prepend(pane,splitter);
    let dragging=false,width=220;const setWidth=value=>{const limit=Math.min(480,Math.max(160,main.clientWidth-326));width=Math.max(160,Math.min(limit,value));splitter.setAttribute('aria-valuemax',limit);pane.style.flexBasis=width+'px';pane.style.width=width+'px';splitter.setAttribute('aria-valuenow',width);};splitter.setAttribute('aria-valuemin','160');splitter.setAttribute('aria-valuemax','480');setWidth(width);sidebarResize=new ResizeObserver(()=>setWidth(width));sidebarResize.observe(main);
    splitter.onpointerdown=e=>{dragging=true;splitter.setPointerCapture(e.pointerId);};splitter.onpointermove=e=>{if(dragging)setWidth(e.clientX-main.getBoundingClientRect().left);};splitter.onpointerup=splitter.onpointercancel=()=>{dragging=false;thumbnails?.pump();};splitter.onkeydown=e=>{if(['ArrowLeft','ArrowRight'].includes(e.key)){e.preventDefault();setWidth(width+(e.key==='ArrowRight'?20:-20));}};
    const toggle=show=>{pane.hidden=splitter.hidden=!show;el('page-sidebar').setAttribute('aria-pressed',String(show));};toggle(!matchMedia('(max-width:700px)').matches);el('page-sidebar').onclick=()=>toggle(pane.hidden);
    thumbnails=new PageThumbnails({parent:pane,pages,renderer:new PagePreviewRenderer({runtime:rendererRuntime,page:async i=>({...pages[i],listUpdate:undefined,resources:resources(pages[i])})}),onSelect:i=>go(i).catch(fail),paused:()=>clock.running||presentation.opened||presenter.opened||dragging});
  }else el('page-sidebar').hidden=true;
  el("full").onclick = () => {
    presentation.invalidate();
    startPresentation(presentation);
  };
  el("slides-only").onclick=()=>{el("preview-more").open=false;presentation.invalidate();startPresentation(presentation,{mode:"slides"});};
  el("windowed").onclick = () => {
    el("preview-more").open = false;
    presentation.invalidate();
    presentation.open({ fullscreen: false });
  };
  const key = (e) => {
    if (
      presentation.opened || presenter.opened ||
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
  let workspace, editor, compilerPool, compilerModule;
  let previewResidents=new Set();
  const previewSession = Math.random().toString(36).slice(2);
  const compiledPages = new Map();
  const fontHashes = new WeakMap();
  function contentHash(data){if(!fontHashes.has(data))fontHashes.set(data,sha256(data));return fontHashes.get(data);}
  function fontVersion() {
    return JSON.stringify(workspace.liveFonts.map(f => {
      const data = f.bytes || (f.file && bytes(f.file));
      if (data && !fontHashes.has(data)) fontHashes.set(data, sha256(data));
      return [f.path, data ? fontHashes.get(data) : null];
    }));
  }
  function getCompilerPool() {
    return compilerPool ||= new WebCompilerPool({
      create:()=>makeWorker('web-compiler-worker.mjs'),
      initialize:async()=>({wasm:await (compilerModule ||= WebAssembly.compile(bytes('runtime/compiler/laymesh.wasm'))),
        fonts:[...new Map((workspace?.liveFonts || manifest.fonts.filter(f=>f.file)).map(f=>[f.path,f])).values()]
          .map(f=>({path:f.path,bytes:f.bytes || bytes(f.file)}))}),
      knownPages:()=>Object.fromEntries([...compiledPages].map(([key,p])=>[key,p.listId])),
      accept(data) {
        if(data.version!=null && data.version!==previewCompiler?.version)return;
        for(const page of data.pages || [])if(page.listUpdate){
          const old=compiledPages.get(page.id);
          if(old?.listId!==page.listUpdate.base)throw Error('Preview drawing baseline mismatch');
          page.list=applyDisplayUpdate(old.list,page.listUpdate.update);
          if(page.resourceDelta)page.resources=Object.fromEntries(Object.entries({...old.resources,...page.resources}).filter(([id])=>page.list.resources[id]));
        }
      },
    });
  }
  function compilerCall(data,onPlan,onProgress) { return getCompilerPool().call(data,onPlan,onProgress); }
  function compileSnapshot() {
    const assets = {
      ...editor.assets,
      "/@laymesh/font-bindings.json": encoder.encode(
        JSON.stringify(workspace?.compilerBindings() || {}),
      ),
    };
    return {
      entry: manifest.project.entry,
      page: manifest.project.page,
      fontVersion: fontVersion(),
      files: Object.fromEntries(Object.entries(editor.files).filter(([p]) => !editor.readonly?.has(p))),
      assets,
      assetKeys: Object.fromEntries(Object.entries(assets).map(([p, b]) => [p, contentHash(b)])),
    };
  }
  async function compile() {
    if (!previewCompiler) return;
    await previewCompiler.ensure();
    if (dirty) throw Error(root.dataset.error || "修改后的场景尚未准备成功");
  }
  function compileState(c) {
    if (closed) return;
    const dependencies = c.records.flatMap(r => r.dependencies || []);
    // Until every page has been examined, workspace transactions must keep the
    // unexamined source/resource paths in their invalidation set too.
    if (c.records.some(r => !r.dependencies))
      dependencies.push(...Object.keys(editor.files), ...Object.keys(editor.assets));
    manifest.project.dependencies = [...new Set(dependencies)];
    dirty = !c.complete;
    root.dataset.documentState = c.complete ? "ready" : c.failure ? "error" : "pending";
    if (!(root.dataset.compileState === "fonts" && c.failure))
      root.dataset.compileState = c.currentReady ? "ready" :
        c.current?.error ? "error" : c.pending ? "pending" : "preparing";
    if (c.currentReady && !c.failure) {
      delete root.dataset.error;
      const warnings = c.complete ? metrics.warnings : c.current?.warnings;
      status(warnings?.length ? warnings.map(w => w.message).join("；") :
        c.complete ? "可以播放" : "当前页已更新 · 正在更新其他页面…");
    }
    sync();
  }
  async function updateCompiledPages(result, records, version) {
    if (typeof result.name === "string") {
      manifest.name = result.name;
      document.title = result.name || "LayMesh";
      root.querySelector(".project-name").textContent = result.name || "LayMesh";
    }
    const selected = pages[index]?.sourceId || pages[index]?.id;
    const fallback = pages[index];
    for (const p of result.pages) {
      const r = records.find(r => r.key === p.id);
      r.value = { ...p, sourceId: p.id, id: p.listId || p.id + "@" + previewSession + ":" + version, list: p.list };
    }
    pages = records.map(r => r.value ? {...r.value,name:r.name} : {
      ...fallback, sourceId: r.key, id: "pending:" + r.key, name: r.name,
    });
    const selectedIndex = records.findIndex(r => r.key === selected);
    index = selectedIndex < 0 ? Math.min(index, pages.length - 1) : selectedIndex;
    pageOptions();
    const known = records.filter(r => r.dependencies);
    if (records.every(r => r.valid)) {
      const faces = new Map(known.flatMap(r => (r.fonts || []).map(f => [f.postscriptName, f])));
      manifest.fontRequirements = [...faces.values()].map(r => {
        const f = workspace.liveFonts.find(f => f.postscriptName === r.postscriptName) || r;
        return { ...r, path: f.path, source: f.source };
      });
      manifest.requestedFonts = [...new Map(known.flatMap(r =>
        (r.requestedFonts || []).map(f => [JSON.stringify(f), f]))).values()];
      workspace.refreshFonts();
    }
    metrics.warnings = records.flatMap(r => r.warnings || []);
    if (result.pages.some(p => p.id === records[index]?.key)) {
      viewStatic = true;
      clock.running = false;
      // Paint the validated transaction before committing its page records.
      // Waiting on ensure() here would wait for this very transaction.
      await go(index, true, version);
      if (version === previewCompiler.version) {
        metrics.editToPreviewMs = performance.now() - (editor.changedAt || performance.now());
        metrics.previewCompileMs = result.compileMs;
        metrics.previewVersion = version;
        previewCompiler.event('current-visible',{page:records[index]?.key});
      }
    } else if (result.unchanged?.includes(records[index]?.key) && metrics.previewVersion !== version) {
      metrics.editToPreviewMs = performance.now() - (editor.changedAt || performance.now());
      metrics.previewCompileMs = result.compileMs;
      metrics.previewVersion = version;
    }
  }
  function resetCompiler() {
    // A workspace transaction also restarts the worker. Its unchanged pages
    // remain usable; actual font bytes/bindings are compared in the snapshot.
    presentation.invalidate();
    previewCompiler?.schedule(0);
    compilerPool?.dispose();
    compilerPool=null;
  }
  async function openEditor() {
    const { createWorkspace } = await import(
      moduleURL("runtime/web-workspace.mjs")
    );
    const schedule = (immediate = false) => {
      editor.seq++;
      schedule.changed(immediate ? 0 : 30);
    };
    schedule.changed = (delay = 30) => {
      clock.running = false;
      dirty = true;
      editor.changedAt = performance.now();
      root.dataset.compileState = "pending";
      presentation.invalidate();
      previewCompiler?.schedule(delay);
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
        previewCompiler.schedule(0);
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
    previewCompiler = new PreviewCompiler({
      cancel:version=>compilerPool?.cancel(version),
      concurrency:()=>navigator.hardwareConcurrency || 1,
      canRun:key=>!compilerPool || compilerPool.canRun(key,[...previewCompiler.jobs.keys()]),
      pages,
      initial: restorePreviewState(manifest,files),
      snapshot: compileSnapshot,
      revision: () => editor.seq,
      active: () => pages[index]?.sourceId || pages[index]?.id,
      paused: () => editor.composing,
      async compile(snapshot,onPlan,onProgress) {
        await workspace.assertFonts();
        metrics.compiles++;
        root.dataset.compileCount = metrics.compiles;
        const { fontVersion, ...request } = snapshot;
        const result = await compilerCall({ ...request, type: "compile" },onPlan,onProgress);
        Object.assign(metrics, { compileMs: result.compileMs, compilerMemoryBytes: compilerPool.memoryBytes });
        return result;
      },
      update: updateCompiledPages,
      commit(result) {
        compilerPool?.retain(result.directory.map(p=>p.id));
        for (const page of result.pages) compiledPages.set(page.id, page);
        for (const key of compiledPages.keys())
          if (!result.directory.some(p => p.id === key)) compiledPages.delete(key);
      },
      validate: result => workspace.checkFontWarnings(result.warnings),
      state: compileState,
      error: (error,key)=>{if(key===previewCompiler.current?.key)fail(error);},
    });
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
      await compile();
      if (dirty) throw Error("源码尚未通过编译，请修正后下载");
    }
    const exportRevision = editor?.seq;
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
          id: page.sourceId || page.id,
          name: page.name,
          list: structuredClone(page.list),
          inspection: page.inspection,
        };

      next.pages.push({ id: clean.id, name: page.name, file });
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
    if(editable&&previewCompiler){const state=makePreviewState(next,out,previewCompiler.records);if(state)out["preview-state.json"]=encoder.encode(JSON.stringify(state));}
    out["manifest.json"] = encoder.encode(JSON.stringify(next));
    out["index.html"] = files["index.html"];
    out["README.txt"] = files["README.txt"];
    out["entries.json"] = encoder.encode(
      JSON.stringify([...Object.keys(out), "entries.json"]),
    );
    if (editor && editor.seq !== exportRevision)
      throw Error("下载准备期间源码已改变，请重试");
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
        await compile();
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
    presenter,
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
    get previewCompiler() { return previewCompiler; },
    diagnostics() { return {schema:1,userAgent:navigator.userAgent,hardwareConcurrency:navigator.hardwareConcurrency,
      timeOrigin:performance.timeOrigin,totals:previewCompiler?.totals,firstEdit:previewCompiler?.firstEdit,currentEdit:previewCompiler?.trace,
      pages:previewCompiler?.records.map(r=>({id:r.key,ready:r.valid,error:r.error?.message,baseline:r.value?.listId || r.value?.id})),
      compilerWorkers:compilerPool?.clients.size || 0,compilerMemoryBytes:compilerPool?.memoryBytes || 0,
      preview:{editToPreviewMs:metrics.editToPreviewMs,version:metrics.previewVersion},presentation: {...presentation.metrics}}; },
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
      compilerPool?.dispose();
      previewCompiler?.dispose();
      runtime?.({ type: "dispose" });
      for (const p of pending.values()) {
        clearTimeout(p.timer);
        p.reject(Error("Page closed"));
      }
      document.removeEventListener("keydown", key);
      presenter.close();
      thumbnails?.dispose();
      sidebarResize?.disconnect();
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
