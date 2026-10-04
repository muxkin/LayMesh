# Data anchors and annotations

<!-- walkthrough:start -->
## Purpose and concepts

Data anchors map original values into instance coordinates. chart.data(x=...,y=...) positions labels and connectors through axis transforms, instance rotation and group resizing. hline/vline draw references inside the data region.

## Minimal complete example

Run this file directly with `laymesh validate` or `laymesh render`; it contains its own canvas and required definitions.

<!-- example:examples/manual/instance-data.lay -->

## Parameters and default behavior

Unitless geometry uses the canvas unit; unitless type and stroke sizes use pt. Explicit call parameters override inherited/theme defaults. The linked interface reference lists accepted types, choices and defaults per parameter.

## Composition

Align a label’s bottom_center with a data point, then add a physical offset. Named-axis anchors accept x_axis/y_axis, keeping secondary-axis curves and annotations consistent.

<!-- example:examples/plot/annotations.lay -->

## Common errors and limits

The definition p is not a placed chart: use the return from chart=page.add(p). Data anchors in hidden axis intervals error; outside values are not silently clamped to the edge.

## Individual functions

### plot-hline

Horizontal reference line layer: add data to this plot, control its appearance with physical style parameters, and select data mappings with named-axis parameters.

Returns: A layer handle that can be included in a shared legend.

[Minimal complete source](../../examples/manual/plot-hline.lay) · [Composition source](../../examples/plot/annotations.lay) · [All parameters](interface-reference.en.md#plot-hline)

### plot-vline

Vertical reference line layer: add data to this plot, control its appearance with physical style parameters, and select data mappings with named-axis parameters.

Returns: A layer handle that can be included in a shared legend.

[Minimal complete source](../../examples/manual/plot-vline.lay) · [Composition source](../../examples/manual/reference-lines-composition.lay) · [All parameters](interface-reference.en.md#plot-vline)

<!-- walkthrough:end -->

## Detailed behavior and further examples

### Parameters

| Parameter | Purpose | Default or requirement |
| --- | --- | --- |
| `data(x=..., y=...)` | Position in data coordinates | Use original data units |
| `plot_*` | Nine plot-area anchors | Follows instance transforms |

### Common usage

Use `plot_area=box(offset=(left,top), size=(width,height))` to lock the data rectangle relative to the outer plot frame. All four values are physical lengths: left/top must be nonnegative, width/height positive, and boundaries finite. Extending beyond `size` produces a warning and retains the rectangle. This option is mutually exclusive with `margins`. Placement width/height overrides change the surrounding space while preserving the data rectangle. Changes to fonts, ticks, labels, legends or colorbars also preserve it; insufficient space produces a located `W_PLOT_LAYOUT` warning and output continues. In this mode axis and colorbar labels stay next to their respective axes when the outer frame grows.

- `chart.data(x=...,y=...)` returns an anchor on a placed plot instance. Values must be finite, unitless numbers within the resolved domains, including endpoints; log coordinates must be positive. It uses that instance's final layout and does not change the domains. Recompiling after an axis-range change keeps the annotation at the same data value. Out-of-range anchors raise an error.
- Annotation `offset` values are physical displacements in the shared container. Text and arrows remain independent elements: they may extend outside the data clip, have no automatic collision avoidance, and do not expand the plot's outer frame.
- Nine data-rectangle anchors are available: `plot_top_left/plot_top_center/plot_top_right`, `plot_middle_left/plot_center/plot_middle_right`, and `plot_bottom_left/plot_bottom_center/plot_bottom_right`. Use `target=chart.plot_top_left` or the quoted placement option `anchor="plot_top_left"` to position the data rectangle directly on the page.
- Lines and arrows support `anchor="start"/"end"` as well as `pointer.start/end` targets. Plot and endpoint anchors rotate with their instances; the original nine outer anchors still use the rotated axis-aligned bounding box. New placement anchor names require quotes and add no reserved words.
- An annotation and its target must belong to the same canvas or group. Place both inside a group to scale or rotate the whole annotated figure together; the usual group rules scale physical dimensions too. Each plot remains independently positionable and editable.

Data coordinates, data-rectangle anchors and physical offsets can be combined directly. `anchor` selects the placed element's own alignment point; `target` supplies its destination. The example offsets follow page coordinates, rather than the rotated chart's local coordinates.

[Editable source](../../examples/plot/annotations.lay) · [SVG](../../examples/plot/annotations.svg) · [PDF](../../examples/plot/annotations.pdf). The example fixes an **88 × 55 mm** data area with its top-left at **(26, 20) mm** on the page. The peak arrow, threshold label and title are separate placements.

### Limits and related topics

[Data input and missing values](data.en.md) · [Plot area and physical size](plot-area.en.md) · [Axes and ticks](axes.en.md) · [Labels and scientific notation](labels.en.md) · [Legends and shared colorbars](legends.en.md)
