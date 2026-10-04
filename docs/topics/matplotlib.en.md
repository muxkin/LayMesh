# Matplotlib import

Reuse an existing Matplotlib Figure through the Python bridge and place it as an image.

Platform wheels contain one native Rust engine and require no separate runtime installation. Before the first PyPI release, install a reviewed local wheel; after publication, use the command below. Native data does not require Matplotlib; Figure import additionally needs the `plot` extra.

```sh
python -m pip install --pre "laymesh[plot,data]"
```

[Installation and source development](install.en.md)

<!-- example:examples/gallery/notebook/vector.lay -->

## Parameters

| Parameter | Purpose | Default or requirement |
| --- | --- | --- |
| `--dpi` | Final PNG resolution | Does not control the Figure asset |
| `--plot-dpi` | Figure fallback PNG resolution | 300 |

## Workflow


Install the `plot` extra described above. This workflow keeps an existing Matplotlib figure and places it on a LayMesh page.

Compute data and make a Figure in Python:

```python
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

x = np.linspace(0, 2 * np.pi, 100)
df = pd.DataFrame({"x": x, "y": np.sin(x)})
fig, ax = plt.subplots(figsize=(4, 2.5))
ax.plot(df["x"], df["y"])
title = f"Peak: {df['y'].max():.2f}"
```

Load the extension and put `.lay` directly in the next cell:

```text
%load_ext laymesh.ipython
```

```lay
%%laymesh -o figure.pdf --save-source figure.lay
page = canvas(size=(160 mm, 100 mm), background="#ffffff")
chart = image(src={{fig}})
placed = page.add(chart,size=(130 mm, auto),
                  target=page.top_left, offset=(15 mm, 10 mm))
caption = text(content={{title}}, font_size=10 pt)
page.add(caption, target=placed.bottom_left, offset=(0 mm, 4 mm))
```



The cell displays an SVG preview and exports `figure.pdf`. `--save-source figure.lay` also saves the expanded layout and plot asset in `figure.assets/`; outside the Notebook you can run `laymesh render figure.lay -o another.pdf`. Repeated execution overwrites only `.lay` files carrying a LayMesh-generated marker, not handwritten layouts. Saved assets have content-hash names; obsolete ones may remain. Omit `--save-source` when the Notebook itself is sufficient as the reproducible source.

Use line magic for an existing layout:

```text
%laymesh figures/layout.lay -o figures/output.png --dpi 300
```

Relative image, font, and module paths then resolve beside `layout.lay`. In `%%laymesh`, they resolve from the Notebook process's working directory; check `Path.cwd()` in Python. `-o` supports SVG, PDF, and PNG; without it, only a temporary preview is produced. `--dpi` controls final PNG pixels. `--plot-dpi` (default 300) controls only a Matplotlib figure that must fall back to a raster asset.

An unquoted `{{name}}` refers to one Notebook variable. Supported values include strings, booleans, finite scalars, `Path`, Matplotlib Figures, numeric lists, 1D/2D ndarrays, Series and DataFrames. Arrays and tables become portable JSON assets for [native plotting](../plotting.en.md). NaN/None/pandas NA represent missing values; infinity is rejected. Undefined names and Python expressions remain unsupported.



Supported Matplotlib line plots stay as SVG vector artwork. Matplotlib normally converts labels into vector paths, which look stable but are not selectable text. Heatmaps or other content outside LayMesh's safe SVG subset fall back to PNG with a Notebook warning. Text created by LayMesh itself retains its usual export behavior. External SVG files do not use this Matplotlib adapter; they still undergo the normal SVG security checks.

`W_PLOT_BOUNDS` means visible Figure content extends outside the original canvas. Jupyter's inline preview may use tight bounds and still show an outlying axis title; LayMesh exports the Figure's original bounds and may clip it. The warning names the affected direction. LayMesh does not silently resize the plot. Call `fig.tight_layout()` or create it with `layout="constrained"`, then rerun the magic.

Ordinary Python scripts use the same bridge:

```python
from laymesh import render_source

result = render_source(
    r"""page=canvas(size=(100 mm,70 mm))
chart=image(src={{fig}})
page.add(chart,size=(80 mm, auto),target=page.center,anchor=center)
""",
    namespace={"fig": fig},
    output="figure.pdf",
)
print(result.output)
```

`render_file("layout.lay", namespace={...}, output="figure.pdf")` performs the same process on an existing source file. Both functions return SVG preview text, the optional output path, and an optional saved-source path.

The [full example Notebook](../../examples/jupyter-integration.ipynb) covers line plots, heatmaps, and rerendering saved source. See the [Notebook gallery](../gallery/notebook.en.md) for actual assets, fallback messages, and both DPI settings.


## Limits and related topics

[Native arrays and DataFrames](python-data.en.md) · [Notebook Magic](notebook.en.md) · [Save source and render independently](save-source.en.md)


## workflow

Complete sources and executable verification fixtures for this workflow are listed in the [feature coverage map](feature-map.en.md). Follow this page’s input conditions and limits when composing features.
