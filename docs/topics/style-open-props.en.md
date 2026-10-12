# Open Props: variables, parameters and complete catalog

[Style and animation libraries](style-libraries.en.md)

## What it is and where it comes from

Open Props is an [open-source design-variable library](https://open-props.style/). LayMesh pins original 1.7.23 CSS under MIT. It provides coordinated colors, gradients, corners, shadows, typography and motion values, consumed by style properties.

## Your first variables

<!-- source:examples/libraries/open-props.lay -->

The `.card` declaration applies variables to properties; `class="card"` applies that appearance. Importing the file alone does not automatically paint every object.

## Where variables belong

| Variable family | Consumer property | Meaning |
| --- | --- | --- |
| `--blue-*`, `--gray-*` | color, background-color, border-color | Coordinated color scales |
| `--gradient-*` | background | Complete gradient recipe |
| `--radius-*` | border-radius | Corner scale |
| `--shadow-*`, `--inner-shadow-*` | box-shadow | Layered outer or inner shadows |
| `--size-*` | padding and supported lengths | Spacing; Lay still sets dimensions and placement |
| Font size, weight, line-height variables | Corresponding properties | Choose a variable matching the property |
| `--ease-*`, `--duration-*` | animation-timing-function, animation-duration | Easing or duration |
| `--animation-*` | animation | Open Props animation recipes and keyframes |

Relative lengths use root font size, canvas size and `layout_dpi`. Variable types are not interchangeable; catalog entries identify their consumer properties.

## Open Props also includes motion

<!-- source:examples/libraries/open-props-motion.lay -->

This imports only Open Props and uses `--animation-fade-in`, rather than the Animate.css `animate__fadeIn` class. They are different sets of definitions.

## Complete searchable catalog

The catalog lists every declared variable and keyframe, including helpers and currently unsupported consumers. Each usable entry has a complete recipe; related entries share one preview. Font families ship no font files, and unsupported consumers or expressions explain their limitations.

Catalog recipes give `rect` a low-specificity color and border so corners, opacity and motion are visible. Class declarations and variable consumers still cascade normally.

<!-- style-catalog:open-props -->
