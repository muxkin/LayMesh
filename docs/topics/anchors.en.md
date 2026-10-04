# Placement, geometric anchors and path locations

<!-- walkthrough:start -->
## Purpose and concepts

Placement states which source point meets which target point and how much offset to add. Nine-point defaults belong to layout bounds. Rounded-rectangle and ellipse box corners often lie off the outline; use path or ink.boundary queries and explicitly index candidates.

## Minimal complete example

Run this file directly with `laymesh validate` or `laymesh render`; it contains its own canvas and required definitions.

<!-- example:examples/manual/path-at.lay -->

## Parameters and default behavior

Unitless geometry uses the canvas unit; unitless type and stroke sizes use pt. Explicit call parameters override inherited/theme defaults. The linked interface reference lists accepted types, choices and defaults per parameter.

## Composition

Target a curve’s arc-length midpoint with text and use its tangent_angle for rotation. offset_space=target applies tangent/normal offsets; choose incoming or outgoing at a corner first.

<!-- example:examples/gallery/positioning/anchors.lay -->

## Common errors and limits

A candidate collection cannot be passed directly to target. Indexing an empty collection errors; continuous overlap and infinitely many nearest points are not arbitrarily sampled. self binds only in anchor, and unplaced material cannot serve as a target.

## Individual functions

### instance-data

Select an original-data point on a placed chart. Axis transforms, breaks, instance transforms and group replay resolve the reference again; the point must be in a valid visible domain.

Returns: anchor

[Minimal complete source](../../examples/manual/instance-data.lay) · [Composition source](../../examples/gallery/positioning/chart-parts.lay) · [All parameters](interface-reference.en.md#instance-data)

### instance-axis

Select a named chart axis at its numeric start, physical midpoint or numeric end. Legacy transforms are retained; axes[name].spine.path provides explicit spine queries.

Returns: anchor

[Minimal complete source](../../examples/manual/instance-axis.lay) · [Composition source](../../examples/manual/geometry-composition.lay) · [All parameters](interface-reference.en.md#instance-axis)

### add

Place material in a canvas or group. Align its anchor to a target with an offset. Reuse the same material with independent size and styling for each placement.

Returns: A placed instance with measured dimensions and anchors for subsequent placement.

Required inputs: `material`.

[Minimal complete source](../../examples/manual/add.lay) · [Composition source](../../examples/gallery/positioning/anchors.lay) · [All parameters](interface-reference.en.md#add)

### ray

Create a query ray with an origin and nonzero direction. The origin shares the path container; direction uses the selected measurement space.

Returns: ray

Required inputs: `origin`, `direction`.

[Minimal complete source](../../examples/manual/ray.lay) · [Composition source](../../examples/manual/geometry-composition.lay) · [All parameters](interface-reference.en.md#ray)

### path-at

Select by arc-length fraction or distance. Original parameter t requires segments[i]. Lengths use placed geometry by default.

Returns: path_anchor

[Minimal complete source](../../examples/manual/path-at.lay) · [Composition source](../../examples/manual/geometry-composition.lay) · [All parameters](interface-reference.en.md#path-at)

### path-nearest

Return all globally nearest path positions in source order. Explicit indexing is required; infinitely many nearest points are diagnosed.

Returns: anchor_collection

Required inputs: `to`.

[Minimal complete source](../../examples/manual/path-nearest.lay) · [Composition source](../../examples/manual/geometry-composition.lay) · [All parameters](interface-reference.en.md#path-nearest)

### path-extrema

Return local coordinate extrema in the selected space, excluding ordinary endpoints and constant intervals.

Returns: anchor_collection

[Minimal complete source](../../examples/manual/path-extrema.lay) · [Composition source](../../examples/manual/geometry-composition.lay) · [All parameters](interface-reference.en.md#path-extrema)

### path-inflections

Return smooth inflections where signed curvature changes sign.

Returns: anchor_collection

[Minimal complete source](../../examples/manual/path-inflections.lay) · [Composition source](../../examples/manual/geometry-composition.lay) · [All parameters](interface-reference.en.md#path-inflections)

### path-corners

Return nonsmooth joining nodes; choose direction explicitly with with_side.

Returns: anchor_collection

[Minimal complete source](../../examples/manual/path-corners.lay) · [Composition source](../../examples/manual/geometry-composition.lay) · [All parameters](interface-reference.en.md#path-corners)

### path-intersections

Return all path positions intersecting a ray. Preserve distinct self-intersection occurrences and diagnose continuous overlap.

Returns: anchor_collection

Required inputs: `ray`.

[Minimal complete source](../../examples/manual/path-intersections.lay) · [Composition source](../../examples/manual/geometry-composition.lay) · [All parameters](interface-reference.en.md#path-intersections)

### path-between

Select a continuous interval in traversal order. Endpoints must belong to the same instance and subpath.

Returns: geometry_path

[Minimal complete source](../../examples/manual/path-between.lay) · [Composition source](../../examples/manual/geometry-composition.lay) · [All parameters](interface-reference.en.md#path-between)

### path-in_space

Choose the measurement space for lengths, nearest points and features. Resulting anchors remain in the current container.

Returns: geometry_path

[Minimal complete source](../../examples/manual/path-in_space.lay) · [Composition source](../../examples/manual/geometry-composition.lay) · [All parameters](interface-reference.en.md#path-in_space)

### anchor-with_side

Choose the incoming or outgoing direction at a corner. The positive normal is the left side along traversal.

Returns: path_anchor

[Minimal complete source](../../examples/manual/anchor-with_side.lay) · [Composition source](../../examples/manual/geometry-composition.lay) · [All parameters](interface-reference.en.md#anchor-with_side)

<!-- walkthrough:end -->

## Detailed behavior and further examples

### Three geometry views

| View | Meaning | Use |
| --- | --- | --- |
| `bounds` | Existing axis-aligned layout box, excluding stroke | Layout, alignment and spacing |
| `path` | Closed shape outline or logical line centerline | Connections, curve annotations and nodes |
| `ink` | Fill and stroke region, including caps, dashes and arrowheads | Drawing-edge placement and occupied bounds |

The nine shortcuts such as `placed.top_left` retain their meaning and equal `placed.bounds.top_left`. `path.bounds` and `ink.bounds` tightly enclose their respective geometry. **Box corners need not lie on the shape.** Select a box corner and a rounded rectangle or ellipse boundary separately:

```lay
page.add(dot, anchor=center, target=placed.bounds.top_left)
page.add(dot, anchor=center,
         target=placed.path.nearest(to=placed.bounds.top_left)[0])
page.add(dot, anchor=center,
         target=placed.ink.boundary.nearest(to=placed.bounds.top_left)[0])
```

An arrow's `path` represents the complete logical centerline, including the portion under its head. A line's `path.bounds` may have zero height; `ink.bounds` includes width, caps and the head. These views cover native vector geometry. Images and text retain `bounds`; no pixel or glyph outlines are extracted.

### Nodes, segments and points along a route

```lay
route = curve.path.subpaths[0]
route.start
route.end
route.nodes[2]
route.segments[1].controls[0]  # May lie off the curve; has no path direction
route.at(fraction=0.5)         # Half the subpath's arc length
route.at(distance=10 mm)      # Arc length from the start
route.segments[1].at(t=0.5)   # Original curve parameter, not arc-length fraction
route.between(route.nodes[1], route.nodes[3]).at(fraction=0.5)
```

Nodes and segment indices follow original geometry rather than rendering subdivisions. One `arc_to` remains one logical segment. Continuous traversal across multiple subpaths requires selecting `subpaths[i]` first. Closed paths preserve their start and traversal direction; `start` and `end` can coincide. Crossing the closed seam requires `between(..., wrap=true)`. Interval endpoints must belong to the same instance, geometric path and subpath; coincident instances cannot exchange endpoints.

The default measurement space is the current container: nonuniform resizing changes lengths, nearest points and extrema. `route.in_space("local")` measures original local geometry; `in_space("parent")` restores container measurements. Results always become anchors in the current container. Bounds shortcuts use the rectangle in the selected measurement space.

### Searches always return collections

```lay
route.nearest(to=another.bounds.center)[0]
route.extrema(axis="y")[0]
route.inflections()[0]
route.corners()[0]
route.intersections(ray(origin=another.bounds.center, direction=(1, 0)))[0]
```

Explicit indexing is required before passing a collection to `target`, even for one candidate. No matches produce an empty collection; invalid indices report `E_INDEX`. Check `len(candidates)` or iterate with `for point in candidates`. Candidates sort by subpath, original segment and parameter. Shared joining nodes are deduplicated; distinct path positions at a self-intersection remain distinct. Continuous overlap and infinitely many globally nearest points report `E_GEOMETRY` rather than inventing a finite sample.

`extrema` finds local coordinate extrema in the selected space, excluding ordinary endpoints and constant intervals. `inflections` finds smooth signed-curvature changes; `corners` finds nonsmooth joining nodes. Rays select their forward half only. Their nonzero direction is interpreted in the path's selected measurement space.

### Source selectors, direction and offsets

```lay
point = placed.path.at(fraction=0.5)
page.add(curve, anchor=self.path.start, target=point,
         offset=(2 mm, 3 mm), offset_space="target",
         rotation=point.tangent_angle)
```

`self` is a symbolic source root, bound only in `anchor` after sizing and transforms. Source selectors cannot depend on another instance. Existing nine-point names and `anchor="start"/"end"` preserve their transform semantics.

Path anchors expose `tangent`, `normal` (two dimensionless components), and `tangent_angle`. The positive normal is the left side along traversal: a rightward tangent has an upward normal in page coordinates. Ambiguous corner or cusp directions require `point.with_side("incoming")` or `with_side("outgoing")`; unspecified direction queries report `E_ANCHOR_DIRECTION`. `start/end` use outgoing/incoming direction respectively. Position alone requires no direction.

| Parameter | Meaning | Default |
| --- | --- | --- |
| `anchor` | Layout anchor name or `self` geometry selector | `top_left` |
| `target` | Placed-instance anchor in the same container | Container top left |
| `offset` | Two physical-length components | `(0 mm, 0 mm)` |
| `offset_space` | `container`: container axes; `target`: target tangent and left normal | `container` |
| `rotation` | Rotation about the instance center, including `point.tangent_angle` | `0deg` |

### Chart components

```lay
chart.plot_area.bounds.top_left
chart.axes["x"].bounds.bottom_center   # Complete spine, ticks and title component
chart.axes["x"].spine.path.at(fraction=0.5)
chart.axes["x"].label.bounds.center
chart.axes["x"].min                    # Numeric domain minimum
chart.axes["x"].max                    # Numeric domain maximum
chart.data(x=2, y=1.8)
```

Axis shortcuts select the complete component layout box; select `spine` for the axis line alone. Horizontal, vertical and top/bottom/left/right axes share this model. `min/max` name numerical domain endpoints and preserve their meaning on reversed axes. A broken spine has multiple subpaths, so select one before continuous traversal. Existing `plot_*` and `axis(name="x",anchor="center")` syntax preserves its transform semantics.

References retain instance identity, component and selection conditions. Group replay and chart reflow recompute paths, components, data anchors and directly read measurements such as `tangent_angle`.

[Complete interface](interface-reference.en.md) · [Units and sizes](units.en.md) · [Transforms](transforms.en.md) · [Groups](groups.en.md)

## geometry-model

The reference lists every view member and method. Original identity and measurement rules are described above; select `.subpaths[i]` before continuously traversing a compound path.

The example below reads the instance, layout box, original path and painted region separately. The same nine-point interface also works on plot areas, complete axes, labels, ticks and exponent labels. Red dots mark layout frames, blue dots mark the painted boundary, and yellow dots mark control points and axis labels. Select controls with `.segments[i].controls[j]`; they provide positions and usually lie off the curve.

<!-- example:examples/manual/geometry-members.lay -->

`point.x` and `point.y` return physical coordinates in the current container. `point.tangent` and `point.normal` return two dimensionless components, accessible with `[0]` and `[1]`. Multiply by a physical length to compute an offset; the positive normal follows the left side of traversal. `target=point` retains the anchor reference; numeric pairs can be used for `offset`. See the [composition source](../../examples/manual/geometry-composition.lay) for dependent placement.

[Geometry view types](interface-reference.en.md#geometry-view-types)

## workflow

Complete sources and executable verification fixtures for this workflow are listed in the [feature coverage map](feature-map.en.md). Follow this page’s input conditions and limits when composing features.
