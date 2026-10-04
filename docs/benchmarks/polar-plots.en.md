# Polar extension verification and measured performance

[中文](polar-plots.zh-CN.md) · [Feature reference](../polar-plots.en.md) · [Raw measurements](polar-plots.json)

This extension implements native polar/radar plots, logarithmic statistical baselines, cross-axis decoration diagnostics and preset warning controls. Automated tests cover fixed geometry, reflected radii, seam handling, physical clipping and original-value anchors.

## Verification

- Full Node suite: 137 passed, none skipped; Python: 18 passed.
- Gallery: 54 examples and Notebook checks passed; Markdown: 52 pages; static docs: 52 pages, 296 image references and internal anchors checked.
- Six original plot examples: SVG/PNG bytes and PDF pixels at 180 DPI match the frozen baseline.
- Six new examples export SVG/PDF/PNG with editable .lay, Python/JSON and inspect results; all three formats visually inspected.
- PNG/PDF pixel checks cover circles, sectors, holes, reflected grids and missing cells; periodic fills do not accumulate alpha. PDF text/formula extraction is checked.
- Layers and anchors share analytical projection; local tessellation tolerance is 0.001 mm without sampling away observations.

## Local measurements

Measured: `2026-10-02T05:55:49.910Z`; Node `v24.13.1`; `linux/x64`; `AMD Ryzen 7 8745H w/ Radeon 780M Graphics`; PNG 180 DPI.

Each case uses three fresh Node processes. Timings and sizes are medians; RSS is the maximum process peak across runs, including dependencies, fonts and sequential exports. Process time includes startup and is not the sum of measured stages. These are fresh measurements, not extrapolations from previous benchmarks.

| Case | Compile ms | SVG ms | PNG ms | PDF ms | Process ms | Peak RSS MiB | SVG MiB | PNG KiB | PDF MiB |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| scatter-50000 | 106.04 | 74.80 | 365.75 | 659.37 | 1544.17 | 545.47 | 8.55 | 253.94 | 3.91 |
| three-axes-independent-breaks | 478.33 | 6.76 | 2941.37 | 821.81 | 4579.14 | 577.87 | 10.72 | 61.01 | 4.17 |
| statistics-20000 | 196.61 | 3.84 | 75.11 | 82.14 | 665.13 | 270.19 | 1.75 | 24.61 | 0.17 |
| polar-scatter-50000 | 114.92 | 92.33 | 353.29 | 695.13 | 1602.76 | 499.07 | 8.60 | 250.54 | 4.05 |
| polar-field-128x128 | 316.02 | 49.23 | 246.33 | 423.92 | 1380.23 | 460.28 | 9.80 | 554.98 | 1.45 |
| radar-ten-series | 82.77 | 2.92 | 76.28 | 26.12 | 502.38 | 189.30 | 0.98 | 130.09 | 0.01 |

Reproduce with `node scripts/benchmark-polar-plots.mjs`. Deterministic inputs cover 50k colored points, three 50k-point lines with independent breaks, 20k statistical samples, 50k polar points, 128×128 annular cells and ten radar series. SVG embeds fonts, so even small figures include font overhead.

Performance depends on data and format; no universal speedup is claimed. Broken axes expand multiple mapping windows, and polar fields retain vector cells. File size and peak memory should be considered alongside timing.
