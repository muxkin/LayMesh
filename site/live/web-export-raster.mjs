// Export-only surfaces at unchanged physical DPI. Skia's saveLayer image filter
// requests the halo outside the output clip itself. Reserve that input area in
// the budget; rendering an extra output halo would run the same blur twice.
// One pixel of overlap is retained for opaque mask assembly in the PDF, which
// avoids image-edge antialias seams without doubling the shadow alpha.
export const exportTilePixels = 16 * 1024 * 1024;
export const exportTileEdge = 4096;

export function exportDensity(transform, dpi) {
  const [a, b, c, d] = transform, sum = a*a+b*b+c*c+d*d, det = a*d-b*c;
  const scale = Math.sqrt((sum + Math.sqrt(Math.max(0, sum*sum-4*det*det))) / 2);
  if (!Number.isFinite(scale) || !Number.isFinite(dpi) || dpi <= 0) throw Error("无效导出变换或 DPI");
  return dpi / 25.4 * scale;
}

// Includes descendant effects when a complete object must be rasterized.
export function exportHaloMM(node) {
  let halo = 0;
  for (const e of node.effects) halo = Math.max(halo, 4*e.blur + Math.abs(e.spread) + Math.max(...e.offset.map(Math.abs)));
  const own=halo;halo=0;
  const k = node.kind;
  if (k.kind === "group") for (const child of [...k.children, ...(k.mask ? [k.mask.node] : [])]) {
    const scale = exportDensity(child.transform, 25.4);
    halo = Math.max(halo, scale * exportHaloMM(child));
  }
  if(k.kind === "path" && k.fill.kind === "pattern") {
    const scale=exportDensity(k.fill.transform,25.4);
    for(const child of k.fill.nodes)halo=Math.max(halo,scale*exportDensity(child.transform,25.4)*exportHaloMM(child));
  }
  return own+halo;
}

export async function rasterExportTiles({ renderer, list, resources, node, density, effectsOnly = null, alphaOnly = effectsOnly !== null, consume }) {
  if (density <= 0) return;
  const localList = { ...list, nodes: [node], animation: null, pageTransition: null };
  await renderer.install(localList, id => resources[id], 0);
  const raw = renderer.prepare(node).bounds;
  const bounds = raw.map((v, i) => i < 2 ? Math.floor(v*density) : Math.ceil(v*density));
  const width = bounds[2]-bounds[0], height = bounds[3]-bounds[1];
  if (width <= 0 || height <= 0) return;
  if (!bounds.every(Number.isSafeInteger) || width*height > 100000000) throw Error("局部特效超过 100,000,000 像素；请减小 DPI 或特效尺寸");
  const halo = Math.ceil(exportHaloMM(node)*density) + 2;
  const edge = Math.min(exportTileEdge, Math.floor(Math.sqrt(exportTilePixels))-2*halo-2);
  if (edge < 64) throw Error("模糊范围超过 64 MiB 分块缓冲预算；请减小模糊范围或 DPI");
  for (let y = 0; y < height; y += edge) for (let x = 0; x < width; x += edge) {
    const w = Math.min(edge, width-x), h = Math.min(edge, height-y);
    const rw = w+2, rh = h+2;
    const view = [bounds[0]+x-1, bounds[1]+y-1, bounds[0]+x+w+1, bounds[1]+y+h+1].map(v => v/density);
    const started=performance.now();
    renderer.render(view, rw, rh, {}, 0, false, false, false, false, effectsOnly);
    const rendered=performance.now();
    // The filter halo stays in Skia. Only a one-pixel assembly overlap is read.
    const snapshot = renderer.surface.makeImageSnapshot();
    let pixels;
    const ck=renderer.ck;
    const dest=alphaOnly?ck.Malloc(Uint8Array,rw*rh):null;
    try {
      pixels = snapshot.readPixels(0, 0, { width: rw, height: rh, colorType: alphaOnly?ck.ColorType.Alpha_8:ck.ColorType.RGBA_8888, alphaType: alphaOnly?ck.AlphaType.Premul:ck.AlphaType.Unpremul, colorSpace: ck.ColorSpace.SRGB },dest||undefined,alphaOnly?rw:undefined);
      if(dest&&pixels)pixels=pixels.slice();
    } finally { if(dest)ck.Free(dest); snapshot.delete(); }
    if (!pixels) throw Error("无法读取导出特效分块");
    const rect = [bounds[0]+x-1, bounds[1]+y-1, bounds[0]+x+w+1, bounds[1]+y+h+1].map(v => v/density);
    await consume({ pixels, alphaOnly, width: rw, height: rh, rect, surfaceBytes: (rw+2*halo)*(rh+2*halo)*4, renderMs:rendered-started, readbackMs:performance.now()-rendered });
  }
}
