# Histograms and ECDF

<!-- walkthrough:start -->
## Purpose and concepts

hist bins raw observations; ECDF computes cumulative proportions at sorted observations. Statistics use original data, independent of display transforms. stat=count/probability/density determines histogram height.

## Minimal complete example

Run this file directly with `laymesh validate` or `laymesh render`; it contains its own canvas and required definitions.

<!-- example:examples/manual/plot-hist.lay -->

## Parameters and default behavior

Unitless geometry uses the canvas unit; unitless type and stroke sizes use pt. Explicit call parameters override inherited/theme defaults. The linked interface reference lists accepted types, choices and defaults per parameter.

## Composition

Explicit bins support unequal widths; density normalization includes bin width. Bind ECDF to a 0–1 secondary axis to compare it with counts or density.

<!-- example:examples/plot/statistics.lay -->

## Common errors and limits

weights must be nonnegative and match observations. Density height is not a bin probability: multiply by width. See detailed parameters for automatic bins and weighted statistics.

## Individual functions

### plot-hist

Histogram layer: add data to this plot, control its appearance with physical style parameters, and select data mappings with named-axis parameters.

Returns: A layer handle that can be included in a shared legend.

[Minimal complete source](../../examples/manual/plot-hist.lay) · [Composition source](../../examples/plot/statistics.lay) · [All parameters](interface-reference.en.md#plot-hist)

### plot-ecdf

Empirical cumulative distribution layer: add data to this plot, control its appearance with physical style parameters, and select data mappings with named-axis parameters.

Returns: A layer handle that can be included in a shared legend.

[Minimal complete source](../../examples/manual/plot-ecdf.lay) · [Composition source](../../examples/plot/statistics.lay) · [All parameters](interface-reference.en.md#plot-ecdf)

<!-- walkthrough:end -->

## Detailed behavior and further examples

### Parameters

| Parameter | Purpose | Default or requirement |
| --- | --- | --- |
| `bins` | Count or increasing edges | 10 |
| `stat` | Statistic | count |
| `weights` | Nonnegative matching weights | Optional |

### Common usage

Histograms support count, probability and density; bin edges strictly increase and the last bin includes its right edge. ECDF sorts and merges duplicates into a right-continuous curve. Statistics are computed in original data units.

[Complete parameters and rules](../cartesian-plots.en.md)

### Limits and related topics

[Boxplots and violin plots](box-violin.en.md)
