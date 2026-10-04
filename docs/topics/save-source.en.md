# Save source and render independently

Save the expanded .lay file and its resource directory to render again without Notebook or Python.

## Prepare the environment

```sh
python -m pip install laymesh
```

## Save layout and data

```python
from laymesh import render_source
import numpy as np
import pandas as pd

x = np.linspace(0, 2 * np.pi, 33)
df = pd.DataFrame({"x": x, "y": np.sin(x)})
source = r"""
page = canvas(size=(120 mm, 90 mm), background="#ffffff")
d = {{df}}
p = plot(size=(110 mm, 80 mm), plot_area=box(offset=(20 mm, 10 mm), size=(80 mm, 55 mm)),
         x=axis(label="x", range=(0, 7)), y=axis(label="sin(x)", range=(-1.2, 1.2)),
         style=plot_style(font_family="DejaVu Sans", font_size=8 pt))
p.line(x=d["x"], y=d["y"], color="#0072B2")
page.add(p, offset=(5 mm, 5 mm))
"""
result = render_source(source, namespace={"df": df},
                       output="native.pdf", save_source="native.lay")
print(result.output)
```

System font names are convenient for a quick start; bundle a font file for reproducible rendering across machines. JSON files in `native.assets/` use content hashes.

## Render independently

```sh
laymesh render native.lay -o native.svg
laymesh render native.lay -o native.png --dpi 300
laymesh inspect native.lay --json
```

Preserve relative locations of the .lay file, .assets directory, referenced .lcss stylesheets, modules and fonts. These commands invoke only the Rust CLI. Share the resources together with the layout.

[Python/Jupyter](../sections/integration.en.md) · [Matplotlib](matplotlib.en.md)

## workflow

Complete sources and executable verification fixtures for this workflow are listed in the [feature coverage map](feature-map.en.md). Follow this page’s input conditions and limits when composing features.
