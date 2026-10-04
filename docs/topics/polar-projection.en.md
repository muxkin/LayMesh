# Projection and radial axes

<!-- walkthrough:start -->
## Purpose and concepts

polar uses theta/r. Its default zero points right and positive direction is counterclockwise, unlike page line angles with downward y. angle_unit determines data-angle units.

## Minimal complete example

Run this file directly with `laymesh validate` or `laymesh render`; it contains its own canvas and required definitions.

<!-- example:examples/manual/plot.lay -->

## Parameters and default behavior

Unitless geometry uses the canvas unit; unitless type and stroke sizes use pt. Explicit call parameters override inherited/theme defaults. The linked interface reference lists accepted types, choices and defaults per parameter.

## Composition

theta_zero, theta_direction and inner_radius create gauges/rings. wrap=shortest crosses periods along the shortest route, while raw keeps angular differences. interpolation=polar/chord connects in data/page coordinates.

<!-- example:examples/plot/polar-directions.lay -->

## Common errors and limits

Negative radii add half a turn and use their magnitude with a warning; logarithmic radial axes require positive values. plot_* still denotes the projection rectangle; query an axis spine for its actual curve.

## Individual functions

<!-- walkthrough:end -->

## Detailed behavior and further examples

### Parameters

| Parameter | Purpose | Default or requirement |
| --- | --- | --- |
| `plot_area` | Projection rectangle | Center at rectangle center |
| `theta / r` | Angular and radial axes | Original data units |

### Common usage

[Editable source](../../examples/plot/polar-directions.lay). The right panel uses a north zero and clockwise angles. The hollow point in the left panel uses a negative radius, producing an expected, suppressible warning.

| Option | Default and behavior |
| --- | --- |
| `projection` | `cartesian`; new values `polar` and `radar` |
| `angle_unit` | `deg` or `rad`, default deg; applies to scalar data, ranges, ticks, angular errors and bar widths |
| `theta_zero` | `0 deg`, measured counterclockwise from the right; `90 deg` points up |
| `theta_direction` | `ccw` or `cw`, default ccw |
| `theta=axis(...)` | Linear angular axis; full circle by default. Increasing ranges span at most one turn; use `(300,420)` for a sector across zero |
| `r=axis(...)` | linear/log/symlog, reversal, rich labels, notation and physical offsets; nonnegative range, strictly positive for log |
| `r_label_angle` | `22.5 deg`, following the chosen zero and direction |
| `inner_radius` | `0 mm`; physical hole smaller than the outer radius; radial range maps between these radii |
| `wrap` | `shortest` connects 350→10 across zero; half-turn ties retain the original sign. `raw` preserves angular differences and multiple turns |
| `interpolation` | `polar` interpolates theta/r before projection; `chord` connects projected endpoints directly |
| `closed` | false for ordinary curves; true explicitly joins last and first |

Angular axis ranges/ticks and layer data are scalar numbers interpreted by `angle_unit`. The orientation options use the existing degree syntax. Degree tick labels include °; radian ticks show numbers, with `tick_text` available for custom π formulas. Automatic linear radial ranges start at zero; logarithmic ranges use positive magnitudes.

`a.data(theta=...,r=...)` accepts original values and rejects anchors outside the radial range or sector. `a.plot_*` still refers to the specified rectangle. `a.axis(name="theta|r",anchor="start|center|end")` references the outer angular arc or radial ray; center means the physical midpoint along that arc/ray. Instance rotation and group scaling are applied afterwards.

Negative radii project as `(theta + half turn, abs(r))` and produce `W_POLAR_NEGATIVE_RADIUS`. Inputs are not modified. Continuous paths split at raw radius zero before projection. Error caps occur only at visible original endpoints. Actual zero radii are invalid on a logarithmic scale; synthetic zero baselines and crossing error intervals are clipped before logarithms. Curve tessellation is bounded by 0.001 mm locally; anchors use analytical coordinates.

### Limits and related topics

[Polar layers](polar-layers.en.md) · [Radar charts](radar.en.md)
