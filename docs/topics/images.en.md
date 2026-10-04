# Image import and cropping

<!-- walkthrough:start -->
## Purpose and concepts

image defines external picture material; add determines size, crop and fit. A crop box uses normalized 0–1 image coordinates. Cropping occurs before fitting to size.

## Minimal complete example

Run this file directly with `laymesh validate` or `laymesh render`; it contains its own canvas and required definitions.

<!-- example:examples/manual/image.lay -->

## Parameters and default behavior

Unitless geometry uses the canvas unit; unitless type and stroke sizes use pt. Explicit call parameters override inherited/theme defaults. The linked interface reference lists accepted types, choices and defaults per parameter.

## Composition

Place a full image and a detail from the same definition. contain keeps all content, cover fills the box and clips overflow, and stretch changes aspect ratio. One size dimension plus auto preserves the cropped aspect.

<!-- example:examples/gallery/images/crop.lay -->

## Common errors and limits

SVG input is allowlisted, and SVG crop is unsupported. TIFF accepts single-page unsigned 8/16-bit grayscale or RGB, with optional alpha; multipage, signed and floating-point TIFF are unsupported. GIF and animated WebP use the first frame; 16-bit intensities are not stretched. Pictures provide bounds without pixel-derived path or ink.

## Individual functions

### image

Define reusable image material from a local file. add controls size, crop and contain/cover/stretch fitting; paths resolve relative to the defining file. Accepts PNG, JPEG, BMP, WebP, GIF, ICO, PNM, TGA, safe SVG, and single-page unsigned 8/16-bit grayscale/RGB TIFF with optional alpha. GIF and animated WebP use the first frame; 16-bit intensities are not stretched.

Returns: material

Required inputs: `src`.

[Minimal complete source](../../examples/manual/image.lay) · [Composition source](../../examples/gallery/images/crop.lay) · [All parameters](interface-reference.en.md#image)

<!-- walkthrough:end -->

## Detailed behavior and further examples

### Parameters

| Parameter | Purpose | Default or requirement |
| --- | --- | --- |
| `src` | Image path | Required |
| `fit` | Image frame fit | contain / cover / stretch |
| `crop` | Source crop | Optional |

### Common usage

Supported inputs are PNG, JPEG, BMP, WebP, GIF, ICO, PNM (PBM/PGM/PPM/PAM), TGA, safe SVG, and single-page unsigned 8/16-bit grayscale or RGB TIFF with optional alpha. GIF and animated WebP use the first frame; ICO uses the decoder-selected primary image. crop selects source content; contain preserves the full image, cover fills and crops, and stretch scales each dimension independently. Instances remain independent.

[Complete parameters and rules](../user-guide.en.md)

### Limits and related topics

[Text and fonts](fonts.en.md) · [Resource paths](resources.en.md) · [Mathematical formulas](formulas.en.md) · [Shapes and paths](shapes.en.md) · [Fills, gradients and outlines](fills.en.md)

### Alpha and 16-bit intensity

Transparent and partially transparent pixels survive image import, image paint and cropping when supported by the source format. Associated TIFF alpha is made straight before color conversion. Intermediate PNGs and crops retain 16-bit depth, including a 16-bit ICC conversion path. Display and 8-bit exports quantize the full intensity range without per-image contrast stretching. Alpha composites with underlying shapes in SVG, PNG and PDF.
