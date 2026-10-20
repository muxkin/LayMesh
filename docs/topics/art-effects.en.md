# Artistic text, static effects and object-level 3D

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

Layout/anchor dimensions retain their original meaning. `subjectBounds` records transformed glyph occupancy; decorated overflow triggers `W_EFFECT_OVERFLOW`. Shadows/glow do not reflow the page. Native and WASM previews share effect parameters and 3D algorithms. Decorative glyphs are vector paths, with source text retained as SVG labels and PDF ActualText; ordinary text remains text.

Without 3D or soft edges, PDF keeps the subject, outlines, warp and 2D extrusion as vectors, and rasterizes only effect layers at the effective export DPI (default 1200). Tiles overlap for blur/spread before cropping, bypassing the dependency's 5000-pixel filter cap. The complete effect budget is 100 million pixels, with a 64 MB per-tile RGBA buffer limit; excessive requests fail explicitly. Very large blur radii may require a lower DPI. Imported SVG remains subject to its existing safe subset; the internal filter renderer does not enable arbitrary filters in imported files.

[Artistic text source](../../examples/effects/art-text.lay) · [Effects source](../../examples/effects/shadow-glow.lay) · [Full API](interface-reference.en.md)

`laymesh inspect --json` includes a `decorations` list with layout, subject and effect bounds plus the local-to-page transform. An explicit span color overrides an inherited text fill; an explicit span text_fill takes priority.

Standard CSS `box-shadow`, `text-shadow` and one `filter: drop-shadow(...)` form independent appearance effects. Existing `effects` constructors retain their blur/sigma, target and ordering semantics. See [CSS effects and partial overrides](lcss.en.md#css-effects).

## Complete styled animation projects

[Library cards and artistic poster](style-animation-examples.en.md) combine themes, layout, keyframes and steps, with complete source for the existing complex dashboard.

## reflection

`reflection(distance=1mm,length=0.5,opacity=0.4,fade=1,blur=0mm,target="content")` mirrors the projected subject below its bottom edge. `distance` is the gap; `length` crops the reflection to a fraction of subject height (0–1, without compressing it). Alpha fades from `opacity` to `opacity*(1-fade)`. `blur` is Gaussian sigma. The reflection does not copy shadows, glow or other reflections.

<!-- example:examples/manual/reflection.lay -->

## soft_edge

`soft_edge(radius=1mm,target="content")` feathers the actual visible alpha inward, including glyph holes, transparent images and projected 3D silhouettes. Subject colors stay sharp and its visible extent does not grow. Zero radius is an identity; multiple soft edges use the largest radius. Softening precedes independent reflection/shadow/glow generation regardless of list order. List order still controls stacking within the back/front decoration layers.

<!-- example:examples/manual/soft_edge.lay -->

## three_d

Set `three_d=three_d(...)` on shapes, text, formulas and images, or override it in `add`. Omitting the parameter keeps the existing 2D path. Defaults are 2 mm depth, zero rotation, no bevel, orthographic projection, matte material and white light from the upper left. Front/back/side faces preserve holes and incorporate strokes. Cropped, rounded images use textured plate geometry and retain their alpha.

| Parameter | Meaning and default |
| --- | --- |
| `depth` | Total thickness, `2mm`; zero is allowed |
| `rotation` | `(X,Y,Z)`, `(0deg,0deg,0deg)`; rotate about the visible contour bounds center in X→Y→Z order, then apply existing 2D placement |
| `bevel_top`, `bevel_bottom` | Front/back `(width,height)`, `(0mm,0mm)`; combined heights must not exceed depth |
| `projection`, `distance` | `"orthographic"` or `"perspective"`; camera on +Z, perspective distance `500mm` |
| `material` | `"flat"`, `"matte"`, `"plastic"`, `"metal"`; the latter three use normals and directional lighting |
| `light_direction` | Nonzero vector toward the light, `(-1,-1,2)`; X right, Y down, Z toward the observer |
| `light_color`, `intensity`, `ambient` | `"#ffffff"`, `1`, `0.25`; nonnegative intensity, ambient fraction 0–1 |

<!-- example:examples/manual/three_d.lay -->

Groups may contain independent 3D objects. Whole-group scenes, plot/video extrusion, inter-object depth or cast shadows, refraction and global illumination are outside this interface. Each object has its own depth buffer; separate objects retain page stacking order. Existing vector `text_extrude()` is unchanged. Degenerate contours, intersecting cameras and bevels that collapse edges or change hole topology produce explicit errors; reduce bevel width for narrow strokes.

Rust shares contour processing, triangulation, projection and software rasterization across native/WASM builds; Skia/CanvasKit supply 2D textures and page composition. Layout boxes and anchors stay unchanged. Projected/effect bounds are tracked separately for clipping, overflow and caching. Display-list version 3 still reads versions 1 and 2; deploy the updated runtime with the updated application. Repeat previews reuse geometry and textures at the current display density; zoom rebuilds textures, and individual object edits retain unrelated caches.

SVG/PDF/PPTX rasterize only complex 3D subjects at the effective export DPI. Softened subjects may also be local rasters; separable decorations are exported independently. Other subjects keep their existing vector/editable-text pipeline. PPTX does not store native PowerPoint 3D settings; `.lay` retains the complete parameters. The 100-million-pixel and tiled memory budgets remain enforced: 3D working blocks are limited to 64 MiB and texture caches to 256 MiB. Failures preserve existing files. Technical fallback details never become visible artwork.

During zoom, pan and resize, the web editor transforms the entire committed frame, then replaces it with one complete sharp viewport at the current device pixel ratio. Zoom does not recompile source, and late results for earlier viewports are discarded. 3D curves refine according to projected error, smooth surfaces interpolate normals, and real corners retain hard edges. Static rendering uses 4×4 coverage sampling. Material alpha is independent of geometric coverage; authored soft edges, glow and reflection blur remain effective. Optional v3 geometry recipes allow rebuilding at a higher density; older lists retain their original mesh rendering. Existing offline HTML files require re-exporting or replacement to receive these fixes.

## Effect presets

Import `canvas(stylesheet="@laymesh/effects.css")` and use, for example, `class="fx-bevel-button"`. Thirty presets cover shadows, glow, reflections, soft edges, front/back bevels, four materials and multiple views. Each has editable source. LCSS uses `three-d: three_d(...)` and the new constructors in `effects`; explicit object arguments override stylesheet values.

<!-- style-catalog:effects -->

[30-preset comparison](../../examples/effects/presets.lay) · [Shape/type/formula/image composition](../../examples/effects/object-3d.lay) · [Preset manifest](../../examples/effects/presets.json). Run `python scripts/build-effects-gallery.py --binary target/debug/laymesh` to build a standalone HTML gallery with PNGs and editable source copies.

Python uses the same source: `render_source(source, output="effects.pdf", dpi=300)`. Notebook, CLI and editor parameter validation is shared; there is no separate Python 3D syntax.

Categories follow [Microsoft's shape effects overview](https://support.microsoft.com/en-us/office/graphics-visuals/add-a-fill-or-effect-to-a-shape-or-text-box), without promising pixel-identical PowerPoint rendering. LayMesh also supports softening after 3D projection.

[Reproducible validation record](../benchmarks/object-effects-2026-10-19/README.zh-CN.md).
