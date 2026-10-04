# Lines and scatter

<!-- walkthrough:start -->
## Purpose and concepts

line connects valid points; scatter draws markers only. Both retain input order. line’s marker can expose samples, and physical style sizes are independent of data domains.

## Minimal complete example

Run this file directly with `laymesh validate` or `laymesh render`; it contains its own canvas and required definitions.

<!-- example:examples/manual/plot-line.lay -->

## Parameters and default behavior

Unitless geometry uses the canvas unit; unitless type and stroke sizes use pt. Explicit call parameters override inherited/theme defaults. The linked interface reference lists accepted types, choices and defaults per parameter.

## Composition

Connect x/y with a light line and emphasize points with markers. scatter’s c plus color_scale shows a third variable; marker_size and opacity can be arrays matching data length.

<!-- example:examples/plot/line-scatter.lay -->

## Common errors and limits

x is not automatically sorted. c and marker_fill are exclusive; per-point arrays must match. Missing samples break lines or skip markers. Supported markers are circle/square/triangle/triangle_down/diamond/none.

## Individual functions

### plot-line

Add a line layer connecting data points in order, optionally with markers. Choose named axes with x_axis and y_axis; polar plots use theta and r, and radar plots use values.

Returns: A layer handle that can be included in a shared legend.

[Minimal complete source](../../examples/manual/plot-line.lay) · [Composition source](../../examples/plot/line-scatter.lay) · [All parameters](interface-reference.en.md#plot-line)

### plot-scatter

Add a scatter layer. Set marker shape, physical size and border, or map numeric c values to colors through color_scale.

Returns: A layer handle that can be included in a shared legend.

[Minimal complete source](../../examples/manual/plot-scatter.lay) · [Composition source](../../examples/plot/line-scatter.lay) · [All parameters](interface-reference.en.md#plot-scatter)

<!-- walkthrough:end -->

## Detailed behavior and further examples

### Parameters

| Parameter | Purpose | Default or requirement |
| --- | --- | --- |
| `x / y` | Equal-length data sequences | Required |
| `marker_size` | Physical marker size | Example: 2.4 mm |
| `label` | Legend text | Optional |

### Common usage

line connects data in input order without sorting; scatter draws individual observations. Overlay them using the same axes and color. Lines break at missing values and scatter skips them.

[Complete parameters and rules](plot-reference.en.md)

### Limits and related topics

[Error bars and bands](uncertainty.en.md) · [Bars and steps](bars.en.md) · [Heatmaps and contours](heatmaps.en.md) · [Multiple axes and breaks](multi-axes.en.md)
