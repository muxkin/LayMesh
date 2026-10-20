# CSS gradients and layered backgrounds

[Comparison gallery: original recipes, actual exports and reference differences](../benchmarks/gradients-2026-10-19/gallery.html)

[Paint constructors](fills.en.md) · [Open Props](style-open-props.en.md) · [Property reference](style-properties.en.md)

## Geometry and stops

`gradient(css="...")` and CSS paints accept `linear-gradient`, `radial-gradient`, `conic-gradient` and all three `repeating-*` forms. Geometry follows [CSS Images](https://www.w3.org/TR/css-images-4/#gradients). Linear directions include angles and `to top right`; radial shapes accept circles, ellipses, explicit dimensions and closest/farthest side/corner sizes. Centers can lie outside the object. Conic zero points upward and angles increase clockwise.

Linear/radial stops accept physical lengths and percentages. Conic stops accept percentages and `deg`, `rad`, `grad`, `turn`, including mixed `calc()` expressions. A color can have two positions; coincident positions preserve hard edges. Omitted positions are distributed after stop fixup. Midpoint hints use the standard power curve. Negative and greater-than-100% positions are retained until the final geometry is known. Repetition keeps the first-to-last stop interval and phase; zero-length repetitions use the specified average-color rule.

Lengths use `layout_dpi`, current/root font size and canvas dimensions. Export DPI never changes layout. `linear_gradient` and `radial_gradient` keep their existing normalized-coordinate API. `conic_gradient(stops=...,center=(0.5,0.5),angle=0deg)` adds the same reusable paint interface; use `gradient(css=...)` for the complete stop grammar.

## Color interpolation

The `in <space>` clause supports `srgb`, `srgb-linear`, `display-p3`, `display-p3-linear`, `a98-rgb`, `prophoto-rgb`, `rec2020`, `xyz`/`xyz-d65`, `xyz-d50`, `lab`, `lch`, `oklab`, `oklch`, `hsl` and `hwb`. Polar spaces accept `shorter`, `longer`, `increasing` and `decreasing hue`. Missing components and premultiplied alpha follow [CSS Color 4](https://www.w3.org/TR/css-color-4/#interpolation).

Source channels stay floating point through interpolation. Gamut mapping occurs afterwards, into the sRGB output surface. Legacy RGB/named/HSL/HWB colors default to sRGB interpolation; modern colors default to OKLab. Rust resolves colors and adaptively samples complex interpolation, preserving duplicate stops at hard edges. Native Skia and CanvasKit consume that same result. Custom color profiles and morphing between gradient paints are outside this interface.

## Layered backgrounds

```lcss
.card {
  background-color: #f8fafc;
  background-image: linear-gradient(90deg,#f008,transparent), url("texture.png");
  background-size: auto, cover;
  background-position: center, right bottom;
  background-repeat: no-repeat;
}
```

The first image is on top; the color is below all images. Image, position, size and repeat lists match by index, repeating shorter lists and ignoring unused values. A `none` image still occupies a layer. `background` resets its components and permits the color only in the final layer. Invalid declarations do not erase earlier valid declarations.

`background-size` supports one/two length-percentages, `auto`, `cover` and `contain`. `background-position` supports percentages, lengths, axis keywords and edge offsets. `background-repeat` supports `repeat`, `no-repeat`, `repeat-x`, `repeat-y`, `space`, `round` and separate axis values. The positioning area remains the LayMesh object's layout box; its path and rounded corners clip the layers. Object opacity, border and shadow apply once. Browser scrolling, attachment, padding/content-box positioning and clipping models are not introduced.

<!-- example:examples/manual/gradient-composition.lay -->

## Preview and export

Native previews, VS Code, the website and offline HTML use Skia gradient shaders. Display-list version 2 introduced conic paints. The current version is 3 (object effects); readers still accept versions 1 and 2, and changing versions replaces the incremental baseline.

SVG/PDF/PPTX preserve representable vectors. Unsupported fills are rasterized locally at the selected export DPI; ordinary text and outlines retain their existing output paths. Normal fallback is silent. Raster regions are tiled under the existing memory budget; impossible dimensions and failed output remain errors. Technical raster details are kept in export jobs and PPTX object names rather than per-object warnings.

When a target cannot represent isolated opacity on overlapping fill and stroke, that object's composition is rasterized together to apply opacity once. Other objects remain independent. The offline playback PDF path checks the error of its PDF library's sampled shading functions and uses local fill fallback when they cannot preserve the resolved colors.

The verification harness is `scripts/test-gradient-parity.cjs`. It preserves original library CSS and records browser version, sRGB conditions, engine/library hashes, editable inputs and actual outputs. Browser approximations can differ from the standard formulas: Chromium's midpoint hints use nine inserted stops. Such differences must be reported separately from a passing pixel comparison.
