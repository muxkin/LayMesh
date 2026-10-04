# Error bars and bands

<!-- walkthrough:start -->
## Purpose and concepts

errorbar draws xerr/yerr at samples; band fills between lower/upper at x positions. Errors use data units, while cap_size and stroke width are physical lengths.

## Minimal complete example

Run this file directly with `laymesh validate` or `laymesh render`; it contains its own canvas and required definitions.

<!-- example:examples/manual/plot-errorbar.lay -->

## Parameters and default behavior

Unitless geometry uses the canvas unit; unitless type and stroke sizes use pt. Explicit call parameters override inherited/theme defaults. The linked interface reference lists accepted types, choices and defaults per parameter.

## Composition

Combine a translucent band, central line and point errors for one experiment. Errors may be symmetric or separately specify lower/upper sides, with corresponding sample positions.

<!-- example:examples/plot/scientific.lay -->

## Common errors and limits

Errors cannot be negative and arrays must match. Broken-axis/projection clipping does not invent error caps. Missing inputs retain diagnostics. Bounds are not error magnitudes.

## Individual functions

### plot-errorbar

Add error bars. xerr and yerr accept symmetric errors or separate lower and upper errors; cap_size sets the physical cap length.

Returns: A layer handle that can be included in a shared legend.

[Minimal complete source](../../examples/manual/plot-errorbar.lay) · [Composition source](../../examples/plot/scientific.lay) · [All parameters](interface-reference.en.md#plot-errorbar)

### plot-band

Filled band layer: add data to this plot, control its appearance with physical style parameters, and select data mappings with named-axis parameters.

Returns: A layer handle that can be included in a shared legend.

[Minimal complete source](../../examples/manual/plot-band.lay) · [Composition source](../../examples/plot/scientific.lay) · [All parameters](interface-reference.en.md#plot-band)

<!-- walkthrough:end -->

## Detailed behavior and further examples

### Parameters

| Parameter | Purpose | Default or requirement |
| --- | --- | --- |
| `yerr / xerr` | Error ranges | Match observations |
| `lower / upper` | Band boundaries | Same length as x |

### Common usage

Error bars express uncertainty per observation; band uses lower/upper bounds for intervals. The left panel combines lines, points, bands, error bars and a reference line; the right panel shows a heatmap.

[Complete parameters and rules](plot-reference.en.md)

### Limits and related topics

[Lines and scatter](line-scatter.en.md) · [Bars and steps](bars.en.md) · [Heatmaps and contours](heatmaps.en.md) · [Multiple axes and breaks](multi-axes.en.md)
