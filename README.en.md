# LayMesh

**Plot data and compose precise, reproducible scientific figures in readable code.** Combine native plots, images, vector shapes, text and formulas in one `.lay` file, then export SVG, PDF, or PNG, JPEG, TIFF, WebP and other images at your chosen DPI.

[Documentation](https://muxkin.github.io/LayMesh/en/) · [中文](README.md) · [Feature gallery](docs/sections/examples.en.md) · [Installation](docs/topics/install.en.md)

![LayMesh scientific figures and precise layouts](site/media/showcase-1920.webp)

[Showcase source](examples/showcase.lay) · [Scientific plot examples](docs/gallery/plots.en.md)

[Artistic text, shadows and glow](docs/topics/art-effects.en.md) · [Editable example](examples/effects/art-text.lay) · [Release notes](release/notes/0.4.0.md)

## VS Code (recommended)

Install [LayMesh by Hyacine](https://marketplace.visualstudio.com/items?itemName=Hyacine.laymesh-language) from Extensions, or use **Extensions: Install from VSIX** with a platform package. **Version 0.4.0 VSIX packages for Windows x64, Linux x64 / ARM64 and macOS Intel / Apple Silicon include the native engine: no Python, Rust or npm installation is needed.** With Remote SSH, install the package matching the remote extension host.

Open a trusted local or Remote SSH folder and save this as `figure.lay`:

```lay
page = canvas(size=(15cm, 10cm), unit="cm", background="#ffffff")
rec = rect(size=(2cm, 2cm), fill="#888888")
page.add(rec, offset=(0.1cm, 0.1cm))
```

Click the preview icon or run **LayMesh: Open Preview** for a live figure beside the source. Ctrl+Space completes variables and members; hover shows types; F12 goes to a definition; Shift+F12 finds workspace references and F2 renames a binding. Unsaved edits refresh the preview. See the [extension guide](extensions/vscode/README.md) for rulers, navigation and platform packages.

## Python and CLI installation

Requires **Python 3.10+**. The **`0.4.0`** sources synchronize Python, Rust and the VS Code extension, adding artistic text and inner/outer shadows and glow. Stable tags build reviewed packages; PyPI approval precedes publication and the GitHub Release. Check PyPI for the currently published version. Install from [PyPI](https://pypi.org/project/laymesh/):

```sh
python -m pip install laymesh
python -m laymesh --version
```

Platform wheels bundle the native Rust engine and formula fonts for the CLI, Python API and Jupyter magics. Rendering requires no Rust compilation and downloads no engine or fonts. You can also install a [locally built wheel](release/README.en.md).

The standard installation includes NumPy, pandas, Matplotlib and IPython. Data bindings, Matplotlib Figure import and Jupyter magics are ready to use.

Supported wheel platforms are Windows x64, macOS 14+ on Intel / Apple Silicon, and Linux x64 / arm64. Published Linux wheels require glibc 2.35+. The release workflow checks all five platforms on Python 3.10, 3.13 and 3.14. The Windows wheel statically links its runtime and needs no separate Visual C++ Redistributable; see the [release procedure](release/README.en.md). Body fonts come from the system or user-provided files; missing glyphs warn and display vector boxes. Supply the fonts with your figure for reproducible rendering across machines.

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
laymesh render figure.lay -o figure.jpg --dpi 300 --quality 95
laymesh render figure.lay -o figure.tif --dpi 600 --compression lzw
laymesh render figure.lay -o figure.webp --dpi 300 --webp-lossless false --quality 90 --webp-method 6
```

`python -m laymesh` invokes the same CLI and works when the command is absent from PATH. `--dpi` changes raster pixel dimensions while preserving physical page size. `.lay` is a restricted standalone language that does not execute arbitrary Python or JavaScript code.


The VS Code preview **Export** button and **LayMesh: Export Figure** command support the same formats and options, using unsaved buffers. [Formats, transparency and all options](docs/topics/export.en.md).

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

Finish chart layers and decorations before the first placement; panels use explicit positioning. Each layout produces one page. Fixed plot areas retain their physical dimensions; insufficient decoration space warns. Image inputs include PNG, JPEG, BMP, WebP, GIF, ICO, PNM, TGA, safe SVG and single-page unsigned 8/16-bit grayscale/RGB TIFF, including alpha. GIF and animated WebP use the first frame; 16-bit inputs retain their intensity range without automatic contrast stretching. Native RaTeX typesets formulas without a TeX installation. See the [feature coverage map](docs/topics/feature-map.en.md) for detailed scope.

## Documentation and development

[Getting started](docs/sections/start.en.md) · [Language reference](docs/language-reference.en.md) · [API](docs/api-reference.en.md) · [Architecture](docs/architecture.en.md) · [Examples and recorded results](docs/examples-and-results.en.md)

Source development requires **Rust 1.93.1**; build and release scripts require Python 3.11+. From the repository root:

```sh
cargo build --release --locked -p laymesh-cli
python -m pip install -e './python'
python -m laymesh render examples/basic.lay -o basic.pdf
cargo test --workspace --locked
python -m unittest discover -s python/tests -v
```

For direct native use, invoke `target/release/laymesh` (`laymesh.exe` on Windows). The [release procedure](release/README.en.md) covers wheel builds, audits, isolated installation checks and PyPI Trusted Publishing. Build the [documentation site](site/README.en.md) with `python scripts/build-docs.py`; the GitHub Pages workflow publishes it.

LayMesh is [MIT licensed](LICENSE). Native dependencies, formula fonts and colormap data retain their own licenses; see [third-party notices](release/THIRD_PARTY_NOTICES.md).
