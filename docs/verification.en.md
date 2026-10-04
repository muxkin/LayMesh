# Verification record

> Updated 2026-09-26: the dated SHA-256 values and “byte-identical” claims below describe historical runs before the arrow-head and round-seam fixes. For current images and hashes, use the [latest execution results](examples-and-results.en.md#8-arrow-and-round-seam-regression).

## Local run — 2026-09-24

- Platform: Linux x86-64; Node.js 24.13.1; npm 11.19.1.
- Fonts: bundled DejaVu Sans Book, version 2.37, TTF, and GFS Neohellenic, static OTF. Redistribution licenses are in `examples/assets/DejaVuSans.LICENSE` and `tests/assets/GFSNeohellenic.LICENSE`.
- Locked runtime dependencies used: Chevrotain 11.2.0, Sharp 0.34.5, fontkit 2.0.4, resvg-js 2.6.2, PDFKit 0.17.2, SVG-to-PDFKit 0.1.8. Exact transitive versions are in `package-lock.json`.
- Commands: `npm test`; `laymesh validate examples/basic.lay` and `examples/scripted.lay`; render both examples to SVG/PDF/PNG; PNG export tested at 96, 120, 150 and 300 DPI.
- Automated checks passed: 14/14, including the original 10. New checks cover three nested groups with resized parent and dependent anchors, scoped variables and unit arithmetic, conditions, `range` and list iteration, `while`, functions, module re-exports, assets relative to a Chinese-named component directory and a moved copy of it, TTF/OTF text, PNG/JPEG/SVG/TIFF assets, and located diagnostics for missing assets/exports, import cycles, unit errors, forbidden attributes, loop/function/group depth limits.
- Visual inspection: example PNG and a rasterized PDF page had matching object positions, rotation and order. A PDF containing imported SVG material rendered correctly; its structure contained no image XObject. `pdftotext` extracted the selectable demo text; `pdfinfo` reported a 510.236 × 340.157 pt page for 180 × 120 mm.
- Background simplification: replacing the explicit white paper rectangle with `canvas(background="#ffffff")` produced pixel-identical PNG output and retained the PDF page size and selectable text.
- Backward visual check: the current `examples/basic.lay` PNG at 300 DPI is byte-identical to the pre-extension baseline (`sha256 42c9b36cd123294df15a6a7dc8c5e6c08742dd5086e2d603ece86d7a285ee81f`).
- Component visual check: `examples/scripted.lay` exported to all three formats. PDF reports one 510.236 × 255.118 pt page; `pdftotext` extracts “Card 1”, “Card 2”, and “Card 3”. PNG and a 150 DPI PDF rasterization showed matching card/image/text/icon positions and order. A nested group at 50% opacity rasterized from PDF at half opacity. Static OTF embedded and `pdftotext` extracted its text; a variable OTF from the local system produced a located `E_FONT` diagnostic because PDFKit/fontkit could not embed it reliably.

Windows and WSL execution is not available in this local environment. The CI matrix runs the same acceptance tests on Windows and Ubuntu when the repository is pushed; those runs are pending.

## Vector extension — 2026-09-24

- Added PathKit WASM 1.0.0 for path normalization, stroke expansion and union. The `.lay` parser and CLI remain the same restricted interpreter.
- `npm test` passes 19/19: the original 14 plus five vector checks. These exercise all seven new path constructors, quadratic/cubic/elliptical arc commands, multiple contours and an evenodd hole, linear/radial gradients, custom dash arrays, colored text spans, reusable placements, explicit and nearest fusion, all three seam styles, group/module reuse, dashed curved fusion and located errors.
- `examples/vector.lay` validates and exports to SVG, PDF and 150 DPI PNG. Visual inspection of the PNG and rasterized PDF showed matching layout, fills, hole, dashed stroke, text colors and fusion order. `pdftotext` extracted “LayMesh vectors” from the vector PDF. The PDF uses a font file and vector paths, without image XObjects.
- A 40 × 20 mm radial gradient was compared between 150 DPI PNG and a 150 DPI PDF rasterization. Sampled interior RGB values differed by at most two levels; the mean difference was about 1.14 levels per channel, including antialiasing and the one-pixel page rounding difference.
- The 300 DPI `examples/basic.lay` PNG is still byte-identical to the pre-vector baseline: SHA-256 `42c9b36cd123294df15a6a7dc8c5e6c08742dd5086e2d603ece86d7a285ee81f`.
- The existing Ubuntu and Windows CI matrix now validates and renders `examples/vector.lay` in all three formats. Windows and WSL remain unverified locally; CI execution awaits a push.

## Reusable outline extension — 2026-09-24

- `outline(...)` adds reusable single, double and triple vector bands; preset or custom dashes; cap and join settings; and outline opacity independent from fill. Old `stroke` parameters remain available, while mixing both forms is a located error.
- `npm test` passes 24/24. The five new checks cover transparent gaps in a double outline, separate outline opacity, reuse, dashed lines and arrows, compound outlines in `fuse`, module exports, invalid style diagnostics and the existing `dot` material name in `basic.lay`.
- `examples/outlines.lay` validates and exports as SVG, PDF and 150 DPI PNG. Visual inspection of PNG and rasterized PDF showed matching single/double/triple bands and dash patterns; `pdftotext` recovered the title and all six labels. Its PDF is a one-page 510.236 × 311.811 pt vector document.
- `examples/basic.lay` at 300 DPI retains the original SHA-256 `42c9b36cd123294df15a6a7dc8c5e6c08742dd5086e2d603ece86d7a285ee81f`; the 150 DPI vector example PNG is also byte-identical to its pre-outline preview.
- CI now validates and renders the outline example on Ubuntu and Windows. Those Windows CI results are pending a push; local verification was Linux only.

## 16 cm showcase and 1200 DPI export — 2026-09-24

- `examples/showcase.lay` keeps its six-panel design on a 297 × 420 mm internal grid and scales the complete group to a 160 × 226.262626 mm page. Both instance width and height are specified so the scaling is uniform. It includes imported media and independent crops; shapes and gradients; complex paths, a hole and colored text spans; reusable outlines; nested groups, a loop and conditional sizing, relative anchors; and overlaid versus fused instances.
- `laymesh validate examples/showcase.lay` passed with one top-level group. SVG, PDF and PNG exports completed. The print PNG is 7559 × 10690 px with 1200 × 1200 pixels-per-inch metadata; the 150 DPI preview is 945 × 1336 px. `pdfinfo` reports one 453.543 × 641.374 pt page. The SVG declares 160 × 226.262626 mm and no external `href` targets.
- Visual inspection of the 1200 DPI PNG and a 150 DPI PDF rasterization showed the same arrangement, colors, gradients, text, outlines, holes, rotation and connector geometry. `pdffonts` reports embedded DejaVu Sans with Unicode mapping; `pdftotext` extracts the title, six section headings, colored span text and all three imported card labels. Imported SVG icons and drawn shapes stay vector in PDF; raster photos are embedded.
- `npm test` passed 24/24. A fresh 300 DPI render of `examples/basic.lay` retained SHA-256 `42c9b36cd123294df15a6a7dc8c5e6c08742dd5086e2d603ece86d7a285ee81f`. The CI matrix now validates and renders the showcase in all three formats, including 1200 DPI PNG, on Ubuntu and Windows; local visual checks were on Linux only.

## Typography and MathJax extension — 2026-09-24

- `mathjax@4.1.3` and `linebreak@1.1.0` were added. MathJax uses a local worker only when a formula is present; no TeX or Typst binary is required. The `mathjax` npm tarball unpacks to about 20 MB and its default `@mathjax/mathjax-newcm-font` dependency to about 49 MB; extra math fonts are separate packages.
- `npm test` passes 32/32. New tests cover reusable text at two widths, mixed spans, center/right/justified alignment, system font fallback with located CLI warnings, two-face TTC/OTC collections, module-relative span font paths, inline and display formulas, PDF LaTeX source extraction, invalid formulas and unavailable math fonts.
- `examples/typography.lay` validates without warnings on this Linux host. The 160 × 100 mm page renders to SVG, PDF and a 945 × 591 px PNG at 150 DPI. Poppler and MuPDF text extraction recover ordinary text and the original LaTeX source of both formulas; Chrome SVG display, PNG and a PDF rasterization show the same layout.
- The TTC fixture's second face deliberately renders `A` as a `B` outline. Chrome rendered its standalone SVG as `B`; PDF and PNG did as well. This exposed and then verified the fix for selecting a collection face in SVG/PNG.
- A fresh 300 DPI render of `examples/basic.lay` retains SHA-256 `42c9b36cd123294df15a6a7dc8c5e6c08742dd5086e2d603ece86d7a285ee81f`. Local validation was Linux only; Ubuntu and Windows CI jobs for the new example await a push.

## PDF formula selection fix — 2026-09-24

- The former PDF semantic layer put the entire LaTeX source in `/ActualText` on one invisible `M`. Poppler extracted the source, but MuPDF assigned the first replacement character the whole formula width and the remaining characters zero-width positions. This made mouse selection unreliable.
- The PDF now places the source characters as invisible text across the formula width while retaining `/ActualText` for exact source extraction. The typography test checks the PDF text stream, both formula kinds, and MuPDF character positions when `mutool` is installed. `npm test` passes 32/32; Poppler still extracts both complete sources.
- The updated `examples/typography.pdf` rasterizes pixel-identically to the previous PDF at 150 DPI. Edge and WPS selection still need a direct manual check; neither GUI reader was available in this headless session.
