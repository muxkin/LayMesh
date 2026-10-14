// Universal HTML/ZIP transport. ZIP bytes are HTML plaintext, never executable.
// fflate is vendored under MIT. The DOM recovery codec is project-owned.
import {
  zipSync,
  unzipSync,
  gzipSync,
  gunzipSync,
  deflateSync,
  zlibSync,
} from "../../vendor/fflate/fflate.mjs";
export function deflatePDFBytes(bytes) { return zlibSync(bytes, { level: 1 }); }
const webCp1252 = [
  8364, 129, 8218, 402, 8222, 8230, 8224, 8225, 710, 8240, 352, 8249, 338, 141,
  381, 143, 144, 8216, 8217, 8220, 8221, 8226, 8211, 8212, 732, 8482, 353, 8250,
  339, 157, 382, 376,
];
const webTransportMetrics = { recoveryMs: 0 };
export function webCrc32(bytes) {
  let c = 0xffffffff;
  for (const b of bytes) {
    c ^= b;
    for (let i = 0; i < 8; i++) c = (c >>> 1) ^ (c & 1 ? 0xedb88320 : 0);
  }
  return (c ^ 0xffffffff) >>> 0;
}
export function webZip(files) {
  const selected = {};
  for (const [name, entry] of Object.entries(files)) {
    const [data, options = {}] = Array.isArray(entry) ? entry : [entry];
    const level = options.level ?? 9;
    const stored =
      level === 0 || deflateSync(data, { level }).byteLength >= data.byteLength;
    selected[name] = [data, { ...options, level: stored ? 0 : level }];
  }
  return zipSync(selected, { level: 9 });
}
export function webPayload(bytes) {
  const v = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  let end = bytes.length - 22;
  while (
    end >= Math.max(0, bytes.length - 65557) &&
    v.getUint32(end, true) !== 0x06054b50
  )
    end--;
  if (end < 0) throw Error("ZIP directory is missing");
  const start = end - v.getUint32(end + 12, true) - v.getUint32(end + 16, true);
  if (start < 0) throw Error("Invalid ZIP offset");
  return bytes.subarray(start);
}
export function webUnzip(bytes) {
  const started = performance.now();
  const files = unzipSync(bytes),
    view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  let end = bytes.length - 22;
  while (
    end >= Math.max(0, bytes.length - 65557) &&
    view.getUint32(end, true) !== 0x06054b50
  )
    end--;
  if (end < 0) throw Error("ZIP directory is missing");
  const count = view.getUint16(end + 10, true),
    size = view.getUint32(end + 12, true);
  let p = end - size;
  for (let i = 0; i < count; i++) {
    if (view.getUint32(p, true) !== 0x02014b50)
      throw Error("Invalid ZIP directory");
    const n = view.getUint16(p + 28, true),
      extra = view.getUint16(p + 30, true),
      comment = view.getUint16(p + 32, true),
      name = new TextDecoder().decode(bytes.subarray(p + 46, p + 46 + n));
    if (!files[name] || webCrc32(files[name]) !== view.getUint32(p + 16, true))
      throw Error("ZIP CRC mismatch: " + name);
    p += 46 + n + extra + comment;
  }
  Object.assign(webTransportMetrics, {
    unzipMs: performance.now() - started,
    packageBytes: bytes.byteLength,
    unpackedBytes: Object.values(files).reduce(
      (sum, data) => sum + data.byteLength,
      0,
    ),
  });
  return files;
}
export function webRecover(text, length, records) {
  const started = performance.now();
  const packed = gunzipSync(
      Uint8Array.from(atob(records), (c) => c.charCodeAt(0)),
    ),
    cr = [];
  let pos = 0;
  for (let i = 0; i < packed.length; ) {
    let value = 0,
      shift = 0,
      b;
    do {
      if (i >= packed.length || shift > 49)
        throw Error("Invalid recovery record");
      b = packed[i++];
      value += (b & 127) * 2 ** shift;
      shift += 7;
    } while (b & 128);
    pos += Math.floor(value / 2);
    cr.push([pos, !!(value % 2)]);
  }
  const out = new Uint8Array(length);
  let i = 0,
    j = 0,
    r = 0;
  while (j < length) {
    if (i >= text.length) throw Error("Truncated Universal archive");
    const code = text.charCodeAt(i++);
    if (r < cr.length && j === cr[r][0]) {
      if (code !== 10) throw Error("Invalid newline recovery");
      out[j++] = 13;
      if (cr[r++][1]) out[j++] = 10;
    } else {
      const k = webCp1252.indexOf(code);
      out[j++] = code === 65533 ? 0 : k >= 0 ? k + 128 : code;
    }
  }
  if (i !== text.length) throw Error("Unexpected Universal archive tail");
  webTransportMetrics.recoveryMs = performance.now() - started;
  return out;
}
export function webUniversal(zip, bootstrap) {
  const cr = [];
  let previous = 0;
  for (let i = 0; i < zip.length; i++)
    if (zip[i] === 13) {
      const pair = zip[i + 1] === 10;
      let value = (i - previous) * 2 + Number(pair);
      previous = i;
      while (value >= 128) {
        cr.push(value % 128 | 128);
        value = Math.floor(value / 128);
      }
      cr.push(value);
      if (pair) i++;
    }
  const records = btoa(
    Array.from(gzipSync(new Uint8Array(cr), { level: 9 }), (b) =>
      String.fromCharCode(b),
    ).join(""),
  );
  const header =
    '<!doctype html><meta charset="windows-1252"><meta name="viewport" content="width=device-width,initial-scale=1"><title>LayMesh</title><style>plaintext{display:none}body{margin:0;background:#f6f7fa;color:#263247;font:16px system-ui}</style><div id="app">Preparing LayMesh...</div><script type="module">' +
    bootstrap.replace(/<\/script/gi, "<\\/script") +
    '\ntry{const zipBytes=location.protocol.startsWith("http")?webPayload(new Uint8Array(await (await fetch(location.href)).arrayBuffer())):webRecover(document.getElementById("laymesh-zip").textContent,' +
    zip.length +
    "," +
    JSON.stringify(records) +
    ');document.getElementById("laymesh-zip").remove();await webBoot(webUnzip(zipBytes));}catch(error){document.getElementById("app").textContent=String(error);console.error(error);}<' +
    '/script><plaintext hidden id="laymesh-zip">';
  return new Blob([header, zip], { type: "text/html;charset=windows-1252" });
}
export async function webBoot(files) {
  const started = performance.now();
  const text = (p) =>
    new TextDecoder().decode(
      files[p] ??
        (() => {
          throw Error("Missing package entry: " + p);
        })(),
    );
  const manifest = JSON.parse(text("manifest.json"));
  if (![1, 2].includes(manifest.schemaVersion))
    throw Error("Unsupported LayMesh package version");
  const urls = new Map(),
    building = new Set();
  const path = (from, relative) => {
    const parts = from.split("/");
    parts.pop();
    for (const s of relative.split("/"))
      if (s === "..") parts.pop();
      else if (s !== ".") parts.push(s);
    return parts.join("/");
  };
  function moduleURL(name) {
    if (urls.has(name)) return urls.get(name);
    if (building.has(name)) throw Error("Cyclic package module: " + name);
    building.add(name);
    let code = text(name);
    code = code.replace(
      /((?:^|[\n;])\s*(?:import|export)\s*(?:[^'";]*?\s+from\s*)?['"])([^'"]+)(['"])/g,
      (all, a, b, c) => {
        const dest = b.startsWith(".")
          ? path(name, b)
          : "runtime/vendor/" + b + "/index.js";
        return a + moduleURL(dest) + c;
      },
    );
    const url = URL.createObjectURL(
      new Blob([code], { type: "text/javascript" }),
    );
    urls.set(name, url);
    building.delete(name);
    return url;
  }
  const app = await import(moduleURL("runtime/web-player.mjs"));
  await app.mountWebPlayer({
    manifest,
    files,
    moduleURL,
    archive: { zip: webZip, universal: webUniversal },
    transportMetrics: {
      ...webTransportMetrics,
      moduleInitMs: performance.now() - started,
    },
    root: document.getElementById("app"),
  });
  window.addEventListener(
    "pagehide",
    () => {
      for (const url of urls.values()) URL.revokeObjectURL(url);
    },
    { once: true },
  );
}
