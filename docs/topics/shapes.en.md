# Shapes and paths

<!-- walkthrough:start -->
## Purpose and concepts

Closed shapes use fill and border_*; open lines use line_*. path(commands=...) preserves source nodes and segments rather than render subdivisions. Shaft and heads affect ink, while the complete centerline reaches logical endpoints.

## Minimal complete example

Run this file directly with `laymesh validate` or `laymesh render`; it contains its own canvas and required definitions.

<!-- example:examples/manual/line.lay -->

## Parameters and default behavior

Unitless geometry uses the canvas unit; unitless type and stroke sizes use pt. Explicit call parameters override inherited/theme defaults. The linked interface reference lists accepted types, choices and defaults per parameter.

## Composition

Use line() or style-only line(...) as material and supply start/end in add, or retain dx/dy or length/angle, never both. Construction may omit geometry; placement must determine it. start_head points opposite the start tangent and end_head along the end tangent. triangle/open/stealth locate their tip; dot/diamond/bar locate their center. Zero length plus an explicit angle creates standalone heads.

<!-- example:examples/gallery/shapes/path-commands.lay -->

## Common errors and limits

Closed subpaths reject heads; each open subpath receives its own endpoint styles. Short lines do not shrink heads. Zero-length layout retains cap/head dimension rules while path length remains zero. arrow(...) is removed; migrate to line(...,end_head=head(...)). Missing material geometry without start/end is reported at add. Coincident connection endpoints error rather than creating a zero-length head.

## Individual functions

### rect

Create a rectangle material. Set physical dimensions with size, interior paint with fill, the outline with border_*, and rounded corners with border_radius. Place it on the page with add.

Returns: Reusable rectangle material.

Required inputs: `size`.

[Minimal complete source](../../examples/manual/rect.lay) · [Composition source](../../examples/basic.lay) · [All parameters](interface-reference.en.md#rect)

### ellipse

Define ellipse material with size=(width,height). fill and border_* control interior and outline; layout corners need not lie on the ellipse.

Returns: material

Required inputs: `size`.

[Minimal complete source](../../examples/manual/ellipse.lay) · [Composition source](../../examples/basic.lay) · [All parameters](interface-reference.en.md#ellipse)

### line

Create reusable line material. Omit geometry and supply start/end in add, or define dx/dy or length/angle. Placement requires geometry; zero length requires an explicit angle. Supports independent caps and heads.

Returns: material

[Minimal complete source](../../examples/manual/line.lay) · [Composition source](../../examples/basic.lay) · [All parameters](interface-reference.en.md#line)

### path

Define a path from original move_to/line_to/quad_to/cubic_to/arc_to/close commands. It may contain multiple subpaths; open subpaths support caps and heads.

Returns: material

Required inputs: `commands`.

[Minimal complete source](../../examples/manual/path.lay) · [Composition source](../../examples/gallery/shapes/path-commands.lay) · [All parameters](interface-reference.en.md#path)

### polygon

Define an automatically closed polygon from at least three points. Node order determines traversal; fill_rule controls self-intersecting fills.

Returns: material

Required inputs: `points`.

[Minimal complete source](../../examples/manual/polygon.lay) · [Composition source](../../examples/gallery/shapes/polygon-polyline.lay) · [All parameters](interface-reference.en.md#polygon)

### polyline

Define an open polyline from at least two points. line_* controls stroke, start_head/end_head locate at logical endpoints, and line_join controls corners.

Returns: material

Required inputs: `points`.

[Minimal complete source](../../examples/manual/polyline.lay) · [Composition source](../../examples/gallery/shapes/polygon-polyline.lay) · [All parameters](interface-reference.en.md#polyline)

### arc

Define an open circular arc with radius/start/end. Page angles point right at 0 and down at 90; caps and heads are supported without closing to a sector.

Returns: material

Required inputs: `radius`, `start`, `end`.

[Minimal complete source](../../examples/manual/arc.lay) · [Composition source](../../examples/gallery/shapes/arc-sector.lay) · [All parameters](interface-reference.en.md#arc)

### sector

Define a closed sector by radius and endpoint angles. Fill covers the region between center and arc; border_* controls the outline.

Returns: material

Required inputs: `radius`, `start`, `end`.

[Minimal complete source](../../examples/manual/sector.lay) · [Composition source](../../examples/gallery/shapes/arc-sector.lay) · [All parameters](interface-reference.en.md#sector)

### star

Define a closed star with point count, outer/inner radii and rotation. inner_radius must be smaller than outer_radius.

Returns: material

Required inputs: `points`, `outer_radius`, `inner_radius`.

[Minimal complete source](../../examples/manual/star.lay) · [Composition source](../../examples/gallery/shapes/star-ring.lay) · [All parameters](interface-reference.en.md#star)

### ring

Define a ring with a transparent inner hole. inner_radius is smaller than outer_radius; the hole reveals underlying paint rather than covering it with background.

Returns: material

Required inputs: `outer_radius`, `inner_radius`.

[Minimal complete source](../../examples/manual/ring.lay) · [Composition source](../../examples/gallery/shapes/star-ring.lay) · [All parameters](interface-reference.en.md#ring)

### move_to

Create a command beginning a subpath at x/y without connecting it to the previous one. Use only inside path(commands=...).

Returns: path-command

[Minimal complete source](../../examples/manual/move_to.lay) · [Composition source](../../examples/gallery/shapes/path-commands.lay) · [All parameters](interface-reference.en.md#move_to)

### line_to

Create a straight segment command from the current node to x/y. A preceding move_to is required; original node/segment identities are retained.

Returns: path-command

[Minimal complete source](../../examples/manual/line_to.lay) · [Composition source](../../examples/gallery/shapes/path-commands.lay) · [All parameters](interface-reference.en.md#line_to)

### quad_to

Create a quadratic Bezier segment: cx/cy is the control and x/y the endpoint. Controls usually lie off-curve; t differs from arc-length fraction.

Returns: path-command

[Minimal complete source](../../examples/manual/quad_to.lay) · [Composition source](../../examples/gallery/shapes/path-commands.lay) · [All parameters](interface-reference.en.md#quad_to)

### cubic_to

Create a cubic Bezier segment from two controls and an endpoint. controls queries retain originals and indices do not depend on render subdivisions.

Returns: path-command

[Minimal complete source](../../examples/manual/cubic_to.lay) · [Composition source](../../examples/gallery/shapes/path-commands.lay) · [All parameters](interface-reference.en.md#cubic_to)

### arc_to

Create an elliptical arc command with radii, rotation, large_arc/sweep flags and endpoint. One command remains one original logical segment.

Returns: path-command

[Minimal complete source](../../examples/manual/arc_to.lay) · [Composition source](../../examples/gallery/shapes/path-commands.lay) · [All parameters](interface-reference.en.md#arc_to)

### close

Create a command closing the current subpath to its start while preserving traversal. Closed start/end may coincide; seam-crossing intervals require explicit wrap=true.

Returns: path-command

[Minimal complete source](../../examples/manual/close.lay) · [Composition source](../../examples/gallery/shapes/evenodd-hole.lay) · [All parameters](interface-reference.en.md#close)

### head

Reusable endpoint head configuration, not a placeable material. size gives along-direction and transverse physical dimensions.

Returns: head

[Minimal complete source](../../examples/manual/head.lay) · [Composition source](../../examples/basic.lay) · [All parameters](interface-reference.en.md#head)

<!-- walkthrough:end -->

## Detailed behavior and further examples

### Parameters

| Parameter | Purpose | Default or requirement |
| --- | --- | --- |
| `path` | Path commands and coordinates | Coordinates use length units |
| `fill_rule` | Fill rule | nonzero / evenodd |

### Common usage

Use rect, ellipse, polygon, polyline, path, line for vector materials. Path coordinates use physical units; fill rules control overlapping areas and holes. Fusion requires already placed vector instances in the same container.

[Complete parameters and rules](../language-reference.en.md)

### Limits and related topics

[Image import and cropping](images.en.md) · [Text and fonts](fonts.en.md) · [Resource paths](resources.en.md) · [Mathematical formulas](formulas.en.md) · [Fills, gradients and outlines](fills.en.md)


### Line direction and endpoint styles

`arrow(...)` has been removed. Use `line(...,end_head=head(...))`. Editors report the migration and offer a fix for safely identifiable builtin calls.

```lay
page.add(line(length=40mm,angle=30deg,line_width=1mm,
              start_cap="round",end_head=head(size=(5mm,4mm))),offset=(10mm,20mm))
page.add(line(length=0mm,angle=30deg,end_head=head(size=(5mm,4mm))),anchor=self.path.end,offset=(60mm,20mm))
```

Choose either dx/dy or length/angle. Length is nonnegative; nonzero length defaults to zero degrees, while zero length requires an explicit angle. Zero degrees points right and 90 degrees down. Angles accept deg/rad or unitless degrees.

line, polyline, open path and arc share start_head/end_head and start_cap/end_cap. Caps are butt/round/square and inherit line_cap for line/polyline/arc, or border_cap for path; internal dash caps retain that base setting. Head shapes are triangle/open/stealth/dot/diamond/bar. size gives longitudinal and transverse dimensions; dot is an ellipse and bar a rectangle. Triangular, open and stealth heads use their tip as the endpoint; other shapes use their center. Set fill, border_color, border_width and opacity independently.

Default head length is max(4×stroke width,1.5mm), and width is max(0.9×head length,stroke width). Closed heads inherit the line color without an outline; open heads inherit outline color and width, with no fill. Short and zero-length lines retain full head size; ink.bounds includes the actual region. Zero length has no shaft; without heads, round/square caps may draw a dot or square.

A zero-length line uses its head or cap geometry for its layout frame, independently of opacity. A headless butt cap reserves a stroke-width layout frame without producing ink. Start and end remain coincident, with zero arc length. Use `anchor=self.path.end` to align that point to the target and keep it fixed when changing head size.

The logical path retains its full centerline: pointed tips coincide with start/end, and the stroke enters the head interior. An endpoint envelope clips exposed stroke near each tip after constructing the complete stroke; the shaft is not shortened to its first centerline intersection. Dot, diamond and bar heads remain centered on the endpoint. Overlap uses the head paint, with equal paints unioned to prevent dark translucent seams. Closed hollow heads keep an empty interior; open V heads retain the interior shaft. Curve heads use one-sided endpoint tangents and diagnose undefined direction. Each open subpath receives the endpoint style; decorated paths containing closed subpaths are rejected.

## workflow

Complete sources and executable verification fixtures for this workflow are listed in the [feature coverage map](feature-map.en.md). Follow this page’s input conditions and limits when composing features.
