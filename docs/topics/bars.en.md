# Bars and steps

<!-- walkthrough:start -->
## Purpose and concepts

bar positions, values, baseline and data_width use data units. step uses pre/mid/post to locate transitions; area fills between a curve and baseline.

## Minimal complete example

Run this file directly with `laymesh validate` or `laymesh render`; it contains its own canvas and required definitions.

<!-- example:examples/manual/plot-bar.lay -->

## Parameters and default behavior

Unitless geometry uses the canvas unit; unitless type and stroke sizes use pt. Explicit call parameters override inherited/theme defaults. The linked interface reference lists accepted types, choices and defaults per parameter.

## Composition

Use explicit positions for grouped bars and per-point baseline for stacked bars. Combine a zero hline, a step curve and translucent area to compare discrete/continuous representations.

<!-- example:examples/plot/statistics.lay -->

## Common errors and limits

Bar width is not mm and differs from physical stroke width. Category positions, sorting and stacking are not inferred. Negative values can fall below baseline; log axes still require positive values.

## Individual functions

### plot-bar

Bar plot layer: add data to this plot, control its appearance with physical style parameters, and select data mappings with named-axis parameters.

Returns: A layer handle that can be included in a shared legend.

[Minimal complete source](../../examples/manual/plot-bar.lay) · [Composition source](../../examples/plot/statistics.lay) · [All parameters](interface-reference.en.md#plot-bar)

### plot-step

Step plot layer: add data to this plot, control its appearance with physical style parameters, and select data mappings with named-axis parameters.

Returns: A layer handle that can be included in a shared legend.

[Minimal complete source](../../examples/manual/plot-step.lay) · [Composition source](../../examples/plot/statistics.lay) · [All parameters](interface-reference.en.md#plot-step)

### plot-area

Area plot layer: add data to this plot, control its appearance with physical style parameters, and select data mappings with named-axis parameters.

Returns: A layer handle that can be included in a shared legend.

[Minimal complete source](../../examples/manual/plot-area.lay) · [Composition source](../../examples/plot/polar-data.lay) · [All parameters](interface-reference.en.md#plot-area)

<!-- walkthrough:end -->

## Detailed behavior and further examples

### Parameters

| Parameter | Purpose | Default or requirement |
| --- | --- | --- |
| `data_width` | Bar width in data units | 0.8 |
| `baseline` | Bar baseline | 0 |
| `where` | Step placement | post |

### Common usage

Bar positions, width and baseline use data units. Group bars with explicit positions and stack them using baseline. step uses pre/mid/post placement and preserves input order.

[Complete parameters and rules](../cartesian-plots.en.md)

### Limits and related topics

[Lines and scatter](line-scatter.en.md) · [Error bars and bands](uncertainty.en.md) · [Heatmaps and contours](heatmaps.en.md) · [Multiple axes and breaks](multi-axes.en.md)
