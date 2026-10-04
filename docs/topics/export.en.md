# Export formats and resolution

## Workflow

```text
laymesh validate <file.lay>
laymesh inspect <file.lay> --json
laymesh render <file.lay> -o <output.svg|pdf|png> [--dpi <positive-number>]
```

In this repository, prefix the commands with `cargo run --release --locked -p laymesh-cli --`. `validate` checks syntax, names, units, imports, image/font loading, glyph coverage, and placement constraints. `render` checks the same conditions before writing. `--dpi` applies **only to PNG** and defaults to 96. Errors report `file:line:column: E_CODE: reason` and return nonzero; warnings such as `W_FONT`, `W_MATH_FONT`, and `W_PLOT_LAYOUT` go to stderr without necessarily failing validation. CLI usage errors exit 2; layout/export errors exit 1.

SVG retains editable paths and ordinary `<text>`; PDF retains vector graphics and embedded ordinary text while storing formula LaTeX source; PNG rasterizes the same Scene. SVG input is allowlisted and rejects scripts, external links, DTDs, filters, and unsupported effects with `E_SVG`. PNG/JPEG/TIFF inputs are normalized to sRGB PNG for the Scene; TIFF is limited to single-page 8-bit grayscale or RGB. See [actual output and failure examples](../examples-and-results.en.md) and the [feature gallery](../gallery/README.en.md).

`inspect --json` exposes plot geometry, mappings, transformations and all diagnostics. Each command accepts `--warnings show|hide`, overriding `LAYMESH_WARNINGS` (default `show`). Hiding warnings does not hide errors or discard inspection diagnostics.

## Limits and related topics

[Using the CLI](cli.en.md) · [Inspection and warnings](diagnostics.en.md)

## workflow

Complete sources and executable verification fixtures for this workflow are listed in the [feature coverage map](feature-map.en.md). Follow this page’s input conditions and limits when composing features.
