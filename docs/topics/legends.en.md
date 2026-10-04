# Legends and shared colorbars

<!-- walkthrough:start -->
## Purpose and concepts

Legends explain discrete series; colorbars explain numeric color mappings. Layer label supplies legend entries. color_scale shares one numeric rule among scatter, heatmaps, contours and standalone colorbars.

## Minimal complete example

Run this file directly with `laymesh validate` or `laymesh render`; it contains its own canvas and required definitions.

<!-- example:examples/manual/color_scale.lay -->

## Parameters and default behavior

Unitless geometry uses the canvas unit; unitless type and stroke sizes use pt. Explicit call parameters override inherited/theme defaults. The linked interface reference lists accepted types, choices and defaults per parameter.

## Composition

p.legend/p.colorbar decorate a chart. legend(layers=...) and colorbar(scale=...) are independently placed material. Shared mapping requires an explicit domain and the same configuration object, not merely the same palette name.

<!-- example:examples/plot/shared-colors.lay -->

## Common errors and limits

color_scale cannot be combined with a layer’s private cmap/vmin/vmax. Continuous scales interpolate RGB and alpha. Standalone legend layers must belong to valid charts in the current work.

## Individual functions

### color_scale

Define a shared numeric color mapping. Continuous norms use explicit bounds; boundary uses interval edges. Layers and standalone colorbars reference the same configuration.

Returns: color_scale

[Minimal complete source](../../examples/manual/color_scale.lay) · [Composition source](../../examples/plot/shared-colors.lay) · [All parameters](interface-reference.en.md#color_scale)

### legend

Create independently placed legend material from existing layers. label supplies text and layer style supplies samples; add positions it on a page or group.

Returns: material

Required inputs: `layers`.

[Minimal complete source](../../examples/manual/legend.lay) · [Composition source](../../examples/plot/complete-data.lay) · [All parameters](interface-reference.en.md#legend)

### colorbar

Create standalone colorbar material from color_scale. length/thickness are physical and ticks share the numeric mapping; use add for placement.

Returns: material

Required inputs: `scale`.

[Minimal complete source](../../examples/manual/colorbar.lay) · [Composition source](../../examples/plot/shared-colors.lay) · [All parameters](interface-reference.en.md#colorbar)

### plot-legend

Add legend decoration to an unplaced chart, using labeled layers by default. position uses chart-local coordinates; legend(layers=...) creates standalone material.

Returns: decoration

[Minimal complete source](../../examples/manual/plot-legend.lay) · [Composition source](../../examples/plot/first-plot.lay) · [All parameters](interface-reference.en.md#plot-legend)

### plot-colorbar

Add an internal colorbar for a chart layer. A layer reference determines mapping; position/length are chart-local. Use colorbar(scale=...) for a shared standalone bar.

Returns: decoration

[Minimal complete source](../../examples/manual/plot-colorbar.lay) · [Composition source](../../examples/plot/colorbars.lay) · [All parameters](interface-reference.en.md#plot-colorbar)

<!-- walkthrough:end -->

## Detailed behavior and further examples

### Parameters

| Parameter | Purpose | Default or requirement |
| --- | --- | --- |
| `color_scale` | Shared color mapping | Explicitly reuse the same object |
| `orientation` | Colorbar direction | vertical / horizontal |
| `length` | Physical colorbar length | Positive length |

### Common usage

Continuous norms linear/log/symlog/centered require explicit finite increasing vmin/vmax. Log requires positive bounds and valid data. Symlog accepts positive constant; centered requires center strictly inside the range. Discrete scales use `norm="boundary",boundaries=[...]` instead of vmin/vmax. An internal boundary belongs to the interval on its right; the maximum belongs to the last interval.

cmap accepts the seven existing palettes or at least two #RGB/#RRGGBB colors. Continuous sequences interpolate linearly in RGB; discrete sequences select interval colors. Missing data is transparent. Outside values clamp by default, with optional under/over colors. Heatmaps, points, contours and colorbars reuse the exact mapping. Legacy heatmap cmap/vmin/vmax retain a private linear mapping; combining them with color_scale errors.

Scatter c and marker_fill are mutually exclusive. c must match x/y length. marker_size accepts a matching list of physical lengths, opacity a matching array in 0–1. Each stroke stays within its marker's final outer box. Legends use the layer's scalar default sample for per-point size/opacity; a colorbar explains numerical point colors.

Heatmap x_edges/y_edges are increasing cell boundaries, with columns+1/rows+1 entries. They conflict with extent; an unspecified dimension retains default uniform boundaries. Nonlinear axes, broken axes and nonuniform cells use vector cell geometry for accurate mapping. Original uniform linear heatmaps retain their default raster path.

contour/contourf require at least a 2×2 rectangular matrix and explicit increasing levels. x/y specify grid sample coordinates; defaults are `0..columns-1`/`0..rows-1`, or sample endpoints from extent. x/y and extent are mutually exclusive. [D3 contour](https://d3js.org/d3-contour/contour) coordinates `(i+0.5,j+0.5)` are corrected to actual samples, including nonuniform spacing. Cells adjacent to missing samples remain empty; scattered points are never interpolated automatically.

Independent decorations are reusable materials, placed through page/group.add with anchors, physical offsets, rotation and scaling. They never reserve plot margins or move panels. Legends preserve the explicit layer order without merging duplicate labels; empty labels and heatmaps are omitted. Options include rich title, columns, font_size, background, frame, sample_width, sample_gap, gap and padding. Optional style accepts plot_style; legends otherwise inherit the first layer's plot style.

A standalone colorbar requires scale and physical length; defaults are vertical and thickness=3 mm. It accepts ticks/format/notation/exponent/exponent_offset/label/label_offset/style. Its measured material bounds include all text, so add's top_left refers to that full boundary. Plain strings inherit styles; explicitly styled text/formula assets retain their formatting.

Local p.legend uses the same options. p.colorbar(layer,...) accepts a same-plot heatmap or any layer with a color scale; still at most one per plot. Existing side/manual positioning remains, with independent label_offset added. Standalone bars need no plot. Values increase left-to-right horizontally and bottom-to-top vertically, using the same normalization as their layers.

### Limits and related topics

[Data input and missing values](data.en.md) · [Plot area and physical size](plot-area.en.md) · [Axes and ticks](axes.en.md) · [Labels and scientific notation](labels.en.md) · [Data anchors and annotations](annotations.en.md)
