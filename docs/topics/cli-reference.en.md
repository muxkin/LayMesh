# CLI options

## Workflow

The [CLI entry point](../../crates/laymesh-cli/src/main.rs) has three document commands and a language-server command:

```text
laymesh validate <file.lay> [--warnings show|hide]
laymesh inspect <file.lay> --json [--warnings show|hide]
laymesh render <file.lay> -o <output.svg|pdf|png|jpg|tif|webp|bmp|gif|ico|pnm|tga> [--dpi <positive-number>] [--warnings show|hide]
```

Use `cargo run --release --locked -p laymesh-cli --` as the prefix in this workspace. `-o` and `--output` are equivalent; the extension selects the format; `--dpi` applies to raster exports and the PDF image cap (default 1200). `--config PATH` overrides project configuration; `--pdf-*` controls PDF image compression and preservation. See [export configuration](export.en.md). Successful validation prints dimensions and the **top-level** placement count. Successful rendering prints the output path. Warnings go to stderr without changing a success exit code of 0. Source/render errors exit 1, usage errors exit 2. The CLI renders to a temporary file before renaming to the destination, avoiding partial outputs on ordinary failures. See [actual stdout](../examples-and-results.en.md).

## Limits and related topics

[dsl-reference](../language-reference.en.md) · [Plot objects and methods](plot-reference.en.md) · [Rust/WASM API](node-reference.en.md) · [Python API](python-reference.en.md)

## workflow

Complete sources and executable verification fixtures for this workflow are listed in the [feature coverage map](feature-map.en.md). Follow this page’s input conditions and limits when composing features.

## LSP / version / help

```sh
laymesh lsp --stdio
laymesh --version
laymesh --help
```

lsp runs a long-lived JSON-RPC server on stdin/stdout. Keep stdout reserved for protocol messages. --version/-V and --help/-h are standalone commands. --warnings show|hide overrides LAYMESH_WARNINGS; inspect retains all diagnostic data even if display is hidden.

Raster formats include PNG, JPEG, TIFF, WebP, BMP, GIF, ICO, PNM and TGA. See [Export formats and encoding options](export.en.md) for quality, compression, matte and WebP mode options.
