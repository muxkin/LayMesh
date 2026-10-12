# Using the CLI

## Workflow

Install a platform wheel following [installation](install.en.md). Run `python -m pip install laymesh` to install all features, or install a local wheel. Save the complete example in [installation](install.en.md) as `figure.lay`, then run:

```sh
laymesh validate figure.lay
laymesh render figure.lay -o figure.pdf
```

The CLI has three commands:

```text
laymesh validate <file.lay|file.laydoc>
laymesh animate <file.lay|file.laydoc> -o <output.mp4|gif>
laymesh inspect <file.lay|file.laydoc> --json
laymesh render <file.lay|file.laydoc> -o <output.svg|pdf|pptx|png|jpg|tif|webp|bmp|gif|ico|pnm|pbm|pgm|ppm|pam|tga> [--dpi <number>]
```

`python -m laymesh` invokes the same CLI. Source developers build with `cargo build --release --locked -p laymesh-cli` and invoke `target/release/laymesh` (`laymesh.exe` on Windows), or prefix commands with `cargo run --release --locked -p laymesh-cli --`. `validate` checks source and assets without writing an output file. `render` performs the same checks and then writes the requested file. Errors return a nonzero status and include the source file, line, column, and reason.

`inspect --json` reports data rectangles, axis mappings, transforms and warnings. validate, inspect and static render support `--warnings show|hide`; the explicit flag overrides `LAYMESH_WARNINGS`, which defaults to `show`. Hiding warnings does not suppress errors or remove diagnostics from inspection JSON.

## Limits and related topics

[Export formats and resolution](export.en.md) · [Inspection and warnings](diagnostics.en.md)

## workflow

Complete sources and executable verification fixtures for this workflow are listed in the [feature coverage map](feature-map.en.md). Follow this page’s input conditions and limits when composing features.

## Animation and documents

.laydoc supports complete PDF, selected pages and numbered exports; non-PDF output requires --page or --all-pages. animate exports a continuous MP4/GIF timeline and needs FFmpeg. --time/--step currently export PNG snapshots only. See [Documents](documents.en.md) and [Animation export](animation-export.en.md).
