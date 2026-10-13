import { imageDensity } from "./web-image-density.mjs";

export async function exportBrowserJPEG(output, options) {
  // WebKit file:// workers cannot read even their own encoded Blob. Encode in
  // the owning window, where Blob reads work; rendering and matte stay in the worker.
  const canvas = new OffscreenCanvas(output.rasterWidth, output.rasterHeight);
  try {
    canvas
      .getContext("2d")
      .putImageData(
        new ImageData(
          new Uint8ClampedArray(
            output.bytes.buffer,
            output.bytes.byteOffset,
            output.bytes.byteLength,
          ),
          output.rasterWidth,
          output.rasterHeight,
        ),
        0,
        0,
      );
    const blob = await canvas.convertToBlob({
      type: "image/jpeg",
      quality: (options.quality ?? 90) / 100,
    });
    if (blob.type !== "image/jpeg") throw Error("浏览器不支持 JPEG 编码");
    return imageDensity(
      new Uint8Array(await blob.arrayBuffer()),
      "jpg",
      options.dpi ?? 1200,
    );
  } finally {
    canvas.width = canvas.height = 1;
  }
}
