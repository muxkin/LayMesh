# Bootstrap: classes, parameters and complete catalog

[Style and animation libraries](style-libraries.en.md)

## What it is and where it comes from

Bootstrap is an [open-source web UI framework](https://getbootstrap.com/docs/5.3/) with components, web layout and utility styles. LayMesh pins original 5.3.8 CSS under MIT and applies supported appearance declarations.

## Your first card

<!-- source:examples/libraries/bootstrap.lay -->

`bg-primary` paints the background, `rounded` rounds corners and `shadow` adds a shadow. Background and text color classes target different parts of an object.

## Learn by function

| Function | Start with | Adjust next |
| --- | --- | --- |
| Background and text | `bg-primary`, `bg-success`, `text-primary` | Color variants and opacity utilities |
| Borders | `border border-primary` | Color, width and removal |
| Corners | `rounded` | Radius variants; explicit `border_radius` for a one-off override |
| Shadows | `shadow-sm`, `shadow`, `shadow-lg` | Inline blur/spread/offset overrides |
| Typography | `fw-bold`, `fst-italic` | Size, line height and project-provided fonts |
| Opacity and spacing | `opacity-50`, supported padding utilities | Spacing does not replace anchors |

[Partial overrides](style-shadows.en.md#partial-overrides) distinguish 2mm blur, spread and offset. Library importance does not override explicit Lay parameters.

## Complete searchable catalog

Every class, variable and keyframe retains upstream declarations and support status. Usable entries switch to complete source and real preview; helper classes may require a base class. Partial support applies supported declarations, rather than implementing a complete web component.

Catalog recipes give `rect` a low-specificity color and border so corners, opacity and motion are visible. Class declarations and variable consumers still cascade normally.

<!-- style-catalog:bootstrap -->
