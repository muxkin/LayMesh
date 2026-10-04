# LayMesh

**Reproducible scientific figures and precise physical layouts from readable code.** Compose native plots, images, vector shapes, text and formulas in one editable `.lay` source; export SVG, PDF and PNG/JPEG/TIFF/WebP/BMP/GIF/ICO/PNM/TGA with a native Rust engine.

[中文文档](https://muxkin.github.io/LayMesh/) · [English documentation](https://muxkin.github.io/LayMesh/en/) · [Examples](https://github.com/muxkin/LayMesh/tree/main/examples) · [Source](https://github.com/muxkin/LayMesh)

## Install

Requires **Python 3.10+**. Version `0.3.1` adds configurable raster export. Install an uploaded release or a reviewed local platform wheel:

```sh
python -m pip install laymesh
python -m laymesh --version
```

Platform wheels bundle one native Rust executable and formula fonts. Rendering requires no Rust compiler, Node.js or TeX installation and downloads no engine or fonts. Body fonts come from the operating system or user-provided files; missing glyphs warn and display vector boxes. Supply font files with a layout for reproducible text across machines.

The standard installation includes NumPy, pandas, Matplotlib and IPython. Data bindings, Matplotlib Figure import and Jupyter magics are ready to use.

Wheel targets are Windows x64, macOS 14+ on Intel and Apple Silicon, and Linux x64 / arm64. Linux requires glibc at least as new as the wheel's `manylinux_2_XX` tag. Availability depends on uploaded wheels; Alpine/musl, 32-bit platforms and Windows ARM64 have no wheels. Plots are rendered by the native engine.

## First figure: CLI

Save this complete source as `figure.lay`:

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

`python -m laymesh` invokes the same CLI. Geometry defaults to mm; typography and line widths default to pt. Raster DPI changes output pixels while preserving physical dimensions. `.lay` is a restricted standalone language, separate from Python.

## Image export options (0.3.1)

```sh
laymesh render figure.lay -o figure.jpg --dpi 300 --quality 95 --background "#ffffff"
laymesh render figure.lay -o figure.tif --dpi 600 --compression deflate
laymesh render figure.lay -o figure.webp --dpi 300 --webp-lossless false --quality 90 --webp-method 6
```

`render_file` and `render_source` accept `dpi`, `quality`, `compression`, `background`, `webp_lossless`, `webp_method`, `webp_alpha_quality` and `webp_near_lossless`. Notebook magics accept the corresponding CLI flags. Raster exports use 8-bit sRGB; PNG/JPEG/TIFF/BMP record DPI metadata. JPEG uses a matte; GIF has binary alpha; RGBA formats retain partial transparency. [Export options](https://muxkin.github.io/LayMesh/en/docs/topics/export.html).

## Python API

```python
from laymesh import render_source

source = '''
page = canvas(size=(120 mm, 90 mm), background="#ffffff")
p = plot(size=(110 mm, 80 mm),
         x=axis(label="Time (s)"), y=axis(label="Signal"))
p.line(x=[0, 1, 2, 3], y=[1, 3, 2, 4], label="Experiment")
p.legend(position="top_left")
page.add(p, offset=(5 mm, 5 mm))
'''
result = render_source(source, output="python-figure.svg", save_source="python-figure.lay")
print(result.output)
```

Use `render_file("figure.lay", output="figure.pdf")` to render existing source. Pass Python variables via `namespace={"values": values}` and refer to them with an unquoted `{{values}}` placeholder in `.lay`. Lists, dictionaries, NumPy arrays and pandas DataFrames can feed native plots. Matplotlib Figures can be placed as image assets. Keep calculations in Python; placeholders accept variable names, not expressions.

`save_source` saves expanded source and a sibling `.assets/` directory for independent CLI rendering. Preserve that directory and any referenced modules, stylesheets or fonts with the `.lay` file. [Python API and data examples](https://muxkin.github.io/LayMesh/en/docs/topics/python-reference.html).

## Jupyter

Install LayMesh in the active kernel's environment. First run:

```python
%load_ext laymesh.ipython
values = [1, 3, 2, 4]
```

Then run this separate cell:

```text
%%laymesh -o notebook.pdf --save-source notebook.lay
page = canvas(size=(120 mm, 90 mm))
p = plot(size=(110 mm, 80 mm))
p.line(x=[0, 1, 2, 3], y={{values}})
page.add(p, offset=(5 mm, 5 mm))
```

The magic displays an SVG preview and writes the requested export. `%laymesh figure.lay -o figure.png --dpi 300` renders an existing file. Both magics support `--warnings show|hide`. [Notebook guide](https://muxkin.github.io/LayMesh/en/docs/topics/notebook.html).

## Scope

LayMesh provides physical units, anchors, cropping, paths, reusable modules, LCSS styles and native scientific plots including statistical, polar and radar layers. Layouts are single-page; panels use explicit positioning. Finish plot layers and decorations before placing a chart. Fixed plot areas keep their physical dimensions; decoration overflow emits a warning. [Language reference](https://muxkin.github.io/LayMesh/en/docs/language-reference.html).

## 中文

LayMesh 用一个可编辑的 `.lay` 文件组织科研图表、图片、文字与公式，导出 SVG、PDF、PNG。需要 Python 3.10+；运行 `python -m pip install laymesh` 即可安装全部功能，包括 NumPy/pandas 数据绑定和 Matplotlib Figure 导入。平台 wheel 内置原生 Rust 引擎和公式字体，正文使用系统或用户字体。支持 `laymesh` 命令、Python API 和 Jupyter Magic；保存源码与资源后可独立重新导出。[中文入门](https://muxkin.github.io/LayMesh/docs/topics/install.zh-CN.html)。

LayMesh is [MIT licensed](https://github.com/muxkin/LayMesh/blob/main/LICENSE). Bundled native dependencies, formula fonts and colormap data retain their own license texts inside the installed package. `_vendor/manifest.json` records engine, platform and build hashes; `_vendor/licenses/manifest.json` records dependency license provenance.
