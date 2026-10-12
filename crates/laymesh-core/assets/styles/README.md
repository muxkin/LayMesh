# Offline CSS libraries

The files are unmodified upstream assets. `manifest.json` pins each version,
source URL and SHA-256. Adjacent `.LICENSE` files retain the upstream MIT licenses.
They are embedded by `laymesh-core`, so CLI, Python, WASM and VS Code need no
network access. `@laymesh/bootstrap.css`, `@laymesh/open-props.css` and
`@laymesh/animate.css` work both in `canvas(stylesheet=...)` and local `@import`.
Unsupported rules/declarations are ignored and summarized once per source file.
