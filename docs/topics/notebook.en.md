# Notebook Magic

Platform wheels bundle the Rust engine. The standard installation includes NumPy, pandas, Matplotlib and IPython, so data bindings, Figure import and Notebook magics are ready to use.

```sh
python -m pip install laymesh
```

[Installation and source development](install.en.md)

## Workflow

`%load_ext laymesh.ipython` invokes `load_ipython_extension(ipython)` to register `LayMeshMagics`:

```text
%laymesh figures/layout.lay -o figures/output.png --dpi 300
%%laymesh -o figure.pdf --save-source figure.lay
page = canvas(size=(100 mm, 70 mm))
```

Both magics display an SVG preview. Line magic reads a file; cell magic reads cell content. Both support `-o/--output`, `--dpi`, `--plot-dpi`, and `--save-source`. Default `--plot-dpi` is 300 and affects only Figure PNG fallback. `load_ipython_extension` lives in [ipython.py](../../python/laymesh/ipython.py), not the top-level `laymesh` exports.

## Limits and related topics

[Native arrays and DataFrames](python-data.en.md) · [Save source and render independently](save-source.en.md) · [Matplotlib import](matplotlib.en.md)

## workflow

Complete sources and executable verification fixtures for this workflow are listed in the [feature coverage map](feature-map.en.md). Follow this page’s input conditions and limits when composing features.

## A complete two-cell example

```python
%load_ext laymesh.ipython
values = [1, 2, 4, 3]
```

```text
%%laymesh -o figure.svg --save-source figure.lay
page=canvas(size=(100mm,75mm))
p=plot(size=(86mm,62mm))
p.line(x=[0,1,2,3],y={{values}},marker=circle)
page.add(p,offset=(7mm,6mm))
```

Run the registration cell first. The cell magic reads the Notebook namespace, expands {{values}}, renders a preview and writes a portable source/resource pair. Placeholders are unquoted names, not arbitrary Python expressions. Re-executing updates generated files; regular manually edited source is protected from accidental generated-file overwrite.
