# LayMesh

[Gradient comparison gallery](https://muxkin.github.io/LayMesh/gradient-gallery.html): 30 original recipes, actual exports and explained reference differences.

**Create scientific figures and precise physical layouts from readable code.** Combine native plots, images, vector shapes, text and formulas in an editable project, then export static figures or multipage presentations.

[Documentation](https://muxkin.github.io/LayMesh/en/) · [Video tutorial (Bilibili)](https://www.bilibili.com/video/BV1gpHn6eE4V) · [中文](README.md) · [Examples](docs/sections/examples.en.md)

![LayMesh showcase](site/media/showcase-1920.webp)

[Showcase source](examples/showcase.lay) · [Getting started](docs/sections/start.en.md)

## Install and preview your first figure

Install the [VS Code extension](https://marketplace.visualstudio.com/items?itemName=Hyacine.laymesh-language) or a [platform VSIX](https://github.com/muxkin/LayMesh/releases/latest). Platform packages include the native engine without Python, Rust or npm. For Remote SSH, choose the remote host's platform.

Save this as figure.lay in a working folder and run **LayMesh: Open Preview**:

~~~lay
page = canvas(size=(120mm,90mm), background="#ffffff")
p = plot(size=(110mm,80mm),
         x=axis(label="Time (s)"), y=axis(label="Signal"))
p.line(x=[0,1,2,3], y=[1,3,2,4], label="Experiment")
p.legend(position="top_left")
page.add(p, offset=(5mm,5mm))
~~~

Geometry defaults to mm, typography and strokes to pt. Explicit lengths accept mm/cm/in/inch/pt/px. Layout px uses canvas.layout_dpi; export DPI independently controls raster density. See [Units](docs/topics/units.en.md).

## CLI, Python and Notebook

Requires Python 3.10+. Platform wheels include the engine and formula fonts:

~~~sh
python -m pip install laymesh
python -m laymesh --version
laymesh validate figure.lay
laymesh render figure.lay -o figure.pdf
laymesh render figure.lay -o figure.png
~~~

~~~python
from laymesh import render_file
result = render_file("figure.lay", output="figure.pdf")
~~~

In Notebook, load %load_ext laymesh.ipython, then use %laymesh figure.lay or %%laymesh. NumPy/pandas bindings, Matplotlib Figure import and portable source/resource saving are supported. [Installation](docs/topics/install.en.md) · [Python API](docs/topics/python-reference.en.md) · [Notebook](docs/topics/notebook.en.md)

## Learn and look up

| Task | Complete guide |
| --- | --- |
| Physical placement, anchors, groups and reuse | [First layout](docs/topics/first-layout.en.md) · [Layout and assets](docs/sections/layout.en.md) |
| Lines, scatter, errors, statistics, heatmaps and polar plots | [First plot](docs/topics/first-plot.en.md) · [Scientific plotting](docs/sections/plotting.en.md) |
| Themes, libraries, typography, gradients and shadows | [Style and animation libraries](docs/topics/style-libraries.en.md) · [Style manual](docs/topics/lcss.en.md) · [Style gallery](docs/gallery/styles.en.md) |
| 97 effects, keyframes, timing and presentation | [Animation manual](docs/topics/animation.en.md) · [Effect catalog](docs/topics/animation-catalog.en.md) · [Complete projects](docs/topics/style-animation-examples.en.md) |
| Video, page audio and continuous background music | [Video and audio](docs/topics/media.en.md) · [Runnable examples](examples/media/README.md) |
| Sequence pages and export a complete PDF | [Documents](docs/topics/documents.en.md) |
| Text, formulas and artistic effects | [Fonts](docs/topics/fonts.en.md) · [Formulas](docs/topics/formulas.en.md) · [Artistic text](docs/topics/art-effects.en.md) |
| SVG/PDF, raster, editable PPTX and MP4/GIF | [Static exports](docs/topics/export.en.md) · [Animation exports](docs/topics/animation-export.en.md) |

.lay describes one page; .laydoc sequences multiple pages. Panels use explicit placement. PDF retains vector subjects with local raster blur effects. VS Code export asks for format/path only, defaults raster to 1200 DPI and TIFF to LZW, and puts other options in settings. Video export requires FFmpeg.

Themes hold repeated appearance; .lay holds size, placement and data. Explicit parameters override styles. Current-source manuals include new offline libraries and diagnostics requiring a build with those changes. [Release notes](https://github.com/muxkin/LayMesh/releases) describe published packages.

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
