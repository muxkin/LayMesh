import { chineseFontName, fontAliases } from "./font-aliases.mjs";
// Read only SFNT directories and naming tables, including TTC/OTC faces.
// A catalog scan never registers glyph data with the compiler.
export async function fontNames(blob) {
  // Most face directories fit in one small read. Avoid a separate Blob read
  // for the header, face header and directory of every catalog entry.
  const prefix = await blob.slice(0, 4096).arrayBuffer();
  const read = async (offset, length) => {
    if (offset < 0 || length < 0 || offset + length > blob.size)
      throw Error("字体命名表越界");
    if (offset + length <= prefix.byteLength)
      return new DataView(prefix, offset, length);
    return new DataView(
      await blob.slice(offset, offset + length).arrayBuffer(),
    );
  };
  const header = await read(0, Math.min(12, blob.size));
  if (header.byteLength < 12) throw Error("字体文件过短");
  const offsets = [];
  if (header.getUint32(0) === 0x74746366) {
    const count = header.getUint32(8);
    if (count > 4096) throw Error("字体集合过大");
    const table = await read(12, count * 4);
    for (let i = 0; i < count; i++) offsets.push(table.getUint32(i * 4));
  } else offsets.push(0);
  const faces = [];
  for (const offset of offsets) {
    const h = await read(offset, 12),
      count = h.getUint16(4);
    if (count > 4096) throw Error("字体目录过大");
    const directory = await read(offset + 12, count * 16);
    for (let i = 0; i < count; i++) {
      const at = i * 16;
      if (directory.getUint32(at) !== 0x6e616d65) continue;
      const table = await read(
        directory.getUint32(at + 8),
        directory.getUint32(at + 12),
      );
      if (table.byteLength < 6) throw Error("字体命名表不完整");
      const records = table.getUint16(2),
        storage = table.getUint16(4),
        names = [];
      const decode = (start, length, encoding = "utf-16be") => {
        if (start + length > table.byteLength) throw Error("字体名称越界");
        return new TextDecoder(encoding)
          .decode(new Uint8Array(table.buffer, table.byteOffset + start, length))
          .replace(/\0/g, "")
          .trim();
      };
      for (let n = 0; n < records; n++) {
        const p = 6 + n * 12;
        if (p + 12 > table.byteLength) throw Error("字体名称记录不完整");
        const platform = table.getUint16(p),
          encoding = table.getUint16(p + 2),
          languageID = table.getUint16(p + 4),
          nameID = table.getUint16(p + 6);
        if (![1, 2, 4, 6, 16, 17].includes(nameID)) continue;
        if (
          platform !== 0 &&
          !(platform === 3 && [0, 1, 10].includes(encoding)) &&
          !(platform === 1 && encoding === 0)
        )
          continue;
        let language =
          platform === 3
            ? {
                0x804: "zh-Hans",
                0x1004: "zh-Hans",
                0x404: "zh-Hant",
                0xc04: "zh-Hant",
                0x1404: "zh-Hant",
                0x409: "en-US",
              }[languageID] || "und"
            : "und";
        if (table.getUint16(0) === 1 && languageID >= 0x8000) {
          const p = 6 + records * 12,
            tags = table.getUint16(p),
            tag = languageID - 0x8000;
          if (tag < tags)
            language = decode(
              storage + table.getUint16(p + 4 + tag * 4),
              table.getUint16(p + 2 + tag * 4),
            );
        }
        const value = decode(
          storage + table.getUint16(p + 10),
          table.getUint16(p + 8),
          platform === 1 ? "macintosh" : "utf-16be",
        );
        if (value) names.push({ nameID, language, value });
      }
      faces.push({
        names,
        postscriptName: names.find((n) => n.nameID === 6)?.value,
      });
      break;
    }
  }
  return faces;
}
const displayCache = new WeakMap(), searchCache = new WeakMap();
export function displayFont(font) {
  const cached = displayCache.get(font);
  if (cached && cached.names === font.names) return cached.value;
  const names = font.names || [],
    rank = (n) =>
      /^zh-Hans|^zh-CN|^zh-SG/i.test(n.language)
        ? 0
        : /^zh/i.test(n.language)
          ? 1
          : /^en/i.test(n.language)
            ? 2
            : 3;
  const choose = (ids, fallback) =>
    names
      .filter((n) => ids.includes(n.nameID))
      .sort(
        (a, b) =>
          rank(a) - rank(b) || ids.indexOf(a.nameID) - ids.indexOf(b.nameID),
      )
      .find((n) => rank(n) < 3)?.value || fallback;
  const value = {
    family: choose([16, 1], chineseFontName(font.family)),
    style: choose([17, 2], font.style || "Regular"),
  };
  displayCache.set(font, { names: font.names, value });
  return value;
}
export function fontSearch(font) {
  const cached = searchCache.get(font);
  if (cached && cached.names === font.names) return cached.value;
  const value = [
    ...fontAliases(font.family),
    font.fullName,
    font.style,
    font.postscriptName,
    ...(font.names || []).map((n) => n.value),
  ]
    .filter(Boolean)
    .join(" ")
    .normalize("NFKC")
    .toLocaleLowerCase();
  searchCache.set(font, { names: font.names, value });
  return value;
}
