# Rust and browser WASM APIs

The engine is a Cargo workspace with no Node.js or npm requirement. Rust libraries and browser WebAssembly replace the former Node/TypeScript entry points.

```rust
use laymesh_core::engine::compile_file;
let scene = compile_file("figure.lay")?;
let svg = laymesh_render::render_svg(&scene)?;
```

For in-memory sources, call `compile_source(source, filename, Host)` and supply resources in `Host.files`. Native rendering discovers system fonts; WASM uses host-registered fonts; documentation pages register their supplied body fonts. Browser bindings expose `render(source, filename, files_json)` and `language_query(files_json, filename, method, offset, locale)`; language offsets use UTF-16. Both targets share parser, layout, font handling, and SVG rendering.

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

## Interactive rendering and animation

Browser live preview uses prepare_render for a reusable job and draw_job for drawing. Drawing jobs expose display_list and resource_bytes; time sampling produces animation frames, rendered by CanvasKit. The SVG return from render remains available for static integration. Hosts register resources and fonts and satisfy compiler resource requests. Language-service offsets use UTF-16.

See the [worker source](../../site/live/worker.mjs) and [animation implementation](animation-internals.en.md).

## Object surfaces

Display-list version 3 carries `hint.surface3d`, reflection and soft-edge parameters; readers accept versions 1–3. The shared browser renderer calls WASM `surface_texture_region(mesh_json, region, density)` to choose each source texture tile and `raster_surface3d(mesh_json, rgba, texture_width, texture_height, region, width, height)` for geometry, visibility and lighting. Hosts with inlined workers inject both functions through `setSurfaceRasterizer` after WASM initialization. Keep compiler, playback, editor and export runtimes together when upgrading. The synchronous `RenderJob.svg()` API also embeds local 3D PNGs; interactive previews use CanvasKit for 2D texturing and composition.
