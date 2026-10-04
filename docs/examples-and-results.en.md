# Examples and actual execution results

> Historical Node baseline only (`codex/node-baseline`). For the current Rust installation, font policy and commands, see [installation](topics/install.en.md) and [architecture](architecture.en.md).

This page records a local Linux run of the repository's v0.2.0 implementation on **2026-09-26**. Commands run from the repository root; `npm test` builds the packages itself. The recorded environment was Node.js `v24.13.1`, npm `11.19.1`, and Python `3.13.11`. Values below are **local observations**, not inferred results from Windows or remote Ubuntu CI.

## 1. Inspect the image, then edit the code

The [feature gallery](gallery/README.en.md) has 44 focused examples under [images](gallery/images.en.md), [positioning](gallery/positioning.en.md), [groups](gallery/containers.en.md), [drawing](gallery/shapes.en.md), [typography](gallery/typography.en.md), and [scripting](gallery/scripting.en.md). [Notebook output](gallery/notebook.en.md) and [complete applications](gallery/comprehensive.en.md) have separate pages.

![Actual PNG-input gallery render](../examples/gallery/images/png-preview.png)

| Source | Functions and capabilities | Actual output | Observed `validate` dimensions |
| --- | --- | --- | --- |
| [hello.lay](../examples/hello.lay) | `canvas/rect/text/add`, center anchor | [PNG](../examples/hello-preview.png) | 160 × 100 mm; 3 top-level placements |
| [basic.lay](../examples/basic.lay) | Image, shapes, text, arrow, group, independent crops and anchors | [PNG](../examples/basic-preview.png) | 180 × 120 mm; 7 placements |
| [functions.lay](../examples/functions.lay) | Built-ins, functions, loops, conditions, unit arithmetic | [PNG](../examples/functions-preview.png) | 120 × 55 mm; 3 placements |
| [scripted.lay](../examples/scripted.lay) | Imports/exports, defaults, loops, nested groups, module-relative assets | [PNG](../examples/scripted-preview.png) | 180 × 90 mm; 3 placements |
| [vector.lay](../examples/vector.lay) | Paths, presets, gradients, dashes, fusion | [PNG](../examples/vector-preview.png) | 180 × 120 mm; 9 placements |
| [outlines.lay](../examples/outlines.lay) | Single/double/triple outlines, dashes, opacity | [PNG](../examples/outlines-preview.png) | 180 × 110 mm; 13 placements |
| [typography.lay](../examples/typography.lay) | Text spans, fonts, wrapping, inline/display formulas | [PNG](../examples/typography-preview.png) · [PDF](../examples/typography.pdf) | 160 × 100 mm; 8 placements |
| [showcase.lay](../examples/showcase.lay) | Combined image, shape, path, text, component, and fusion features | [PNG](../examples/showcase-preview.png) · [SVG](../examples/showcase.svg) · [PDF](../examples/showcase.pdf) · [1200 DPI PNG](../examples/showcase-1200dpi.png) | 160 × 226.262626… mm; 1 top-level group |

Vector commands are exercised by the figures and `tests/vector.test.mjs`; exact parameters are in the [language reference](language-reference.en.md#vector-constructors). Preview images are reduced for fast display. The eight complete examples put images directly below their code in the [complete-applications gallery](gallery/comprehensive.en.md).

![Actual complete showcase render](../examples/showcase-preview.png)

## 2. CLI stdout and file dimensions

After building, validate each example:

```sh
npm run build
for f in examples/hello.lay examples/basic.lay examples/functions.lay examples/scripted.lay examples/vector.lay examples/outlines.lay examples/typography.lay examples/showcase.lay; do
  node packages/cli/dist/main.js validate "$f"
done
```

The actual CLI output is Chinese; it reports a valid path, physical size, and top-level instance count:

```text
有效：examples/hello.lay（160 × 100 mm，3 个顶层实例）
有效：examples/basic.lay（180 × 120 mm，7 个顶层实例）
有效：examples/functions.lay（120 × 55 mm，3 个顶层实例）
有效：examples/scripted.lay（180 × 90 mm，3 个顶层实例）
有效：examples/vector.lay（180 × 120 mm，9 个顶层实例）
有效：examples/outlines.lay（180 × 110 mm，13 个顶层实例）
有效：examples/typography.lay（160 × 100 mm，8 个顶层实例）
有效：examples/showcase.lay（160 × 226.26262626262627 mm，1 个顶层实例）
```

Export the basic figure:

```sh
node packages/cli/dist/main.js render examples/basic.lay -o basic.svg
node packages/cli/dist/main.js render examples/basic.lay -o basic.pdf
node packages/cli/dist/main.js render examples/basic.lay -o basic.png --dpi 300
```

The three calls print `已导出：basic.svg`, `已导出：basic.pdf`, and `已导出：basic.png` (“exported”). The local run used a temporary directory and inspected files with `sharp`, `pdfinfo`, and `pdftotext`:

| File | Observed result | Check |
| --- | --- | --- |
| Basic SVG | Root `width="180mm" height="120mm" viewBox="0 0 180 120"` | Physical page and internal coordinates |
| Basic PDF | 1 page, `510.236 × 340.157 pt` | `180 × 120 mm × 72/25.4` |
| Basic PNG, 300 DPI | `2126 × 1417 px`, 300 DPI metadata | Per-axis rounding; post-arrow-fix SHA-256 `371aabf049894afd265537e6c02538025dcc43384119266fdf2be97f43d9135c` |
| Typography PNG, 150 DPI | `945 × 591 px`, 150 DPI | Layout remains 160 × 100 mm |
| Showcase PNG, 150 DPI | `945 × 1336 px`, 150 DPI | Physical aspect ratio |
| Showcase PNG, 1200 DPI | `7559 × 10690 px`, 1200 DPI | Print-resolution export |

The regenerated [print showcase](../examples/showcase-1200dpi.png) has post-seam-fix SHA-256 `34e191d363496f12c22933a526eedc078e5b097f5d0bd3e29ab4bcc3555f2fe5` in this run. Verify local files with:

```sh
pdfinfo basic.pdf | rg 'Pages:|Page size:'
pdftotext examples/typography.pdf -
```

Other measured PDF page sizes: `scripted.pdf` 510.236 × 255.118 pt; `vector.pdf` 510.236 × 340.157 pt; `outlines.pdf` 510.236 × 311.811 pt; `typography.pdf` 453.543 × 283.465 pt. The stored [typography.pdf](../examples/typography.pdf) was also regenerated from current source.

## 3. Observable functions and text

[functions.lay](../examples/functions.lay) uses `append([2,4],6)`, `for`, `range(3)`, and `while` to reach `total=18`. `min/max/abs` retain mm units; `twice(2 mm)` gives `4 mm`. Run:

```sh
node packages/cli/dist/main.js render examples/functions.lay -o functions.pdf
pdftotext functions.pdf -
```

Actual extractable text:

```text
Functions and control flow
len=3 total=18 ready
min=3 mm max=8 mm abs=4 mm twice=4 mm
```

![Actual function and control-flow render](../examples/functions-preview.png)

The 150 DPI PNG is 709 × 325 px. Further PDF extraction confirms content from components, outlines, and typography:

```text
scripted.pdf:    Card 1 | Card 2 | Card 3
outlines.pdf:    SINGLE | DOUBLE | TRIPLE | CUSTOM
typography.pdf:  文字排版 / Typography
                 行内公式：\frac{a+b}{\sqrt{x}} 与文字共用基线。
                 \int_0^1 x^2\,dx = \frac{1}{3}
```

`pdftotext` retrieves ordinary text and the original formula LaTeX. Visible SVG formulas remain paths. Open [typography.pdf](../examples/typography.pdf) to inspect selection behavior in your PDF reader.

## 4. Node API execution

Run this from the repository root after `npm run build`:

```js
import { parse, compileFile } from '@laymesh/core';
import { renderSvg, renderPdf, renderPng } from '@laymesh/render';
import { readFile } from 'node:fs/promises';

const ast = parse(await readFile('examples/basic.lay', 'utf8'), 'examples/basic.lay');
const scene = await compileFile('examples/basic.lay');
const [svg, pdf, png] = await Promise.all([
  renderSvg(scene), renderPdf(scene), renderPng(scene, 300)
]);
console.log('statements:', ast.length);
console.log('scene:', scene.width, scene.height,
            scene.nodes.length, scene.warnings.length);
console.log('formats:', svg.startsWith('<svg'),
            pdf.subarray(0, 4).toString(), png.subarray(1, 4).toString());
```

Actual output with `node --input-type=module`:

```text
statements: 17
scene: 180 120 7 0
formats: true %PDF PNG
```

`parse` returned 17 top-level statements. Compilation produced seven top-level placements and no warnings, and all renderers returned the expected format data. See the [API contract](api-reference.en.md).

## 5. Python bridge execution

With a built CLI and importable `laymesh` Python package:

```python
from laymesh import render_source

r = render_source(
    r"""page=canvas(size=(20 mm,10 mm))
box=rect(size=(5 mm, 5 mm),fill="#087f8c")
page.add(box,target=page.top_left)
"""
)
print('preview:', r.preview_svg.startswith('<svg'))
print('output:', r.output)
print('saved_source:', r.saved_source)
```

Observed with `PYTHONPATH=python python3`:

```text
preview: True
output: None
saved_source: None
```

The bridge returned a preview without persisting files. Python tests cover Figure vector/raster fallback, bindings, source protection, and magics; both [actual Figure outputs](gallery/notebook.en.md) are stored in the repository. See the [Notebook guide](python-jupyter.en.md).

## 6. Automated tests and failures

```sh
npm test
python3 -m unittest discover -s python/tests -v
```

The recorded run passed **41/41 Node** and **9/9 Python** tests. Node covers assets, units/layout, modules, groups, fusion, outlines, fonts/formulas, located diagnostics, arrow heads, and round seams. Python covers bindings, Figure SVG/PNG adaptation, saved-source protection, and magics. One Python Figure test intentionally emits `W_PLOT_BOUNDS` for content beyond the Figure canvas and still passes.

An example failure is `node packages/cli/dist/main.js validate /tmp/no-such-layout.lay`, which exits **1** and writes a located `E_FILE` message such as:

```text
/tmp/no-such-layout.lay:1:1: E_FILE: 无法读取布局文件：/tmp/no-such-layout.lay
```

Invalid units, missing assets, unsupported SVG/TIFF, loop limits, and fusion failures have nonzero exits or `LayError` assertions in `tests/*.test.mjs`. The [CI workflow](../.github/workflows/acceptance.yml) configures Windows and Ubuntu, but a local Linux pass is not a remote platform result.

## 7. Focused gallery acceptance

`node scripts/build-gallery.mjs --check` runs CLI `validate` and **150 DPI PNG export** on all [44 focused `.lay` files](gallery/README.en.md), checks physical and pixel dimensions, compares preview pixels, and checks both language pages. In the recorded run, 44/44 succeeded at **120 × 80 mm and 709 × 472 px** with no compile or render warnings. Each category page presents exact source, commands, raw validation output, and actual image.

![Actual path-command output from the gallery](../examples/gallery/shapes/path-commands-preview.png)

`python scripts/build-notebook-gallery.py --check` runs the Python bridge. The line plot saves `.svg` with no fallback; the heatmap saves `.png` and reports `Matplotlib 图 fig 已改用 240 DPI PNG` (“Figure fig switched to a 240 DPI PNG”). Both final pages are 709 × 472 px at 150 DPI; the fallback plot asset uses 240 DPI. See [the two actual pages](gallery/notebook.en.md).

Spot checks of [SVG input](../examples/gallery/images/svg.lay), [paths](../examples/gallery/shapes/path-commands.lay), [display formulas](../examples/gallery/typography/display-formula.lay), [colored spans](../examples/gallery/typography/spans.lay), and the [Notebook vector plot](../examples/gallery/notebook/vector.lay) found no PDF image objects with `pdfimages -list`. `pdftotext` extracted `\int_0^1 x^2\,dx=\frac{1}{3}` and `COLOR IN ONE LINE`; `pdfinfo` reported **340.157 × 226.772 pt** for the 120 × 80 mm formula page. Their visual layouts were inspected.

`python scripts/check-gallery-docs.py` checks source links, inline image placement, focused entries, and extra examples across both languages. The checks reject reader-facing internal Scene version identifiers. No formal release was performed.

## 8. Arrow and round-seam regression

The arrow shaft now ends at the head base; a short arrow draws only its head, while `end` still denotes the tip. Round fusion uses local arcs instead of asymmetric probe patches. The [arrow](gallery/shapes.en.md#line-arrow) and [fusion](gallery/shapes.en.md#fuse) images were regenerated. The latter aligns the rectangle midpoint and line start at 45.5 mm.

Regression tests cover horizontal, diagonal, reverse, short, dashed, compound-outline, and fused arrows; aligned, angled, and zero-gap round junctions; and preservation of remote holes. SVG, PDF, and PNG exports of the gallery cases were visually compared. `pdfimages -list` found no bitmap objects. Gallery checking compares actual pixels, so an outdated same-size preview cannot pass.

Additional cases joined horizontal/upward/downward lines to rectangles, ellipses, convex and concave Bézier curves. Twelve PNG cases stayed connected with no broken shaft; SVG and PDF remained vector paths. If a concave curve's bounding-box anchor misses its visible contour, round fusion returns a located `E_FUSE` rather than creating a thin accidental neck. Omitting `points` uses nearest visible-outline points.

![Actual angled line joining a straight boundary](../examples/gallery/shapes/fuse-angled-preview.png)

![Actual angled line joining a free curve](../examples/gallery/shapes/fuse-curved-preview.png)

Hollow rectangle joins were checked at the right-edge midpoint and upper-right corner for `round`, `miter`, and `bevel`. Preserving PathKit's even-odd fill rule keeps the inner hole open after fusion. Both examples retain their hole and a connected outline. See the [edge](gallery/shapes.en.md#fuse-outline-edge) and [corner](gallery/shapes.en.md#fuse-outline-corner) sources and commands.

![Actual join at a hollow rectangle edge](../examples/gallery/shapes/fuse-outline-edge-preview.png)

![Actual join at a hollow rectangle corner](../examples/gallery/shapes/fuse-outline-corner-preview.png)

## 9. Bilingual site and native plot verification (2026-09-27)

The test counts in sections 6 and 7 belong to the 2026-09-26 run. After adding the [native plot references](plotting.en.md), [measured benchmark](benchmarks/native-plots.en.md), and bilingual site, the current run passed **50/50 Node** and **12/12 Python** tests. `npm run gallery:check` revalidated and rerendered 44 focused examples at 150 DPI, compared committed preview pixels, and checked the Notebook SVG asset and heatmap PNG fallback. `python scripts/check-gallery-docs.py` checked **42 Markdown pages, 196 inline images**, and both languages of all 44 focused and 10 extended examples. `npm run docs:build && npm run docs:check` checked **42 static HTML pages, 248 image references**, local links, anchors, and language switches.

Validating the [native plot source](../examples/plot/scientific.lay) returned `有效：examples/plot/scientific.lay（180 × 82 mm，4 个顶层实例）`. Its newly rendered 180 DPI PNG is **1276 × 581 px**, pixel-identical and byte-identical to the committed preview. SVG and PDF were also exported. `pdfinfo` reported one **510.236 × 232.441 pt** page; `pdftotext` extracted `Experiment`, `Reference`, `Intensity`, `Value`, and `Time (s)`. The default PDF heatmap data layer is a **4 × 4 raster image**; axes, labels, and the other layers remain vector. Use `mode="vector"` for per-cell vector heatmap output.

```sh
npm run laymesh -- validate examples/plot/scientific.lay
npm run laymesh -- render examples/plot/scientific.lay -o scientific.png --dpi 180
npm run laymesh -- render examples/plot/scientific.lay -o scientific.pdf
```

![Actual native scientific plot export](../examples/plot/scientific.png)

At a 390 px browser viewport, the Chinese and English plot pages and English benchmark loaded their images without horizontal overflow; the plot-page language switch opened its counterpart. Regression tests also cover missing data, log axes, size reflow, portable saved layouts, clipping, overlapping scatter opacity, and nearest-neighbor heatmap pixel colors. The [benchmark](benchmarks/native-plots.en.md) compares native plotting, standalone Matplotlib, and the Figure bridge; its data supports reduced bridge overhead, not a general speed advantage over standalone Matplotlib. The site was built and previewed locally; no formal release was performed.

## 10. Fixed data area and annotations (2026-09-28)

Added `plot_area=box(offset=(left,top), size=(width,height))`, placed-instance `data(x=...,y=...)`, nine `plot_` data-rectangle anchors, and `anchor="start"/"end"` for lines and arrows. The [standalone example](../examples/plot/annotations.lay) fixes an **88 × 55 mm** data area with its top-left at **(26,20) mm** on the page. Its peak arrow, threshold label and title are separate editable elements. PNG, SVG and PDF were exported and checked; the PDF has one **135 × 100 mm** page, extractable text and no raster image objects.

This run passed **57/57 Node** and **12/12 Python** tests. Seven new tests cover locked data and label geometry after frame changes, font/tick changes, automatic and log mappings, annotations after domain changes, rotated anchors and arrow endpoints, containing-group scaling, and located errors. The existing `scientific.lay` 180 DPI PNG remains byte-identical to its preview. All 44 gallery examples and Matplotlib bridge checks passed. The documentation site validated **42 pages and 250 image references**; performance was not remeasured in this run.

A same-day update changed drawable space shortages, overlapping labels and frame overflow to `W_PLOT_LAYOUT`, retaining settings and continuing output. Margin warnings include the approximate additional mm needed on each side. Nonpositive data rectangles and invalid data remain errors. Node **61/61** tests passed, including cached placement warnings, successful CLI validation and three-format export, retention of crowded content, and final-layout warnings after automatic tick thinning. A Python bridge check displayed the warning while generating an SVG preview and PDF. Both `scientific` and `annotations` PNGs remain byte-identical to their previews.


## 11. Scientific styling and placement precision (2026-10-02)

Added explicit minor ticks, tick directions and lengths, tick-label visibility, major/minor grids, boxed axes, separate tick/label font sizes, and multi-column legends with physical positioning and optional transparent backgrounds. See the [plotting reference](plotting.en.md#scientific-styling-and-precise-placement). Panel data rectangles and placement anchors remain explicitly authored, without automatic alignment or cross-panel resizing. Legends never reserve automatic margins; insufficient space around fixed data rectangles retains geometry and emits a warning.

The [editable example](../examples/plot/publication.lay) exports PDF, SVG, and PNG with two exact **64 × 45 mm** data rectangles at page positions **(20,26) mm** and **(115,26) mm**, without layout warnings. PDF/PNG output was visually checked and PDF text extraction verified. This also exposed and fixed an unpainted PDF path left by transparent legend backgrounds that could be stroked by subsequent nodes.

`npm test` passed **67/67**. Six added regressions cover linear/log ticks, grid clipping and draw order, fixed panel and annotation coordinates, multi-column legends and cached placement warnings, hidden ticks, argument diagnostics, and PDF pixel equivalence with invisible shapes removed. Python `unittest` passed **12/12**. Existing `scientific` and `annotations` SVGs remain byte-identical to the pre-change output. The documentation build and check passed **42 pages and 252 image references**. Performance was not remeasured.

## 12. Markers, scientific labels, and colorbars (2026-10-02)

Added diamonds, downward triangles, transparent interiors, independent outlines, and optional markers for lines/error bars. Data and legend symbols share geometry; error-bar legends show stems and caps in their actual directions. Axis, layer, and colorbar labels accept existing text/formula materials. Tick formatting supports mathematical scientific notation and shared multipliers with physical offsets. Colorbars support four sides, manual coordinates, both orientations, independent length/thickness, and explicit ticks. New compilations use Scene schema 5; schema 4 solid point batches pass renderer compatibility checks. See the [plotting reference](plotting.en.md).

Three new editable examples export SVG, 180 DPI PNG, and PDF without layout warnings: [black-and-white markers](../examples/plot/markers.lay), [scientific labels](../examples/plot/scientific-labels.lay), and [four colorbar sides](../examples/plot/colorbars.lay). SVG/PNG and rasterized PDF were visually inspected for hollow symbols, formula scripts, mixed labels, horizontal/vertical strips, and combined legends. PDF extraction retains ordinary text and formula sources including `I-I_0`, `E(t)`, and multipliers.

Visual inspection also exposed an existing MathJax adapter bug: inline output could contain multiple SVGs, but only the first was retained, truncating formulas such as `I-I_0` and `E=mc^2` after an operator. Formulas now remain complete measured assets. A regression test and the [inline-formula gallery preview](gallery/typography.en.md#inline-formula) were updated.

Final tests passed **82/82 Node** and **13/13 Python**. Coverage includes exact fixed rectangles and adjacent-panel coordinates, original-value data anchors, repeated placement, rotations and group scaling, PNG/PDF marker bounds and opacity, legend order and directions, module-relative fonts, material widths/wrapping, negative/zero values, exponent rounding, very large/small numbers, explicit multipliers, color normalization, and insufficient-space warnings. Existing `scientific/annotations/publication` SVG and 180 DPI PNG outputs remain byte-identical to the baseline; PDF pixels rendered at 180 DPI also match exactly.

All 44 focused gallery examples and Notebook bridge checks passed. Markdown validation covers **42 pages and 206 inline images**; the built site validates **42 pages and 258 image references**. Panels remain manually positioned; insufficient fixed layout space warns without adjustment. Colorbars still bind one heatmap in their own plot, with linear normalization. Performance was not remeasured, and the previous benchmark is not used to infer performance of these additions.

## 13. Independent axis title and multiplier offsets (2026-10-02)

Added `axis(label_offset=(dx,dy))` alongside `exponent_offset` to control titles and multipliers independently. A short centered x-axis title shares the row below the ticks with the right-aligned multiplier. Only overlapping measured default bounds move the title below the multiplier, with a **1.2 mm** gap. Offsets apply after default positioning; manual overlaps and insufficient space retain settings with warnings. Displacements use chart-local physical coordinates and follow chart rotation and containing-group scaling. Colorbar rules and manual panel positioning retain their existing behavior. The [plotting reference](plotting.en.md) also explains page size, plot frame size, and margins/fixed data rectangles.

`npm test` passed **89/89** and Python `unittest` passed **13/13**. Seven new Node checks cover short, long and multiline titles, formula scripts, independent positive/negative offsets for both axes, cached placement overlap warnings, automatic/fixed margins, fixed data rectangles, adjacent panels, original-value/log data anchors, rotation, group scaling, and three-format export. Python coverage saves and rerenders native plot source containing both offsets.

The [scientific-label example](../examples/plot/scientific-labels.lay) was regenerated as SVG, 180 DPI PNG and PDF; the SVG-derived PNG and rasterized PDF were visually inspected. Its right panel shifts the title down 0.5 mm and multiplier up 0.5 mm. Both data rectangles remain **76 × 52 mm** at page positions **(30,32) mm** and **(134,32) mm**, with no layout warnings. PDF text and formula sources remain extractable. The other five examples, `scientific/annotations/publication/markers/colorbars`, retain byte-identical SVG and 180 DPI PNG output against this change's baseline; PDF pixels rendered at 180 DPI match exactly. Performance was not remeasured.

All 44 focused gallery examples and Notebook bridge checks passed. Markdown checking covers **42 pages and 206 inline images**; the site build and check passed **42 pages and 258 image references**.

## 14. Named axes, independent breaks, statistics and shared decorations (2026-10-02)

Implemented named axes/anchors, independent break intervals and physical segments, reversed/symlog/category ticks, bars/steps/areas/histograms/boxes/violins/ECDF, nonuniform heatmaps and gridded contours, per-point styles, five shared color norms, standalone legends/colorbars and `inspect --json`. Fixed data rectangles and manual panel placement remain intact.

Node **114/114**, Python **14/14**, with **48** gallery examples. Six default plots retain identical SVG/PNG bytes and PDF raster pixels. Five new examples export all three formats with visual checks. See [this run](benchmarks/cartesian-plots.en.md) for geometry, numerical and compatibility evidence plus fresh timing/RSS/file sizes, and the [reference](cartesian-plots.en.md) for APIs.
