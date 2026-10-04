# Your first data plot

<!-- walkthrough:start -->
## Purpose and concepts

A native chart is vector material, not an imported image. Create plot, add layers and decorations, then place it. Axes map data values into the plot area.

## Minimal complete example

Run this file directly with `laymesh validate` or `laymesh render`; it contains its own canvas and required definitions.

<!-- example:examples/manual/plot.lay -->

## Parameters and default behavior

Unitless geometry uses the canvas unit; unitless type and stroke sizes use pt. Explicit call parameters override inherited/theme defaults. The linked interface reference lists accepted types, choices and defaults per parameter.

## Composition

Replace inline x/y lists with table columns for experimental data. Explicit axis ranges aid comparisons; plot_area keeps equal physical data regions.

<!-- example:examples/plot/first-plot.lay -->

## Common errors and limits

A chart is sealed after its first placement. x/y lengths must match. Do not add mm to data values: physical styles and data units serve different purposes.

## Individual functions

<!-- walkthrough:end -->

## Detailed behavior and further examples

### Parameters

| Parameter | Purpose | Default or requirement |
| --- | --- | --- |
| `plot_area` | Fixed data area | Example: (20,10,80,55) mm |
| `src` | CSV path | Relative to defining file |

### Data and fixed plot area

The example reads [first-plot.csv](../../examples/plot/first-plot.csv) beside the source. Switch to the CSV tab to read all data; preserve relative font and CSV locations.

The canvas is 120 × 90 mm. The chart is placed at (5,5) mm and its internal data area starts at (20,10) mm, giving a page position of **(25,15) mm** and a size of **80 × 55 mm**. Changing labels or moving the legend does not change that area; insufficient space produces a warning. Finish layers and decorations before `page.add`.

Compare `plot_area` and `page_transform` in the inspection JSON against those dimensions. At 300 DPI the PNG should measure **1417 × 1063 px**.

### Export and inspect

```sh
laymesh validate examples/plot/first-plot.lay
laymesh render examples/plot/first-plot.lay -o first-plot.svg
laymesh render examples/plot/first-plot.lay -o first-plot.pdf
laymesh render examples/plot/first-plot.lay -o first-plot.png --dpi 300
laymesh inspect examples/plot/first-plot.lay --json
```

### Limits and related topics

[Your first layout](first-layout.en.md)
