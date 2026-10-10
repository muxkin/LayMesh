# Animation and Animate.css

## step

A presentation is attached to an otherwise static canvas. `page.step(name="Title", hold=1s)` appends a step. Steps run in creation order; actions inside one step run in parallel. A delay offsets an action within its step. `hold` defaults to `1s`. The automatic window is the latest finite effect end, including negative delays and fractional repetitions. Infinite or paused effects require `duration=...` on the step.

Open previews and ordinary exports show the complete static layout. Enter playback to enable the presentation. Preview controls provide play/pause, previous/next step, skip to the current window's boundary, replay, seeking, speed, and manual/automatic advance. In manual mode playback pauses at the window boundary. Automatic mode advances after the hold, including across document pages. Backward navigation resamples the target time. The first backwards-filling effect can preset its target before its step; later effects cannot pre-empt an earlier effect on the same property. Finished step windows are sampled at their boundary during holds. These are **LayMesh presentation rules**, not CSS animation timing rules.

## animate

Import the upstream file without renaming or preprocessing it:

```css
/* motion.lcss */
@import "./vendor/animate.css";
.title-motion { --animate-duration: 800ms; }
```

```python
page = canvas(size=(160mm, 90mm), stylesheet="motion.lcss")
title = page.add(text("Results", class="title-motion"), offset=(10mm, 10mm))
s1 = page.step(name="Title", hold=1s)
s1.animate(title, class="animate__animated animate__fadeInUp")
s2 = page.step(name="Emphasis", hold=2s)
s2.animate(title, class="animate__animated animate__pulse", duration=600ms)
```

The target must be the instance returned by `add()`. Temporary action classes supply animation declarations, inherited custom properties, transform origin and backface visibility without changing the static layout. `name` references a keyframe directly. Optional overrides are `duration`, `delay`, `easing`, `iterations`, `direction`, `fill`, `vars` and `play_state`. Omitted values use computed CSS. For example, `vars={"--animate-duration": 800ms}`, `iterations=2.5`, or `iterations="infinite"`. Explicit LAY overrides take precedence over CSS, including `!important`.

Complete examples: [presentation.lay](../../examples/animation/presentation.lay), [motion.lcss](../../examples/animation/motion.lcss), and [two-page presentation.laydoc](../../examples/animation/presentation.laydoc).

## Compatibility

The target is the **97 effects in Animate.css v4.1.1**, not arbitrary browser stylesheets. The [unchanged fixture and SHA-256](../../tests/animation/vendor/README.md) identify the exact version. Its original MIT header is retained. Transitions are parsed but do not automatically animate layout edits.

| Area | Supported behavior |
| --- | --- |
| Timing | `@keyframes`, `animation`, all eight animation longhands; defaults `0s`, `ease`, `none`; negative delay, zero/fractional/infinite counts, four directions and fill modes, running/paused |
| Keyframes | Missing endpoints use the underlying property, duplicate offsets merge per property, segment easing, animation-list order replaces earlier values; keyframe `!important` is ignored |
| Easing | `linear`, ease variants, `cubic-bezier()`, `steps()` and jump positions; CSS `steps()` is unrelated to LAY `step()` |
| Cascade | Local `.css`/`.lcss` imports, `:root`, type/classes, descendant/child and class-attribute selectors, inherited `var()`, typed `calc()`, declaration `!important`, standard/`-webkit-` aliases without duplicate effects, `@charset` |
| Media | `screen`, `all`, `print` and `prefers-reduced-motion` conditions used by the pinned library; preview follows system reduced motion, videos default to screen/no-preference |
| Properties | Opacity replaces target opacity; ancestor opacity still multiplies. Transform functions, origin, visibility and backface visibility are sampled independently of layout |
| Units | Animation CSS `96px = 1in`; percentage translations and origins use the untransformed layout box. Existing LAY geometry units retain their original rules |
| 3D | `headShake`, `flip`, `flipInX`, `flipInY`, `flipOutX`, `flipOutY`; local RGBA projection, perspective clipping and hidden backfaces. General web 3D scene layout is outside this feature |

Rust and WASM share the timing/interpolation/rendering implementation. Layout and fonts compile when content changes; frame updates reuse the scene and raster caches. Two-dimensional effects remain vector drawing commands. Perspective targets flatten only their local subtree. Large projected layers have pixel limits with an explicit diagnostic.

## Extensions

`-laymesh-draw` and `-laymesh-reveal` are nonstandard properties ranging from `0` to `1`. Built-in `laymesh-draw`/`draw` and `laymesh-reveal`/`reveal` keyframes can be referenced directly or through your own preset class. Supply a duration and fill mode, e.g. `s.animate(line_instance, name="laymesh-draw", duration=1s, fill="both")`. Drawing applies to stroked paths; plot reveal narrows the data-area clip while leaving axes and legends visible. A general object's reveal uses its layout box.

## Export

FFmpeg must be available on `PATH`:

```sh
laymesh animate examples/animation/presentation.laydoc -o presentation.mp4
laymesh animate examples/animation/presentation.lay -o presentation.gif
laymesh animate examples/animation/presentation.lay -o reduced.mp4 --reduced-motion reduce
laymesh render examples/animation/presentation.lay -o frame.png --time 2.4 --width 1920
laymesh render examples/animation/presentation.lay -o step.png --step 2
```

MP4 defaults to width 1920 and 30 fps; GIF defaults to width 960 and 20 fps. Aspect ratio is retained; MP4 dimensions are rounded to even integers. Document pages are centered with padding using the first selected page's aspect ratio. Each page compiles once. Frames stream to FFmpeg with bounded memory; failed/cancelled exports remove their staging file and preserve an existing destination. Ctrl+C cancels CLI exports; the VS Code notification exposes cancellation. Audio is not included.

VS Code asks only for format and path; choose **MP4 Animation** or **GIF Animation**. The ordinary GIF option remains a static image. Encoding settings live under `laymesh.export.animation.*`. CLI overrides include `--width`, `--fps`, `--crf`, `--preset`, `--background`, `--reduced-motion`, `--media screen|print` and `--page`. Project `.laymesh.json` defaults can separate formats:

```json
{"export":{"animation":{"crf":18,"reduced_motion":false},"animation_formats":{"mp4":{"width":1920,"fps":30},"gif":{"width":960,"fps":20}}}}
```

`inspect --json` includes step names, start/window/hold times and animation targets, including reduced-motion timing. Static SVG/PDF/PNG exports ignore presentation time and print animation rules. Time/step snapshots are PNG only; no native PPTX effects, shape morphing or data interpolation are implied.

## Verification

`cargo test -p laymesh-core --test animation` checks the pinned library and timing semantics. `cargo test -p laymesh-render --test animation` renders all effects and verifies deterministic backward seeking and static restoration. Build the `animation_reference` example and web WASM bindings, then run `node scripts/test-animation-browser.cjs /tmp/animation-qa` with an installed Playwright module. The browser fixture compares native/WASM/CSS sampled properties and provides side-by-side frames for visual checks. `PLAYWRIGHT_MODULE`, `CHROMIUM_PATH` and `LAYMESH_TEST_HOST` select existing local test tools and the server binding.

The browser script checks 873 time samples across 97 effects and compares representative frames for all six non-planar effects. A mean RGB channel error of at most 3/255 allows for antialiasing differences. `python scripts/test-animation-export.py --binary target/release/laymesh` checks MP4/GIF frame counts, dimensions and duration for all 97 effects, plus cancellation and failure cleanup; FFmpeg and ffprobe are required. `python scripts/test-vscode-preview.py --output /tmp/vscode-animation-qa` exercises playback, seeking, static restoration and animation export in the actual extension.

Specifications: [CSS Animations](https://www.w3.org/TR/css-animations-1/), [CSS Transforms](https://www.w3.org/TR/css-transforms-1/), [CSS Easing](https://www.w3.org/TR/css-easing-1/).
