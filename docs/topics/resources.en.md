# Resource paths

<!-- walkthrough:start -->
## Purpose and concepts

Resource paths are relative to the defining file, not the shell working directory. Material, modules and standalone LCSS retain their own resource origin; the entry path organizes dependencies.

## Minimal complete example

Run this file directly with `laymesh validate` or `laymesh render`; it contains its own canvas and required definitions.

<!-- example:examples/manual/image.lay -->

## Parameters and default behavior

Unitless geometry uses the canvas unit; unitless type and stroke sizes use pt. Explicit call parameters override inherited/theme defaults. The linked interface reference lists accepted types, choices and defaults per parameter.

## Composition

Keep pictures, fonts and components beside the work and use relative paths. Python save_source writes bound data/images to a sibling .assets directory; preserve relative locations when sharing.

<!-- example:examples/gallery/containers/module-import.lay -->

## Common errors and limits

WASM’s virtual file map must contain every dependency; a browser cannot freely read local files. Resource errors locate the defining file, and font-name resolution depends on the runtime.

## Individual functions

<!-- walkthrough:end -->

## Detailed behavior and further examples

### Parameters

| Parameter | Purpose | Default or requirement |
| --- | --- | --- |
| `src` | Image or data path | Relative to defining file |
| `font_family` | Font family or file | Optional; depends on installed fonts |

### Directory relationships

```text
examples/
├── assets/
│   └── Custom.ttf
└── plot/
    ├── figure.lay
    └── results.csv
```

`font_family="../assets/Custom.ttf"` resolves from `plot/figure.lay` to the sibling font directory; `table(src="results.csv")` refers to data beside the source. Rename assets if you also update paths. Resources inside components resolve relative to the component file, not the entry file.

Use `font_family="DejaVu Sans"` only if that family is installed on the system. The path above selects a user-provided font. No body fonts are bundled; unavailable glyphs warn and render as vector boxes.

### Limits and related topics

[Image import and cropping](images.en.md) · [Text and fonts](fonts.en.md) · [Mathematical formulas](formulas.en.md) · [Shapes and paths](shapes.en.md) · [Fills, gradients and outlines](fills.en.md)
