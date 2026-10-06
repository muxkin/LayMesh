# Export formats and resolution

## workflow

The CLI, Python, Jupyter and VS Code share one native exporter. The file extension selects the format; `.jpg/.jpeg` and `.tif/.tiff` are equivalent. `.pnm` exports RGBA PAM.

```sh
laymesh render figure.lay -o figure.png --dpi 300 --compression best
laymesh render figure.lay -o figure.jpg --dpi 300 --quality 95 --background "#ffffff"
laymesh render figure.lay -o figure.tif --dpi 600 --compression deflate
laymesh render figure.lay -o figure.webp --dpi 300 --webp-lossless false --quality 90 --webp-method 6 --webp-alpha-quality 100
```

| Format | Options and defaults | Transparency and density |
| --- | --- | --- |
| SVG | No raster encoding options | Vector graphics, physical page size and transparency |
| PPTX (CLI trial) | `dpi` controls local PNG fallback only; default 1200 | One slide; editable text boxes and vector shapes; fonts are not embedded |
| PDF | `pdf_image_compression=auto`, `pdf_jpeg_quality=90`, `pdf_downsample=true` | Vector text/formulas; JPEG or lossless Flate images, never WebP |
| PNG | `compression=fast/default/best`, default `default` | RGBA; records DPI metadata |
| JPEG | `quality=1–100`, default 90; `background="#ffffff"` | Alpha is composited over the matte; records DPI metadata |
| TIFF | `compression=none/lzw/deflate/packbits`, default `lzw`; all lossless | Single-page RGBA, straight alpha; records DPI metadata |
| WebP | `webp_lossless=true`; `quality=0–100`; `webp_method=0–6`, default 4 | RGBA; quality/effort defaults to 100 for lossless, quality 90 for lossy; preserves alpha |
| BMP | No extra options | RGBA; records DPI metadata |
| GIF | `background="#ffffff"` | One frame, up to 256 colors; fully transparent pixels retained, partial alpha composited over matte |
| ICO, TGA | No extra options | RGBA; ICO dimensions up to 256 px; uncompressed TGA |
| PAM, PNM | No extra options | RGBA PAM, preserving alpha |
| PPM, PGM, PBM | `background="#ffffff"` | Composited RGB, grayscale or binary; PBM grayscale threshold 128 |

Every raster format accepts `--dpi`: a positive number up to 25400. CLI, Python/Jupyter and VS Code default to 1200 DPI when omitted. PDF accepts DPI as its image downsampling cap. Each pixel dimension is `round(mm × DPI / 25.4)`, at least 1; the pixel budget is 100000000. DPI does not change layout, type sizes or `layout_dpi`. PNG/JPEG/TIFF/BMP record density metadata; other formats use DPI to set pixel dimensions. JPEG/GIF/TGA have a 65535 px per-axis limit, WebP 16383 px.

WebP also accepts `--webp-alpha-quality 0–100` (default 100; lowering requires lossy mode), and `--webp-near-lossless 0–100` (lossless mode only, default 100, fully lossless). Higher `method` spends more encoding time, usually reducing size; higher lossy `quality` improves visual quality. In lossless mode, `quality` controls compression effort. Near-lossless values below 100 permit approximate RGB samples while retaining alpha. See the [WebP encoder documentation](https://developers.google.com/speed/webp/docs/api).

Native raster exports are **8-bit sRGB RGB/RGBA**. Sixteen-bit inputs retain precision and their intensity range during decoding, ICC conversion, cropping and intermediate PNG generation; final page rasterization is 8-bit. TIFF output does not claim native 16-bit precision. Embedded SVG/PDF assets retain the existing color and alpha pipeline; see [image formats](images.en.md).

## Editable PPTX export trial (CLI only)

```sh
laymesh render examples/export/pptx-editable.lay -o figure.pptx
laymesh render figure.lay -o figure.pptx --dpi 300
```

Each source produces one slide with the original physical canvas dimensions, background and drawing order. Ordinary text becomes editable text boxes positioned per laid-out run, with auto-fit disabled. Formulas become grouped vector outlines rather than editable equation source. Geometry and stroke outlines use custom paths; stroke thickness is represented by geometry, not a PowerPoint line-width property. Plots retain their constituent shapes and text rather than becoming native data charts.

PNG/JPEG assets are embedded as independent pictures. Gradients, patterns, clipping, filters, overlapping group opacity, multi-contour even-odd fills and unrepresentable text/picture transforms rasterize only the smallest complete subtree as a transparent PNG. `W_PPTX_RASTER` identifies each fallback and its reason. Effect lowering can preserve the subject as vectors while rasterizing only its shadow layer.

`--dpi` controls fallback pixels only, defaults to 1200 and supports `export.pptx.dpi` configuration. It never changes slide size. Each fallback image retains the 100000000-pixel limit. `quality`, `compression`, `background`, `pdf_*` and `webp_*` options are rejected for PPTX. `--warnings hide` hides export warnings; failed exports preserve existing destination files.

Fonts are not embedded. `W_PPTX_FONT` lists the required installed families; substitution may alter text layout. Slide width and height must be within 25.4–1422.4 mm (1–56 inches). Transparent canvases use the presentation application's default slide background. Python/Jupyter and the VS Code format picker do not yet expose PPTX. LibreOffice rendering and round-trip checks must be distinguished from native PowerPoint verification.

[Complete trial source](../../examples/export/pptx-editable.lay)

## Python and Jupyter

```python
from laymesh import render_file
render_file("figure.lay", output="figure.jpg", dpi=300, quality=95, background="#ffffff")
render_file("figure.lay", output="figure.tif", dpi=600, compression="lzw")
render_file("figure.lay", output="figure.webp", dpi=300, webp_lossless=True,
            quality=100, webp_method=6, webp_near_lossless=100)
```

`render_source` accepts the same keyword arguments; encoding options require `output`. Both `%laymesh` and `%%laymesh` accept the same CLI flags. The SVG preview remains available while the export uses the chosen codec.

## VS Code

Run **LayMesh: Export Figure** from a `.lay` editor, or click **Export** in live preview. Choose the format and destination, then export directly. Raster defaults to 1200 DPI and TIFF to LZW. Change parameters in `laymesh.export.*` settings; project/global configuration also applies. Old dialog choices are no longer used. Export uses the current unsaved entry and opened imported buffers. No Python, Rust or npm installation is required. In remote windows, output is saved on the extension host machine.

## Diagnostics

Invalid, inapplicable and unknown options are rejected. CLI usage errors exit 2; layout, resource or encoding errors exit 1. All commands accept `--warnings show|hide`, overriding `LAYMESH_WARNINGS`. Rendering completes before a temporary file replaces the destination, preserving existing output on ordinary failures. Resource diagnostics retain source locations. `inspect --json` retains all warnings.

[CLI options](cli-reference.en.md) · [Python API](python-reference.en.md) · [Editors](editors.en.md)

## PDF image policy

Automatic mode samples at most 128 × 128 pixels. Images with at most 32 colors, or at least 90% approximately equal adjacent pixels (each RGB channel differs by at most 2), use lossless Flate. Other opaque 8-bit images use JPEG quality 90. No JPEG/PNG size comparison or duplicate candidate encoding is performed. Transparent and 16-bit images retain lossless encoding by default. Suitable original JPEG bytes are embedded directly when normalization, orientation, crop and resolution permit; `pdf_recompress_jpeg=true` forces recompression.

| PDF option | Default | Meaning |
| --- | --- | --- |
| `pdf_image_compression` | `auto` | `auto`, `lossless`, or `jpeg` |
| `pdf_jpeg_quality` | `90` | 1–100, for newly encoded JPEG |
| `pdf_downsample` | `true` | Only shrink; account for physical size, fit, crop and group scaling |
| `pdf_recompress_jpeg` | `false` | Permit original JPEG passthrough |
| `pdf_preserve_16bit` | `true` | Set false to allow 8-bit conversion |
| `pdf_preserve_alpha` | `true` | Set false to flatten transparency before encoding |
| `pdf_alpha_background` | `#ffffff` | Matte used when alpha preservation is disabled |
| `pdf_auto_palette_limit` | `32` | 0–16384 sampled colors |
| `pdf_auto_flatness_threshold` | `0.9` | 0–1 adjacent pixel flatness ratio |

Forced JPEG still honors enabled depth/alpha preservation and reports `W_PDF_LOSSLESS`. To permit JPEG for a transparent 16-bit image, disable both preservation options. The original file is untouched. All options are available as Python keywords, CLI flags (`--pdf-preserve-16bit false`, etc.) and VS Code settings. Export always uses normalized source pixels, independently of preview JPEG/WebP resources.

## JSON configuration

The nearest `.laymesh.json` above the source file supplies project defaults. `--config PATH` (Python `config=PATH`) selects an explicit project configuration. Global defaults use `$XDG_CONFIG_HOME/laymesh/config.json` on Linux (fallback `~/.config/laymesh/config.json`), `%APPDATA%/laymesh/config.json` on Windows, or `~/Library/Application Support/laymesh/config.json` on macOS.

Precedence, highest first: one-shot API/CLI parameters; explicitly set VS Code folder/workspace values; project configuration; explicitly set VS Code user values; global configuration; built-in defaults. Missing VS Code values do not mask project/global defaults. Configuration changes apply to preview and the next export.

```json
{
  "export": {
    "dpi": 1200,
    "pdf": {"pdf_image_compression": "auto", "pdf_jpeg_quality": 90,
            "pdf_preserve_16bit": true, "pdf_preserve_alpha": true},
    "jpeg": {"quality": 90, "background": "#ffffff"},
    "png": {"compression": "fast"}
  },
  "preview": {"jpeg_quality": 90, "webp_quality": 90, "webp_method": 0,
              "image_threads": 0, "cache_mb": 256, "processing_memory_mb": 128}
}
```

SVG has no DPI. Format groups use canonical names (`jpeg`, `tiff`, `webp`, etc.) and exporter keyword names. Raster exports keep the 100000000-pixel limit; increasing DPI beyond it fails before allocating a page buffer.
