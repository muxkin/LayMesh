# Your first layout

<!-- walkthrough:start -->
## Purpose and concepts

Start with a page, cards and a title to see absolute offsets and relative placement. Page size uses physical lengths; the origin is top left, x points right and y points down.

## Minimal complete example

Run this file directly with `laymesh validate` or `laymesh render`; it contains its own canvas and required definitions.

<!-- example:examples/manual/add.lay -->

## Parameters and default behavior

Unitless geometry uses the canvas unit; unitless type and stroke sizes use pt. Explicit call parameters override inherited/theme defaults. The linked interface reference lists accepted types, choices and defaults per parameter.

## Composition

Place the second instance using target=first.bottom_right and offset=(3mm,3mm). Align a title’s center to a card’s center so font changes preserve the relationship.

<!-- example:examples/hello.lay -->

## Common errors and limits

The default add anchor is top_left, with offsets along container axes. Pages do not grow automatically; objects outside the page can be clipped during export.

## Individual functions

<!-- walkthrough:end -->

## Detailed behavior and further examples

### Parameters

| Parameter | Purpose | Default or requirement |
| --- | --- | --- |
| `size` | Physical canvas size | Example: 160 × 100 mm |
| `anchor` | Instance anchor | top_left |

### Materials and instances

`card` defines a reusable shape; two `page.add(card, ...)` calls create separate instances. `left.center` positions the title at the first instance center. Read and copy the complete source in the source tab.

### Export and inspect

```sh
laymesh validate examples/hello.lay
laymesh render examples/hello.lay -o hello.svg
laymesh render examples/hello.lay -o hello.pdf
laymesh render examples/hello.lay -o hello.png --dpi 300
```

### Limits and related topics

[Your first data plot](first-plot.en.md)
