# LayMesh

**Plot data and compose precise, reproducible scientific figures in readable code.** Combine native plots, images, vector shapes, text and formulas in one `.lay` file, then export the same layout as SVG, PDF or PNG at your chosen DPI.

[Documentation](https://muxkin.github.io/LayMesh/en/) · [中文](README.md) · [Feature gallery](docs/sections/examples.en.md) · [Installation](docs/topics/install.en.md)

![LayMesh scientific figures and precise layouts](site/media/showcase-1920.webp)

[Showcase source](examples/showcase.lay) · [Scientific plot examples](docs/gallery/plots.en.md)

## Install

Requires **Python 3.10+**. The release candidate is **`0.3.0a2`** (Rust engine `0.3.0-alpha.2`), an alpha prerelease. The first PyPI upload is pending; after publication, run:

```sh
python -m pip install --pre laymesh
python -m laymesh --version
```

Platform wheels bundle the native Rust engine and formula fonts for the CLI, Python API and Jupyter magics. Rendering requires no Rust compilation and downloads no engine or fonts. Until publication, install a [locally built wheel](release/README.en.md).

| Optional feature | Install command |
| --- | --- |
| NumPy / pandas data bindings | `python -m pip install --pre "laymesh[data]"` |
| Matplotlib Figure import | `python -m pip install --pre "laymesh[plot]"` |
| Both | `python -m pip install --pre "laymesh[data,plot]"` |

Build targets are Windows x64, macOS 14+ on Intel / Apple Silicon, and Linux x64 / arm64. The minimum Linux glibc version follows each wheel's `manylinux_2_XX` tag; the local build requires glibc 2.35+. See the [release procedure](release/README.en.md) for verification status. Body fonts come from the system or user-provided files; missing glyphs warn and display vector boxes. Supply the fonts with your figure for reproducible rendering across machines.

## First figure

Save this complete source as `figure.lay`. Geometry defaults to mm; type and stroke sizes default to pt. Explicit `mm/cm/in/pt/px` units are also supported.

```lay
page = canvas(size=(120 mm, 90 mm), background="#ffffff")
p = plot(size=(110 mm, 80 mm),
         x=axis(label="Time (s)"), y=axis(label="Signal"))
p.line(x=[0, 1, 2, 3], y=[1, 3, 2, 4], label="Experiment")
p.legend(position="top_left")
page.add(p, offset=(5 mm, 5 mm))
```

```sh
laymesh validate figure.lay
laymesh inspect figure.lay --json
laymesh render figure.lay -o figure.svg
laymesh render figure.lay -o figure.pdf
laymesh render figure.lay -o figure.png --dpi 300
```

`python -m laymesh` invokes the same CLI and works when the command is absent from PATH. `--dpi` changes PNG pixel dimensions while preserving physical page size. `.lay` is a restricted standalone language that does not execute arbitrary Python or JavaScript code.

## Python and Jupyter

Render the file you just saved through the Python API:

```python
from laymesh import render_file

result = render_file("figure.lay", output="figure.pdf")
print(result.output)
```

`render_source(source, namespace=..., save_source=...)` accepts inline source, binds arrays, dictionaries, DataFrames or Matplotlib Figures, and saves layouts and data assets for independent CLI rendering. See the [Python API](docs/topics/python-reference.en.md), [native data bindings](docs/topics/python-data.en.md) and [saving source](docs/topics/save-source.en.md) for complete examples.

Install LayMesh in the Notebook's Python environment, then run a registration cell:

```python
%load_ext laymesh.ipython
values = [1, 3, 2, 4]
```

Use a separate cell to plot, preview and export:

```text
%%laymesh -o notebook.pdf --save-source notebook.lay
page = canvas(size=(120 mm, 90 mm))
p = plot(size=(110 mm, 80 mm))
p.line(x=[0, 1, 2, 3], y={{values}})
page.add(p, offset=(5 mm, 5 mm))
```

`{{values}}` reads the Python variable. Preserve `notebook.lay` and the generated `notebook.assets/`, then run `laymesh render notebook.lay -o notebook.svg`. The [Notebook guide](docs/topics/notebook.en.md) covers line magic, warning controls and other options.

## Capabilities and examples

| Capability | Usage and runnable examples |
| --- | --- |
| Precise composition | Physical units, nine anchors, cropping, rotation, opacity and groups: [basic layout](examples/basic.lay) |
| Scientific plots | Lines, scatter, uncertainty, bars, statistics, heatmaps and contours: [native plotting](docs/sections/plotting.en.md) |
| Panels and coordinates | Named axes, breaks, shared color scales, polar and radar charts: [Cartesian](docs/cartesian-plots.en.md) · [polar](docs/polar-plots.en.md) |
| Colors and series | Dictionary loops, multiple series and 87 colormap presets: [dictionary plot](examples/plot/dictionary-series.lay) · [colormaps](docs/topics/cmaps.en.md) |
| Vectors and typography | Paths, gradients, outline fusion, text and formulas: [vectors](examples/vector.lay) · [typography](examples/typography.lay) |
| Reuse and editing | Functions, local modules, LCSS themes, browser and VS Code completion: [language reference](docs/language-reference.en.md) · [editors](docs/topics/editors.en.md) |

![Native lines, error bars and heatmap](site/media/plot-scientific-1920.webp)

Finish chart layers and decorations before the first placement; panels use explicit positioning. Each layout produces one page. Fixed plot areas retain their physical dimensions; insufficient decoration space warns. Image inputs include PNG/JPEG/SVG and single-page 8-bit TIFF; SVG supports a safe subset. Native RaTeX typesets formulas without a TeX installation. See the [feature coverage map](docs/topics/feature-map.en.md) for detailed scope.

## Documentation and development

[Getting started](docs/sections/start.en.md) · [Language reference](docs/language-reference.en.md) · [API](docs/api-reference.en.md) · [Architecture](docs/architecture.en.md) · [Examples and recorded results](docs/examples-and-results.en.md)

Source development requires **Rust 1.93.1**; build and release scripts require Python 3.11+. From the repository root:

```sh
cargo build --release --locked -p laymesh-cli
python -m pip install -e './python[data,plot]'
python -m laymesh render examples/basic.lay -o basic.pdf
cargo test --workspace --locked
python -m unittest discover -s python/tests -v
```

For direct native use, invoke `target/release/laymesh` (`laymesh.exe` on Windows). The [release procedure](release/README.en.md) covers wheel builds, audits, isolated installation checks and PyPI Trusted Publishing. Build the [documentation site](site/README.en.md) with `python scripts/build-docs.py`; the GitHub Pages workflow publishes it.

LayMesh is [MIT licensed](LICENSE). Native dependencies, formula fonts and colormap data retain their own licenses; see [third-party notices](release/THIRD_PARTY_NOTICES.md).
