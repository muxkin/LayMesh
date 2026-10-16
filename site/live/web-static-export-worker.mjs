import init, { encode_tiff } from "./exporter/laymesh_wasm.js";
import { initializeSkia, SkiaRenderer } from "./skia-renderer.mjs";
import { vectorSVG, imageDataURL } from "./web-vector-export.mjs";
import { imageDensity } from "./web-image-density.mjs";
import { exportDensity, rasterExportTiles } from "./web-export-raster.mjs";
import { preparePDFTile } from "./web-pdf-resources.mjs";
let exportSkia;
let pendingPDF, sequence=0;
const sendPDF = (message, transfer=[]) => new Promise(resolve => {
  const seq=++sequence; pendingPDF={seq,resolve}; self.postMessage({...message,seq},transfer);
});
self.onmessage = async ({ data }) => {
  if(data.type === "pdfAck"){if(pendingPDF?.seq===data.seq){const {resolve}=pendingPDF;pendingPDF=null;resolve(data.page ?? data.resourceId ?? data.pdfMs);}return;}
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
      warnings = [],
      metrics={renderMs:0,readbackMs:0,encodeMs:0,pdfMs:0,tiles:0,peakTileRGBABytes:0,resourceBytes:0};
    renderer = new SkiaRenderer(ck, new OffscreenCanvas(1, 1), {
      software: true,
      rasterOnly: true,
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
      await renderer.install(list, (id) => resources[id], 0);
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
    const layerCache=new Map();
    for (let i = 0; i < data.pages.length; i++) {
      self.postMessage({
        progress: `正在处理效果 ${i + 1} / ${data.pages.length} 页…`,
      });
      const page = data.pages[i].list ? data.pages[i] : await sendPDF({requestPage:i}),
        list = page.list;
      // A resource identifier in an older file need not be a content hash.
      // PDF streams still deduplicate by bytes after rendering these pages.
      if(Object.keys(page.resources).length)layerCache.clear();
      let bytes, rasterWidth, rasterHeight;
      if (["pdf", "svg"].includes(data.format)) {
        await renderer.install(list, (id) => page.resources[id], 0);
        const warningStart=warnings.length;
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
            if(data.format === "pdf") {
              const density=exportDensity(t,dpi);
              const key=JSON.stringify([node,density,effectsOnly],(k,v)=>k==="name"?undefined:v);
              if(layerCache.has(key))return layerCache.get(key);
              const variants=effectsOnly===null?[node]:node.effects.filter(e=>e.inner===effectsOnly).map(e=>({...node,effects:[e]}));
              const fragments=[];
              for(const variant of variants) {
                const tiles=[],color=effectsOnly===null?null:variant.effects[0].rgba;
                await rasterExportTiles({renderer,list,resources:page.resources,node:variant,density,effectsOnly,
                consume:async tile=>{
                  metrics.tiles++;metrics.peakTileRGBABytes=Math.max(metrics.peakTileRGBABytes,tile.surfaceBytes);
                  metrics.renderMs+=tile.renderMs;metrics.readbackMs+=tile.readbackMs;
                  const begin=performance.now(),resource=preparePDFTile(tile,color);
                  metrics.encodeMs+=performance.now()-begin;
                  if(!resource)return;
                  const [x0,y0,x1,y1]=resource.crop, r=tile.rect;
                  const rect=[r[0]+x0/density,r[1]+y0/density,r[0]+x1/density,r[1]+y1/density];
                  metrics.resourceBytes+=resource.data.length+resource.alpha.length;
                  const resourceId=await sendPDF({pdfResource:resource},[resource.data.buffer,resource.alpha.buffer]);
                  tiles.push({id:resourceId,rect});
                }
              });
                if(tiles.length) {
                  const rect=[Math.min(...tiles.map(t=>t.rect[0])),Math.min(...tiles.map(t=>t.rect[1])),Math.max(...tiles.map(t=>t.rect[2])),Math.max(...tiles.map(t=>t.rect[3]))];
                  const id=await sendPDF({pdfLayer:{tiles,color,rect}});
                  fragments.push(`<image x="${rect[0]}" y="${rect[1]}" width="${rect[2]-rect[0]}" height="${rect[3]-rect[1]}" preserveAspectRatio="none" data-laymesh-image="laymesh-pdf:${id}"/>`);
                }
              }
              const markup=fragments.join("");
              // Retain only compact references to resources already in the PDF.
              layerCache.set(key,markup);
              return markup;
            }
            const density=exportDensity(t,dpi), scale=density*25.4/dpi;
            if(density<=0)return null;
            const raw=renderer.prepare(node).bounds;
            const view=raw.map((v,i)=>(i<2?Math.floor(v*density):Math.ceil(v*density))/density);
            if(view[2]<=view[0]||view[3]<=view[1])return null;
            const [w,h]=dimensions((view[2]-view[0])*scale,(view[3]-view[1])*scale);
            const pixels=await raster({...list,nodes:[node],animation:null,pageTransition:null},page.resources,view,w,h,effectsOnly);
            return {rect:view,bytes:await encode(pixels,w,h)};
          },
        });
        for(let w=warningStart;w<warnings.length;w++)warnings[w].message=`第 ${page.number} 页：${warnings[w].message}`;
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
      if(data.format === "pdf") {
        metrics.pdfMs+=await sendPDF({pdfPage:{bytes,page:page.number,width:list.width,height:list.height}},[bytes.buffer]);
        data.pages[i]=null;
        continue;
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
        metrics,
        warnings: [...new Map(warnings.map((w) => [w.message, w])).values()],
        pdfStreamed: data.format === "pdf",
      },
      output.map((o) => o.bytes.buffer),
    );
  } catch (e) {
    self.postMessage({ error: String(e) });
  } finally {
    renderer?.dispose();
  }
};
