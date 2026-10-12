# Style manual

Start with [style and animation libraries](style-libraries.en.md): learn bundled libraries, sources and imports, then explore entries, parameters and composition. One `.lay` file is enough to begin.

Keep dimensions, anchors, data and placement in .lay, with repeated appearance in a theme. Follow these chapters and use the property reference for details.

This manual describes current source. New offline style libraries and diagnostics require an engine build containing those changes; release notes describe published packages.

## Learning path

1. [Getting started and libraries](style-start.en.md)
2. [Selectors, variables and cascade](style-cascade.en.md)
3. [Appearance properties and units](style-properties.en.md)
4. [Shadows and effects](style-shadows.en.md)
5. [Compatibility and troubleshooting](style-compatibility.en.md)

## Complete projects

Combine libraries and motion in [three worked projects](style-animation-examples.en.md): [styled cards](style-animation-examples.en.md#styled-cards), [artistic poster](style-animation-examples.en.md#art-direction), and the recovered [complex dashboard](style-animation-examples.en.md#complex-dashboard). Each includes editable source, themes and real playback.

## Complete runnable example

<!-- example:examples/manual/style-project.lay -->

The first card sets its page margin relative to `page.top_left`. Subsequent cards follow `top_right`, and each label follows its card's `bottom_left`. `card_size` and `gap` control dimensions and spacing: change width from 30mm to 26mm and the following cards and labels move with their anchors. Themes save appearance, loops remove repetition, and anchors save layout relationships. See [placement](anchors.en.md) and [loops/functions](control-flow.en.md).

## Common questions

Explicit .lay parameters override LCSS, including important declarations. Omit the matching object parameter when the theme should control it; see [cascade](style-cascade.en.md). Change only blur to 2mm with [partial overrides](style-shadows.en.md#partial-overrides).

<details>
<summary>Previous section links</summary>

## Loading and precedence

[Read the chapter](style-start.en.md)

## Paints and borders

[Read the chapter](style-start.en.md)

## Resources and limits

[Read the chapter](style-start.en.md)

## style-properties

[Read the chapter](style-properties.en.md)

## css-libraries

[Read the chapter](style-start.en.md)

## shadow-overrides

[Read the chapter](style-shadows.en.md)

## css-units

[Read the chapter](style-properties.en.md)

## css-effects

[Read the chapter](style-shadows.en.md)

## css-compatibility

[Read the chapter](style-compatibility.en.md)

## workflow

[Read the chapter](style-start.en.md)

</details>
