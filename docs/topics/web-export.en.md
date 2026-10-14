# Offline HTML / ZIP playback and editing

Universal ZIP-HTML packages precompiled scenes, timelines, images and the player into one `.zip.html` file. Open it directly in a desktop browser, or extract its ZIP entries. Plain `.html` uses the same container. CanvasKit draws vector outlines at the current viewport and device pixel ratio.

```sh
laymesh render examples/animation/presentation.laydoc -o presentation.zip.html
laymesh render examples/animation/presentation.laydoc -o editable.zip.html --html-editable
laymesh render examples/animation/presentation.laydoc -o presentation.zip
```

Documents export all pages by default; `--page N` selects one page. Extract standard ZIP packages and serve `index.html` over HTTP. Universal files need neither a server, extension nor CDN.

Playback exports contain the drawing runtime, animation controls and static exporters. Editable exports add the source dependency graph, CodeMirror, compiler, language services, font requirements and bundled stylesheets. Completion, hover, signatures, diagnostics and definition navigation work offline. Import files to replace assets or add fonts. Edits pause playback and prepare every page; compilation errors retain the previous successful scene. Downloads produce updated editable files, playback files, or standard ZIP packages.

ZIP Deflate compresses scenes, sources, fonts and WASM. Images default to WebP quality 85, preserving pixel dimensions and transparency; larger encodings are discarded. `--html-image-compression auto|original|lossless` and `--html-image-quality 0–100` control the policy. There is no user-configured size cap; browser memory and ZIP implementation limits still apply. Complete CJK fonts can dominate editable package size.

Editable packages retain original project image files and logical paths for recompilation; playback resources are optimized separately. Lossless exports preserve RGB values under fully transparent pixels. Browser downloads retain the original encoding when lossless WebP encoding is unavailable.

```json
{"export":{"html":{"html_image_compression":"auto","html_image_quality":85,"html_editable":false,"html_embed_fonts":false}}}
```

Python render functions accept `.html/.zip` and corresponding `html_*` keyword options. HTML/ZIP does not accept output DPI. VS Code provides playback/editable HTML and ZIP choices; other controls live in `laymesh.export.html.*` settings.

Before building native distributions, run `python scripts/build-web-export.py` with the Rust WASM target and matching wasm-bindgen CLI installed. Native binaries embed the runtime. `LAYMESH_WEB_RUNTIME` explicitly overrides its ZIP for development.

Run `node scripts/test-web-export.cjs` with Playwright Chromium, Firefox and WebKit installed to produce size/timing JSON and screenshots. `window.__LAYMESH_WEB.metrics` records container recovery, unzip, display preparation, editing compilation, drawing, runtime/compiler/CanvasKit heap sizes and estimated resident cache size. These do not measure total browser-process or GPU memory; GPU draw times measure submission.

Editable exports open a light workspace with a file tree, closable source/resource tabs and a separate preview. Imported images open immediately over a transparency checkerboard with dimensions, byte size, path/reference copying, fit, zoom and pan. Choose Replace or Keep Both for name conflicts. Project fonts always remain resources. Unreferenced added assets can be saved with the precompiled scenes.

System fonts are excluded by default. Opt in with `--html-embed-fonts`, Python `html_embed_fonts=True`, project `export.html.html_embed_fonts` or VS Code `laymesh.export.html.embedFonts`. The HTML export panel offers the same option. Clicking Connect local fonts requests Local Font Access permission in supporting secure desktop Chromium/Edge contexts; unsupported browsers or denied access can import fonts. Only required or selected font bytes are read. Missing fonts preserve the last successful scene and require import or an explicit, saved substitution before recompilation. Precompiled playback needs no font permission.

Preview starts with the complete layout, with real viewport zoom/pan, millimeter rulers and coordinates. Present opens the shared `Presentation` session used by VS Code and the site, starting at the document's first page; its floating controls, auto hide, shortcuts and Escape behavior are shared. Exit restores the workspace page, viewport, focus and playback state. Downloads retain the `.zip.html` filename and Universal ZIP-HTML container; `.html` remains accepted. `node scripts/test-web-workspace.cjs` covers offline workspace, font policy and round trips in Chromium, Firefox and WebKit.

The font dialog separates **Original font → Replace with**. Search by Chinese/English family aliases, system names, style and PostScript name. The bundled dictionary avoids reading unrelated fonts; only required or selected font bytes are loaded. Selections remain drafts until **Confirm and apply**, which loads the selected faces and prepares the document once. Closing preserves the drafts. An unavailable replacement remains unresolved even when a saved binding exists.

The preview shares unit conversion and rulers with VS Code. Native horizontal and vertical scrollbars appear when the page exceeds the viewport. Retained pixels move immediately; a bounded worker queue restores sharp DPR pixels. Pan, zoom and scroll do not reinstall scenes or compile source. `pageInstalls` and `compiles` stay unchanged during viewport interaction. The cross-engine `scripts/test-web-fonts-viewport.cjs` covers foreign/localized font catalogs and retained motion.

## Workspace files and fonts

Editable HTML uses Headless Tree for nested folders, multi-selection, drag-and-drop moves and inline renaming. The plus button and contextual menus expose creation, file/directory import, cut/copy/paste, downloads and undoable deletion. Folder downloads are ZIP archives; empty folders persist in editable exports. Operations affect the embedded workspace only.

Moves and renames update statically resolvable resource references and relative paths inside moved sources. Dynamic paths and incomplete sources are listed for manual checking. The entry can be renamed or moved, but cannot be deleted. Generated resources and bundled libraries remain read-only. File-operation undo is separate from text undo; undo subsequent text edits before undoing a file operation.

Contextual menus cover the tree, tabs, editor, preview, presentation and text inputs, including Shift+F10. Clipboard restrictions fall back to instructions for the normal keyboard shortcuts.

Font catalogs no longer scan every font file for localized names. A bundled dictionary supplies common Chinese/English family aliases. Connecting local fonts loads only faces needed by the document. Importing the original font automatically resolves its requirement; choosing a different family requires an explicit replacement. Embedded fonts work without local font permission.

## Browser image and document export

Choose Export, a format, current/all pages and a filename, then save. Editable files retain all 15 formats: PDF, SVG, PPTX, PNG, JPEG, TIFF, WebP, BMP, static GIF, ICO, PAM/PNM, PPM, PGM, PBM and TGA. Playback files carry only PDF, SVG, PNG, JPEG and TIFF. All pages produce one multi-page PDF with original page sizes, or a ZIP of numbered per-page files for other formats. Editable HTML, playback HTML and standard ZIP remain available in the same panel.

Defaults match the editor: **1200 DPI** and **TIFF LZW**. Open Settings to change resolution, TIFF compression, PNG compression in editable files, JPEG quality and opaque backgrounds; the work retains these settings. Exports use the complete layout, independently of zoom, scrolling and animation time. ICO dimensions above 256 pixels and raster exports above 100,000,000 pixels produce an actionable error instead of silently reducing resolution.

SVG/PDF retain vectors and outline text without silently embedding system font files; Editable PDF uses pinned svg2pdf 0.13.0. Playback PDF uses jsPDF 4.2.1 and svg2pdf.js 2.8.1; unsupported masks, opacity groups and gradient blending use local high-DPI images with a diagnostic. Both reuse CanvasKit for effects, retaining surrounding vector geometry. PPTX shares the native DrawingML exporter and retains editable text and shapes, with reported local raster fallbacks for complex clipping, masks and effects. Browser WebP is lossless. Advanced PDF image optimization, lossy WebP controls and MP4/animated GIF encoding remain native CLI/VS Code features.

PDF effects are tiled at the requested DPI, with independent shadow bounds and filter-input overlap; each RGBA tile has a 64 MiB input budget. Playback PDF stores solid-color effects as a color plus a grayscale soft mask, skips transparent regions and deduplicates resources. Pages and compressed tiles are transferred with acknowledgements instead of retaining all SVG pages and decoded images. Simple shape opacity remains vector; local fallbacks identify the page, object and reason. The dialog reports preparation, effect processing and PDF writing, and supports cancellation and retry. Editable export retains the full Rust/WASM converter and original codecs. Native VSIX, CLI, Python and Jupyter formats, options and PDF text semantics are unchanged. Re-export older offline files to include the fix.

Raster rendering, PNG/TIFF encoding, SVG assembly and editable PDF/PPTX export run in a cancellable worker. Playback JPEG uses the owning browser window encoder to avoid WebKit file-origin Blob read failures. The lightweight playback PDF converter uses the browser DOM and yields between pages; cancellation is checked between page conversions. All codecs are bundled offline. Supported browsers offer a save-location picker; others use downloads. Prepared pages export without font authorization or recompilation. Edited sources must compile successfully before export, so an error cannot silently export a stale scene. Playback files support static export too. Older HTML packages without static export controls must be recreated with LayMesh 0.6.3 or later.

## Package size and license inventory

SVG/PDF are assembled on demand from the retained drawing commands and shared image resources; offline pages no longer duplicate geometry and images in stored SVG snapshots. PDF reuses CanvasKit for effect-only transparent layers while svg2pdf keeps the subjects as vectors. The separate resvg filter renderer is not bundled. Playback reuses CanvasKit PNG encoding and the browser JPEG encoder and carries a small TIFF encoder; the full encoder WASM is limited to editable files. Saving an editable file as playback prunes the full codecs and their licenses using the same runtime manifest as native export. `python scripts/check-web-export-budget.py` enforces a conservative 300 KiB compressed increment including applicable licenses. Only the export codecs use a size-oriented Rust build profile; interactive rendering and compilation retain their performance profile.

`licenses/manifest.json` in the offline ZIP maps each runtime component to complete, original UTF-8 license and attribution texts by SHA-256. Byte-identical texts are stored once, and playback copies omit editor-only dependencies. VSIX and Python packages also deduplicate identical license files, retaining their original filenames in the inventory and `aliases.json`. No license terms or copyright notices are abbreviated.
