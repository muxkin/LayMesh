import init, { encode_tiff } from "./exporter/laymesh_wasm.js";
import { initializeSkia, SkiaRenderer } from "./skia-renderer.mjs";
import { vectorSVG, imageDataURL } from "./web-vector-export.mjs";
import { imageDensity } from "./web-image-density.mjs";
let exportSkia;
self.onmessage = async ({ data }) => {
  let renderer;
  try {
    const dpi = data.options.dpi ?? 1200;
    if (!Number.isFinite(dpi) || dpi <= 0 || dpi > 25400)
      throw Error("DPI 必须大于 0 且不超过 25400");
    const dimensions = (w, h, density = dpi) => {
      const width = Math.max(1, Math.round((w * density) / 25.4)),
        height = Math.max(1, Math.round((h * density) / 25.4));
      if (
        ![w, h, width, height].every((v) => Number.isFinite(v) && v > 0) ||
        !Number.isSafeInteger(width) ||
        !Number.isSafeInteger(height) ||
        width * height > 100000000
      )
        throw Error("导出超过 100,000,000 像素；请在导出设置中减小 DPI");
      return [width, height];
    };
    if (data.format === "tif") await init({ module_or_path: data.wasm });
    exportSkia ||= await initializeSkia(data.skia);
    const ck = exportSkia,
      output = [],
      warnings = [];
    renderer = new SkiaRenderer(ck, new OffscreenCanvas(1, 1), {
      software: true,
      cacheMB: 32,
    });
    const encode = async (pixels, w, h, format = "png") => {
      const image = ck.MakeImage(
        {
          width: w,
          height: h,
          colorType: ck.ColorType.RGBA_8888,
          alphaType: ck.AlphaType.Unpremul,
          colorSpace: ck.ColorSpace.SRGB,
        },
        pixels,
        w * 4,
      );
      if (!image) throw Error("无法创建导出图片");
      try {
        const bytes = image.encodeToBytes(
          format === "jpg" ? ck.ImageFormat.JPEG : ck.ImageFormat.PNG,
          data.options.quality ?? 90,
        );
        if (!bytes) throw Error("图片编码失败");
        return bytes.slice();
      } finally {
        image.delete();
      }
    };
    const raster = async (
      list,
      resources,
      view,
      width,
      height,
      effectsOnly = null,
    ) => {
      if (width * height > 100000000)
        throw Error("导出超过 100,000,000 像素；请减小 DPI");
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
      const page = data.pages[i],
        list = page.list;
      let bytes, rasterWidth, rasterHeight;
      if (["pdf", "svg"].includes(data.format)) {
        await renderer.install(list, (id) => page.resources[id], 0, 32);
        const svg = await vectorSVG(list, {
          pdf: data.format === "pdf",
          warnings,
          bounds: (node) => Array.from(renderer.prepare(node).bounds),
          image: async (id) => {
            const mime = list.resources[id]?.mime,
              b = page.resources[id];
            if (!b?.length) throw Error(`缺少图片资源：${id}`);
            if (mime === "image/png" || mime === "image/jpeg")
              return imageDataURL(b, mime);
            const image = renderer.images.get(id),
              png = image?.encodeToBytes(ck.ImageFormat.PNG);
            if (!png) throw Error(`无法导出图片：${id}`);
            return imageDataURL(png);
          },
          raster: async (node, t, effectsOnly) => {
            const [a, b, c, d] = t,
              sum = a * a + b * b + c * c + d * d,
              det = a * d - b * c;
            const scale = Math.sqrt(
              (sum + Math.sqrt(Math.max(0, sum * sum - 4 * det * det))) / 2,
            );
            if (!Number.isFinite(scale)) throw Error("无效导出变换");
            if (scale <= 0) return null;
            const density = (dpi / 25.4) * scale,
              raw = renderer.prepare(node).bounds;
            const view = raw.map(
              (v, i) =>
                (i < 2 ? Math.floor(v * density) : Math.ceil(v * density)) /
                density,
            );
            if (view[2] <= view[0] || view[3] <= view[1]) return null;
            const [w, h] = dimensions(
              (view[2] - view[0]) * scale,
              (view[3] - view[1]) * scale,
            );
            const pixels = await raster(
              { ...list, nodes: [node], animation: null, pageTransition: null },
              page.resources,
              view,
              w,
              h,
              effectsOnly,
            );
            return { rect: view, bytes: await encode(pixels, w, h) };
          },
        });
        bytes = new TextEncoder().encode(svg);
      } else {
        const [w, h] = dimensions(list.width, list.height);
        const pixels = await raster(
          list,
          page.resources,
          [0, 0, list.width, list.height],
          w,
          h,
        );
        if (data.format === "tif")
          bytes = encode_tiff(
            w,
            h,
            pixels,
            dpi,
            data.options.compression ?? "lzw",
          );
        else if (data.format === "jpg") {
          if (w > 65535 || h > 65535)
            throw Error("JPEG 宽高不能超过 65535 像素");
          const bg = data.options.background ?? "#ffffff";
          if (!/^#[0-9a-f]{6}$/i.test(bg)) throw Error("无效 JPEG 背景颜色");
          const color = [1, 3, 5].map((i) => parseInt(bg.slice(i, i + 2), 16));
          for (let i = 0; i < pixels.length; i += 4) {
            const a = pixels[i + 3] / 255;
            for (let c = 0; c < 3; c++)
              pixels[i + c] = Math.round(
                pixels[i + c] * a + color[c] * (1 - a),
              );
            pixels[i + 3] = 255;
          }
          bytes = pixels;
          rasterWidth = w;
          rasterHeight = h;
        } else if (data.format === "png")
          bytes = imageDensity(
            await encode(pixels, w, h, data.format),
            data.format,
            dpi,
          );
        else throw Error(`浏览器不支持此导出格式：${data.format}`);
      }
      output.push({
        bytes,
        page: page.number,
        rasterWidth,
        rasterHeight,
        width: list.width,
        height: list.height,
      });
    }
    self.postMessage(
      {
        output,
        warnings: [...new Map(warnings.map((w) => [w.code, w])).values()],
      },
      output.map((o) => o.bytes.buffer),
    );
  } catch (e) {
    self.postMessage({ error: String(e) });
  } finally {
    renderer?.dispose();
  }
};
