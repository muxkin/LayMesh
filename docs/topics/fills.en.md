# Fills, gradients and outlines

<!-- walkthrough:start -->
## Purpose and concepts

Colors are HEX strings or rgb/hsv/oklch values. HEX’s last two digits encode alpha: #ffffff90 is about 56.47% opacity. Color alpha multiplies object, stroke and ancestor-group opacity.

## Minimal complete example

Run this file directly with `laymesh validate` or `laymesh render`; it contains its own canvas and required definitions.

<!-- example:examples/manual/rgb.lay -->

## Parameters and default behavior

Unitless geometry uses the canvas unit; unitless type and stroke sizes use pt. Explicit call parameters override inherited/theme defaults. The linked interface reference lists accepted types, choices and defaults per parameter.

## Composition

Use colors in fills, text, strokes, gradient stops and chart palettes. linear_gradient and radial_gradient use normalized layout-box coordinates. hatch uses physical spacing; image_fill fits an image inside a shape.

<!-- example:examples/gallery/shapes/linear-gradient.lay -->

## Common errors and limits

RGB is 0–255; HSV S/V and OKLCH L are 0–1; C is nonnegative and alpha is 0–1. Invalid or nonfinite values error. Out-of-gamut OKLCH reduces chroma with a mapping notice. HSL/CMYK are not supported.

## Individual functions

### linear_gradient

Create linear-gradient paint with stops in 0–1 and start/end normalized to material bounds. Color alpha multiplies stop opacity.

Returns: paint

Required inputs: `stops`.

[Minimal complete source](../../examples/manual/linear_gradient.lay) · [Composition source](../../examples/gallery/shapes/linear-gradient.lay) · [All parameters](interface-reference.en.md#linear_gradient)

### radial_gradient

Create radial-gradient paint with center/radius relative to bounds. Reuse the paint across different shapes; placement dimensions determine final rendering.

Returns: paint

Required inputs: `stops`.

[Minimal complete source](../../examples/manual/radial_gradient.lay) · [Composition source](../../examples/gallery/shapes/radial-gradient.lay) · [All parameters](interface-reference.en.md#radial_gradient)

### hatch

Create slash/cross/dots pattern paint. spacing and line_width are physical dimensions; the pattern is confined to the material fill region.

Returns: paint

[Minimal complete source](../../examples/manual/hatch.lay) · [Composition source](../../examples/unified.lay) · [All parameters](interface-reference.en.md#hatch)

### image_fill

Use a local image as paint inside a shape, with fit controlling adaptation. Unlike image material, paint is confined to the shape outline, including rounded rectangles.

Returns: paint

Required inputs: `src`.

[Minimal complete source](../../examples/manual/image_fill.lay) · [Composition source](../../examples/manual/paint-composition.lay) · [All parameters](interface-reference.en.md#image_fill)

### rgb

Create a RGB color with alpha=0–1.

Returns: color

Required inputs: `r`, `g`, `b`.

[Minimal complete source](../../examples/manual/rgb.lay) · [Composition source](../../examples/gallery/shapes/colors-alpha.lay) · [All parameters](interface-reference.en.md#rgb)

### hsv

Create a HSV color with alpha=0–1.

Returns: color

Required inputs: `h`, `s`, `v`.

[Minimal complete source](../../examples/manual/hsv.lay) · [Composition source](../../examples/gallery/shapes/colors-alpha.lay) · [All parameters](interface-reference.en.md#hsv)

### oklch

Create a OKLCH color with alpha=0–1.

Returns: color

Required inputs: `l`, `c`, `h`.

[Minimal complete source](../../examples/manual/oklch.lay) · [Composition source](../../examples/gallery/shapes/colors-alpha.lay) · [All parameters](interface-reference.en.md#oklch)

<!-- walkthrough:end -->

## Detailed behavior and further examples

### Parameters

| Parameter | Purpose | Default or requirement |
| --- | --- | --- |
| `fill` | Color or gradient | Also accepts none |
| `border_* / line_*` | Closed/open stroke settings | Defaults and choices in the shared reference |

### Common usage

fill accepts colors, none, gradients, hatch and image_fill paint. Use border_* for closed outlines and line_* for open strokes; LCSS classes reuse settings. outline(...) is removed. Gradient strokes are not implemented.

[Complete parameters and rules](../language-reference.en.md)

### Limits and related topics

[Image import and cropping](images.en.md) · [Text and fonts](fonts.en.md) · [Resource paths](resources.en.md) · [Mathematical formulas](formulas.en.md) · [Shapes and paths](shapes.en.md)


### RGB, HSV, OKLCH and alpha

Colors accept #RGB/#RGBA/#RRGGBB/#RRGGBBAA with trailing alpha, e.g. #ffffff90 means 144/255 alpha. DSL constructors include rgb(255,80,40,alpha=0.6), hsv(200deg,0.7,0.8,alpha=0.6), and oklch(0.7,0.15,200deg,alpha=0.6). Strings and LCSS accept space-separated functions with / alpha, e.g. oklch(0.7 0.15 200 / 0.6). HSV is a LayMesh extension; CMYK is not supported.

RGB channels range from 0 to 255; HSV saturation/value and OKLCH lightness range from 0 to 1. OKLCH chroma is nonnegative; alpha ranges from 0 to 1. Hue accepts deg/rad or unitless degrees. OKLCH retains source channels and uses deterministic constant-lightness/hue chroma reduction with 48 binary-search iterations to map to sRGB. The editor reports gamut mapping; CLI, WASM and exports use identical results.

Color alpha multiplies drawing opacity. Fills, strokes, text, gradients and color scales share the rule. Color lists retain linear RGB channel interpolation and interpolate alpha as well.
