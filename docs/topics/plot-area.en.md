# Plot area and physical size

<!-- walkthrough:start -->
## Purpose and concepts

Page size, chart size and plot-area size are three distinct dimensions. plot_area=box(...) fixes the data region’s physical position and dimensions, useful for comparable experimental panels.

## Minimal complete example

Run this file directly with `laymesh validate` or `laymesh render`; it contains its own canvas and required definitions.

<!-- example:examples/manual/plot.lay -->

## Parameters and default behavior

Unitless geometry uses the canvas unit; unitless type and stroke sizes use pt. Explicit call parameters override inherited/theme defaults. The linked interface reference lists accepted types, choices and defaults per parameter.

## Composition

An explicit plot_area retains the data region during chart reflow. margins retain edge spacing while the data area follows the frame. Shared plot_style sets text/stroke defaults; explicit layer parameters win.

<!-- example:examples/plot/publication.lay -->

## Common errors and limits

plot_area and margins are exclusive. Content does not enlarge the page; cramped legends or labels generate layout warnings. Reflowing the chart frame differs from scaling an enclosing group.

## Individual functions

### box

Create a rectangular region configuration for plot_area or crop. box draws nothing by itself; units depend on use, with crop using normalized 0–1 coordinates.

Returns: box

Required inputs: `size`.

[Minimal complete source](../../examples/manual/box.lay) · [Composition source](../../examples/plot/publication.lay) · [All parameters](interface-reference.en.md#box)

### plot

Create a native plot with physical dimensions and add data through layer methods. Configure axes with x and y, or fix the drawing area with plot_area. Place the plot on the page with add.

Returns: A plot accepting data layers and page placement.

Required inputs: `size`.

[Minimal complete source](../../examples/manual/plot.lay) · [Composition source](../../examples/plot/publication.lay) · [All parameters](interface-reference.en.md#plot)

### plot_style

Define shared chart/layer/decoration style defaults. Explicit call parameters win; this is not an inline/display string option.

Returns: plot_style

[Minimal complete source](../../examples/manual/plot_style.lay) · [Composition source](../../examples/plot/publication.lay) · [All parameters](interface-reference.en.md#plot_style)

<!-- walkthrough:end -->

## Detailed behavior and further examples

### Parameters

| Parameter | Purpose | Default or requirement |
| --- | --- | --- |
| `size` | Outer chart size | Explicit physical size |
| `plot_area` | Data area x/y/width/height | Optional; exclusive with margins |

### Common usage

Size has three levels: `canvas(size=...)` defines the page, `plot(size=...)` defines the chart frame, and `plot_area` or `margins` defines the inner data region. Content does not automatically enlarge the page or frame.

`plot_area=box(offset=(left,top), size=(width,height))` uses physical lengths relative to the chart frame. Left/top are nonnegative and width/height are positive. It is exclusive with `margins=(left,top,right,bottom)`. Only when both are omitted does text measurement determine automatic margins.

Overriding a chart instance's `size` retains the position and size of explicit `plot_area`, changing surrounding space. Fixed margins instead retain the four margins while the data region follows the frame. Labels, legends and colorbars do not shrink a fixed data area; insufficient room produces `W_PLOT_LAYOUT` while preserving output.

The chart is sealed after first placement: it can be placed again, but cannot receive more layers or decorations. Changing a chart frame preserves physical text, stroke and marker sizes; scaling an enclosing group scales all children.

Both data areas in this example measure **64 × 45 mm**. Source explicitly places their page top-left corners at **(20,26) mm** and **(115,26) mm**. Use plot-area anchors for manual panel alignment; LayMesh does not automatically equalize or move neighboring panels.

See [axes and ticks](axes.en.md), [legends and shared colorbars](legends.en.md), and [annotations](annotations.en.md) for their respective rules.

### Limits and related topics

[Data input and missing values](data.en.md) · [Axes and ticks](axes.en.md) · [Labels and scientific notation](labels.en.md) · [Legends and shared colorbars](legends.en.md) · [Data anchors and annotations](annotations.en.md)
