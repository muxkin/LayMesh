# Units and sizes

<!-- walkthrough:start -->
## Purpose and concepts

Unitless geometry uses canvas.unit, initially mm; unitless font sizes and stroke widths use pt. Explicit units are easiest to read and reproduce across themes. Layout px uses layout_dpi; PNG output DPI only determines final pixel count.

## Minimal complete example

Run this file directly with `laymesh validate` or `laymesh render`; it contains its own canvas and required definitions.

<!-- example:examples/manual/line.lay -->

## Parameters and default behavior

Unitless geometry uses the canvas unit; unitless type and stroke sizes use pt. Explicit call parameters override inherited/theme defaults. The linked interface reference lists accepted types, choices and defaults per parameter.

## Composition

Multiplying a length by a scalar retains length; dividing compatible lengths yields a ratio. size=(40mm,auto) often preserves image aspect or natural text height; auto support depends on material.

<!-- example:examples/gallery/positioning/dependent-size.lay -->

## Common errors and limits

Page dimensions must be positive and cannot use auto. Angles cannot be added to lengths. line(angle=...) accepts deg/rad or unitless degrees; polar data angles use angle_unit.

## Individual functions

<!-- walkthrough:end -->

## Detailed behavior and further examples

### Physical lengths and scalars

```lay
page = canvas(size=(150, 100), unit="mm")
panel = rect(size=(40, 25), fill="#eef5f2", border_color="#245447", border_width=0.6)
page.add(panel, offset=(2cm, 35))
```

Sizes, positions, gaps, geometric radii and padding use the canvas unit. Bare font sizes, line heights, border widths, line widths and dash lengths use pt. Explicit units take precedence: `2cm`, `2 cm`, `2inch` and `2in` are supported. `layout_dpi=96` controls conversion from px; PNG `--dpi` independently controls exported pixel density.

Data, opacity, ratios, counts and axis domains remain scalars. Bare numbers are converted only at physical parameter boundaries: `2mm + 3` is invalid, while `2mm + 3cm` is valid.

### Automatic dimensions

For images, `size=(80, auto)` fixes the width and preserves the source aspect ratio. For text it sets a wrapping width and derives the height. `page.add(plot, size=(120, 90))` lays out the plot again; a fixed plot area retains its physical size.

```lay
photo = image(src="sample.png")
page.add(photo, size=(2inch, auto), offset=(2cm, 35))
```

Instance `.width` and `.height` remain read-only measurements. Constructor and placement arguments `width=` and `height=` are removed.

### Named regions

Use `plot_area=box(offset=(20, 10), size=(80, 55))`. Image cropping uses `crop=box(offset=(0.1, 0.1), size=(0.8, 0.8))`; crop coordinates remain normalized scalars from 0 to 1.

[All parameters](interface-reference.en.md) · [Math](formulas.en.md) · [LCSS](lcss.en.md)

## workflow

Complete sources and executable verification fixtures for this workflow are listed in the [feature coverage map](feature-map.en.md). Follow this page’s input conditions and limits when composing features.
