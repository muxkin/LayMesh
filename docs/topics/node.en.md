# Rust and browser WASM APIs

The engine is a Cargo workspace with no Node.js or npm requirement. Rust libraries and browser WebAssembly replace the former Node/TypeScript entry points.

```rust
use laymesh_core::engine::compile_file;
let scene = compile_file("figure.lay")?;
let svg = laymesh_render::render_svg(&scene)?;
```

For in-memory sources, call `compile_source(source, filename, Host)` and supply resources in `Host.files`. Native rendering discovers system fonts; WASM uses user-loaded fonts only. Browser bindings expose `render(source, filename, files_json)` and `language_query(files_json, filename, method, offset, locale)`; language offsets use UTF-16. Both targets share parser, layout, font handling, and SVG rendering.

Python and Jupyter retain their public interfaces and invoke the native executable. `LAYMESH_CLI` overrides the native binary for development.

## workflow

Complete sources and executable verification fixtures for this workflow are listed in the [feature coverage map](feature-map.en.md). Follow this page’s input conditions and limits when composing features.

## Minimal Rust source

```rust
use laymesh_core::{engine::compile_source, model::Host};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = "page=canvas(size=(60mm,40mm))\npage.add(rect(size=(30mm,20mm),fill=\"#087f8c\"))";
    let scene = compile_source(source, "/figure.lay", Host::default())?;
    std::fs::write("figure.svg", laymesh_render::render_svg(&scene)?)?;
    std::fs::write("figure.pdf", laymesh_render::render_pdf(&scene)?)?;
    std::fs::write("figure.png", laymesh_render::render_png(&scene, 300.)?)?;
    Ok(())
}
```

Put this in a Cargo binary depending on laymesh-core and laymesh-render from this workspace. Host.files maps absolute normalized resource names to bytes for in-memory input; Host::default() enables native resource/font loading. WASM uses native=false and explicitly supplied resources.
