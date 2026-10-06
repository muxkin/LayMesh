# Artistic text, shadows and glow

Decoration is opt-in. `border_*` frames the layout box; `text_stroke_*` traces the actual glyphs. Fonts remain user-provided and obey the existing embedding restrictions.

<!-- example:examples/effects/art-text.lay -->

## Glyph paint

`text_fill` accepts a color or existing paint, including linear/radial gradients. A gradient spans the complete transformed text, rather than restarting at each glyph. `text_stroke_color` and `text_stroke_width` add a centered solid outline; unitless stroke widths are points. `color` is the fallback when `text_fill` is omitted. Spans may override glyph paint and stroke independently.

## text_path

`path=text_path(route,start=0mm,align="center",reverse=false)` follows one local nonzero open path material (`path`, `line`, `polyline` or `arc`). Glyph centers follow arc length and local tangents; kerning, shaping and font fallback are preserved. Alignment uses the available route length. Negative starts, insufficient length, multiple subpaths, closed routes and multiline text are errors. Inline formulas may use warp/extrusion but cannot follow a text path in this version.

## text_warp

`warp=text_warp("arc",angle=45deg)` bends outlines into an upright arc. `text_warp("wave",amplitude=2mm,wavelength=20mm,phase=0deg)` shifts the baseline sinusoidally. `text_warp("perspective",corners=[(0mm,0mm),(60mm,2mm),(55mm,15mm),(5mm,12mm)])` maps the subject into a convex quadrilateral, ordered top-left, top-right, bottom-right, bottom-left. Degenerate/self-intersecting corners are rejected. Nonlinear warps use vector contours with 0.002 mm source flattening tolerance.

## text_extrude

`extrude=text_extrude(depth=1mm,angle=45deg,color="#555555")` adds vector back faces and swept side faces. This is a two-dimensional extrusion with configurable direction and color, without a 3D camera or lighting. Processing order is shaping/layout, path placement, warp, extrusion, stroke and effects.

## shadow

`effects=[shadow(color="#000000",opacity=0.5,blur=1mm,spread=0mm,offset=(1mm,1mm),mode="outer",target="content")]` uses the rendered alpha, including transparent image contours. `blur` is Gaussian standard deviation, `spread` is nonnegative dilation, and offsets follow local x/right and y/down. `mode="inner"` clips the effect to the source alpha. Colors may carry alpha, multiplied by the effect opacity.

## glow

`glow(color="#ffffff",opacity=1,blur=1mm,spread=0mm,mode="outer",target="content")` shares the shadow options without an offset. Effects are ordered within their back/front layers: outer effects behind the subject, inner effects above it. Effects derive from the subject alpha, not from previous sibling effects. `target="object"` includes an optional frame/background; content effects exclude that frame. A group's content includes its children. Group scaling and rotation also transform effects.

<!-- example:examples/effects/shadow-glow.lay -->

## LCSS decorations

`text-fill`, `text-stroke-color`, `text-stroke-width`, `effects`, `path`, `warp` and `extrude` are shared LCSS properties. Configurations use literal constructor expressions; quoted enums are recommended. Paths in LCSS must be literal local materials, not references to `.lay` variables.

```lcss
.title {
  text-fill: linear-gradient(to right, #ea366d, #365de6);
  text-stroke-color: #ffffff;
  text-stroke-width: 0.7pt;
  effects: [shadow(blur=0.8mm, offset=(1mm,2mm))];
  warp: text_warp("wave", amplitude=1mm, wavelength=25mm);
  extrude: text_extrude(depth=1mm, color="#213864");
}
```

## Geometry and export

Layout/anchor dimensions retain their original meaning. `subjectBounds` records transformed glyph occupancy; decorated overflow triggers `W_EFFECT_OVERFLOW`. Shadows/glow do not reflow the page. SVG, WASM and VS Code preview use the same filter definitions as native raster export. Decorative glyphs are vector paths, with source text retained as SVG labels and PDF ActualText; ordinary text remains text.

PDF keeps the subject, outlines, warp and extrusion as vectors, and rasterizes only effect layers at the effective export DPI (default 1200). Tiles overlap for blur/spread before cropping, bypassing the dependency's 5000-pixel filter cap. The complete effect budget is 100 million pixels, with a 64 MB per-tile RGBA buffer limit; excessive requests fail explicitly. Very large blur radii may require a lower DPI. Imported SVG remains subject to its existing safe subset; the internal filter renderer does not enable arbitrary filters in imported files.

[Artistic text source](../../examples/effects/art-text.lay) · [Effects source](../../examples/effects/shadow-glow.lay) · [Full API](interface-reference.en.md)

`laymesh inspect --json` includes a `decorations` list with layout, subject and effect bounds plus the local-to-page transform. An explicit span color overrides an inherited text fill; an explicit span text_fill takes priority.
