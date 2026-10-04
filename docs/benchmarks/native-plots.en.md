# Native plot benchmark

> Historical Node baseline only (`codex/node-baseline`). For the current Rust installation, font policy and commands, see [installation](../topics/install.en.md) and [architecture](../architecture.en.md).

[中文](native-plots.md) · [Raw JSON](native-plots.json) · [Reproduction script](../../scripts/benchmark-plots.py) · [Plot reference](../plotting.en.md)

Measured locally on **2026-09-27**. Native LayMesh plotting reduced the cost of converting and bridging a Matplotlib Figure in these cases. The results do **not** establish a general speed or memory advantage over standalone Matplotlib. A vector preview with 100,000 scatter points still consumes substantial memory.

## Method and limits

- Host: AMD Ryzen 7 8745H, 16 logical CPUs, Linux 6.12.73, Node 24.13.1, Python 3.13.11, Matplotlib 3.10.8, NumPy 2.4.4.
- All paths read the same JSON data, use a 120 × 90 mm page and 74 × 46 mm plot area, DejaVu Sans at 8 pt, 0.6 pt lines, 3 pt scatter diameters, fixed ranges and ticks. Each produces an SVG preview and a 254 DPI PNG, checked at 1200 × 900 px.
- Lines and scatters contain 1,000 or 100,000 points; heatmaps are 64 × 64 or 512 × 512. No sampling was used. Matplotlib path simplification was disabled; heatmaps used nearest-neighbor display and the same viridis range.
- Cold startup launches a separate process each time, including imports and exit; the median of three runs is reported. Repeated generation warms a process once, then reports the median of three complete generations. Input reading, layout, SVG preview, and PNG writing are timed; original data preparation is excluded.
- Approximate peak RSS is the maximum sum over a process tree sampled every 10 ms. This may count shared memory more than once and is not an exact peak or a JavaScript heap measurement.
- The Figure bridge includes conversion, SVG compatibility checking, CLI output, and preview. Heatmaps trigger its existing whole-Figure PNG fallback; native heatmaps rasterize only the data layer. Equal output size does not imply pixel-identical rendering because typography, antialiasing, and SVG font encoding differ.
- File sizes below distinguish the final PNG from the in-memory SVG preview. Compression size alone does not measure data precision; embedded native fonts and Matplotlib glyph paths also affect SVG size.

## Results

Time in seconds, memory in MiB, file size in KiB. These are a small set of reproducible workflow measurements, not a stable latency distribution or cross-machine performance guarantee.

| Data | Workflow | Cold median | Repeated median | Approx. peak RSS | PNG | SVG preview |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| Line, 1,000 | Native LayMesh | 0.425 | 0.081 | 259.0 | 22.9 | 1012.6 |
| Line, 1,000 | Matplotlib | 0.347 | 0.040 | 75.8 | 44.8 | 34.7 |
| Line, 1,000 | Figure bridge | 1.331 | 1.030 | 229.6 | 24.3 | 45.6 |
| Line, 100,000 | Native LayMesh | 0.534 | 0.189 | 393.0 | 22.8 | 2892.3 |
| Line, 100,000 | Matplotlib | 0.421 | 0.109 | 113.3 | 44.7 | 2377.6 |
| Line, 100,000 | Figure bridge | 1.644 | 1.324 | 312.2 | 24.3 | 3169.4 |
| Scatter, 1,000 | Native LayMesh | 0.437 | 0.087 | 237.1 | 31.3 | 1079.8 |
| Scatter, 1,000 | Matplotlib | 0.367 | 0.049 | 76.8 | 35.0 | 98.6 |
| Scatter, 1,000 | Figure bridge | 1.413 | 1.115 | 245.0 | 32.5 | 130.7 |
| Scatter, 100,000 | Native LayMesh | 1.240 | 0.914 | 766.4 | 22.4 | 9630.4 |
| Scatter, 100,000 | Matplotlib | 1.090 | 0.776 | 126.8 | 35.5 | 8725.6 |
| Scatter, 100,000 | Figure bridge | 8.526 | 8.261 | 1185.3 | 23.9 | 11633.4 |
| Heatmap, 64 × 64 | Native LayMesh | 0.419 | 0.084 | 229.0 | 36.3 | 1001.3 |
| Heatmap, 64 × 64 | Matplotlib | 0.371 | 0.050 | 80.8 | 31.8 | 23.3 |
| Heatmap, 64 × 64 | Figure bridge | 1.380 | 1.077 | 235.9 | 40.4 | 53.6 |
| Heatmap, 512 × 512 | Native LayMesh | 0.505 | 0.142 | 301.8 | 116.5 | 1080.1 |
| Heatmap, 512 × 512 | Matplotlib | 0.468 | 0.141 | 120.2 | 140.0 | 58.1 |
| Heatmap, 512 × 512 | Figure bridge | 1.499 | 1.188 | 272.1 | 122.7 | 163.4 |

For 100,000 line points, repeated native generation took 0.189 s versus 0.109 s for standalone Matplotlib and 1.324 s for the Figure bridge. For 100,000 scatter points, native took 0.914 s and about 766 MiB, versus 0.776 s and about 127 MiB for Matplotlib. Native plotting chiefly integrates drawing and layout, preserves extractable axis text, avoids whole-page heatmap fallback, and removes Figure conversion overhead. Large scatter memory use remains an optimization target.

## Reproduce

```sh
npm ci
npm run build
python -m pip install -e './python' pillow psutil
python scripts/benchmark-plots.py --repeats 3 --output docs/benchmarks/native-plots.json
```

The JSON stores individual observations and warnings. Re-run after changes to code, dependencies, fonts, or hardware. This Markdown records one run; the reproduction script does not rewrite its prose.
