"""Normalize SVG produced by a Matplotlib Figure for LayMesh's safe SVG subset."""

import re


class SvgCompatibilityError(ValueError):
    pass


_STYLE_BLOCK = re.compile(r"<style(?:\s[^>]*)?>(.*?)</style>", re.DOTALL)
_METADATA = re.compile(r"<metadata(?:\s[^>]*)?>.*?</metadata>", re.DOTALL)
_COMMENT = re.compile(r"<!--.*?-->", re.DOTALL)


def normalize_matplotlib_svg(source: str) -> str:
    # Matplotlib supplies this document itself. The strict LayMesh sanitizer still
    # checks the result; arbitrary SVG files never pass through this adapter.
    start = source.find("<svg ")
    if start < 0:
        raise SvgCompatibilityError("Matplotlib 未生成 SVG 根元素")
    svg = source[start:].strip()
    svg = _COMMENT.sub("", svg)
    svg = _METADATA.sub("", svg)
    if not svg.endswith("</svg>") or "<!" in svg or "<?" in svg:
        raise SvgCompatibilityError("Matplotlib SVG 包含不受支持的文档结构")

    styles = _STYLE_BLOCK.findall(svg)
    if any(re.sub(r"\s+", "", style) != "*{stroke-linejoin:round;stroke-linecap:butt}" for style in styles):
        raise SvgCompatibilityError("Matplotlib SVG 使用了不受支持的样式表")
    svg = _STYLE_BLOCK.sub("", svg)
    if styles:
        svg = svg.replace("<svg ", '<svg style="stroke-linejoin:round;stroke-linecap:butt" ', 1)
    return svg
