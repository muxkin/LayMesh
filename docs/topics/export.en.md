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
| SVG, PDF | No raster encoding options | Vector graphics, physical page size and transparency |
| PNG | `compression=fast/default/best`, default `default` | RGBA; records DPI metadata |
| JPEG | `quality=1–100`, default 90; `background="#ffffff"` | Alpha is composited over the matte; records DPI metadata |
| TIFF | `compression=none/lzw/deflate/packbits`, default `lzw`; all lossless | Single-page RGBA, straight alpha; records DPI metadata |
| WebP | `webp_lossless=true`; `quality=0–100`; `webp_method=0–6`, default 4 | RGBA; quality/effort defaults to 100 for lossless, quality 90 for lossy; preserves alpha |
| BMP | No extra options | RGBA; records DPI metadata |
| GIF | `background="#ffffff"` | One frame, up to 256 colors; fully transparent pixels retained, partial alpha composited over matte |
| ICO, TGA | No extra options | RGBA; ICO dimensions up to 256 px; uncompressed TGA |
| PAM, PNM | No extra options | RGBA PAM, preserving alpha |
| PPM, PGM, PBM | `background="#ffffff"` | Composited RGB, grayscale or binary; PBM grayscale threshold 128 |

Every raster format accepts `--dpi`: a positive number up to 25400. The CLI/Python default is 96 when omitted. Each pixel dimension is `round(mm × DPI / 25.4)`, at least 1; the pixel budget is 100000000. DPI does not change layout, type sizes or `layout_dpi`. PNG/JPEG/TIFF/BMP record density metadata; other formats use DPI to set pixel dimensions. JPEG/GIF/TGA have a 65535 px per-axis limit, WebP 16383 px.

WebP also accepts `--webp-alpha-quality 0–100` (default 100; lowering requires lossy mode), and `--webp-near-lossless 0–100` (lossless mode only, default 100, fully lossless). Higher `method` spends more encoding time, usually reducing size; higher lossy `quality` improves visual quality. In lossless mode, `quality` controls compression effort. Near-lossless values below 100 permit approximate RGB samples while retaining alpha. See the [WebP encoder documentation](https://developers.google.com/speed/webp/docs/api).

Native raster exports are **8-bit sRGB RGB/RGBA**. Sixteen-bit inputs retain precision and their intensity range during decoding, ICC conversion, cropping and intermediate PNG generation; final page rasterization is 8-bit. TIFF output does not claim native 16-bit precision. Embedded SVG/PDF assets retain the existing color and alpha pipeline; see [image formats](images.en.md).

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

Run **LayMesh: Export Figure** from a `.lay` editor, or click **Export** in live preview. Choose the format, DPI, applicable encoding options and destination. Options are remembered by workspace and format; `laymesh.export.*` settings control defaults. Export uses the current unsaved entry and opened imported buffers. No Python, Rust or npm installation is required. In remote windows, output is saved on the extension host machine.

## Diagnostics

Invalid, inapplicable and unknown options are rejected. CLI usage errors exit 2; layout, resource or encoding errors exit 1. All commands accept `--warnings show|hide`, overriding `LAYMESH_WARNINGS`. Rendering completes before a temporary file replaces the destination, preserving existing output on ordinary failures. Resource diagnostics retain source locations. `inspect --json` retains all warnings.

[CLI options](cli-reference.en.md) · [Python API](python-reference.en.md) · [Editors](editors.en.md)
