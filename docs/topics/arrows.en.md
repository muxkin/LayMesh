# Filled arrows

<!-- walkthrough:start -->
## Purpose and concepts

Filled arrows are single closed silhouettes. Construct a material and place it with add. arrow() is straight; independent constructors such as arrow.arc() share paints, borders and effects.

## Minimal complete example

Run this file directly with `laymesh validate` or `laymesh render`; it contains its own canvas and required definitions.

<!-- example:examples/manual/arrow.lay -->

## Parameters and default behavior

Unitless geometry uses the canvas unit; unitless type and stroke sizes use pt. Explicit call parameters override inherited/theme defaults. The linked interface reference lists accepted types, choices and defaults per parameter.

## Composition

Arc and custom-path arrows append centered triangular heads along shaft endpoint tangents. Custom paths retain the complete curve; arcs reserve head angles within the total sweep. Connect final tips or unheaded ends using start/end in add; shaft_width interpolates along shaft arclength.

<!-- example:examples/gallery/shapes/arrows.lay -->

## Common errors and limits

Radius, total sweep, head dimensions and endpoint distance must agree. Missing positive shaft solutions, coincident endpoints, full-circle sweeps, invalid widths and self-intersections error. centerline includes head axes; path is the closed outline.

## Individual functions

### arrow

Create a reusable filled straight arrow. Connect anchors using start/end in add, and style with fill, border_* and effects.

Returns: Reusable filled arrow material.

[Minimal complete source](../../examples/manual/arrow.lay) · [Composition source](../../examples/gallery/shapes/arrows.lay) · [All parameters](interface-reference.en.md#arrow)

### arrow-arc

Append straight triangular heads along an arc shaft’s endpoint tangents. The total sweep includes heads; connections solve for the complete endpoints.

Returns: Reusable filled arrow material.

[Minimal complete source](../../examples/manual/arrow-arc.lay) · [Composition source](../../examples/gallery/shapes/arrows.lay) · [All parameters](interface-reference.en.md#arrow-arc)

### arrow-bent

Create an L-shaped arrow with centerline span and corner radius, optionally connected between anchors.

Returns: Reusable filled arrow material.

[Minimal complete source](../../examples/manual/arrow-bent.lay) · [Composition source](../../examples/gallery/shapes/arrows.lay) · [All parameters](interface-reference.en.md#arrow-bent)

### arrow-uturn

Create a U-turn arrow. Endpoint connections transform its centerline while retaining physical shaft and head dimensions.

Returns: Reusable filled arrow material.

[Minimal complete source](../../examples/manual/arrow-uturn.lay) · [Composition source](../../examples/gallery/shapes/arrows.lay) · [All parameters](interface-reference.en.md#arrow-uturn)

### arrow-chevron

Create a single-headed notched arrow whose tail and shaft share one continuous closed silhouette.

Returns: Reusable filled arrow material.

[Minimal complete source](../../examples/manual/arrow-chevron.lay) · [Composition source](../../examples/gallery/shapes/arrows.lay) · [All parameters](interface-reference.en.md#arrow-chevron)

### arrow-path

Preserve a complete custom open path as the shaft and append straight triangular heads along its outward endpoint tangents; supports variable width and independent head sizes.

Returns: Reusable filled arrow material.

Required inputs: `path`.

[Minimal complete source](../../examples/manual/arrow-path.lay) · [Composition source](../../examples/gallery/shapes/arrows.lay) · [All parameters](interface-reference.en.md#arrow-path)

<!-- walkthrough:end -->

## Detailed behavior and further examples

## Geometry and placement

`arrow()` creates straight arrows. Use `arrow.arc()`, `arrow.bent()`, `arrow.uturn()`, `arrow.chevron()` and `arrow.path()` for other templates. Every constructor returns a reusable closed shape styled with `fill`, `border_*` and `effects`. Line endpoint heads continue to use `line(..., end_head=head(...))`.

```lay
page=canvas(size=(160mm,110mm))
a=page.add(rect(size=(20mm,14mm),fill="#dde5f2"),offset=(10mm,60mm))
b=page.add(rect(size=(20mm,14mm),fill="#dde5f2"),offset=(120mm,60mm))
page.add(arrow(shaft_width=(2mm,5mm),fill="#087f8c"),start=a.middle_right,end=b.middle_left)
page.add(arrow.arc(sweep_angle=90deg,fill="#ee784b"),start=a.top_center,end=b.top_center)
page.add(arrow.arc(radius=-80mm,fill="#5273c5"),start=a.bottom_center,end=b.bottom_center)
```

Local arcs require a positive `radius`, `start_angle` (default 0°) and a signed `sweep_angle`. The radius belongs to the arc shaft centerline; start_angle is the polar angle of the final logical start around the circle center. sweep_angle is the total polar sweep including both heads. Page angles point right at 0° and down at 90°; positive sweeps are clockwise, and absolute sweeps above 180° and below 360° create major arcs.

Straight triangular heads extend outward along the shaft endpoint tangents, with each base centered on its neck. A head of length `L` on a shaft of radius `R` occupies `atan(L/R)` of polar angle. Subtract both head angles from the total sweep to obtain the arc shaft sweep, which must remain positive. An unheaded end uses `L=0`.

Endpoint connections accept a sweep alone to solve the radius and center, or a signed radius alone: positive selects a clockwise minor arc, negative a counterclockwise minor arc. The solver uses the complete endpoints including heads. Their distances from the circle center are `sqrt(R²+L_start²)` and `sqrt(R²+L_end²)`; the minor-arc semicircle limit is their sum, replacing the bare-arc half-chord constraint. When both radius and sweep are specified, the radius must be positive and the complete endpoint distance must match. Constraints are never silently changed. If a sweep produces multiple radii with positive shaft sweeps, choose the larger radius before checking the silhouette. Connections determine starting orientation without mutating the reusable material.

A bent centerline passes through `(0,h)`, `(0,0)`, `(w,0)`; a U-turn continues to `(w,h)`. `span=(w,h)` defaults to `(30mm,20mm)` and `corner_radius` to one quarter of the smaller span; zero makes sharp corners. Chevron `notch_depth` defaults to the tail shaft width. Custom paths accept one open `line/arc/path/polyline` material; paint and endpoint styles on that source are ignored. The complete source curve becomes the shaft. Its endpoints are necks: append the end head along the final tangent and the start head against the initial tangent. Heads do not shorten the source curve, so leave layout space for their additional length.

Straight connections recompute length and direction; arcs solve the constraints above. Other templates and custom paths translate, rotate and uniformly scale their shaft centerline, keeping physical shaft and head dimensions. Custom-path connections include the appended head lengths in the endpoint equation; choose the larger positive scale when multiple solutions exist. start/end refer to the final tips or unheaded ends. Ordinary `add(size=...)` scales the entire shape using existing placement rules. Straight, bent, U-turn and chevron dimensions retain their previous meanings.

Connections retain `start_offset/end_offset`, per-end `*_offset_space`, and overall `offset`. They are mutually exclusive with explicit `anchor/target/rotation/size`. Endpoints must belong to the same container and group replay rebinds their references. See [anchors and placement](anchors.en.md).

## Variable widths and heads

`shaft_width=3mm` is constant; `(2mm,5mm)` tapers linearly; `[(0,2mm),(0.5,6mm),(1,3mm)]` supplies width stops. Positions measure normalized shaft arclength after excluding heads, must increase strictly from 0 to 1, and interpolate linearly between stops.

`heads` accepts `end` (default), `start` or `both`; chevrons require one head. `head_size=(longitudinal length,transverse width)` sets a shared size; `start_head_size/end_head_size` override it individually. Both dimensions default to twice the corresponding neck width. Head width cannot be less than neck width. Tips stay on logical endpoints and follow endpoint tangents; heads and shaft form one closed silhouette. Arc and custom-path head bases are centered on the neck and perpendicular to the shaft endpoint tangent, with symmetric shoulders; the curved shaft is not clipped against an off-center head base.

Short paths, undefined cusp directions, invalid widths and self-intersecting silhouettes produce diagnostics. The engine does not shrink heads or change specified widths to conceal invalid geometry.

## centerline

Instance `.start/.end` are logical endpoints. `.centerline` retains the open generating path and supports existing path queries such as `.at(fraction=0.5)`, `.segments`, `.length` and tangents. For arcs and custom paths, centerline includes the shaft and appended straight head axes. Its length includes the head lengths, and its endpoint tangents agree with the final tips. Custom-path source curves are preserved; a start head shifts their segment indices by one, and an end head adds the last segment. Source segments do not become render samples. `.path` is the closed shape outline, `.ink` its painted region, and `.bounds` its layout frame.

The [anchor example](../../examples/gallery/shapes/arrow-connections.lay) positions its sweep label at the centerline midpoint.

## LCSS arrow styles

All templates share the `arrow` type selector. `arrow.demo` selects arrows with class `demo`; it does not name a constructor. Styles include `shaft-width`, `heads`, `head-size`, `start-head-size`, `end-head-size`, and existing paints, borders and effects. Size pairs can use `8mm 10mm`; width stops use the list syntax.

<!-- example:examples/manual/arrow-style.lay -->

## Export and compatibility

Arrow subjects stay vector in SVG/PDF; blur and glow use existing effect handling. Plain PPTX arrows remain editable custom vector geometry. Special effects may invoke local raster fallback with `W_PPTX_RASTER`; PowerPoint preset adjustment handles are not provided.

`arrow()` now means a filled shape. Legacy calls containing `line_*`, line caps or `start_head/end_head` are identifiable as line arrows; a safe migration fix is offered only when every parameter applies to `line`. Geometry-only old calls using `length/angle` or `dx/dy` cannot be distinguished automatically and now use filled-shape semantics. Migration never rewrites new namespaced constructors or user-defined names.

[Templates](../../examples/gallery/shapes/arrows.lay) · [Styles and effects](../../examples/gallery/shapes/arrow-effects.lay)
