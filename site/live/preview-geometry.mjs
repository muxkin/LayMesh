// Inspection and display-list geometry are in millimetres; UI uses CSS pixels.
export const mmPerUnit = (unit, dpi = 96) =>
  ({ mm: 1, cm: 10, in: 25.4, inch: 25.4, pt: 25.4 / 72, px: 25.4 / dpi })[
    unit
  ] ?? 1;
export function niceStep(minimum) {
  const base = 10 ** Math.floor(Math.log10(minimum));
  return ([1, 2, 5, 10].find((n) => n * base >= minimum) || 10) * base;
}
export function canvasAt(page, rect, x, y) {
  if (
    rect.width <= 0 ||
    rect.height <= 0 ||
    x < rect.left ||
    x > rect.left + rect.width ||
    y < rect.top ||
    y > rect.top + rect.height
  )
    return null;
  const mm = [
    ((x - rect.left) / rect.width) * page.width,
    ((y - rect.top) / rect.height) * page.height,
  ];
  return {
    mm,
    display: mm.map((v) => v / mmPerUnit(page.unit, page.layout_dpi)),
  };
}
export function drawPreviewRuler(
  canvas,
  axis,
  { length, start, scale, page, ink, font, pointer = null },
) {
  const dpr = devicePixelRatio || 1,
    width = axis === "x" ? length : 24,
    height = axis === "x" ? 24 : length;
  if (canvas.width !== Math.round(width * dpr))
    canvas.width = Math.round(width * dpr);
  if (canvas.height !== Math.round(height * dpr))
    canvas.height = Math.round(height * dpr);
  canvas.style.width = width + "px";
  canvas.style.height = height + "px";
  const ctx = canvas.getContext("2d");
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.clearRect(0, 0, width, height);
  ctx.strokeStyle = ink;
  ctx.fillStyle = ink;
  ctx.font = font;
  ctx.lineWidth = 1;
  const factor = mmPerUnit(page.unit, page.layout_dpi),
    density = scale * factor,
    step = niceStep(60 / density),
    minor = step / 5,
    max = (axis === "x" ? page.width : page.height) / factor;
  const first = Math.max(0, Math.ceil(-start / density / minor)),
    last = Math.min(
      Math.floor(max / minor),
      Math.floor((length - start) / density / minor),
    );
  for (let i = first; i <= last; i++) {
    const major = i % 5 === 0;
    if (!major && minor * density < 8) continue;
    const pos = start + i * minor * density,
      size = major ? 10 : 5;
    ctx.beginPath();
    if (axis === "x") {
      ctx.moveTo(pos, 24);
      ctx.lineTo(pos, 24 - size);
    } else {
      ctx.moveTo(24, pos);
      ctx.lineTo(24 - size, pos);
    }
    ctx.stroke();
    if (major) {
      const label = String(Number((i * minor).toPrecision(6)));
      if (axis === "x") ctx.fillText(label, pos + 3, 10);
      else {
        ctx.save();
        ctx.translate(10, pos + 3);
        ctx.rotate(-Math.PI / 2);
        ctx.fillText(label, 0, 0);
        ctx.restore();
      }
    }
  }
  if (pointer != null) {
    ctx.beginPath();
    if (axis === "x") {
      ctx.moveTo(pointer, 0);
      ctx.lineTo(pointer, 24);
    } else {
      ctx.moveTo(0, pointer);
      ctx.lineTo(24, pointer);
    }
    ctx.stroke();
  }
}
