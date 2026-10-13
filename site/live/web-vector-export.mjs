// Serialize retained geometry only at export time. Fonts have already been
// shaped; shared images remain separate until this standalone SVG is assembled.
export function imageDataURL(bytes, mime = "image/png") {
  const chunks = [];
  for (let i = 0; i < bytes.length; i += 32768)
    chunks.push(String.fromCharCode(...bytes.subarray(i, i + 32768)));
  return `data:${mime};base64,${btoa(chunks.join(""))}`;
}

export async function vectorSVG(
  list,
  { pdf = false, image, bounds, raster, warnings = [] },
) {
  const ident = [1, 0, 0, 1, 0, 0];
  const mul = ([a, b, c, d, e, f], [g, h, i, j, k, l]) => [
    a * g + c * h,
    b * g + d * h,
    a * i + c * j,
    b * i + d * j,
    a * k + c * l + e,
    b * k + d * l + f,
  ];
  const matrix = (t) => `matrix(${t.join(" ")})`;
  const rectArray = (r) => (Array.isArray(r) ? r : [r.x0, r.y0, r.x1, r.y1]);
  const point = (p) => (Array.isArray(p) ? p : [p.x, p.y]);
  const pathData = (path) =>
    path
      .map((p) =>
        p === "ClosePath"
          ? "Z"
          : p.MoveTo
            ? `M${point(p.MoveTo)}`
            : p.LineTo
              ? `L${point(p.LineTo)}`
              : p.QuadTo
                ? `Q${p.QuadTo.flatMap(point)}`
                : p.CurveTo
                  ? `C${p.CurveTo.flatMap(point)}`
                  : "",
      )
      .join("");
  const rgb = (c) =>
    `rgb(${c
      .slice(0, 3)
      .map((v) => Math.round(Math.max(0, Math.min(1, v)) * 255))
      .join(",")})`;
  const at = (url, b) =>
    `<image x="${b[0]}" y="${b[1]}" width="${b[2] - b[0]}" height="${b[3] - b[1]}" preserveAspectRatio="none" xlink:href="${url}"/>`;
  const defs = [],
    images = new Map();
  let next = 0;
  const id = () => `lm${++next}`;
  const nodes = async (items, parent) => {
    const result = [];
    for (const item of items) result.push(await node(item, parent));
    return result.join("");
  };
  const paint = async (p, t) => {
    if (p.kind === "none") return ["none", 1];
    if (p.kind === "solid") return [rgb(p.rgba), p.rgba[3]];
    const key = id();
    let def;
    if (p.kind === "pattern")
      def = `<pattern id="${key}" patternUnits="userSpaceOnUse" width="${p.width}" height="${p.height}" patternTransform="${matrix(p.transform)}">${await nodes(p.nodes, mul(t, p.transform))}</pattern>`;
    else {
      const stops = p.stops
        .map(
          (s) =>
            `<stop offset="${s.offset}" stop-color="${rgb(s.rgba)}" stop-opacity="${s.rgba[3]}"/>`,
        )
        .join("");
      const attrs = `id="${key}" gradientUnits="userSpaceOnUse" gradientTransform="${matrix(p.transform)}" spreadMethod="${p.spread}"`;
      def =
        p.kind === "linear"
          ? `<linearGradient ${attrs} x1="${p.start[0]}" y1="${p.start[1]}" x2="${p.end[0]}" y2="${p.end[1]}">${stops}</linearGradient>`
          : `<radialGradient ${attrs} cx="${p.center[0]}" cy="${p.center[1]}" fx="${p.focal[0]}" fy="${p.focal[1]}" r="${p.radius}">${stops}</radialGradient>`;
    }
    defs.push(def);
    return [`url(#${key})`, 1];
  };
  const layer = async (n, t, effectsOnly = null) => {
    const local = { ...n, transform: ident, opacity: 1 };
    const result = await raster(local, t, effectsOnly);
    return result ? at(imageDataURL(result.bytes), result.rect) : "";
  };
  const node = async (n, parent) => {
    if (n.opacity <= 0) return "";
    const t = mul(parent, n.transform),
      k = n.kind;
    // svg2pdf.js cannot represent SVG masks, isolated opacity groups, gradient
    // spread, or varying stop alpha correctly. Only those local objects fall
    // back to CanvasKit, at the requested physical DPI; surrounding ink stays vector.
    const complex =
      pdf &&
      (n.opacity < 1 ||
        (k.kind === "group" && k.mask) ||
        (k.kind === "path" &&
          ["linear", "radial"].includes(k.fill.kind) &&
          (k.fill.spread !== "pad" ||
            k.fill.stops.some((s) => s.rgba[3] !== 1))));
    if (complex) {
      warnings.push({
        code: "W_PDF_RASTER",
        message: "PDF 中的复杂蒙版或透明混合使用局部高分辨率图片。",
      });
      return `<g transform="${matrix(n.transform)}" opacity="${n.opacity}">${await layer(n, t)}</g>`;
    }
    let body = "";
    if (k.kind === "group") {
      let attrs = "";
      if (k.clip) {
        const key = id();
        defs.push(
          `<clipPath id="${key}" clipPathUnits="userSpaceOnUse"><path d="${pathData(k.clip.path)}" clip-rule="${k.clip.even_odd ? "evenodd" : "nonzero"}"/></clipPath>`,
        );
        attrs += ` clip-path="url(#${key})"`;
      }
      if (k.mask) {
        const key = id(),
          b = bounds(n);
        defs.push(
          `<mask id="${key}" maskUnits="userSpaceOnUse" maskContentUnits="userSpaceOnUse" mask-type="${k.mask.luminance ? "luminance" : "alpha"}" x="${b[0]}" y="${b[1]}" width="${b[2] - b[0]}" height="${b[3] - b[1]}">${await node(k.mask.node, t)}</mask>`,
        );
        attrs += ` mask="url(#${key})"`;
      }
      body = `<g${attrs}>${await nodes(k.children, t)}</g>`;
    } else if (k.kind === "path") {
      const [fill, alpha] = await paint(k.fill, t),
        s = k.stroke;
      const stroke = s
        ? `stroke="${rgb(s.rgba)}" stroke-opacity="${s.rgba[3]}" stroke-width="${s.width}" stroke-linecap="${s.cap}" stroke-linejoin="${s.join}" stroke-miterlimit="${s.miter}" stroke-dasharray="${s.dash.length ? s.dash.join(" ") : "none"}" stroke-dashoffset="${s.offset}"`
        : "";
      body = `<path d="${pathData(k.path)}" fill="${fill}" fill-opacity="${alpha}" fill-rule="${k.even_odd ? "evenodd" : "nonzero"}" ${stroke}/>`;
    } else if (k.kind === "glyphs")
      body = k.outlines
        .map(
          (p) =>
            `<path d="${pathData(p)}" fill="${rgb(k.color)}" fill-opacity="${k.color[3]}"/>`,
        )
        .join("");
    else if (k.kind === "image") {
      let key = images.get(k.resource);
      if (!key) {
        key = id();
        defs.push(
          `<image id="${key}" width="1" height="1" preserveAspectRatio="none" xlink:href="${await image(k.resource)}"/>`,
        );
        images.set(k.resource, key);
      }
      const b = rectArray(k.rect);
      body = `<use xlink:href="#${key}" transform="translate(${b[0]} ${b[1]}) scale(${b[2] - b[0]} ${b[3] - b[1]})" image-rendering="${k.nearest ? "pixelated" : "auto"}"/>`;
    } else throw Error(`不支持的导出绘制指令：${k.kind}`);
    if (n.effects.some((e) => !e.inner))
      body = (await layer(n, t, false)) + body;
    if (n.effects.some((e) => e.inner)) body += await layer(n, t, true);
    return `<g transform="${matrix(n.transform)}" opacity="${n.opacity}">${body}</g>`;
  };
  const body = await nodes(list.nodes, ident);
  return `<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="${list.width}mm" height="${list.height}mm" viewBox="0 0 ${list.width} ${list.height}"><defs>${defs.join("")}</defs>${body}</svg>`;
}
