import init, {
  dimensions,
  encode_pixels,
  clear_resources,
  register_resource,
  PptxJob,
  PptxDocumentJob,
  VectorJob,
  PdfJob,
} from "./exporter-full/laymesh_wasm.js";
import { initializeSkia, SkiaRenderer } from "./skia-renderer.mjs";
import { exportHaloMM, exportTilePixels } from "./web-export-raster.mjs";
let exportSkia;
let pendingPage,sequence=0;
const getPage=index=>new Promise(resolve=>{
  const seq=++sequence;pendingPage={seq,resolve};self.postMessage({requestPage:index,seq});
});
self.onmessage = async ({ data }) => {
  if(data.type === "pdfAck") {
    if(pendingPage?.seq===data.seq){const {resolve}=pendingPage;pendingPage=null;resolve(data.page);}return;
  }
  let renderer, pdfJob, pptxDocument;
  try {
    await init({ module_or_path: data.wasm });
    const options = JSON.stringify(data.options),
      output = [],
      warnings = [];
    if(data.format === "pdf")pdfJob=new PdfJob(options);
    if(data.format === "pptx")pptxDocument=new PptxDocumentJob();
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
        rasterOnly: true,
      });
      await renderer.install(list, (id) => resources[id], 0);
      let halo=0;
      if(data.format === "pdf") {
        const density=Math.max(width/(view[2]-view[0]),height/(view[3]-view[1]));
        halo=Math.ceil(exportHaloMM(list.nodes[0])*density)+2;
        if((width+2*halo)*(height+2*halo)>exportTilePixels)throw Error("模糊范围超过 64 MiB 分块缓冲预算");
        // Skia expands filter input beyond the output clip; do not blur an extra output halo.
      }
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
      const snapshot=renderer.surface.makeImageSnapshot();
      let pixels;
      try{pixels=snapshot.readPixels(0,0,{width,height,colorType:exportSkia.ColorType.RGBA_8888,alphaType:exportSkia.AlphaType.Unpremul,colorSpace:exportSkia.ColorSpace.SRGB});}finally{snapshot.delete();}
      if (!pixels) throw Error("无法读取导出画面，请减小 DPI 后重试");
      return pixels;
    };
    for (let i = 0; i < data.pages.length; i++) {
      self.postMessage({
        progress: `正在导出 ${i + 1} / ${data.pages.length} 页…`,
      });
      let page = data.pages[i].list ? data.pages[i] : await getPage(i);
      if(data.format === "pptx" && i>0 && page.list.media)page.list={...page.list,media:{...page.list.media,cues:page.list.media.cues.filter(c=>c.scope!=="document")}};
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
          const pptx=data.format === "pptx";
          const requests=pptx?JSON.parse(job.requests()):null;
          const count=pptx?requests.length:job.request_count();
          for (let j = 0; j < count; j++) {
            const r = pptx?requests[j]:JSON.parse(job.request(j)),
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
          if (data.format === "pdf") {
            self.postMessage({progress:`正在写入 PDF 第 ${page.number} 页…`});
            pdfJob.add_page(result,page.list.width,page.list.height);
          }
          else if (data.format === "svg")
            bytes = new TextEncoder().encode(result);
          else {
            pptxDocument.add_page(result,page.list.width,page.list.height);
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
      data.pages[i]=null;
    }
    if (data.format === "pdf")
      output.push({ bytes: pdfJob.finish() });
    if(pptxDocument){output.push({bytes:pptxDocument.finish()});pptxDocument=null;}
    self.postMessage(
      { output, warnings },
      output.map((o) => o.bytes.buffer),
    );
  } catch (e) {
    self.postMessage({ error: String(e) });
  } finally {
    renderer?.dispose();
    pdfJob?.free();
    pptxDocument?.free();
  }
};
