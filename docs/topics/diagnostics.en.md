# Inspection and warnings

Validate inputs and inspect layout and mappings. Hiding warnings changes their display, without removing diagnostics or suppressing errors.

## Workflow

## Parameters

| Parameter | Purpose | Default or requirement |
| --- | --- | --- |
| `--warnings` | Show or hide warnings | show |
| `LAYMESH_WARNINGS` | Environment default | show / hide |
| `show_warnings` | Python override | None |

```sh
export LAYMESH_WARNINGS=hide
laymesh render figure.lay -o figure.pdf --warnings show
laymesh validate figure.lay --warnings hide
laymesh inspect figure.lay --json --warnings hide
```

```python
from laymesh import render_file
result = render_file("figure.lay", output="figure.pdf", show_warnings=False)
# Notebook: %laymesh figure.lay --warnings hide
```

Precedence is explicit call/CLI option, then environment, then show. The environment accepts show/hide; Python accepts None/True/False. Controls cover LayMesh font, data, radius and layout warnings without changing Python's global warning filters. Duplicate diagnostics from a Python export and its preview appear once.

Hiding warnings preserves diagnostics, rendering and mappings. `inspect --json` includes all warnings, rectangle, center, radii, angular range/direction, radial/category mappings, clip paths, decoration bounds and page transforms. Syntax, argument, range and export errors still fail.

[Python arrays and missing-region example](../../examples/plot/polar-data.py): run `PYTHONPATH=python python examples/plot/polar-data.py` to save editable .lay and hashed JSON assets. See [measured verification and performance](../benchmarks/polar-plots.en.md).

## Limits and related topics

[Using the CLI](cli.en.md) · [Export formats and resolution](export.en.md)

## workflow

Complete sources and executable verification fixtures for this workflow are listed in the [feature coverage map](feature-map.en.md). Follow this page’s input conditions and limits when composing features.
