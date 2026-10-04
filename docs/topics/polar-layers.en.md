# Polar layers

<!-- walkthrough:start -->
## Purpose and concepts

Polar layers reuse line, scatter, errors, bars and field APIs, with x/y interpreted as theta/r. Error and bar widths use angular/radial data units; stroke widths remain physical.

## Minimal complete example

Run this file directly with `laymesh validate` or `laymesh render`; it contains its own canvas and required definitions.

<!-- example:examples/manual/plot-line.lay -->

## Parameters and default behavior

Unitless geometry uses the canvas unit; unitless type and stroke sizes use pt. Explicit call parameters override inherited/theme defaults. The linked interface reference lists accepted types, choices and defaults per parameter.

## Composition

Compare polar interpolation and chord lines on one page, then add angular/radial errors. Sector cells and contours can share a color scale.

<!-- example:examples/plot/polar-errors.lay -->

## Common errors and limits

Projection handles visibility on original geometry; zero crossings and missing points can split paths. Do not use length units in angular data arrays; angle-configuration parameters support angle-unit syntax.

## Individual functions

<!-- walkthrough:end -->

## Detailed behavior and further examples

### Parameters

| Parameter | Purpose | Default or requirement |
| --- | --- | --- |
| `theta / r` | Angular and radial data | Matching sequence lengths |
| `plot_area` | Physical plot area | Does not shrink when explicitly fixed |

### Common usage

| Method | Polar parameters |
| --- | --- |
| `line/scatter/step` | `theta/r`; step retains pre/mid/post; scatter supports per-point color, physical size and opacity |
| `errorbar` | `theta/r` with `thetaerr/rerr`; scalar, per-point or asymmetric two-row errors; angular errors are arcs |
| `area/band` | `theta/r/baseline` or `theta/lower/upper`; baselines can be arrays |
| `bar` | `positions/values/width/baseline`; angle positions/width, signed radial increments; default width 20° or its radian equivalent |
| `hist` | `values/bins/weights/stat`; wrap angles into one turn, default 10 bins over the current angular range |
| `heatmap` | Required `z/theta_edges/r_edges`; rows are radial, columns angular; cells are annular sectors |
| `contour/contourf` | Required `z/theta/r/levels`; increasing, optionally nonuniform grids; compute in original grid units then project |

Histogram probability uses included total weight; density also divides by bin width in the chosen angle unit. Radial height represents the statistic directly, without square-root area normalization. Observations outside a partial sector are excluded with a warning.

Contours use `periodic=true` explicitly for full-circle grids, with increasing theta samples that do not repeat the endpoint. The last sample connects to the first. Default false does not infer periodicity. Cells adjacent to missing samples remain empty. Seam handling does not accumulate fill opacity. Negative radial grids use the same half-turn reflection.

[Field source](../../examples/plot/polar-field.lay) · [Errors and hatching](../../examples/plot/polar-errors.lay) · [Angular histogram and stacking](../../examples/plot/polar-rose.lay). Reuse existing color scales, local/standalone legends and colorbars, and physical hatches. Local colorbars follow the data rectangle sides; overlap warns without moving anything.

Polar plots currently have one angular and one radial axis. Additional axes, breaks, and polar box/violin layers are rejected explicitly.

### Limits and related topics

[Projection and radial axes](polar-projection.en.md) · [Radar charts](radar.en.md)
