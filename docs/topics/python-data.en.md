# Native arrays and DataFrames

Pass NumPy or pandas data to native layers; this workflow does not require Matplotlib.

Platform wheels bundle the Rust engine. The standard installation includes NumPy, pandas, Matplotlib and IPython, so data bindings, Figure import and Notebook magics are ready to use.

```sh
python -m pip install laymesh
```

[Installation and source development](install.en.md)

## Workflow

Compute the data in a Python cell:

```python
import numpy as np
import pandas as pd

x = np.linspace(0, 2 * np.pi, 33)
df = pd.DataFrame({"x": x, "y": np.sin(x)})
```

Load the extension once:

```text
%load_ext laymesh.ipython
```

In a separate cell, supply the data directly to the native plot:

```lay
%%laymesh -o native.pdf --save-source native.lay
page = canvas(size=(120 mm, 90 mm), background="#ffffff")
d = {{df}}
p = plot(size=(110 mm, 80 mm), plot_area=box(offset=(20 mm, 10 mm), size=(80 mm, 55 mm)),
         x=axis(label="x", range=(0, 7)), y=axis(label="sin(x)", range=(-1.2, 1.2)),
         style=plot_style(font_family="DejaVu Sans", font_size=8 pt))
p.line(x=d["x"], y=d["y"], color="#0072B2")
page.add(p, offset=(5 mm, 5 mm))
```

LayMesh builds the axes, labels and line; no Figure is created. `DejaVu Sans` must be installed on your system or supplied as a font file; body fonts are not bundled. Numeric lists/tuples, 1D/2D ndarrays, Series and DataFrames are supported. A DataFrame needs unique nonempty string column names; its index is not exported automatically. Use `d["column"]` for table data and pass an array binding directly to a layer argument such as `y` or `z`.

`None`, NaN and pandas NA become JSON `null`: lines break at missing samples, points skip them and heatmap cells become transparent, with diagnostics retained. Infinity and unsupported types are errors. Bindings accept one unquoted variable name, not an expression; keep calculations in Python.

## Save and run independently

Use `--save-source` to save the expanded layout and data assets. See [save source and render independently](save-source.en.md) for a complete Python script and standalone CLI commands.

## Limits and related topics

[Notebook Magic](notebook.en.md) · [Save source and render independently](save-source.en.md) · [Matplotlib import](matplotlib.en.md)

## workflow

Complete sources and executable verification fixtures for this workflow are listed in the [feature coverage map](feature-map.en.md). Follow this page’s input conditions and limits when composing features.

## Python dictionary bindings

Bind a Mapping using `series={{series}}`, then iterate `for name,ys in series.items()`. String key order and empty/nested dictionaries are preserved. Values accept JSON containers, scalar strings/bools/numbers/null, and one- or two-dimensional NumPy arrays. NaN becomes null; infinity, oversized integers, non-string keys, cyclic references and arbitrary objects fail before rendering with a field path. Dictionary assets are included by --save-source and can be replayed without the Python namespace.

[Runnable Python dictionary binding example](../../examples/plot/dictionary-series.py).
