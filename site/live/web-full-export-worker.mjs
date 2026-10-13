import init, {
  dimensions,
  encode_pixels,
  export_pdf,
  clear_resources,
  register_resource,
  PptxJob,
  VectorJob,
} from "./exporter-full/laymesh_wasm.js";
import { initializeSkia, SkiaRenderer } from "./skia-renderer.mjs";
let exportSkia;
self.onmessage = async ({ data }) => {
  let renderer;
  try {
    await init({ module_or_path: data.wasm });
    const options = JSON.stringify(data.options),
      output = [],
      warnings = [];
    const vectorPages = [];
    const raster = async (
      list,
      resources,
      view,
      width,
      height,
      effectsOnly = null,
    ) => {
      if (width * height > 100000000)
        throw Error("导出超过 100,000,000 像素；请在导出设置中减小 DPI");
      exportSkia ||= await initializeSkia(data.skia);
      // A dedicated software surface avoids screen-size/GPU texture limits.
      renderer ||= new SkiaRenderer(exportSkia, new OffscreenCanvas(1, 1), {
        software: true,
        cacheMB: 32,
      });
      await renderer.install(list, (id) => resources[id], 0, 32);
      renderer.render(
        view,
        width,
        height,
        {},
        0,
        false,
        false,
        false,
        false,
        effectsOnly,
      );
      const pixels = renderer.readPixels();
      if (!pixels) throw Error("无法读取导出画面，请减小 DPI 后重试");
      return pixels;
    };
    for (let i = 0; i < data.pages.length; i++) {
      self.postMessage({
        progress: `正在导出 ${i + 1} / ${data.pages.length} 页…`,
      });
      const page = data.pages[i];
      let bytes;
      if (["pptx", "pdf", "svg"].includes(data.format)) {
        clear_resources();
        for (const [id, b] of Object.entries(page.resources))
          register_resource(id, b);
        const job =
          data.format === "pptx"
            ? new PptxJob(JSON.stringify(page.list), options)
            : new VectorJob(
                JSON.stringify(page.list),
                options,
                data.format === "pdf",
              );
        try {
          const requests = JSON.parse(job.requests());
          for (let j = 0; j < requests.length; j++) {
            const r = requests[j],
              pixels = await raster(
                r.list,
                page.resources,
                r.rect,
                r.width,
                r.height,
                r.effectsOnly ?? null,
              );
            job.set_image(
              j,
              encode_pixels(
                r.width,
                r.height,
                pixels,
                "png",
                JSON.stringify({ dpi: data.options.dpi || 1200 }),
              ),
            );
          }
          const result = job.finish();
          if (data.format === "pdf")
            vectorPages.push({
              svg: result,
              width: page.list.width,
              height: page.list.height,
            });
          else if (data.format === "svg")
            bytes = new TextEncoder().encode(result);
          else {
            bytes = result;
            warnings.push(...JSON.parse(job.warnings()));
          }
        } finally {
          job.free();
          clear_resources();
        }
      } else {
        const [w, h] = dimensions(
          page.list.width,
          page.list.height,
          data.format,
          options,
        );
        const pixels = await raster(
          page.list,
          page.resources,
          [0, 0, page.list.width, page.list.height],
          w,
          h,
        );
        bytes = encode_pixels(w, h, pixels, data.format, options);
      }
      if (bytes) output.push({ bytes, page: page.number });
    }
    if (data.format === "pdf")
      output.push({ bytes: export_pdf(JSON.stringify(vectorPages), options) });
    self.postMessage(
      { output, warnings },
      output.map((o) => o.bytes.buffer),
    );
  } catch (e) {
    self.postMessage({ error: String(e) });
  } finally {
    renderer?.dispose();
  }
};
