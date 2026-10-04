# Transforms and drawing order

<!-- example:examples/gallery/positioning/rotation.lay -->

## Parameters

| Parameter | Purpose | Default or requirement |
| --- | --- | --- |
| `rotation` | Rotation angle | 0 |
| `opacity` | Opacity | 1 |

## Common usage

The canvas and layout use physical dimensions: `mm`, `cm`, `in`, `pt`, or `px`. Layout `px` is converted using `canvas(layout_dpi=96)` by default. Export `--dpi` changes only PNG pixel dimensions. For example, 160 × 100 mm at 300 DPI gives about 1890 × 1181 px, with the same SVG/PDF page geometry.

On placement, `anchor` belongs to the new instance, `target` belongs to the canvas, current group, or an earlier instance, and `offset` shifts the alignment point:

The nine anchors are `top_left`, `top_center`, `top_right`, `middle_left`, `center`, `middle_right`, `bottom_left`, `bottom_center`, and `bottom_right`. Relative targets must already exist in the same coordinate space. Later `add` calls paint over earlier ones. Each placement may set `rotation` and `opacity`; image placements can also override `width`, `height`, `fit`, and `crop` independently.

## Limits and related topics

[Units and dimensions](units.en.md) · [Placement and anchors](anchors.en.md) · [Groups and reuse](groups.en.md)


## workflow

Complete sources and executable verification fixtures for this workflow are listed in the [feature coverage map](feature-map.en.md). Follow this page’s input conditions and limits when composing features.
