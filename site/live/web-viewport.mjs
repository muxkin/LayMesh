import { canvasAt, drawPreviewRuler } from "./preview-geometry.mjs";
// Retained foreground pixels move with native scrolling. Only newly exposed
// areas or a changed density need a fresh CanvasKit paint.
export class WebViewport {
  constructor(stage, view, getPage, onChange) {
    this.stage = stage;
    this.view = view;
    this.getPage = getPage;
    this.onChange = onChange;
    this.scroll = document.createElement("div");
    this.scroll.className = "preview-scrollport";
    this.space = document.createElement("div");
    this.space.className = "preview-scrollspace";
    this.figure = document.createElement("div");
    this.figure.className = "preview-figure";
    this.canvas = stage.querySelector("canvas");
    this.overview = document.createElement("canvas");
    this.overview.className = "preview-overview";
    this.overview.hidden = true;
    this.figure.append(this.overview, this.canvas);
    this.space.append(this.figure);
    this.scroll.append(this.space);
    stage.prepend(this.scroll);
    stage.querySelector(".page-outline")?.remove();
    this.scroll.onscroll = () => {
      this.readScroll();
      this.schedule();
    };
    this.observer = new ResizeObserver(() => this.schedule());
    this.observer.observe(this.scroll);
  }
  readScroll() {
    if (!this.page) return;
    const r = this.rect();
    this.view.pan = [
      r.left - (this.scroll.clientWidth - r.width) / 2,
      r.top - (this.scroll.clientHeight - r.height) / 2,
    ];
  }
  panTo(pan) {
    if (!this.page) return;
    this.scroll.scrollLeft =
      this.origin[0] -
      (this.scroll.clientWidth - this.page.width * this.view.pixelsPerMM) / 2 -
      pan[0];
    this.scroll.scrollTop =
      this.origin[1] -
      (this.scroll.clientHeight - this.page.height * this.view.pixelsPerMM) /
        2 -
      pan[1];
    this.readScroll();
    this.schedule();
  }
  panBy(x, y) {
    this.panTo([this.view.pan[0] + x, this.view.pan[1] + y]);
  }
  layout() {
    const p = this.getPage();
    if (!p) return;
    const v = this.view,
      s = this.scroll;
    this.page = {
      ...p.inspection?.page,
      width: p.list.width,
      height: p.list.height,
      layout_dpi: p.list.layout_dpi || 96,
    };
    this.stage.classList.toggle("no-rulers", !v.rulers);
    let width = s.clientWidth,
      height = s.clientHeight;
    const scale =
      v.mode === "fit"
        ? Math.max(
            0.01,
            Math.min(
              (width - 48) / this.page.width,
              (height - 48) / this.page.height,
            ),
          )
        : (v.scale * this.page.layout_dpi) / 25.4;
    v.pixelsPerMM = scale;
    const w = this.page.width * scale,
      h = this.page.height * scale,
      extentW = Math.max(width, w + 48),
      extentH = Math.max(height, h + 48);
    Object.assign(this.space.style, {
      width: extentW + "px",
      height: extentH + "px",
    });
    this.origin = [(extentW - w) / 2, (extentH - h) / 2];
    Object.assign(this.figure.style, {
      left: this.origin[0] + "px",
      top: this.origin[1] + "px",
      width: w + "px",
      height: h + "px",
    });
    s.scrollLeft = this.origin[0] - (width - w) / 2 - v.pan[0];
    s.scrollTop = this.origin[1] - (height - h) / 2 - v.pan[1];
    this.readScroll();
    this.position();
    this.rulers();
  }
  rect() {
    return {
      left: this.origin[0] - this.scroll.scrollLeft,
      top: this.origin[1] - this.scroll.scrollTop,
      width: this.page.width * this.view.pixelsPerMM,
      height: this.page.height * this.view.pixelsPerMM,
    };
  }
  position() {
    if (!this.presented || !this.page) return;
    const p = this.presented,
      scale = this.view.pixelsPerMM;
    Object.assign(this.canvas.style, {
      left: p.viewport[0] * scale + "px",
      top: p.viewport[1] * scale + "px",
      width: (p.viewport[2] - p.viewport[0]) * scale + "px",
      height: (p.viewport[3] - p.viewport[1]) * scale + "px",
    });
    // One coherent placeholder, never a low-resolution patchwork around a
    // sharp rectangle. A completed viewport replaces it in the same commit.
    const visible = this.request(0)?.viewport;
    const covers = visible?.every((v, i) => i < 2 ? v >= p.viewport[i] - 1e-9 : v <= p.viewport[i] + 1e-9);
    const overview = !covers && p.time === null && this.overviewID === p.id && this.overviewVersion === p.contentVersion;
    this.overview.hidden = !overview;
    this.canvas.style.visibility = overview ? "hidden" : "";
    if (overview) Object.assign(this.overview.style, {
      width: this.page.width * scale + "px", height: this.page.height * scale + "px", clipPath: "none",
    });
  }
  request(padding = 96) {
    const r = this.rect(),
      s = this.view.pixelsPerMM,
      pad = padding / s,
      w = this.scroll.clientWidth,
      h = this.scroll.clientHeight,
      p = this.page;
    const area = [
      Math.max(0, -r.left / s - pad),
      Math.max(0, -r.top / s - pad),
      Math.min(p.width, (w - r.left) / s + pad),
      Math.min(p.height, (h - r.top) / s + pad),
    ];
    if (area[2] <= area[0] || area[3] <= area[1]) return null;
    const dpr = devicePixelRatio || 1;
    const key = JSON.stringify([this.getPage()?.id, p.width, p.height, r.left, r.top, s, dpr, w, h]);
    if (key !== this.revisionKey) { this.revisionKey = key; this.revision = (this.revision || 0) + 1; }
    return {
      viewRevision: this.revision,
      viewport: area,
      width: Math.max(1, Math.ceil((area[2] - area[0]) * s * dpr)),
      height: Math.max(1, Math.ceil((area[3] - area[1]) * s * dpr)),
      scale: s,
      dpr,
    };
  }
  matches(request) {
    const current = this.request();
    return !!current && request?.viewRevision === current.viewRevision &&
      request.width === current.width && request.height === current.height && request.dpr === current.dpr &&
      request.scale === current.scale && request.viewport.every((v, i) => Math.abs(v - current.viewport[i]) < 1e-9);
  }
  covered(time, id) {
    const r = this.request(0),
      p = this.presented;
    return (
      !!r &&
      !!p &&
      p.time === time &&
      p.id === id &&
      p.scale === r.scale &&
      p.dpr === r.dpr &&
      r.viewport.every((x, i) =>
        i < 2 ? x >= p.viewport[i] - 1e-9 : x <= p.viewport[i] + 1e-9,
      )
    );
  }
  present(request, time, id) {
    this.presented = { ...request, time, id };
    const a = request.viewport,
      p = this.page;
    if (
      time === null &&
      a[0] === 0 &&
      a[1] === 0 &&
      a[2] === p.width &&
      a[3] === p.height
    ) {
      const density = Math.min(
        768 / p.width,
        2048 / p.height,
        Math.sqrt(500000 / (p.width * p.height)),
      );
      this.overview.width = Math.max(1, Math.floor(p.width * density));
      this.overview.height = Math.max(1, Math.floor(p.height * density));
      this.overview
        .getContext("2d")
        .drawImage(
          this.canvas,
          0,
          0,
          this.overview.width,
          this.overview.height,
        );
      this.overviewID = id;
      this.overviewVersion = request.contentVersion;
    }
    this.position();
  }
  reset() {
    this.presented = null;
    this.overview.hidden = true;
    this.canvas.style.visibility = "";
  }
  coordinates(clientX, clientY) {
    const box = this.scroll.getBoundingClientRect();
    return canvasAt(
      this.page,
      this.rect(),
      clientX - box.left,
      clientY - box.top,
    );
  }
  rulers() {
    const r = this.rect(),
      p = this.page;
    if (!this.view.rulers) return;
    for (const axis of ["x", "y"])
      drawPreviewRuler(
        this.stage.querySelector(
          ".ruler." + (axis === "x" ? "horizontal" : "vertical"),
        ),
        axis,
        {
          length:
            axis === "x" ? this.scroll.clientWidth : this.scroll.clientHeight,
          start: axis === "x" ? r.left : r.top,
          scale: this.view.pixelsPerMM,
          page: p,
          ink: "#738399",
          font: "10px system-ui",
        },
      );
    this.stage.querySelector(".ruler-corner").textContent = p.unit || "mm";
  }
  schedule() {
    if (this.frame) return;
    this.frame = requestAnimationFrame(() => {
      this.frame = 0;
      this.onChange();
    });
  }
  dispose() {
    this.observer.disconnect();
    cancelAnimationFrame(this.frame);
    this.scroll.onscroll = null;
  }
}
