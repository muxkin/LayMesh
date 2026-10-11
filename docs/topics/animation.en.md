# Animation and Animate.css

## step

A presentation is attached to an otherwise static canvas. `page.step(name="Title", hold=1s)` appends a step. Steps run in creation order; actions inside one step run in parallel. A delay offsets an action within its step. `hold` defaults to `1s`. The automatic window is the latest finite effect end, including negative delays and fractional repetitions. Infinite or paused effects require `duration=...` on the step.

Open previews and ordinary exports show the complete static layout. Enter playback to enable the presentation. Preview controls provide play/pause, previous/next step, skip to the current window's boundary, replay, seeking, speed, and manual/automatic advance. In manual mode playback pauses at the window boundary. Automatic mode advances after the hold, including across document pages. Backward navigation resamples the target time. The first backwards-filling effect can preset its target before its step; later effects cannot pre-empt an earlier effect on the same property. Finished step windows are sampled at their boundary during holds. These are **LayMesh presentation rules**, not CSS animation timing rules.

## Presenting a document

Choose **Present** on a web example to enter browser fullscreen. The VS Code **Present** menu offers embedded playback and **Present in browser**. The browser reuses native compiled artifacts through a temporary, session-scoped service bound to the Tailscale address. Closing its browser page or preview session releases the service. Embedded playback requires no external browser.

Previews initially show the complete static layout. After the first frame, a separate compilation queue prepares the entire document and reports completed pages. Presentation starts as soon as the first page and its drawing resources are ready; remaining pages compile in the background. Navigation prioritizes a pending target and retains the current picture with the clock stopped until the target first frame completes. A failed future page does not interrupt the current slide; navigating to it offers retry. Preparation never plays an animation or advances its clock. Errors identify the page and location and offer retry/cancel. A running presentation keeps its document version; edits apply after exit.

Manual navigation is the default. The first advance starts step one; advancing during an animation completes that step and holds it. The next advance starts the following step. After all steps finish, advance again to change pages. Backward navigation samples step boundaries; going back from the page start shows the previous page's completed state. Static pages advance directly and hold for one second in automatic mode. Automatic playback, speed, seeking, page navigation and keyboard help live in the floating controls. Controls are hidden by default. Tab reveals them and cycles through buttons; clicking the slide dismisses them. Right-click → “Show controls” offers “Always visible”, “Show on hover”, and “Hidden”. Hover mode responds only near the bar at the bottom, not to motion elsewhere or page changes. Tab works in all three modes; the choice lasts for the current preview session. Menus suspend the clock and restore its previous state when closed. A hidden browser page pauses and requires manual resume on return.

| Input | Action |
| --- | --- |
| Click, Space, Enter, Right/Down, PageDown, N | Advance / complete the current animation step |
| Left/Up, PageUp, Backspace, P | Previous step boundary; previous page at page start |
| Number followed by Enter | Go to that page |
| Home / End | First page initial state / last page completed state |
| S | Pause / resume |
| B / W | Black / white screen; repeat to restore |
| Right click | Presentation menu |
| Esc | Close menu; exit when the menu is closed |

Shortcuts apply only inside the presentation and ignore key repeat. Exit restores the preview page, zoom, scroll and focus. Controls overlay the image rather than reserving a black strip. Slides retain their aspect ratio without cropping or stretching. Embedded playback fills the VS Code preview; the browser entry provides browser fullscreen.

Compiled content and resources are deduplicated. Artifacts beyond the memory budget spill to session-scoped IndexedDB or native temporary files. GPU caches favor the current and neighboring pages. Reusable content is built directly, without a rehearsal loop. Nonresident navigation retains the previous picture until the target's real first frame is ready, then starts the clock. Pause and menus retain drawing caches; exit releases drawing resources.

Editing previews use fixed one-line toolbars and status bars. Errors update a single count entry and preserve the last successful picture. Scrollable diagnostic and performance popovers open only on request, support copying and source navigation, and never resize the canvas or steal focus automatically.

After the first page, web preparation uses at most four independent compiler workers, or two on devices reporting at most 4 GiB memory, with one pending artifact per worker. The document is evaluated once and resources are deduplicated by ID. VS Code uses up to four lower-priority native processes, bounded by available cores and memory. Editing previews interrupt in-progress background compilation and block new background jobs; unfinished pages retry afterward while completed artifacts remain available; edits cancel obsolete preparation and restart it after the updated preview is painted. Finished compiler processes exit while artifacts remain session-scoped. Display preparation analyzes transforms, constructs object textures directly and waits for GPU completion without playing animations. Actual projection density is still checked on every frame: extrema outside the estimate or cache admission budget retain full drawing quality, so arbitrary custom effects can still require a first-use build. Test results separate compilation, resource preparation and first/repeat-play texture builds.

### Exit and displacement endpoints

Exit effects can remain hidden. For example, `s.animate(item, class="animate__animated animate__fadeOut", fill="both")` keeps zero opacity after completion; `fill="none"` restores the underlying value. Animate.css already declares `both` on `animate__animated`, so an explicit override is usually unnecessary. A subsequent effect on the same property follows normal animation ordering.

The final keyframe determines displacement. Define `@keyframes move { from { transform: translateX(0px); } to { transform: translateX(320px); } }`, then call `s.animate(item, name="move", duration=1s, fill="forwards")` to retain a displacement of 84.67 mm during presentation. Effects such as `pulse` have matching endpoints and return to their starting state. Presentation transforms do not alter static layout or other objects' anchors; leaving presentation restores the original complete layout. Rewind and seeking resample the target time.

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

### Click triggers and automatic playback

`step(start="click")` is the default click boundary. In an action, `start="step"` (default) starts with its step, `start="with-previous"` follows the previous action's earliest actual start, and `start="after-previous"` follows its latest end. `delay` is relative to this boundary. A CSS animation list counts as one action; timing includes CSS variables, media conditions, negative delays and fractional repetitions. Changing the preceding duration automatically reschedules dependent actions. Empty actions add no duration. An infinite or paused predecessor has no end: use a separate step with a finite window before an automatically triggered step.

```python
first = page.step(name="Click to explain", hold=500ms)
first.animate(title, class="animate__animated animate__fadeInUp", duration=600ms)
first.animate(chart, name="laymesh-reveal", duration=800ms, fill="both", start="with-previous")
first.animate(note, class="animate__animated animate__fadeIn", duration=400ms,
              start="after-previous", delay=200ms)
second = page.step(name="Continue automatically", start="after-previous", hold=1s)
second.animate(title, class="animate__animated animate__pulse", duration=800ms)
third = page.step(name="Wait for the next click")
```

`page.step(start="after-previous")` automatically begins after the preceding step's window and hold, including in manual mode. On the first step it starts on page entry. Skipping the current step, rewinding or seeking still pauses. These are LAY scheduling extensions, not CSS properties. Mark every step this way for automatic execution within a page; the completed page remains visible. **Automatic playback** in the presentation menu also advances across pages. Video export uses the same continuous windows and holds independently of click triggers. See the runnable [linked animation example](../../tests/presentation/linked.lay).

Wheel down advances and wheel up goes back within the focused presentation.
One continuous wheel/touchpad gesture produces one action; a gap of more than
180 ms starts another gesture. Menus retain native scrolling. Horizontal and
modified wheel input does not navigate. Editing-preview zoom is unchanged.

The [32-page academic deck](../../tests/presentation/academic/report.laydoc)
contains 24 narrative slides and 8 effect-comparison appendices, with 141 plots,
119 formulas and all 97 effects applied to mixed academic content. See its
[reproduction notes](../../tests/presentation/academic/README.md). Data are
analytical/computed teaching examples, not experimental measurements.

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
| 3D | `headShake`, `flip`, `flipInX`, `flipInY`, `flipOutX`, `flipOutY`; Skia local layers and GPU projection, perspective clipping and hidden backfaces. General web 3D scene layout is outside this feature |

Rust and WASM share timing, interpolation and property sampling. Layout compiles only after content changes. CanvasKit retains vector commands and images; native exports use Skia CPU and SkPDF. The display buffer always follows the requested viewport and device pixel ratio, without a playback quality reduction. Details show buffer dimensions, backend, sample time, CPU draw submission, GPU query time when available, submission fps and presentation estimates. Browser refresh callbacks cannot prove hardware scan-out. Software GPU drivers such as SwiftShader/llvmpipe do not count as 4K60 hardware validation.

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
