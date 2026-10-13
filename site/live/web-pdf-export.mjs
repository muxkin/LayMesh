import { jsPDF } from "pdf-export";

// svg2pdf.js uses browser SVG geometry APIs. Keep its DOM private and yield
// between pages so the dialog can paint progress and accept cancellation.
export async function exportVectorPDF(pages, { cancelled, progress }) {
  let pdf;
  const host = document.createElement("div");
  Object.assign(host.style, {
    position: "fixed",
    left: "-100000px",
    top: "0",
    opacity: "0",
    pointerEvents: "none",
  });
  document.body.append(host);
  const check = () => {
    if (cancelled()) throw new DOMException("已取消导出", "AbortError");
  };
  try {
    for (let i = 0; i < pages.length; i++) {
      check();
      progress(`正在生成 PDF ${i + 1} / ${pages.length} 页…`);
      await new Promise((resolve) => setTimeout(resolve, 0));
      check();
      const page = pages[i],
        width = (page.width * 72) / 25.4,
        height = (page.height * 72) / 25.4;
      if (![width, height].every(value => Number.isFinite(value) && value > 0 && value <= 14400))
        throw Error("放映版 PDF 页面宽高须大于 0 且不超过 5080mm");
      const orientation = width > height ? "landscape" : "portrait";
      if (!pdf)
        pdf = new jsPDF({
          unit: "pt",
          format: [width, height],
          orientation,
          compress: true,
          putOnlyUsedFonts: true,
          floatPrecision: 8,
        });
      else pdf.addPage([width, height], orientation);
      const parsed = new DOMParser().parseFromString(
        new TextDecoder().decode(page.bytes),
        "image/svg+xml",
      );
      if (parsed.querySelector("parsererror"))
        throw Error("无法解析导出的 SVG 页面");
      const svg = document.importNode(parsed.documentElement, true);
      host.replaceChildren(svg);
      await pdf.svg(svg, {
        x: 0,
        y: 0,
        width,
        height,
        loadExternalStyleSheets: false,
      });
    }
    check();
    return new Uint8Array(pdf.output("arraybuffer"));
  } finally {
    host.remove();
  }
}
