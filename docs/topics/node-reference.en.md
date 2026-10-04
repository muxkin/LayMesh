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

## Using Node with the native CLI

The former TypeScript package is removed. Node can invoke the same native executable using its standard library; this example has no npm dependencies. Arguments remain separate, preserving paths with spaces. Source resources resolve relative to the .lay file.

```js
// Node uses the existing native CLI; there is no separate Node layout engine.
import {execFileSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
const binary=process.env.LAYMESH_CLI||fileURLToPath(new URL('../../target/release/laymesh',import.meta.url));
const source=process.argv[2]||fileURLToPath(new URL('../manual/line.lay',import.meta.url));
const output=process.argv[3]||'figure.svg';
execFileSync(binary,['validate',source],{stdio:'inherit'});
execFileSync(binary,['render',source,'-o',output],{stdio:'inherit'});
```

```sh
node examples/integration/render.mjs examples/manual/line.lay /tmp/figure.svg
```

Inspect nonzero process exit results in your application. LAYMESH_CLI selects another native binary. Browser code instead imports generated browser WASM bindings; a Node program cannot use browser-only font discovery.

## Browser WASM lifecycle

```js
import init, {render, language_query} from "./wasm/laymesh_wasm.js";
await init();
const source = 'page=canvas(size=(60mm,40mm))\npage.add(rect(size=(30mm,20mm),fill="#087f8c"))';
const filename = "/figure.lay";
const result = JSON.parse(render(source, filename, "{}"));
console.log(result.svg, result.inspection, result.warnings);
const help = JSON.parse(language_query(
    JSON.stringify({[filename]: source}), filename, "hover", 7, "en"
));
console.log(help);
```

render returns JSON, not a bare SVG string. Resources are a JSON name→text/byte-array map. Register actual font bytes before rendering text; prepare_render returns a job with svg()/inspection() when layout/render timing needs separation. language_query accepts UTF-16 offsets and analyzes source without evaluation. Error results throw JS exceptions containing diagnostics.

Native `laymesh_render::render_export(&scene, extension, &ExportOptions)` exposes the same options as the CLI; `ExportOptions::validate` supports preflight validation. This interface requires the native feature; browser WASM continues to provide SVG. [Export formats and encoding options](export.en.md)
