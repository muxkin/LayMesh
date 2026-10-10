#!/usr/bin/env python3
"""Compare the previous two-process PNG preview with one-process JPEG preview.

Uses native engines only; times exclude Python binding and browser scheduling.
Warm once, alternate five times, and measure image decoding separately.
"""
import argparse
import hashlib
import io
import json
from pathlib import Path
import statistics
import subprocess
import tempfile
import time

import numpy as np
from PIL import Image
from PIL.JpegImagePlugin import get_sampling

ROOT = Path(__file__).resolve().parents[1]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run(command):
    # GNU time spawns the engine from a small controller. Direct wait4 on a
    # Python child can include the NumPy controller's inherited resident memory.
    if Path("/usr/bin/time").is_file():
        with tempfile.NamedTemporaryFile() as memory:
            subprocess.run(["/usr/bin/time", "-f", "%M", "-o", memory.name, "--", *command],
                           check=True, capture_output=True)
            return int(Path(memory.name).read_text().strip())
    subprocess.run(command, check=True, capture_output=True)
    return None


def decode_ms(data):
    samples = []
    for index in range(11):
        start = time.perf_counter()
        with Image.open(io.BytesIO(data)) as image:
            image.load()
        if index:
            samples.append((time.perf_counter() - start) * 1000)
    return statistics.median(samples)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", type=Path, required=True)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/release/laymesh")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--scientific", type=Path)
    args = parser.parse_args()
    destination = args.output.resolve()
    destination.mkdir(parents=True, exist_ok=True)
    cases = {
        "vector": ROOT / "examples/gallery/notebook/vector.lay",
        "heatmap": ROOT / "examples/gallery/notebook/heatmap.lay",
        "typography": ROOT / "examples/typography.lay",
        "formula-regressions": ROOT / "experiments/opentype-math/regressions.lay",
        "chemical-bonds": ROOT / "experiments/opentype-math/chemical-bonds.lay",
    }
    if args.scientific:
        cases["scientific"] = args.scientific.resolve()
    evidence = {"baseline_sha256": sha(args.baseline), "native_sha256": sha(args.binary),
                "script_sha256": sha(Path(__file__)), "dpi": 192, "quality": 95,
                "sampling": "4:4:4", "background": "#ffffff",
                "scope": "Native export+preview processes; excludes Python binding and UI",
                "rss_method": "GNU time child peak RSS (KiB on Linux), excludes NumPy controller" if Path("/usr/bin/time").is_file() else None,
                "timing_includes_gnu_time": Path("/usr/bin/time").is_file(),
                "warmups": 1, "repeats": 5, "cases": {}}
    for name, source in cases.items():
        png = destination / f"{name}-before.png"
        jpeg = destination / f"{name}-after.jpg"
        old_pdf = destination / f"{name}-before.pdf"
        new_pdf = destination / f"{name}-after.pdf"
        old = [[str(args.baseline.resolve()), "render", str(source), "-o", str(old_pdf), "--warnings", "hide"],
               [str(args.baseline.resolve()), "render", str(source), "-o", str(png), "--dpi", "192", "--warnings", "hide"]]
        new = [[str(args.binary.resolve()), "render", str(source), "-o", str(new_pdf),
                "--preview-output", str(jpeg), "--warnings", "hide"]]
        timings = {"before": [], "after": []}
        memory = {"before": [], "after": []}
        for repeat in range(6):
            for key in (("before", "after") if repeat % 2 == 0 else ("after", "before")):
                start = time.perf_counter()
                peaks = [run(command) for command in (old if key == "before" else new)]
                if repeat:
                    timings[key].append((time.perf_counter() - start) * 1000)
                    memory[key].append(max(p for p in peaks if p is not None) if any(p is not None for p in peaks) else None)
        reference_png = destination / f"{name}-reference.png"
        run([str(args.binary.resolve()), "render", str(source), "-o", str(reference_png), "--dpi", "192", "--warnings", "hide"])
        assert png.read_bytes() == reference_png.read_bytes(), "CPU raster output changed"
        with Image.open(png) as image:
            rgba = image.convert("RGBA")
            background = Image.new("RGBA", image.size, "white")
            reference = np.asarray(Image.alpha_composite(background, rgba).convert("RGB"), dtype=np.float64)
        with Image.open(jpeg) as image:
            assert image.size == rgba.size and image.info["dpi"] == (192, 192)
            assert get_sampling(image) == 0
            actual = np.asarray(image.convert("RGB"), dtype=np.float64)
        error = (actual - reference) ** 2
        edges = np.any(np.abs(reference - np.roll(reference, 1, axis=0)) > 16, axis=2)
        edges |= np.any(np.abs(reference - np.roll(reference, 1, axis=1)) > 16, axis=2)
        mse = float(error.mean())
        edge_mse = float(error[edges].mean()) if edges.any() else mse
        data_png, data_jpeg = png.read_bytes(), jpeg.read_bytes()
        evidence["cases"][name] = {
            "source_sha256": sha(source), "pixels": list(rgba.size), "cpu_png_unchanged": True,
            "before_ms": timings["before"], "after_ms": timings["after"],
            "before_median_ms": statistics.median(timings["before"]),
            "after_median_ms": statistics.median(timings["after"]),
            "peak_rss_kib": memory,
            "png_bytes": len(data_png), "jpeg_bytes": len(data_jpeg),
            "png_decode_median_ms": decode_ms(data_png), "jpeg_decode_median_ms": decode_ms(data_jpeg),
            "psnr_db": float(10 * np.log10(255 ** 2 / mse)) if mse else None,
            "edge_psnr_db": float(10 * np.log10(255 ** 2 / edge_mse)) if edge_mse else None,
        }
        print(name, round(statistics.median(timings["before"]), 1), "->",
              round(statistics.median(timings["after"]), 1), "ms", flush=True)
    (destination / "performance.json").write_text(json.dumps(evidence, indent=2) + "\n")


if __name__ == "__main__":
    main()
