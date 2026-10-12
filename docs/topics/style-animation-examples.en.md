# Styles and animation: complete projects

Start with [style and animation libraries](style-libraries.en.md): learn bundled libraries, sources and imports, then explore entries, parameters and composition. One `.lay` file is enough to begin.

[Style manual](lcss.en.md) · [Animation manual](animation.en.md)

Three complete projects progress from library composition to artistic text and a complex scene. Each includes editable `.lay`, themes, resources and a real WASM preview. Static images and animation use the same source. Inspect the complete layout, then choose Present: their first step uses `after-previous` and starts on entry.

| Project | Appearance | Motion |
| --- | --- | --- |
| [Library cards](#styled-cards) | Bootstrap colors, 2mm blur override, Open Props gradients/radii/shadows, inset shadow | Title entrance, staggered cards, stroke drawing, custom perspective emphasis |
| [Artistic poster](#art-direction) | Glyph gradients, outlines, vector extrusion, curved text, wave warp, shadows, glow, transparent image | Independent children, repeated alternating motion, perspective on the whole group |
| [Complex dashboard](#complex-dashboard) | 256 vector motifs, 12 plots, 24 equations, translucent panels and effects | Parallel loops, negative delays, perspective flips, 12 drawn strokes, finite window |

## styled-cards

### Three offline libraries and one project theme

The layout stores dimensions, placement, component content and steps. `styled-theme.lcss` manages appearance and keyframes together, explicitly importing Bootstrap, Open Props and Animate.css. `metric()` reuses the card structure. A loop reads three content records and assigns their appearance classes. The first card follows the note; each subsequent card connects to its predecessor's `top_right`. Text inside each group also follows `bottom_left` anchors.

The first combines `bg-primary rounded shadow custom` and changes only Bootstrap shadow blur to 2mm. The second uses Open Props variables for gradient, radius and layered shadow; the third uses an inset shadow. Descendant selectors such as `.metric .label` unify child text. Explicit parameter priority still applies.

<!-- example:examples/animation/styled-cards.lay -->

`instances = append(instances,item)` saves each placed instance. `append()` returns a new list, so assign its result. A second loop passes the instances to `animate()` with `delay=i*150ms` for staggered entrances. Changing card size or font size moves adjacent cards and subsequent text through their anchors, without editing individual page coordinates.

| Global time | Action |
| --- | --- |
| 0–0.6s | Title and note enter together; note has a 0.2s delay |
| 0.6–0.8s | Hold |
| 0.8–1.8s | Cards start 0.15s apart; the divider draws concurrently |
| 1.8–2.2s | Hold |
| 2.2–3.1s | The second card runs the theme's custom perspective motion |
| 3.1–3.5s | Hold, then retain the final state |

Edit `box-shadow-blur:2mm` in the theme: both static and animated shadows change. `box-shadow-spread:2mm` changes expansion instead. Edit `focus-card` to change motion. Explicit `duration` in an action overrides CSS timing; see [step parameters](animation-timing.en.md).

[Layout source](../../examples/animation/styled-cards.lay) · [Project theme](../../examples/animation/styled-theme.lcss)

## art-direction

### Animate the existing artistic text and effects examples

This project adapts the existing [artistic text example](../../examples/effects/art-text.lay) and [transparent image/group effects](../../examples/effects/images-groups.lay), moving repeated appearance into `art-direction.lcss`. The repository's `alpha-ring.png` needs no download.

<!-- example:examples/animation/art-direction.lay -->

`.hero` combines glyph gradient, white outline, two-dimensional vector extrusion and an existing `effects` shadow. `.curved` uses CSS text shadow, `.wave` uses wave warp and glow, and `.stamp` uses an alpha-based drop-shadow. The plate's box shadow follows a different outline. Each retains the blur semantics explained in [Shadows and effects](style-shadows.en.md).

The poster follows the page heading; its caption follows `bottom_left`, and the stamp aligns inside the plate's `bottom_right`. The curve stays in `.lay`. Path and path text share a local origin, and their group aligns with the plate. Control points describe geometry rather than each object's page placement. A draw action progressively shows the stroke. The three steps are:

1. Title entrance and stroke drawing run concurrently; curved text appears 0.2s after drawing ends.
2. Wave text moves vertically for two alternating iterations; the transparent image performs `flipInY` concurrently.
3. The placed poster group rotates in perspective, carrying every child and effect.

Total time is 7.6s, including holds. `float-wave` occupies 2.4s and `inspect-poster` occupies 2s. Glyph warp belongs to static appearance; playback `transform` moves the result without reshaping text. The last keyframe returns to zero rotation. Exit or Static restores the original design.

[Layout source](../../examples/animation/art-direction.lay) · [Project theme](../../examples/animation/art-direction.lcss) · [Transparent image](../../examples/effects/alpha-ring.png)

## complex-dashboard

### Organize a complex animation with loops

This project uses the content counts of `09-motion-3d` from the existing [stress-scene recipe](../../tests/skia/stress-scenes.mjs), rewritten as a maintainable Lay program. Component functions define structure; loops create objects; lists retain instances; anchors connect neighbours. Samples live separately in `dashboard-data.json`. These deterministic synthetic signals are illustrative data, not experimental results.

<!-- example:examples/animation/complex-dashboard.lay -->

| Content | Composition |
| --- | --- |
| 256 vector motifs | Gradient plate, ellipse and crossed stroke; every fourth motif rotates |
| 12 plots | Eight three-curve/scatter plots and four heatmaps; translucent card groups with shadows/glow |
| 24 equations | Nested formula cards with shadows, projected together with each card |
| 12 open strokes | Alternating draw loops |
| Title, plots and equations | Concurrent actions with negative delays to vary loop phases |

These counts describe input content, not lowered paths or measured frame rates. An explicit `duration=12s` bounds the step, allowing infinite loops to play, seek and export without changing their CSS iteration counts. This page has one step; Previous returns to its start. Use the first two projects to practise multiple steps.

Read the source in three layers:

1. `motif()`, `chart()` and `equation_card()` define reusable structures. The theme stores repeated appearance and keyframes.
2. `in_grid()` seeds only the first item. Each neighbour connects to the preceding item's `top_right`; a new row connects to the previous row's `bottom_left`. Equations follow the final chart row, and strokes follow the final equation row. `gap` controls spacing.
3. Creation loops collect placed instances. Animation loops traverse those lists, calculating negative delays from indices. `range(0,len(motifs),4)` selects every fourth motif for rotation.

`cycle_index()` uses a loop to cycle palettes and equations; current Lay has no `%` operator. Curves and heatmap matrices load from JSON, whose editor tab is directly editable. The Node maintenance script recomputes synthetic samples only; it does not generate or overwrite `.lay` or `.lcss`. Existing data ships with the project, so compilation and playback need no Node step.

~~~sh
laymesh validate examples/animation/complex-dashboard.lay
# Only needed when updating the synthetic data recipe:
node scripts/build-dashboard-data.mjs
~~~

This advanced stress scene puts more load on a device than the first two projects. Fit shows the whole layout; expand the workspace for plot/equation detail. Measure performance on the actual device; see [playback implementation and verification](animation-internals.en.md).

[Complete layout](../../examples/animation/complex-dashboard.lay) · [Project theme](../../examples/animation/dashboard-theme.lcss) · [Sample data](../../examples/animation/dashboard-data.json) · [Data maintenance](../../scripts/build-dashboard-data.mjs)

## Save, export and practise

Keep the theme and relative resources with the layout. The engine provides the three offline libraries, so projects need no copied upstream CSS. Static export shows the complete design; MP4/GIF uses the same source's continuous timeline.

~~~sh
laymesh validate examples/animation/styled-cards.lay
laymesh render examples/animation/styled-cards.lay -o cards.pdf --dpi 150
laymesh animate examples/animation/styled-cards.lay -o cards.mp4 --width 960 --fps 20
laymesh render examples/animation/art-direction.lay -o poster.png --time 4 --width 960
laymesh animate examples/animation/complex-dashboard.lay -o dashboard.mp4 --width 960 --fps 10
~~~

Change one factor at a time: blur from 2mm to 1mm, then card delay, then keyframe angles. Compare static layout, time snapshots and actual playback. Address import warnings, missing animation and effect overflow with [style troubleshooting](style-compatibility.en.md) and [animation troubleshooting](animation-export.en.md).
