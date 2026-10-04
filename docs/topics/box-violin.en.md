# Boxplots and violin plots

<!-- walkthrough:start -->
## Purpose and concepts

boxplot summarizes quantiles, whiskers and outliers; violin shows a kernel density estimate. Both accept observations rather than an already summarized five-number result.

## Minimal complete example

Run this file directly with `laymesh validate` or `laymesh render`; it contains its own canvas and required definitions.

<!-- example:examples/manual/plot-boxplot.lay -->

## Parameters and default behavior

Unitless geometry uses the canvas unit; unitless type and stroke sizes use pt. Explicit call parameters override inherited/theme defaults. The linked interface reference lists accepted types, choices and defaults per parameter.

## Composition

Place boxplot and violin for one sample at neighboring positions with explicit data_width. R-7 quantiles, default 1.5 IQR whiskers and Scott bandwidth specify the statistical convention.

<!-- example:examples/plot/statistics.lay -->

## Common errors and limits

Single/constant samples cannot form an ordinary violin density and show a median with a warning. points controls estimate sampling, not the observation count; it does not change data.

## Individual functions

### plot-boxplot

Box plot layer: add data to this plot, control its appearance with physical style parameters, and select data mappings with named-axis parameters.

Returns: A layer handle that can be included in a shared legend.

[Minimal complete source](../../examples/manual/plot-boxplot.lay) · [Composition source](../../examples/plot/statistics.lay) · [All parameters](interface-reference.en.md#plot-boxplot)

### plot-violin

Violin plot layer: add data to this plot, control its appearance with physical style parameters, and select data mappings with named-axis parameters.

Returns: A layer handle that can be included in a shared legend.

[Minimal complete source](../../examples/manual/plot-violin.lay) · [Composition source](../../examples/plot/statistics.lay) · [All parameters](interface-reference.en.md#plot-violin)

<!-- walkthrough:end -->

## Detailed behavior and further examples

### Parameters

| Parameter | Purpose | Default or requirement |
| --- | --- | --- |
| `whisker` | Boxplot whisker IQR multiplier | 1.5 |
| `bandwidth` | KDE bandwidth | scott |
| `points` | Violin sample count | 128 |

### Common usage

Boxplots use R-7 quantiles and default whiskers reaching the most distant observation within 1.5 IQR. Violins use a Gaussian kernel and Scott bandwidth; constant or single-point samples show only the median with a warning.

[Complete parameters and rules](../cartesian-plots.en.md)

### Limits and related topics

[Histograms and ECDF](hist-ecdf.en.md)
