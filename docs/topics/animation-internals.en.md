# Playback implementation and verification

For maintainers. User controls are documented in [Presentation](animation-presentation.en.md).

Compiled content and resources are deduplicated and retained with valid document/page versions. Caches do not evict by a fixed capacity or page count. Replaced unreferenced versions, deleted pages and closed documents release their resources. Reusable content is built directly without rehearsal playback. Requesting an unready page retains the current picture and raises that page’s priority; its first frame starts the clock. Pause and menus retain drawing caches; exit releases drawing resources.

The offline editor uses a persistent compiler Worker pool. It commits the current page first, then schedules remaining work using browser parallelism; stable page ownership reuses semantic sessions, fonts and drawing baselines. The standalone site editor also prioritizes the current page and prepares later pages in parallel. VS Code schedules native compilation using machine parallelism and retains valid page results; edits invalidate obsolete tasks and prioritize the current preview. Current-page drawing and other-page preparation use separate Workers so animation and preparation can progress together. Playback starts once the starting page matches the latest source; save and export require the entire document to succeed. Object textures follow actual projection density, which is checked during playback. Custom animations can still require first-use construction without reducing display precision. Tests separately report compilation, resource preparation and texture rebuilds during first and repeat playback.

| Area | Supported behavior |
| --- | --- |
| Timing | `@keyframes`, `animation`, all eight animation longhands; defaults `0s`, `ease`, `none`; negative delay, zero/fractional/infinite counts, four directions and fill modes, running/paused |
| Keyframes | Missing endpoints use the underlying property, duplicate offsets merge per property, segment easing, animation-list order replaces earlier values; keyframe `!important` is ignored |
| Easing | `linear`, ease variants, `cubic-bezier()`, `steps()` and jump positions; CSS `steps()` is unrelated to LAY `step()` |
| Cascade | Local `.css`/`.lcss` imports, `:root`, type/classes, descendant/child and class-attribute selectors, inherited `var()`, typed `calc()`, declaration `!important`, standard/`-webkit-` aliases without duplicate effects, `@charset` |
| Media | `screen`, `all`, `print` and `prefers-reduced-motion` conditions used by the pinned library; preview follows system reduced motion, videos default to screen/no-preference |
| Properties | Opacity replaces target opacity; ancestor opacity still multiplies. Transform functions, origin, visibility and backface visibility are sampled independently of layout |
| Units | Static and animated CSS share `canvas.layout_dpi`: `layout_dpi` px = 1in (default 96). Export DPI does not change physical lengths. Percentage translations and origins use the untransformed layout box |
| 3D | `headShake`, `flip`, `flipInX`, `flipInY`, `flipOutX`, `flipOutY`; Skia local layers and GPU projection, perspective clipping and hidden backfaces. General web 3D scene layout is outside this feature |

Rust and WASM share timing, interpolation and property sampling. Layout compiles only after content changes. CanvasKit retains vector commands and images; native exports use Skia CPU and SkPDF. The display buffer always follows the requested viewport and device pixel ratio, without a playback quality reduction. Details show buffer dimensions, backend, sample time, CPU draw submission, GPU query time when available, submission fps and presentation estimates. Browser refresh callbacks cannot prove hardware scan-out. Software GPU drivers such as SwiftShader/llvmpipe do not count as 4K60 hardware validation.

## Verification

`cargo test -p laymesh-core --test animation` checks the pinned library and timing semantics. `cargo test -p laymesh-render --test animation` renders all effects and verifies deterministic backward seeking and static restoration. Build the `animation_reference` example and web WASM bindings, then run `node scripts/test-animation-browser.cjs /tmp/animation-qa` with an installed Playwright module. The browser fixture compares native/WASM/CSS sampled properties and provides side-by-side frames for visual checks. `PLAYWRIGHT_MODULE`, `CHROMIUM_PATH` and `LAYMESH_TEST_HOST` select existing local test tools and the server binding.

The browser script checks 873 time samples across 97 effects and compares representative frames for all six non-planar effects. A mean RGB channel error of at most 3/255 allows for antialiasing differences. `python scripts/test-animation-export.py --binary target/release/laymesh` checks MP4/GIF frame counts, dimensions and duration for all 97 effects, plus cancellation and failure cleanup; FFmpeg and ffprobe are required. `python scripts/test-vscode-preview.py --output /tmp/vscode-animation-qa` exercises playback, seeking, static restoration and animation export in the actual extension.
