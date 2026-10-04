# Groups and reuse

<!-- walkthrough:start -->
## Purpose and concepts

group is a reusable container with local coordinates. Populate it, then place it on a page. Internal targets refer to existing instances in that group; placement replays these dependencies.

## Minimal complete example

Run this file directly with `laymesh validate` or `laymesh render`; it contains its own canvas and required definitions.

<!-- example:examples/manual/group.lay -->

## Parameters and default behavior

Unitless geometry uses the canvas unit; unitless type and stroke sizes use pt. Explicit call parameters override inherited/theme defaults. The linked interface reference lists accepted types, choices and defaults per parameter.

## Composition

Combine a panel, title and connector in a group, then reuse, rotate or resize it. fuse instead merges two already placed vector instances in the same container into a new outline; the inputs leave drawing and can no longer be referenced.

<!-- example:examples/gallery/containers/group-reuse.lay -->

## Common errors and limits

First placement seals a group. It cannot contain itself, and anchors from different containers cannot be mixed directly. fuse rejects images, text and groups and is not a public general-purpose Boolean API.

## Individual functions

### group

Create a reusable container with local coordinates. Populate it before placement; first placement seals contents and reuse replays internal positioning dependencies.

Returns: material

[Minimal complete source](../../examples/manual/group.lay) · [Composition source](../../examples/gallery/containers/group-reuse.lay) · [All parameters](interface-reference.en.md#group)

### fuse

Fuse visible outlines of two placed vector instances in one container; inputs leave drawing and references. Connecting a gap requires positive bridge_width; images/text/groups are unsupported.

Returns: instance

[Minimal complete source](../../examples/manual/fuse.lay) · [Composition source](../../examples/gallery/shapes/fuse-angled.lay) · [All parameters](interface-reference.en.md#fuse)

<!-- walkthrough:end -->

## Detailed behavior and further examples

### Parameters

| Parameter | Purpose | Default or requirement |
| --- | --- | --- |
| `width / height` | Group instance dimensions | Optional; scales children too |
| `target / offset` | Placement in the container | Use local coordinates inside a group |

### Common usage

Finish group contents before placing the group on a canvas. Groups can be reused and nested; group scaling also scales children, including text and strokes.

[Complete parameters and rules](../user-guide.en.md)

### Limits and related topics

[Units and dimensions](units.en.md) · [Placement and anchors](anchors.en.md) · [Transforms and drawing order](transforms.en.md)

## workflow

Complete sources and executable verification fixtures for this workflow are listed in the [feature coverage map](feature-map.en.md). Follow this page’s input conditions and limits when composing features.
