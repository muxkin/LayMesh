# Basic concepts

<!-- walkthrough:start -->
## Purpose and concepts

Create a page, define material, then use add to create instances. Material holds reusable content and default style; an instance holds this placement’s dimensions, transform and position. A definition alone draws nothing. Instance anchors have container coordinates.

## Minimal complete example

Run this file directly with `laymesh validate` or `laymesh render`; it contains its own canvas and required definitions.

<!-- example:examples/manual/canvas.lay -->

## Parameters and default behavior

Unitless geometry uses the canvas unit; unitless type and stroke sizes use pt. Explicit call parameters override inherited/theme defaults. The linked interface reference lists accepted types, choices and defaults per parameter.

## Composition

Place one material repeatedly. Save the return from add so later instances can target its bounds or path. Changing a definition does not retroactively edit completed placements.

<!-- example:examples/hello.lay -->

## Common errors and limits

Do not query page anchors on rect(...) itself: first use placed=page.add(material). An entry has one canvas; modules provide material and functions.

## Individual functions

### canvas

Create a page with physical dimensions, a background and a default geometry unit. Place material with add. Unitless geometry initially uses mm; font sizes and line widths use pt.

Returns: A page that accepts placed material.

Required inputs: `size`.

[Minimal complete source](../../examples/manual/canvas.lay) · [Composition source](../../examples/hello.lay) · [All parameters](interface-reference.en.md#canvas)

<!-- walkthrough:end -->

## Detailed behavior and further examples

### Parameters

| Parameter | Purpose | Default or requirement |
| --- | --- | --- |
| `canvas` | Create the entry canvas | One entry canvas |
| `add` | Create a placed instance | Drawn in insertion order |

### Common usage

Material definitions are reusable; add creates placed instances. A canvas is a page, a group is a local coordinate container, and a plot is a placeable material. Name an instance when later placement needs to reference it.

[Complete parameters and rules](../language-reference.en.md)

### Limits and related topics

[Installation and validation](install.en.md)
