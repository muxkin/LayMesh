# Radar charts

<!-- walkthrough:start -->
## Purpose and concepts

radar assigns each category an angle and its own domain, normalizing values into a common physical radius. It compares multi-metric shapes without implying that different units have equal meaning.

## Minimal complete example

Run this file directly with `laymesh validate` or `laymesh render`; it contains its own canvas and required definitions.

<!-- example:examples/manual/plot.lay -->

## Parameters and default behavior

Unitless geometry uses the canvas unit; unitless type and stroke sizes use pt. Explicit call parameters override inherited/theme defaults. The linked interface reference lists accepted types, choices and defaults per parameter.

## Composition

Match categories with ranges and use closed=true line or area to join endpoints. Original-data anchors accept category/value; labels use categories or separate text.

<!-- example:examples/plot/radar.lay -->

## Common errors and limits

Category order is not inferred. Each series must match category count with valid domains and visible values. Continuous polar theta selection is not a category index.

## Individual functions

<!-- walkthrough:end -->

## Detailed behavior and further examples

### Parameters

| Parameter | Purpose | Default or requirement |
| --- | --- | --- |
| `categories` | Categories and order | Explicitly specified |
| `range` | Radial data range | Linear and non-reversed |

### Common usage

At least three unique categories retain input order, equally spaced, starting at the top clockwise by default. `category_labels` accepts matching rich labels. `radar_frame="polygon|circle"` defaults to polygon with aligned grid vertices. `line/scatter/area(values=...)` accept matching arrays; lines and areas close by default and use straight edges.

Without `ranges`, every category shares `r.range`, or one automatic range computed across all series. Explicit `ranges` normalizes each indicator linearly and conflicts with a common `r.range`. In this mode r.ticks specifies common 0–1 positions while displayed values use each indicator's original units. With offset notation and independent ranges, each spoke displays its own multiplier. Negative original values inside their ranges keep their category direction. Out-of-range values error; missing values break lines and suppress the affected series' closed area.

`a.data(category=...,value=...)` uses raw units; `a.axis(name=category,anchor=...)` references a spoke. Radar uses linear, non-reversed scales, with no theta axis, inner hole or breaks.

### Limits and related topics

[Projection and radial axes](polar-projection.en.md) · [Polar layers](polar-layers.en.md)
