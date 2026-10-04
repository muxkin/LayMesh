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

The [navigation manifest](navigation.json) defines areas, groups and topics. The [example registry](../scripts/site_support.py) reuses the bilingual gallery catalogs and connects source, summaries and dependencies. Topic Markdown lives in `docs/topics/`; overviews live in `docs/sections/`.

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

Maintain `site/media/` alongside source; do not commit `site/dist/`. The media manifest fingerprints source, dependencies and renderer configuration. Stale or missing previews block builds. Icons remain SVG; CLI export formats remain SVG, PDF and PNG.

Summaries and full source come from the same file. `# BEGIN DEMO` / `# END DEMO` identify the named summary region; other examples declare the main region in the registry. Images always correspond to complete source.

## Editing and live previews

Examples share one workspace: code on the left and the figure on the right at widths of 900px or more; narrower workspaces stack code above the figure. Drag the divider or use its arrow keys to resize; double-click or press Enter to reset the split. The page contents panel always overlays the article.

Click a summary line (or focus the summary and press Enter) to edit full source at the corresponding location. Selecting and copying summary text does not switch modes. Existing drafts are preserved; the summary continues to show the original figure. Run and font controls appear in editable source mode; Python remains read-only.

The near-black preview stage can switch to white or a checkerboard without changing the source-defined canvas or exports. Fit, zoom, pan and expanded workspace controls share the same preview for WebP and SVG. Background, split and zoom choices stay in page memory. The status footer shows this run’s total time; Details separates compilation, SVG, initialization and resource loading. Errors open diagnostics and label the retained image and timing as the last successful result.

Full native source, modules and CSV/JSON files are editable. A browser Worker recompiles 250ms after typing stops. “Run now” skips the delay; “Restore original” resets all files and undo history. The summary always shows the original example. Python/Notebook source remains read-only.

Edits and user-loaded fonts remain in page memory, without uploads or browser storage. Reloading, reopening and back/forward cache restoration reset edits. Errors retain a labeled last-successful result and identify source locations. Computation exceeding 10 seconds can be stopped and retried.

The timing bar uses actual `performance.now()` measurements for compilation, SVG generation, initialization and resource preparation. Debounce and resource waits are excluded from computation timings. Browser measurements are not CLI benchmarks.

`python scripts/build-docs.py` builds Rust/WASM with Cargo and copies the locked static editor modules, Worker and example resources. The Page alone ships DejaVu Sans and Noto Sans CJK SC body fonts and their licenses; the Worker prefers a CDN download verified by SHA-256 and falls back to the Page copy. The fonts do not enter wheels or the CLI. The preview needs no server runtime or Python. The shared Rust decoder handles TIFF and other image assets, while the registry preserves original source paths. Compilation only accesses registered resources.

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
