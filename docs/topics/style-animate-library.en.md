# Animate.css: base class, utilities and effects

[Style and animation libraries](style-libraries.en.md)

## What it is and where it comes from

Animate.css is an [open-source CSS animation library](https://animate.style/), pinned to 4.1.1 under MIT. Its classes and keyframes define entrances, exits and emphasis; Lay sets object dimensions and placement.

## Your first complete animation

<!-- source:examples/libraries/animate.lay -->

Import, place an instance, create a step and schedule an action. `animate__animated` supplies base timing and fill; the effect class selects keyframes. Step `start` controls whether presentation waits for input.

## From effects to parameters

1. Choose an entrance, exit or emphasis in the [97-effect catalog](animation-catalog.en.md).
2. Set [action parameters](animation-timing.en.md) such as `duration=800ms` and `delay=200ms`; explicit arguments override CSS.
3. `animate__fast/faster/slow/slower` adjust speed; `animate__delay-1s` through `delay-5s` delay; `animate__repeat-1/2/3` and `animate__infinite` repeat.
4. `--animate-duration`, `--animate-delay` and `--animate-repeat` support shared settings and action variables. Remove explicit overrides when testing utilities.
5. Infinite motion needs a finite step window. Print and reduced-motion branches have separate rules; see [export and troubleshooting](animation-export.en.md).

## Complete class, variable and keyframe catalog

The existing 97-effect guide retains its descriptions and shared player. This catalog also covers base classes, utilities, variables and keyframes with individual recipes.

Catalog recipes give `rect` a low-specificity color and border so corners, opacity and motion are visible. Class declarations and variable consumers still cascade normally.

<!-- style-catalog:animate -->
