// CanvasKit encodes pixels once; inject physical density without re-encoding.
// Reuse the container CRC routine for the 9-byte PNG pHYs payload.
import { webCrc32 } from "./web-archive.mjs";
export function imageDensity(bytes, format, dpi) {
  if (format === "jpg") {
    let p = 2;
    while (p + 4 < bytes.length && bytes[p] === 255) {
      const marker = bytes[p + 1];
      if (marker === 218 || marker === 217) break;
      const length = (bytes[p + 2] << 8) | bytes[p + 3];
      if (length < 2 || p + 2 + length > bytes.length) break;
      if (
        marker === 224 &&
        length >= 16 &&
        String.fromCharCode(...bytes.subarray(p + 4, p + 9)) === "JFIF\0"
      ) {
        const out = bytes.slice(),
          v = new DataView(out.buffer);
        out[p + 11] = 1;
        v.setUint16(p + 12, Math.round(dpi));
        v.setUint16(p + 14, Math.round(dpi));
        return out;
      }
      p += 2 + length;
    }
    const out = new Uint8Array(bytes.length + 18);
    out.set(bytes.subarray(0, 2));
    out.set([255, 224, 0, 16, 74, 70, 73, 70, 0, 1, 1, 1, 0, 0, 0, 0, 0, 0], 2);
    const v = new DataView(out.buffer);
    v.setUint16(14, Math.round(dpi));
    v.setUint16(16, Math.round(dpi));
    out.set(bytes.subarray(2), 20);
    return out;
  }
  const parts = [bytes.subarray(0, 8)];
  let offset = 8;
  while (offset < bytes.length) {
    const length = new DataView(
        bytes.buffer,
        bytes.byteOffset + offset,
        4,
      ).getUint32(0),
      end = offset + 12 + length;
    if (end > bytes.length) throw Error("无效 PNG 数据");
    const name = String.fromCharCode(...bytes.subarray(offset + 4, offset + 8));
    if (name !== "pHYs") parts.push(bytes.subarray(offset, end));
    if (name === "IHDR") {
      const chunk = new Uint8Array(21),
        v = new DataView(chunk.buffer);
      v.setUint32(0, 9);
      chunk.set([112, 72, 89, 115], 4);
      v.setUint32(8, Math.round(dpi / 0.0254));
      v.setUint32(12, Math.round(dpi / 0.0254));
      chunk[16] = 1;
      v.setUint32(17, webCrc32(chunk.subarray(4, 17)));
      parts.push(chunk);
    }
    offset = end;
  }
  const out = new Uint8Array(parts.reduce((s, p) => s + p.length, 0));
  offset = 0;
  for (const part of parts) {
    out.set(part, offset);
    offset += part.length;
  }
  return out;
}
