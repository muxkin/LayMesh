# Documentation build and GitHub Pages

Bilingual Markdown, a navigation manifest and an example registry generate the site. Articles use the full available width; page contents float above them. Source is readable inline and previews use WebP rendered from real examples.

## Local build

Use Rust 1.93.1, Python 3.11+, the wasm32-unknown-unknown target and wasm-bindgen CLI 0.2.129:

```sh
python -m pip install -r release/docs-requirements.txt
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129 --locked
python scripts/build-docs.py
python scripts/build-docs.py --check
```

Output lives in `site/dist/`. Relative page and asset links support a project prefix such as `/LayMesh/`. The Python build consumes checked-in high-resolution media; the published static site needs no Python runtime.

## Update content and media

The [navigation manifest](navigation.json) defines areas, groups and topics. The [example registry](../scripts/site_support.py) reuses the bilingual gallery catalogs and connects complete source and dependencies. Topic Markdown lives in `docs/topics/`; overviews live in `docs/sections/`.

```sh
cargo build --release --locked -p laymesh-cli
python -m pip install -e './python' pillow
python scripts/build-gallery.py --write
python scripts/build-docs-media.py
python scripts/build-gallery.py --check
python scripts/build-docs.py
python scripts/build-docs.py --check
```

`python scripts/build-docs-media.py` rerenders .lay sources at 3840 pixels wide and produces lossless WebP at widths of 960, 1920 and 3840. Notebook examples use saved `.lay` source and assets; to regenerate assets from their Python recipes, first run `python scripts/build-notebook-gallery.py --write`. Original bitmaps remain limited by their source resolution.

Maintain `site/media/` alongside source; do not commit `site/dist/`. The media manifest fingerprints source, dependencies and renderer configuration. Stale or missing previews block builds. Icons remain SVG. Complete static and animation formats are documented in [Export](../docs/topics/export.en.md) and [Animation export](../docs/topics/animation-export.en.md).

The source area displays complete files directly, without extracting summaries. File tabs switch between project files. Images always correspond to complete source.

## Manual generation and verification

Style and animation entry pages link to independently maintained chapters. api.json generates the API reference; the same metadata and pinned libraries generate the property reference and all 97 effects. Chapter prose stays outside generated markers, so regeneration does not append duplicate explanations.

```sh
python scripts/build-api-reference.py --check
python scripts/build-topic-walkthroughs.py --check
python scripts/build-style-animation-docs.py --check
python scripts/check-feature-docs.py
python scripts/check-gallery-docs.py
python scripts/build-docs-media.py --check
python scripts/build-docs.py --check
node scripts/test-docs-manual.cjs SITE_URL OUTPUT_DIRECTORY
```

After metadata changes, run the generators without --check, then rebuild the native engine, media and website. The browser script uses existing Playwright/Chromium to verify one shared preview, compilation of all 97 selections, real playback, static restoration, mobile width and theme editing. PLAYWRIGHT_MODULE and CHROMIUM_PATH select installed runtimes. Bind previews to Tailscale and keep QA artifacts outside the repository.

The complex layout and theme are maintained as a Lay program using components, loops and anchors. `node scripts/build-dashboard-data.mjs --check` checks synthetic data only; runtime needs no Node. `node scripts/test-docs-manual.cjs SITE_URL OUTPUT_DIRECTORY --cases-only` checks anchor reflow after a width edit, theme edits, real presentation, advance/back and static restoration in the three projects.

## Editing and live previews

Examples share one workspace: code on the left and the figure on the right at widths of 900px or more; narrower workspaces stack code above the figure. Drag the divider or use its arrow keys to resize; double-click or press Enter to reset the split. The page contents panel always overlays the article.

All tutorials display full source immediately, ready to edit. File tabs switch between project files while preserving drafts and the current preview. Run and font controls appear for editable files; Python remains read-only.

The near-black preview stage can switch to white or a checkerboard without changing the source-defined canvas or exports. Fit, zoom, pan and expanded workspace controls share the same preview for WebP and SVG. Background, split and zoom choices stay in page memory. The preview toolbar has a Present button for fullscreen and a dropdown with Windowed presentation. Escape ends fullscreen directly. Regular preview supports clicks, context menus and arrow keys; initial entry waits for a click, then automatic links continue normally. The status footer shows this run’s total time; Details separates compilation, drawing commands, rasterization, initialization and resource loading. Errors open diagnostics and label the retained image and timing as the last successful result.

Full native source, modules and CSV/JSON files are editable. A browser Worker recompiles 250ms after typing stops. “Run now” skips the delay; “Restore original” resets all files and undo history. Source and figure reflect the current draft. Python/Notebook source remains read-only.

Edits and user-loaded fonts remain in page memory, without uploads or browser storage. Reloading, reopening and back/forward cache restoration reset edits. Errors retain a labeled last-successful result and identify source locations. Computation exceeding 10 seconds can be stopped and retried.

The timing bar uses actual `performance.now()` measurements for compilation, drawing commands, rasterization, initialization and resource preparation. Debounce and resource waits are excluded from computation timings. Browser measurements are not CLI benchmarks.

`python scripts/build-docs.py` builds Rust/WASM with Cargo and copies the locked static editor modules, Worker and example resources. The Page alone ships DejaVu Sans and Noto Sans CJK SC body fonts and their licenses; pinned FontTools 4.62.1 extracts characters from every example and dependency to subset Noto while preserving all 10 faces, order, names and layout features. The build checks coverage and SHA-256. Content-addressed fonts load from the same origin alongside WASM and the resource manifest. The first source edit loads complete fonts on demand. Edits made before fonts are ready retain the current image, then compile only the latest draft. The fonts do not enter wheels or the CLI. The preview needs no server runtime or Python. The shared Rust decoder handles TIFF and other image assets, while the registry preserves original source paths. Compilation only accesses registered resources.

Full source always displays; source editor modules load when the workspace approaches the viewport. Animation plays, pauses and seeks directly; switching file tabs preserves playback. Pages open in complete static layout, restored with Static. Play requests made during loading start at step one after the first frame is ready; failed loads can be retried. Details includes preparation and first-frame times measured from navigation on the current device.

Playback follows the browser refresh clock, retaining only one in-flight frame and the latest pending frame. Playback always retains native screen pixel density; step holds reuse their boundary frame to avoid redundant rasterization. The timeline, Rust/WASM interpolation and export quality are unchanged. Details counts distinct animation frames at browser refreshes and shows the recent motion-frame rate.

Run interaction regressions and five cold/warm samples with `node scripts/test-browser-animation-startup.cjs URL OUTPUT_DIR --corpus` (existing Playwright installation; optionally set `PLAYWRIGHT_MODULE` and `CHROMIUM_PATH`). `--corpus` compares raw RGBA checksums for every example using startup versus complete fonts.

## Enable Pages once

In **Settings → Pages → Build and deployment → Source**, select **GitHub Actions**.

- Push to `main`: build, check, then publish on success.
- Pull request: build and check only.
- Manual run: choose `docs-pages` in Actions and select `main`.

The workflow uses Rust 1.93.1 and Python 3.13, runs `python scripts/build-docs.py` and `python scripts/build-docs.py --check`, then uploads `site/dist/`. Official Pages Actions deploy through the `github-pages` environment without cancelling an in-progress deployment.

Check **Actions → docs-pages** for build and deployment results, and **Settings → Pages** for the final URL. Project sites use the repository name as a prefix, such as `/LayMesh/`. Local verification does not establish that a live deployment has succeeded; verify the first workflow run after enabling Pages.

## Privacy and preview

Public documentation, source, image metadata and diagnostics must not contain personal directories, device addresses, credentials or private data. Deployment excludes dependency caches and temporary output; checks reject user absolute paths. Existing open-source copyright attribution is preserved.

Bind preview services to the local Tailscale interface. Keep device addresses and browser QA artifacts out of Git commits and release packages.

## Pinned library catalogs

`python scripts/build-library-docs.py` extracts all classes, variables and keyframes from the pinned upstream CSS using the compiler parser, probes supported consumers, and compiles/renders or samples every usable recipe. `--check` compares the complete inventory, source locations, pinned hashes, engine fingerprint and verified examples. Run it before rebuilding media and the site. The Bootstrap, Open Props and Animate pages each use one live WASM workspace; unsupported entries stay visible with reasons.
