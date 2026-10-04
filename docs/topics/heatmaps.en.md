# Heatmaps and contours

<!-- walkthrough:start -->
## Purpose and concepts

heatmap represents cell values, contour represents sampled-field isolines and contourf fills level intervals. All require rectangular matrices, but cell edges differ from sample coordinates.

## Minimal complete example

Run this file directly with `laymesh validate` or `laymesh render`; it contains its own canvas and required definitions.

<!-- example:examples/manual/plot-heatmap.lay -->

## Parameters and default behavior

Unitless geometry uses the canvas unit; unitless type and stroke sizes use pt. Explicit call parameters override inherited/theme defaults. The linked interface reference lists accepted types, choices and defaults per parameter.

## Composition

Share color_scale across heatmaps and contours. x_edges/y_edges are heatmap boundaries; contour x/y are sample coordinates. Nonuniform or broken axes use vector cells for accurate mapping.

<!-- example:examples/plot/shared-colors.lay -->

## Common errors and limits

levels must increase and contours need at least 2×2 samples. Missing regions are not interpolated and scattered observations do not become a grid automatically. Coordinates and extent are exclusive; ragged rows error.

## Individual functions

### plot-heatmap

Heatmap layer: add data to this plot, control its appearance with physical style parameters, and select data mappings with named-axis parameters.

Returns: A layer handle that can be included in a shared legend.

[Minimal complete source](../../examples/manual/plot-heatmap.lay) · [Composition source](../../examples/plot/shared-colors.lay) · [All parameters](interface-reference.en.md#plot-heatmap)

### plot-contour

Contours layer: add data to this plot, control its appearance with physical style parameters, and select data mappings with named-axis parameters.

Returns: A layer handle that can be included in a shared legend.

[Minimal complete source](../../examples/manual/plot-contour.lay) · [Composition source](../../examples/plot/shared-colors.lay) · [All parameters](interface-reference.en.md#plot-contour)

### plot-contourf

Filled contours layer: add data to this plot, control its appearance with physical style parameters, and select data mappings with named-axis parameters.

Returns: A layer handle that can be included in a shared legend.

[Minimal complete source](../../examples/manual/plot-contourf.lay) · [Composition source](../../examples/plot/shared-colors.lay) · [All parameters](interface-reference.en.md#plot-contourf)

<!-- walkthrough:end -->

## Detailed behavior and further examples

### Parameters

| Parameter | Purpose | Default or requirement |
| --- | --- | --- |
| `z` | 2D data matrix | Equal-length rows |
| `x_edges / y_edges` | Cell edges | Strictly increasing; exclusive with extent |
| `levels` | Contour levels | Strictly increasing |

### Common usage

Continuous norms linear/log/symlog/centered require explicit finite increasing vmin/vmax. Log requires positive bounds and valid data. Symlog accepts positive constant; centered requires center strictly inside the range. Discrete scales use `norm="boundary",boundaries=[...]` instead of vmin/vmax. An internal boundary belongs to the interval on its right; the maximum belongs to the last interval.

cmap accepts all [Matplotlib 3.11.2 presets](cmaps.en.md), an immutable cmap(...) handle, or at least two custom colors. Continuous sequences interpolate linearly in RGB; discrete sequences select interval colors. Missing data is transparent. Outside values clamp by default, with optional under/over colors. Heatmaps, points, contours and colorbars reuse the exact mapping. Legacy heatmap cmap/vmin/vmax retain a private linear mapping; combining them with color_scale errors.

Scatter c and marker_fill are mutually exclusive. c must match x/y length. marker_size accepts a matching list of physical lengths, opacity a matching array in 0–1. Each stroke stays within its marker's final outer box. Legends use the layer's scalar default sample for per-point size/opacity; a colorbar explains numerical point colors.

Heatmap x_edges/y_edges are increasing cell boundaries, with columns+1/rows+1 entries. They conflict with extent; an unspecified dimension retains default uniform boundaries. Nonlinear axes, broken axes and nonuniform cells use vector cell geometry for accurate mapping. Original uniform linear heatmaps retain their default raster path.

contour/contourf require at least a 2×2 rectangular matrix and explicit increasing levels. x/y specify grid sample coordinates; defaults are `0..columns-1`/`0..rows-1`, or sample endpoints from extent. x/y and extent are mutually exclusive. [D3 contour](https://d3js.org/d3-contour/contour) coordinates `(i+0.5,j+0.5)` are corrected to actual samples, including nonuniform spacing. Cells adjacent to missing samples remain empty; scattered points are never interpolated automatically.

Filled levels define interval lower bounds: regions below the first are empty, the highest extends upward. Subtracting adjacent nested superlevel sets produces disjoint bands, preserving translucent opacity. Without a scale, contourf creates a private viridis mapping from finite data, while contour uses the layer's single color.

### Limits and related topics

[Lines and scatter](line-scatter.en.md) · [Error bars and bands](uncertainty.en.md) · [Bars and steps](bars.en.md) · [Multiple axes and breaks](multi-axes.en.md)
