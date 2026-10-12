# Local third-party CSS and shared themes

[Style and animation libraries](style-libraries.en.md)

## Keep third-party CSS unchanged

Third-party libraries normally use `.css`; import them without renaming. Download them into the project and retain licenses and relative dependencies. Current compilation does not download URL/CDN imports. This example vendors original [Milligram](https://milligram.io/) 1.4.1 CSS under MIT. Its [manifest](../../examples/libraries/third-party/vendor/manifest.json) records the pinned source and SHA-256. It is a project dependency, not another bundled library.

## Complete project layout

```text
third-party/
  direct.lay
  main.lay
  theme.css
  palette.css
  assets/pattern.svg
  vendor/milligram.css
  vendor/LICENSE.txt
  vendor/manifest.json
```

The [source project](../../examples/libraries/third-party/main.lay) exposes individual source/resource tabs. Keep images, themes and dependencies together.

## Method 1: stylesheet arguments

<!-- example:examples/libraries/third-party/direct.lay -->

Use `stylesheet="vendor/milligram.css"` for one file, or an ordered list for several. The project palette follows the library. The button class supplies supported appearance declarations.

## Method 2: string and url imports

<!-- example:examples/libraries/third-party/main.lay -->

`theme.css` uses `@import "vendor/milligram.css";` and `@import url("palette.css");`. Inline Lay style blocks accept the same forms. Import paths resolve from the importing file; background SVG paths resolve from the declaring stylesheet, rather than the terminal working directory.

## Then share a theme between pages

<!-- example:examples/libraries/shared.laydoc -->

Both pages import `shared-theme.lcss`. An authored theme can also use `.css`; `.lcss` identifies LayMesh styling and extensions to the editor. Extract a separate theme when multiple files reuse it.

## Compatibility, overrides and editor support

Loading a full stylesheet does not implement every web feature. Supported colors, fonts, borders, corners and shadows remain effective; unsupported selectors, properties and rules produce W_CSS summaries. See [compatibility](style-compatibility.en.md). Missing files and cycles remain errors.

Later declarations win at equal importance/specificity; explicit Lay parameters always win. See [shadow overrides](style-shadows.en.md).

External `.css` keeps VS Code native CSS editing. LayMesh indexes it as a dependency, supplying imported classes, variables and keyframes to completion, hover and navigation in `.lay`/`.lcss`. Unsaved changes refresh the index. Web editing and Python queries share that service.
