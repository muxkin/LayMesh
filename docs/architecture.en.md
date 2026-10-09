# Rust + RaTeX architecture

LayMesh shares one Rust implementation of `.lay` parsing, LCSS, resources, physical layout, plotting and formulas across native CLI, Python/Jupyter and browser WebAssembly. Node.js, npm and embedded JavaScript engines are not required.

| Component | Responsibility |
| --- | --- |
| [laymesh-core](../crates/laymesh-core/src/lib.rs) | Restricted DSL, units, layout, plots, fonts and RaTeX formulas |
| [laymesh-render](../crates/laymesh-render/src/lib.rs) | Versioned DisplayList; Vello CPU rasters, krilla vector PDF, direct DrawingML PPTX and SVG file adapters |
| [laymesh-language](../crates/laymesh-language/src/lib.rs) | Static completion, diagnostics, hover, signatures and imports |
| [laymesh-cli](../crates/laymesh-cli/src/main.rs) | validate, render, inspect and stdio LSP |
| [laymesh-wasm](../crates/laymesh-wasm/src/lib.rs) | Browser Worker bindings for rendering and language services |

Only KaTeX formula fonts are embedded. Body fonts come from the system or user files; browsers require permission or explicit font selection. Missing glyphs warn and render as vector boxes while export continues.

A Python wheel carries one native executable and preserves the existing bridge API, Notebook magic and saved-source workflow. The website uses locked static ESM editor modules. Python builds documentation and assets; Cargo and wasm-bindgen build WebAssembly.

The VS Code client uses its supplied extension-host interfaces to start `laymesh lsp --stdio`. It contains no npm dependency tree. An embedded offline WASM CPU renderer draws Canvas previews from protocol 3 commands, resources, inspection and generation supplied by the native compiler. Fonts selected by native Rust are passed as resources.


The drawing pipeline is `Scene → DisplayList v1 → output backend`. Positioned glyphs retain font IDs, source character ranges and formula text. SVG parsing occurs at the import boundary; a separate serializer exports SVG files. Raster, PDF, PPTX and new previews never generate or parse internal SVG. Explicit preview protocols 1/2 and WASM `svg()` remain available for compatibility; new clients never fall back automatically.

Vello CPU 0.3.0 uses the same f32 pipeline in native and WASM builds. Ordinary output uses 1024-pixel tiles, allowing narrow images wider than 65535 pixels while retaining the 100,000,000-pixel budget. Independent Rust alpha processing handles shadow, glow and inner shadow with overlapping effect tiles and a 16,000,000-pixel RGBA tile budget. krilla 0.8.2 directly writes vector subjects, subset fonts, positioned text, gradients and original encoded images. Effects rasterize locally; ActualText retains formula and art text sources. PPTX preserves editable objects and reports selective raster fallbacks.
