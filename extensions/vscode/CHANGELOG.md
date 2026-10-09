# 0.5.1

- Add reusable filled arrows: `arrow()`, `arrow.arc()`, `arrow.bent()`, `arrow.uturn()`, `arrow.chevron()` and `arrow.path()` with namespace completion, signatures and hover.
- Connect arrows to layout, data and selected path anchors; retain independent head dimensions, shaft-width profiles and reusable group placement.
- Center tangent heads on curved shafts and preserve one silhouette for fills, outlines, opacity and effects.
- Preserve `contain` image-fill margins in LibreOffice PPTX exports while retaining editable geometry and the original image resource.
- Improve OpenType math fonts, multilingual formula text, chemical bonds, nested math alphabets and visible missing-glyph placeholders using the pinned RaTeX fork.
- `arrow()` now constructs a filled shape. Migrate legacy line arrows to `line(..., end_head=head(...))`; only calls identified by line-only parameters receive an automatic migration.

# 0.5.0

- Add editable PowerPoint (PPTX) export using the current unsaved entry and imported buffers.
- Preserve native text, shapes, strokes, groups and image resources; rasterize unsupported effects locally with diagnostics.
- Retain editable text when the source font lacks glyphs; presentation applications resolve installed fonts.
- Add distinct light/dark SVG file icons for LayMesh and LCSS.
- Add native document and selection formatting for both languages and embedded styles.
- Wrap long calls and collections at a configurable 100-column default; respect editor indentation and save-time formatting settings.
- Preserve source spelling and verify parse equivalence before returning edits.

# 0.4.0

- Add glyph paint and outlines, path text, arc/wave/perspective warps and vector extrusion.
- Add inner/outer shadows and glow across text, shapes, transparent images and groups.
- Preserve vector PDF subjects; rasterize only effect layers at configured DPI.
- Build five stable platform packages automatically; publish extensions manually.
- Publish PyPI and GitHub Release from reviewed stable version tags.

# 0.3.9

- Connect reusable lines with `start` and `end`, including physical coordinates, instance anchors, chart data points and selected path anchors.
- Allow geometry-free `line()` materials; diagnose missing geometry at placement and support per-end physical offsets.
- Synchronize the extension, Python package and native engine at 0.3.9; build Windows x64, Linux x64/ARM64 and macOS Intel/Apple Silicon packages.

# 0.3.8

- Export directly after choosing the format and destination; move all encoding options to Settings.
- Raster defaults remain 1200 DPI; TIFF defaults to lossless LZW compression.

# 0.3.7

- Cache preview image hashes and font fallback rankings to reduce refresh latency.
- Default preview debounce to 100 ms; add a command to configure 0–5000 ms.
- Bundle LayMesh 0.3.2 for Linux and Windows.

# Changelog

## 0.3.6

- Export SVG/PDF and PNG/JPEG/TIFF/WebP/BMP/GIF/ICO/PNM/TGA from the preview toolbar and editor commands.
- Configure DPI, JPEG quality and matte, TIFF compression, PNG compression, and WebP mode, quality, method, alpha quality and near-lossless fidelity.
- Export the current unsaved entry and imported buffers with the bundled 0.3.1 engine; retain options by workspace and format.
- Rebuild Windows x64 and Linux x64 packages with a statically linked WebP encoder.

## 0.3.5 — Marketplace prerelease

- Import BMP, WebP, GIF, ICO, PNM and TGA; use the first GIF/animated WebP frame.
- Support single-page unsigned 8/16-bit grayscale/RGB TIFF with alpha, preserve intensity range and crop precision, and normalize associated alpha before 16-bit ICC conversion.
- Show inferred variable types and correct binding definitions; add workspace references, read/write highlights and version-checked rename, including import aliases and user function parameters.
- Index unopened files across workspace folders while preserving unsaved buffers and excluding generated/dependency directories.
- Put the standalone VS Code workflow first in both repository READMEs.

## 0.3.4 — Marketplace prerelease

- Add a live SVG preview beside the editor with unit-aware rulers, crosshairs, canvas coordinates, and Cartesian, polar and radar data coordinates.
- Preserve zoom and pan while refreshing unsaved sources and dependencies; retain the last successful figure on errors and restart the isolated renderer on timeout.
- Support English and Simplified Chinese preview controls, coordinate labels, status, commands and Settings descriptions.
- Retain native language completion, hover, signature help, diagnostics, source navigation and color editing.
- Provide Windows x64 and Linux x64 Marketplace packages under the `Hyacine` publisher with bundled native services, a PNG icon, documentation links and dependency licenses.
- Package the extension license as `LICENSE.txt` and audit asset content types to fix Marketplace upload validation.

本次预发布提供实时预览、标尺、鼠标坐标、中英文界面及错误恢复，沿用原有语言服务和选色器；商店包支持 Windows x64 和 Linux x64。
