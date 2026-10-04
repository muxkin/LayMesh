# Axes and ticks

<!-- walkthrough:start -->
## Purpose and concepts

axis configures domain, mapping, ticks and labels. The default primary axes are x/y. Explicit range makes charts comparable. linear, log and symlog are data transforms, not page rotations.

## Minimal complete example

Run this file directly with `laymesh validate` or `laymesh render`; it contains its own canvas and required definitions.

<!-- example:examples/manual/axis.lay -->

## Parameters and default behavior

Unitless geometry uses the canvas unit; unitless type and stroke sizes use pt. Explicit call parameters override inherited/theme defaults. The linked interface reference lists accepted types, choices and defaults per parameter.

## Composition

Pair ticks with tick_text for category strings or formulas. minor_ticks and grid control subdivisions and background grids. Select chart.axes["x"].spine.path for the spine, or component bounds for title alignment.

<!-- example:examples/plot/multi-axes-breaks.lay -->

## Common errors and limits

log requires positive bounds and valid data. Explicit tick_text must match ticks. axis.min/max mean numerical domain endpoints, including on reversed axes.

## Individual functions

### axis

Configure data domain, linear/log/symlog mapping, ticks, labels and breaks. Domains use original values; text/stroke dimensions are physical.

Returns: axis

[Minimal complete source](../../examples/manual/axis.lay) · [Composition source](../../examples/plot/multi-axes-breaks.lay) · [All parameters](interface-reference.en.md#axis)

<!-- walkthrough:end -->

## Detailed behavior and further examples

### Parameters

| Parameter | Purpose | Default or requirement |
| --- | --- | --- |
| `range` | Data range | May be explicit |
| `minor_ticks` | Minor ticks | []; explicitly enable auto |
| `reverse` | Reverse display | false |

### Common usage

The primary names are x/y. Added names must be nonempty and unique. `side="left|right|top|bottom"` defaults to right. `offset=0 mm` is measured outward from the corresponding data edge; negative values are allowed. Axes on the same side do not push one another apart. Every layer accepts `x_axis/y_axis` (defaults x/y), rejecting missing names and wrong directions. Each automatic domain uses only its own bound data; an unused axis needs an explicit range.

`chart.data(...)` uses original values with the selected axes' transforms, reversal and segments. `chart.axis(name=...,anchor="start|center|end")` uses the domain minimum/maximum for start/end even on a reversed axis. Center is the physical midpoint, possibly within a gap. Both kinds of anchors follow instance rotation and group scaling. Placement offsets use the surrounding page or group's coordinate system.

| Axis option | Behavior |
| --- | --- |
| `reverse=false` | Reverse display, preserving input data and statistics |
| `scale="symlog", constant=1` | Transform `sign(x)*log(1+abs(x)/constant)`; positive constant in original data units, allowing zero and negatives |
| `tick_text=[...]` | Strings, text or formula definitions; requires equally long explicit ticks, in input order |
| `tick_rotation=0 deg` | Rotate around each label center; placement measures rotated bounds |
| `tick_offset=(0 mm,0 mm)` | Local physical offset, right/down positive |
| `tick_font_size=...`, `tick_color="#..."` | Independent tick and multiplier styles; tick marks also use tick_color. Explicit text/formula asset styles remain intact |
| `line_color="#..."`, `line_width=...` | Axis line/default text color and physical stroke width; tick_color can override tick color |
| `spine=false` | Hide the spine and its break marks while retaining ticks and labels |
| `minor_ticks="auto"` | Opt in; subdivide numeric intervals into fifths for linear/symlog axes, or decimal 2–9 ticks for log axes. Original default remains an empty list |
| `grid="none|major|both"` | Per-axis grids behind the data; added axes default to none |

Existing label_offset/exponent_offset remain independent. Categories require explicit numeric positions and tick_text; neither category order nor axis selection is inferred.

### Limits and related topics

[Data input and missing values](data.en.md) · [Plot area and physical size](plot-area.en.md) · [Labels and scientific notation](labels.en.md) · [Legends and shared colorbars](legends.en.md) · [Data anchors and annotations](annotations.en.md)
