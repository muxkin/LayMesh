import { exportBrowserJPEG } from "./web-jpeg-export.mjs";
import { createPDFExport } from "./web-pdf-export.mjs";
const formats = [
  ["pdf", "PDF 文档"],
  ["svg", "SVG 矢量图"],
  ["pptx", "PowerPoint（PPTX）"],
  ["png", "PNG"],
  ["jpg", "JPEG"],
  ["tif", "TIFF"],
  ["webp", "WebP"],
  ["bmp", "BMP"],
  ["gif", "GIF 静态图片"],
  ["ico", "ICO"],
  ["pam", "PAM / PNM"],
  ["ppm", "PPM"],
  ["pgm", "PGM"],
  ["pbm", "PBM"],
  ["tga", "TGA"],
];
const mime = {
  pdf: "application/pdf",
  svg: "image/svg+xml",
  pptx: "application/vnd.openxmlformats-officedocument.presentationml.presentation",
  png: "image/png",
  jpg: "image/jpeg",
  tif: "image/tiff",
  webp: "image/webp",
  bmp: "image/bmp",
  gif: "image/gif",
  ico: "image/x-icon",
  zip: "application/zip",
};
const clean = (s) =>
  String(s || "LayMesh")
    .replace(/[\\/:*?"<>|]/g, "_")
    .replace(
      /\.(pdf|svg|pptx|png|jpe?g|tiff?|webp|bmp|gif|ico|p[abgp]m|pnm|tga|zip\.html)$/i,
      "",
    );
export function installExportDialog({
  root,
  manifest,
  prepare,
  pages,
  index,
  resources,
  makeWorker,
  bytes,
  archive,
  status,
  onError,
}) {
  const el = (id) => root.querySelector("#" + id),
    panel = el("export-panel");
  const section = document.createElement("div");
  section.className = "file-export";
  section.innerHTML = `<div class="export-field"><label for="export-format">格式</label><select id="export-format">${formats
    .filter(
      ([f]) =>
        manifest.editable || ["pdf", "svg", "png", "jpg", "tif"].includes(f),
    )
    .map(([value, name]) => `<option value="${value}">${name}</option>`)
    .join(
      "",
    )}</select><button id="export-settings" class="export-settings-link">设置</button></div><div class="export-field"><label for="export-scope">页面</label><select id="export-scope"><option value="all">全部页面</option><option value="current">当前页</option></select></div><div class="export-field"><label for="export-name">文件名</label><input id="export-name" type="text"><span id="export-extension"></span></div><p id="export-format-note" class="export-note"></p><div class="export-actions"><button id="export-file" class="primary">导出文件</button><button id="export-cancel" hidden>取消</button><span id="export-progress" role="status" aria-live="polite"></span></div>`;
  panel.querySelector(".dialog-heading").after(section);
  const settings = document.createElement("dialog");
  settings.id = "export-settings-panel";
  settings.className = "export-settings-panel";
  settings.innerHTML = `<form><div class="dialog-heading"><h2>导出设置</h2><button type="button" class="dialog-close" aria-label="关闭导出设置">×</button></div><label>图片分辨率（DPI）<input id="export-dpi" type="number" min="1" max="25400" required></label><label>PNG 压缩<select id="export-png-compression"><option value="fast">快速</option><option value="default">标准</option><option value="best">最小体积</option></select></label><label>TIFF 压缩<select id="export-tiff-compression"><option value="lzw">LZW</option><option value="deflate">Deflate</option><option value="packbits">PackBits</option><option value="none">不压缩</option></select></label><label>JPEG 质量<input id="export-jpeg-quality" type="number" min="1" max="100" required></label><label>不透明图片背景<input id="export-background" type="text" required placeholder="#ffffff"></label><p class="export-note">设置随作品保存。WebP 使用无损编码；SVG、PDF 保留矢量内容。PPTX 的复杂特效按此分辨率生成局部图片。</p><div class="export-actions"><button type="button" id="export-defaults">恢复默认</button><button type="submit" class="primary">保存设置</button></div></form>`;
  root.append(settings);
  if (!manifest.editable) {
    el("export-png-compression").closest("label").hidden = true;
    settings.querySelector(".export-note").textContent =
      "设置随作品保存。PDF、SVG 保留矢量内容；PDF 的复杂蒙版和透明混合生成局部图片。";
  }
  const defaults = {
    dpi: 1200,
    png: "default",
    tiff: "lzw",
    jpeg: 90,
    background: "#ffffff",
  };
  const showSettings = (value) => {
    for (const [key, id] of Object.entries({
      dpi: "dpi",
      png: "png-compression",
      tiff: "tiff-compression",
      jpeg: "jpeg-quality",
      background: "background",
    }))
      el("export-" + id).value = value[key];
  };
  el("export-settings").onclick = () => {
    showSettings({ ...defaults, ...manifest.exportSettings });
    settings.showModal();
  };
  settings.querySelector(".dialog-close").onclick = () => settings.close();
  el("export-defaults").onclick = () => showSettings(defaults);
  settings.querySelector("form").onsubmit = (e) => {
    e.preventDefault();
    const background = el("export-background").value.trim();
    if (!/^#[\da-f]{6}$/i.test(background)) {
      el("export-background").setCustomValidity(
        "请输入六位十六进制颜色，例如 #ffffff",
      );
      el("export-background").reportValidity();
      return;
    }
    manifest.exportSettings = {
      dpi: Number(el("export-dpi").value),
      png: el("export-png-compression").value,
      tiff: el("export-tiff-compression").value,
      jpeg: Number(el("export-jpeg-quality").value),
      background,
    };
    settings.close();
  };
  el("export-background").oninput = () =>
    el("export-background").setCustomValidity("");
  const options = (format) => {
    const s = { ...defaults, ...manifest.exportSettings };
    return {
      ...(format === "svg" && manifest.editable ? {} : { dpi: s.dpi }),
      ...(format === "png" ? { compression: s.png } : {}),
      ...(format === "tif" ? { compression: s.tiff } : {}),
      ...(format === "jpg" ? { quality: s.jpeg } : {}),
      ...(["jpg", "gif", "ppm", "pgm", "pbm"].includes(format)
        ? { background: s.background }
        : {}),
    };
  };
  const extension = () =>
    el("export-scope").value === "all" && el("export-format").value !== "pdf"
      ? "zip"
      : el("export-format").value;
  const update = () => {
    const f = el("export-format").value;
    el("export-extension").textContent = "." + extension();
    el("export-format-note").textContent =
      f === "pdf"
        ? "保留各页尺寸与矢量内容。"
        : el("export-scope").value === "all"
          ? "每页单独导出，合并下载为 ZIP。"
          : f === "pptx"
            ? "保留可编辑对象；复杂特效转换为局部图片。"
            : f === "ico"
              ? "ICO 最大 256 × 256 像素，可在设置中调整 DPI。"
              : "导出完整页面，尺寸与缩放和播放进度无关。";
  };
  el("export-name").value = clean(manifest.name);
  el("export-format").onchange = () => {
    el("export-scope").value =
      el("export-format").value === "pdf" ? "all" : "current";
    update();
  };
  el("export-scope").onchange = update;
  update();
  let worker,
    cancel,
    active = false;
  const cancelExport = () => cancel?.();
  el("export-cancel").onclick = cancelExport;
  panel.addEventListener("cancel", (e) => {
    if (active) {
      e.preventDefault();
      cancelExport();
    }
  });
  panel.querySelector(".dialog-close").addEventListener("click", cancelExport);
  el("export-file").onclick = async () => {
    if (active) return;
    active = true;
    let aborted = false;
    cancel = () => {
      aborted = true;
      worker?.terminate();
      cancel = null;
    };
    const controls = [
        ...panel.querySelectorAll(
          "button:not(#export-cancel):not(.dialog-close),input,select",
        ),
      ],
      previous = controls.map((c) => c.disabled);
    controls.forEach((c) => (c.disabled = true));
    el("export-cancel").hidden = false;
    el("export-progress").textContent = "正在准备…";
    let rejectWork, pdfWriter;
    const originalCancel = cancel;
    cancel = () => {
      originalCancel();
      rejectWork?.(new DOMException("已取消导出", "AbortError"));
    };
    try {
      const format = el("export-format").value,
        ext = extension(),
        name = clean(el("export-name").value) + "." + ext;
      let handle;
      if (typeof window.showSaveFilePicker === "function")
        try {
          handle = await window.showSaveFilePicker({ suggestedName: name });
        } catch (e) {
          if (e.name === "AbortError")
            throw e; /* download works without filesystem permission */
        }
      await prepare();
      if (aborted) throw new DOMException("已取消导出", "AbortError");
      const selection = pages()
        .map((p, i) => ({ ...p, number: i + 1 }))
        .filter((_, i) => el("export-scope").value === "all" || i === index());
      if(format === "pdf" && !manifest.editable)pdfWriter=createPDFExport({
        cancelled:()=>aborted,
        progress:message=>{el("export-progress").textContent=message;}
      });
      const result = await new Promise((resolve, reject) => {
        rejectWork = reject;
        const jobWorker = makeWorker();
        worker = jobWorker;
        jobWorker.onerror = (e) => reject(Error(e.message));
        jobWorker.onmessage = async ({ data }) => {
          try {
            if(data.requestPage!==undefined) {
              if(aborted)return;
              const p=selection[data.requestPage];
              if(!p)throw Error("无效导出页面请求");
              jobWorker.postMessage({type:"pdfAck",seq:data.seq,page:{number:p.number,list:p.list,resources:resources(p)}});
              return;
            }
            if(data.pdfResource || data.pdfLayer || data.pdfPage) {
              if(aborted)return;
              let resourceId,pdfMs;
              const begin=performance.now();
              if(data.pdfResource)resourceId=pdfWriter.resource(data.pdfResource);
              else if(data.pdfLayer)resourceId=pdfWriter.layer(data.pdfLayer);
              else {await pdfWriter.page(data.pdfPage);pdfMs=performance.now()-begin;}
              if(!aborted)jobWorker.postMessage({type:"pdfAck",seq:data.seq,resourceId,pdfMs});
              return;
            }
          if (data.progress) el("export-progress").textContent = data.progress;
          else if (data.error) reject(Error(data.error));
          else resolve(data);
          } catch(error){jobWorker.terminate();reject(error);}
        };
        jobWorker.postMessage({
          format,
          options: options(format),
          pages: selection.map(p=>({number:p.number})),
          wasm: bytes(
            manifest.editable
              ? "runtime/exporter-full/laymesh.wasm"
              : "runtime/exporter/laymesh.wasm",
          ),
          skia: bytes("runtime/skia/canvaskit.wasm"),
        });
      });
      if (aborted) throw new DOMException("已取消导出", "AbortError");
      if (format === "jpg" && !manifest.editable) {
        for (const output of result.output) {
          if (aborted) throw new DOMException("已取消导出", "AbortError");
          output.bytes = await exportBrowserJPEG(output, options(format));
        }
      }
      if (format === "pdf" && !manifest.editable) {
        result.output = [
          {
            bytes: pdfWriter.finish(),
          },
        ];
      }
      if (aborted) throw new DOMException("已取消导出", "AbortError");
      const data =
        ext === "zip"
          ? archive.zip(
              Object.fromEntries(
                result.output.map((o) => [
                  "page-" + String(o.page).padStart(3, "0") + "." + format,
                  o.bytes,
                ]),
              ),
            )
          : result.output[0].bytes;
      const blob = new Blob([data], {
        type: mime[ext] || "application/octet-stream",
      });
      if (handle) {
        const stream = await handle.createWritable();
        try {
          await stream.write(blob);
          await stream.close();
        } catch (e) {
          await stream.abort().catch(() => {});
          throw e;
        }
      } else {
        const url = URL.createObjectURL(blob),
          a = document.createElement("a");
        a.href = url;
        a.download = name;
        a.click();
        setTimeout(() => URL.revokeObjectURL(url), 60000);
      }
      const warnings = [...new Set(result.warnings.map((w) => w.message))];
      status(
        "已导出 " + name + (warnings.length ? " · " + warnings.join("；") : ""),
      );
      el("export-progress").textContent = "";
      panel.close();
    } catch (e) {
      if (e.name === "AbortError") {
        status("已取消导出");
        el("export-progress").textContent = "已取消导出";
      } else {
        onError(e);
        el("export-progress").textContent = e.message;
      }
    } finally {
      worker?.terminate();
      pdfWriter?.dispose();
      worker = null;
      cancel = null;
      rejectWork = null;
      active = false;
      controls.forEach((c, i) => (c.disabled = previous[i]));
      el("export-cancel").hidden = true;
    }
  };
  return () => {
    cancelExport();
    settings.remove();
  };
}
