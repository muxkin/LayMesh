# Editors, completion and hover

The website and VS Code share a language-analysis layer for `.lay` and `.lcss`. Analysis executes no user code and loads no fonts, formula engine or renderer.

## Website workspace

Choose Full source to edit. `Ctrl+Space` requests function, method, parameter, predefined variable, unit, class and part completion. Hover over parameters for meanings, units and defaults. `Ctrl+Shift+Space` displays a call signature; `F12` goes to a definition. Renamed-parameter diagnostics offer quick fixes. `.lcss`, modules, CSV and JSON have file tabs and jointly affect preview.

Rendering remains in a separate Worker with 250ms debounce and IME protection. Edits, theme changes and undo history live only in page memory. Reloading, restoring or returning from the browser's back-forward cache restores original sources. Python/Notebook files remain read-only.

## Completion keys and call help

Use **Tab to accept the selected completion** and **Enter to insert a newline**. With no candidate, Tab indents and Shift+Tab unindents. Arrow keys select candidates, Esc closes help, and Ctrl+Space requests completion. Press Esc, then Tab within two seconds to leave the editor with the keyboard. Input-method composition keeps its normal confirmation behavior.

The candidate list and selected item's details stay available while typing, whether there are many matches or one. Full call help appears only on `Ctrl+Shift+Space`; hovering over a function or parameter also shows documentation. Ordinary typing and cursor movement do not open full call help. Call help shows the method purpose and return value, a signature with the current parameter highlighted, and that parameter's meaning, type, units, default and example. Long signatures expand on click. Candidate details and hover help render Markdown; on small screens candidate documentation appears below the list.

[Library documentation comments](library-docs.en.md) supply the same help for your functions, parameters and exported variables.

## VS Code

Install the [LayMesh extension](https://marketplace.visualstudio.com/items?itemName=Hyacine.laymesh-language) or a platform VSIX. Version 0.3.9 platform VSIX builds cover Windows x64, Linux x64 / ARM64 and macOS Intel / Apple Silicon. Packages bundle the engine and need no Python, Rust or npm; Remote SSH packages match the remote host. Variable hover shows inferred types, Shift+F12 finds workspace references and F2 safely renames bindings, prioritizing unsaved buffers. Explicit import aliases rename locally; exported-name changes update import source names and unaliased uses. Incomplete affected code and name conflicts prevent rename. The following commands build packages for source developers.

Run from the repository root:

```sh
python scripts/build-editors.py
python scripts/package-editor.py --output /tmp/laymesh-language.vsix
code --install-extension /tmp/laymesh-language.vsix
```

The extension source is in `extensions/vscode`. Open it in a VS Code extension development host, or install a locally packaged VSIX. Its LSP server provides completion, hover, signatures, diagnostics, definitions and rename fixes. It reads local module and stylesheet files without executing source.

[Parameter reference](interface-reference.en.md) · [LCSS](lcss.en.md) · [Diagnostics](diagnostics.en.md)

The extension respects your VS Code key bindings. Tab normally accepts a selected suggestion. It no longer opens full parameter help automatically on `(`, `,` or `=`; press `Ctrl+Shift+Space` or hover over code for documentation. To make Enter insert a newline for these two languages, add the following language-specific settings:

```json
{
  "[laymesh]": { "editor.acceptSuggestionOnEnter": "off" },
  "[lcss]": { "editor.acceptSuggestionOnEnter": "off" }
}
```

## Help language

Website help follows the page language; VS Code defaults to its display language. Function names, parameter names, type names and inserted code are not translated. Completion details, hover, call help, documentation links and documentation-comment warnings share the same language selection.

Search VS Code settings for **LayMesh: Language**, or set:

```json
{
  "laymesh.language": "auto"
}
```

Supported values are `auto`, `zh-CN` and `en`. Changes affect subsequent help and documentation warnings without closing files or losing unsaved edits. `auto` uses the editor UI language: Chinese tags select Simplified Chinese; English and currently untranslated languages select English. The language server never guesses from its host operating system.

Library authors can supply translations using [`@lang` documentation](library-docs.en.md#multilingual-documentation). Missing translations fall back to authored text; no automatic translation is performed.

## LSP integration in other IDEs

The built server supports standard input/output independently of VS Code. Start it from the repository root, or configure an absolute server path when starting elsewhere:

```sh
laymesh lsp --stdio
```

A client can send its UI language in `initialize.locale`, and an explicit startup preference through `initializationOptions`:

```json
{
  "locale": "en",
  "initializationOptions": { "language": "zh-CN" }
}
```

This is an initialization-parameter fragment, not a complete request. English is the default when the client supplies no language. Initialization options also work for clients without a settings UI.

Clients supporting `workspace/configuration` should return the `laymesh` section, for example `{"language":"en"}`. Send `workspace/didChangeConfiguration` after changes. Clients without configuration-request support can send these notification parameters directly:

```json
{
  "settings": { "laymesh": { "language": "en" } }
}
```

A valid configuration `language` overrides initialization options, including `auto`; removing it restores the initialization preference. `auto` uses the initial `locale`. Restart the language server to change preferences supplied only at initialization. Follow the IDE's own restart requirements when changing its display language.

The server negotiates Markdown or plain text independently for each feature; clients not advertising Markdown receive plain text. Signature parameters use label ranges when supported and parameter text otherwise. Popup appearance and key bindings remain controlled by the IDE.

## Color editing

VS Code and the web editor show editable swatches beside static colors. HEX, RGB, HSV, OKLCH and alpha are supported. Out-of-gamut colors display a mapping note while original channels remain available. Static discovery never evaluates documents or loads fonts; dynamic expressions do not receive swatches.

Click a web swatch to open a nonmodal, approximately 500px picker 6px below it. The picker flips above when necessary, stacks on narrow screens and follows the swatch while scrolling. HEX, the preview and the source-format selector share one row. The source format defaults to the original representation and is independent of the RGB/HSV/OKLCH channel tabs.

Drag the saturation/value plane, vertical value strip, rainbow hue strip or channel tracks. Percentage channels display percentages and serialize to the DSL's 0–1 ranges; alpha and previews use checkerboards. The thumb, preview, HEX, channel values and gradients update together in the same frame. Dragging does not modify source or invoke rendering.

Click outside or press Esc to commit the last input and close. Moving the source swatch out of the viewport uses the same flow. A session makes one source replacement: Ctrl+Z in the editor undoes it and Ctrl+Shift+Z redoes it. Unchanged sessions create no undo record. Incomplete HEX and invalid channels stay visible and block close until corrected. Concurrent edits reject the replacement and display a note; closing again dismisses the stale session and preserves the document edits. Switching swatches commits the previous session before resolving the new range in the latest document.

VS Code's “LayMesh: Edit Color” command keeps its existing panel: Apply writes once; Cancel or Close keeps source unchanged. The web close-to-commit behavior does not change VS Code interaction.

## workflow

Complete sources and executable verification fixtures for this workflow are listed in the [feature coverage map](feature-map.en.md). Follow this page’s input conditions and limits when composing features.

## Parameter values and smooth local preview

Completion after `anchor=` suggests nine bounds names and `self.`; legacy start/end apply to line endpoints and plot_* names to charts. Named and positional option values insert bare variables; completion inside quotes preserves the string. Hover `round` shows only `round: string = "round"`. Hover `start_cap` explains butt/round/square and the inherited line_cap default. User variables remain expression candidates.

The web computes necessary colors locally during input events and coalesces complete DOM commits by animation frame. Hidden spaces are evaluated on demand and alpha changes reuse chromatic results. RGB and HSV tracks use piecewise sRGB gradients; OKLCH tracks retain gamut-mapping precision. Inputs keep their identity and focus during dragging, and gradients do not wait for a pause. Closing validates/formats through Rust and makes one undoable replacement; invalid drafts and concurrent document edits cannot overwrite source.


## VS Code figure preview

The preview supports English and Simplified Chinese. `laymesh.language: "auto"` follows VS Code; select `"en"` or `"zh-CN"` to override it. An open preview changes labels immediately without compiling or resetting its view. Command titles and Settings descriptions follow VS Code's display language. Native compiler details and figure text retain their original language.

Use the editor title preview icon, context menu, or **LayMesh: Open Preview** on a saved `.lay` file. Each entry has its own panel beside the editor. Edits refresh after 100 ms, including unsaved modules, LCSS and text data; local resource changes refresh it too. Preview uses an independent Rust process in a trusted workspace.

Rulers and canvas X/Y follow `canvas(unit=...)`; px uses `layout_dpi`. Hovering a plot also shows all named data axes, including log, symlog, reversed and broken scales. Gaps show `—`. Polar plots show θ/r without an angle at the center; radar charts show dimension values near their spokes.

Ctrl/Command + wheel zooms about the pointer; Space + drag or middle drag pans. The toolbar offers refresh, fit, 100%, zoom and rulers. 100% uses layout pixels. Errors retain the last figure and mark it stale; click the error to open its source location. Configure `laymesh.preview.debounceMs`, `laymesh.preview.renderTimeoutMs`, and `laymesh.preview.showRulers` for refresh delay, timeout and default rulers.

Run **LayMesh: Set Preview Delay** to enter an integer from 0 to 5000 ms (default 100; 0 disables debounce). It saves to workspace settings when a workspace is open, otherwise user settings, and applies immediately.

Run **LayMesh: Export Figure** from a `.lay` editor or click **Export** in preview, then choose the format and destination to export directly. Images default to 1200 DPI and TIFF to LZW; change parameters in `laymesh.export.*` settings. Unsaved entry and opened imported buffers are used; remote windows save on the extension host. [Export formats and encoding options](export.en.md)

## Full-resolution native preview

VS Code directly loads full-resolution normalized images: opaque images use JPEG quality 90, including 16-bit inputs converted to 8-bit for display. Only images with actually nonopaque alpha pixels use lossy WebP quality 90, method 0 (fast). Alpha is retained; no PNG candidate comparison, proxy images, idle upgrades or double buffering are used.

Validation and layout share a content cache. Independent native threads encode owned pixels (automatic thread count up to 2), with a 128 MiB concurrent working-memory budget. An oversized image runs alone. Active document pixels can exceed the 256 MiB reusable cache budget; eviction bounds reusable entries, not the necessary active working set. Encoded image/font files use stable content URIs; inline SVG refreshes transfer geometry, inspection and newly referenced resources. Text-only changes reuse resources. Closing the final preview releases the worker and resource directory.

Settings: `laymesh.preview.jpegQuality` (90), `webpQuality` (90), `webpMethod` (0), `imageThreads` (0=auto), `cacheMb` (256), `processingMemoryMb` (128). These map to JSON `preview` snake_case keys. The browser asynchronously decodes the full images. Older executables use the existing embedded SVG compatibility path. Zoom, pan, cursor readings and vector annotations stay independent of image encoding. The source files and export precision are untouched.
