# Python API

## Workflow

In the Notebook Python environment, run `python -m pip install laymesh` to install all features, including native plots, NumPy/pandas data bindings and Matplotlib Figure import. You can also install a local wheel as described in [installation](install.en.md). Python 3.10+ is required. The bridge invokes the existing CLI rather than implementing another parser. CLI lookup order is `LAYMESH_CLI`, the native executable inside the wheel, the repository's `target/release/laymesh` or `target/debug/laymesh`, and finally `laymesh` in `PATH`. No JavaScript CLI fallback is supported.

```python
from laymesh import render_source, render_file, RenderResult, LayMeshBridgeError

result = render_source(
    r"""page=canvas(size=(20 mm,10 mm))
box=rect(size=(5 mm, 5 mm),fill="#087f8c")
page.add(box,target=page.top_left)
"""
)
print(result.preview_svg.startswith('<svg'), result.output, result.saved_source)
# True None None
```

| Export | Arguments | Behavior |
| --- | --- | --- |
| `render_source(source, *, namespace=None, base_dir=None, output=None, dpi=None, plot_dpi=300, save_source=None, show_warnings=None)` | `.lay` source; `{{name}}` variables; asset/module base; optional SVG/PDF/PNG path; final PNG DPI; fallback plot DPI; saved expanded `.lay` name | Returns an SVG preview and optionally writes an export and expanded source. Temporary source files are cleaned up; saved assets use a sibling `.assets/` folder. |
| `render_file(file, *, namespace=None, output=None, dpi=None, plot_dpi=300, save_source=None, show_warnings=None)` | Reads an existing `.lay`; asset paths resolve beside it | Uses the same binding and rendering path without modifying the original. |
| `RenderResult` | `preview_svg: str`, `output: Path\|None`, `saved_source: Path\|None` | Immutable dataclass. A preview is available even without `output`. |
| `LayMeshBridgeError` | Exception | Raised for binding, asset, CLI startup, or CLI result failures. |

`namespace` supports strings, `Path`, booleans, finite numbers, NumPy scalars, numeric lists/tuples, 1D/2D arrays, Series, DataFrames, dictionaries and Matplotlib `Figure`. A placeholder must be a single unquoted `{{name}}`; it does not evaluate Python expressions. Arrays and DataFrames can also feed native plots directly; see the [plotting guide](../plotting.en.md). Compatible Matplotlib figures are saved as safe SVG; unsupported SVG content emits a warning and falls back to PNG at `plot_dpi`. Repeated `save_source` overwrites only files marked as LayMesh-generated. See the [Python/Jupyter guide](../python-jupyter.en.md).

`%load_ext laymesh.ipython` invokes `load_ipython_extension(ipython)` to register `LayMeshMagics`:

```text
%laymesh figures/layout.lay -o figures/output.png --dpi 300
%%laymesh -o figure.pdf --save-source figure.lay
page = canvas(size=(100 mm, 70 mm))
```

Both magics display an SVG preview. Line magic reads a file; cell magic reads cell content. Both support `-o/--output`, `--dpi`, `--plot-dpi`, and `--save-source`. Default `--plot-dpi` is 300 and affects only Figure PNG fallback. `load_ipython_extension` lives in [ipython.py](../../python/laymesh/ipython.py), not the top-level `laymesh` exports.

`show_warnings=False` hides displayed warnings; `True` shows them; `None` follows `LAYMESH_WARNINGS` (default `show`). Both Notebook magics accept `--warnings show|hide`. Source development uses an editable install after building the Rust CLI; see [installation](install.en.md).

## Limits and related topics

[dsl-reference](../language-reference.en.md) · [Plot objects and methods](plot-reference.en.md) · [Rust/WASM API](node-reference.en.md) · [CLI options](cli-reference.en.md)

## workflow

Complete sources and executable verification fixtures for this workflow are listed in the [feature coverage map](feature-map.en.md). Follow this page’s input conditions and limits when composing features.
