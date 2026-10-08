# LCSS themes and regional fills

LCSS is LayMesh's styling language. Separate files use `.lcss`. Selectors and declarations resemble CSS, but this is not browser CSS: geometry, data, domains and placement remain in `.lay`.

<!-- example:examples/unified.lay -->

## Loading and precedence

`canvas(stylesheet="language/paper.lcss")` loads a local stylesheet; an ordered path list is also supported. Entry-file `style { … }` blocks follow external sheets. Specificity and source order decide ties; explicit object parameters win. Module styles are scoped to objects from their module and act as component defaults; entry themes can override them.

```lcss
@import "base.lcss";
plot::axis(x) { line-color: #444444; line-width: 0.6pt; }
.annotation {
  --paper: #ffffff;
  background: var(--paper);
  border: 0.5pt solid #b9d5cb;
  border-radius: 1mm;
  padding: 1mm;
}
```

Use `class="paper annotation"` in `.lay`. Selectors support types, classes, combinations, comma-separated groups, descendants and direct children (`>`). Parts include `plot::area`, `plot::axis(x)`, `plot::tick-label`, `plot::axis-label`, `plot::legend` and `plot::colorbar`.

## Paints and borders

`fill` paints a shape's interior; `background` paints a region. Values include `none`, colors, `linear-gradient(...)`, `radial-gradient(...)`, `hatch(slash, #666666, 2mm, 0.5pt)`, cross/dot hatches and `url("texture.png")`. DSL counterparts are `linear_gradient`, `radial_gradient`, `hatch` and `image_fill`.

Configure canvas, plot frame, plot area, each axis and its labels, text/formula boxes, legends/colorbars and imported-image boxes independently. Plot-area paint follows polar outlines and visible broken-axis segments. Paint alone does not change data mapping. `padding` is inner spacing and contributes to measured bounds.

Closed outlines use `border_*`; open lines use `line_*`. LCSS uses hyphens, such as `border-width` and `line-width`. Typography inherits; backgrounds and placement do not. Parent and child opacity multiply. Instances of the same material can select independent classes.

## Resources and limits

LCSS lengths require explicit units (except zero), so a theme is independent of the canvas default unit. Image and font paths are relative to the declaring `.lcss` file. Variables support `var(--name, fallback)`; variable and import cycles are errors.

Web layout, media queries, animation, interactive pseudo-classes and `!important` are unsupported and produce diagnostics. [All parameters](interface-reference.en.md) · [Migration](migration.en.md)

## style-properties

Each property maps to the following DSL parameter. Types, units and inheritance use the same metadata as parameter hover.

| LCSS | DSL |
| --- | --- |
| `font-family` | `font_family` |
| `math-font` | `math_font` |
| `math-text-fallback` | `math_text_fallback` (`true` / `false`) |
| `font-size` | `font_size` |
| `font-weight` | `font_weight` |
| `font-style` | `font_style` |
| `color` | `color` |
| `line-height` | `line_height` |
| `background` | `background` |
| `padding` | `padding` |
| `border-radius` | `border_radius` |
| `border-color` | `border_color` |
| `border-width` | `border_width` |
| `border-style` | `border_style` |
| `border-dash` | `border_dash` |
| `border-dash-offset` | `border_dash_offset` |
| `border-cap` | `border_cap` |
| `border-join` | `border_join` |
| `border-miter-limit` | `border_miter_limit` |
| `border-opacity` | `border_opacity` |
| `line-color` | `line_color` |
| `line-width` | `line_width` |
| `line-style` | `line_style` |
| `line-dash` | `line_dash` |
| `line-dash-offset` | `line_dash_offset` |
| `line-cap` | `line_cap` |
| `line-join` | `line_join` |
| `line-miter-limit` | `line_miter_limit` |
| `line-opacity` | `line_opacity` |
| `fill` | `fill` |
| `opacity` | `opacity` |
| `tick-font-size` | `tick_font_size` |
| `label-font-size` | `label_font_size` |
| `tick-color` | `tick_color` |
| `grid-color` | `grid_color` |
| `grid-line-width` | `grid_line_width` |
| `grid-line-dash` | `grid_line_dash` |
| `start-head` | `start_head` |
| `end-head` | `end_head` |
| `start-cap` | `start_cap` |
| `end-cap` | `end_cap` |

Endpoint properties accept `head(...)`, for example `end-head: head(shape=triangle,size=(5mm,4mm));`. LCSS identifiers are style values; they are not reads from the DSL variable scope.

## workflow

Complete sources and executable verification fixtures for this workflow are listed in the [feature coverage map](feature-map.en.md). Follow this page’s input conditions and limits when composing features.
