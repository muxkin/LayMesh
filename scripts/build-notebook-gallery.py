#!/usr/bin/env python3
"""Run the real Python bridge for two reproducible gallery figures."""
from __future__ import annotations

import argparse
import os
import sys
import tempfile
import warnings
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "python"))
os.environ.setdefault("SOURCE_DATE_EPOCH", "0")
os.environ.setdefault("LAYMESH_CLI", str(ROOT / "target/release" / ("laymesh.exe" if sys.platform == "win32" else "laymesh")))

import matplotlib as mpl  # noqa: E402
mpl.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402
import numpy as np  # noqa: E402
from PIL import Image  # noqa: E402
from laymesh import render_source  # noqa: E402

mpl.rcParams["svg.hashsalt"] = "laymesh-gallery"

SOURCE = 'page = canvas(size=(120 mm, 80 mm), background="#f7f9fc")\nplot = image(src={{fig}})\nplaced = page.add(plot,size=(104 mm, 58 mm), fit=contain,\n                  target=page.top_left, offset=(8 mm, 8 mm))\ncaption = text(content={{caption}}, font_family="DejaVu Sans",\n               font_size=10 pt, color="#203864")\npage.add(caption, target=page.bottom_left, offset=(8 mm, -11 mm))\n'


def figure(kind: str):
    fig, ax = plt.subplots(figsize=(5.2, 2.9), layout="constrained")
    if kind == "vector":
        x = np.linspace(0, 2 * np.pi, 121)
        ax.plot(x, np.sin(x), color="#087f8c", linewidth=2.5, label="sin(x)")
        ax.plot(x, np.cos(x), color="#d66853", linewidth=2, label="cos(x)")
        ax.set(xlabel="x", ylabel="amplitude", title="Vector line plot")
        ax.legend(frameon=False)
    else:
        x = np.linspace(-2.5, 2.5, 46)
        y = np.linspace(-1.8, 1.8, 32)
        z = np.sin(x[None, :] * 2) * np.cos(y[:, None] * 2)
        ax.imshow(z, cmap="viridis", origin="lower", aspect="auto")
        ax.set(title="Heatmap PNG fallback", xlabel="column", ylabel="row")
    return fig


def build(target: Path, kind: str):
    target.mkdir(parents=True, exist_ok=True)
    fig = figure(kind)
    with warnings.catch_warnings(record=True) as captured:
        warnings.simplefilter("always")
        result = render_source(
            SOURCE, namespace={"fig": fig, "caption": "Vector plot" if kind == "vector" else "Heatmap fallback"},
            base_dir=target, output=target / f"{kind}-preview.png", dpi=150,
            plot_dpi=240, save_source=f"{kind}.lay",
        )
    plt.close(fig)
    messages = [str(item.message) for item in captured]
    assets = list((target / f"{kind}.assets").iterdir())
    extension = ".svg" if kind == "vector" else ".png"
    assert len(assets) == 1 and assets[0].suffix == extension, (kind, assets)
    if kind == "vector":
        assert not any("已改用" in message for message in messages), messages
    else:
        assert any("已改用 240 DPI PNG" in message for message in messages), messages
    with Image.open(result.output) as image:
        assert image.size == (709, 472), image.size
    return result, assets[0], messages


def main():
    parser = argparse.ArgumentParser()
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--write", action="store_true")
    group.add_argument("--check", action="store_true")
    args = parser.parse_args()
    destination = ROOT / "examples/gallery/notebook"
    if args.write:
        for kind in ("vector", "heatmap"):
            _, asset, messages = build(destination, kind)
            print(f"{kind}: {asset.relative_to(ROOT)}, 120 × 80 mm, 709 × 472 px, warnings={messages or 'none'}")
    else:
        with tempfile.TemporaryDirectory(dir=ROOT / "examples/gallery") as folder:
            for kind in ("vector", "heatmap"):
                _, asset, messages = build(Path(folder), kind)
                committed = destination / f"{kind}-preview.png"
                assert committed.is_file(), committed
                with Image.open(committed) as image:
                    assert image.size == (709, 472)
                saved = destination / f"{kind}.lay"
                assert saved.is_file() and f"{kind}.assets/" in saved.read_text()
                assert list((destination / f"{kind}.assets").glob(f"*{asset.suffix}"))
                print(f"{kind}: bridge checked, {asset.suffix} asset, warnings={messages or 'none'}")


if __name__ == "__main__":
    main()
