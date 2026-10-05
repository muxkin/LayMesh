"""IPython extension: %laymesh and %%laymesh."""

import argparse
import shlex
from pathlib import Path

from IPython.core.magic import Magics, line_cell_magic, magics_class
from IPython.display import SVG, display

from .bridge import render_file, render_source


def _options(line: str, *, cell: bool) -> argparse.Namespace:
    parser = argparse.ArgumentParser(prog="%%laymesh" if cell else "%laymesh")
    if not cell:
        parser.add_argument("file", type=Path)
    parser.add_argument("-o", "--output", type=Path)
    parser.add_argument("--dpi", type=float)
    parser.add_argument("--quality", type=float)
    parser.add_argument("--compression")
    parser.add_argument("--background")
    parser.add_argument("--webp-lossless", choices=("true", "false"))
    parser.add_argument("--webp-method", type=int)
    parser.add_argument("--webp-alpha-quality", type=int)
    parser.add_argument("--webp-near-lossless", type=int)
    parser.add_argument("--config", type=Path)
    parser.add_argument("--pdf-image-compression", choices=("auto", "lossless", "jpeg"))
    parser.add_argument("--pdf-jpeg-quality", type=int)
    parser.add_argument("--pdf-downsample", choices=("true", "false"))
    parser.add_argument("--pdf-preserve-16bit", choices=("true", "false"))
    parser.add_argument("--pdf-preserve-alpha", choices=("true", "false"))
    parser.add_argument("--pdf-alpha-background")
    parser.add_argument("--pdf-recompress-jpeg", choices=("true", "false"))
    parser.add_argument("--pdf-auto-palette-limit", type=int)
    parser.add_argument("--pdf-auto-flatness-threshold", type=float)
    parser.add_argument("--plot-dpi", type=float, default=300)
    parser.add_argument("--save-source", type=Path)
    parser.add_argument("--warnings", choices=("show", "hide"))
    return parser.parse_args(shlex.split(line))


@magics_class
class LayMeshMagics(Magics):
    @line_cell_magic
    def laymesh(self, line: str, cell: str | None = None) -> None:
        options = _options(line, cell=cell is not None)
        encoding = dict(quality=options.quality, compression=options.compression, background=options.background,
                        webp_lossless=None if options.webp_lossless is None else options.webp_lossless == "true",
                        webp_method=options.webp_method, webp_alpha_quality=options.webp_alpha_quality,
                        webp_near_lossless=options.webp_near_lossless,
                        pdf_image_compression=options.pdf_image_compression, pdf_jpeg_quality=options.pdf_jpeg_quality, pdf_downsample=None if options.pdf_downsample is None else options.pdf_downsample == "true", pdf_preserve_16bit=None if options.pdf_preserve_16bit is None else options.pdf_preserve_16bit == "true", pdf_preserve_alpha=None if options.pdf_preserve_alpha is None else options.pdf_preserve_alpha == "true", pdf_alpha_background=options.pdf_alpha_background, pdf_recompress_jpeg=None if options.pdf_recompress_jpeg is None else options.pdf_recompress_jpeg == "true", pdf_auto_palette_limit=options.pdf_auto_palette_limit, pdf_auto_flatness_threshold=options.pdf_auto_flatness_threshold)
        if cell is None:
            result = render_file(
                options.file, namespace=self.shell.user_ns, output=options.output, config=options.config,
                dpi=options.dpi, plot_dpi=options.plot_dpi,
                save_source=options.save_source,
                show_warnings=None if options.warnings is None else options.warnings == "show",
                **encoding,
            )
        else:
            result = render_source(
                cell, namespace=self.shell.user_ns, output=options.output, config=options.config,
                dpi=options.dpi, plot_dpi=options.plot_dpi,
                save_source=options.save_source,
                show_warnings=None if options.warnings is None else options.warnings == "show",
                **encoding,
            )
        display(SVG(data=result.preview_svg))


def load_ipython_extension(ipython) -> None:
    ipython.register_magics(LayMeshMagics)
