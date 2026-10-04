# Rust + RaTeX architecture

LayMesh shares one Rust implementation of `.lay` parsing, LCSS, resources, physical layout, plotting and formulas across native CLI, Python/Jupyter and browser WebAssembly. Node.js, npm and embedded JavaScript engines are not required.

| Component | Responsibility |
| --- | --- |
| [laymesh-core](../crates/laymesh-core/src/lib.rs) | Restricted DSL, units, layout, plots, fonts and RaTeX formulas |
| [laymesh-render](../crates/laymesh-render/src/lib.rs) | SVG; native builds enable resvg PNG and krilla PDF |
| [laymesh-language](../crates/laymesh-language/src/lib.rs) | Static completion, diagnostics, hover, signatures and imports |
| [laymesh-cli](../crates/laymesh-cli/src/main.rs) | validate, render, inspect and stdio LSP |
| [laymesh-wasm](../crates/laymesh-wasm/src/lib.rs) | Browser Worker bindings for rendering and language services |

Only KaTeX formula fonts are embedded. Body fonts come from the system or user files; browsers require permission or explicit font selection. Missing glyphs warn and render as vector boxes while export continues.

A Python wheel carries one native executable and preserves the existing bridge API, Notebook magic and saved-source workflow. The website uses locked static ESM editor modules. Python builds documentation and assets; Cargo and wasm-bindgen build WebAssembly.

The VS Code client uses its supplied extension-host interfaces to start `laymesh lsp --stdio`. It contains no npm dependency tree and only translates protocol and editor interfaces.
