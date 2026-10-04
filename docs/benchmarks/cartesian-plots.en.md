# Cartesian plotting verification and measured performance

> Historical Node baseline only (`codex/node-baseline`). For the current Rust installation, font policy and commands, see [installation](../topics/install.en.md) and [architecture](../architecture.en.md).

[中文](cartesian-plots.zh-CN.md) · [Reference](../cartesian-plots.en.md) · [Raw measurements](cartesian-plots.json)

Measured at `2026-10-02T04:36:38.114Z` on local Linux, Node `v24.13.1`, CPU `AMD Ryzen 7 8745H w/ Radeon 780M Graphics`. These are fresh observations, without inference from the earlier benchmark, remote CI or other plotting tools.

## Reproduction and methodology

```sh
npm run build
node scripts/benchmark-cartesian-plots.mjs
node scripts/check-plot-baseline.mjs
```

The `benchmark script`（`codex/node-baseline`） starts three fresh Node processes per case. Compilation includes parsing, statistics, mapping and layout. SVG, 180 DPI PNG and PDF are timed sequentially; PNG includes another SVG generation. Process wall time includes startup/module loading. Peak RSS covers the full child process, compilation and all exports, rather than an isolated backend. No points are removed or sampled. Times are medians, RSS is the maximum, and file sizes are first-run bytes converted to MiB.

| Case | Compile ms | SVG ms | PNG ms | PDF ms | Process ms | Peak RSS MiB | SVG / PNG / PDF MiB |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| scatter-50000 | 103.7 | 87.1 | 371.8 | 665.3 | 1559.6 | 555.2 | 8.550 / 0.248 / 3.913 |
| three-axes-independent-breaks | 460.9 | 5.2 | 2935.9 | 808.0 | 4539.8 | 457.1 | 10.725 / 0.060 / 4.173 |
| statistics-20000 | 194.2 | 3.9 | 75.8 | 80.6 | 674.9 | 248.0 | 1.747 / 0.024 / 0.166 |

Scatter uses 50000 color-valued points, 0.8 mm markers and individually painted opacity. The multi-axis case draws three 50000-point lines with a broken x axis, different breaks on two y axes and one continuous y axis. Statistics use 20000 deterministic observations for 40-bin density, a box, a violin and ECDF. Segment windows retain clipped geometry; dense paths can increase file size and PNG time.

## Geometry, numerical and output checks

- Node: 114/114; Python: 14/14. Commands: `npm test` and `PYTHONPATH=python python -m unittest discover -s python/tests -v`.
- Six frozen defaults: scientific, annotations, publication, markers, scientific-labels, colorbars. SVG/180 DPI PNG bytes match; PDF pages rasterized with the same Poppler at 180 DPI match pixel-for-pixel. Existing example artifacts are the frozen baseline, with [SHA-256 records](default-plot-baseline.json). Timestamp-bearing PDF bytes are not compared.
- Independent three-axis breaks, reversal/symlog, all named sides, lines/bars/fills/heatmaps/error bars crossing gaps, physical segment lengths/gaps, selected data/axis anchors, repeat placement, outer-frame overrides, rotation and group scaling.
- Hand-checkable histogram endpoints/weights/density integrals, R-7 quartiles/outliers, Gaussian KDE, duplicate ECDF, five color norms, D3 half-grid correction and missing-cell coverage.
- Four editable manual examples and the Python data example export all three formats. PNG and PDF raster pages receive visual inspection; SVG is checked through the PNG renderer. Pixel tests also cover gaps, opacity and marker stroke bounds; PDF text extraction checks plain labels and formula source.
- Merged missing-grid clipping rectangles preserve exact cell coverage without overlap, avoiding one complete copy of contour paths per valid cell.
- 48 gallery examples, Notebook checks and bilingual documentation builds/link/image checks. Constant-violin and Python out-of-bin/missing-data warnings are intentional.

Panels remain manually positioned. Memory and file-size requirements depend on the actual layers and output format; these measurements do not promise performance for all plots.
