#!/usr/bin/env python3
"""Compare complete plotting workflows at equal input, geometry and PNG size.

Requires numpy, matplotlib, pillow, psutil, and the built LayMesh packages.
Runs each workflow in isolated processes; samples process-tree RSS every 10 ms.
The bridge inherently also converts the Matplotlib Figure and launches the CLI.
"""
from __future__ import annotations
import argparse
import json
import os
from pathlib import Path
import platform
import statistics
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]


def cpu_name():
    if platform.processor():
        return platform.processor()
    if Path('/proc/cpuinfo').exists():
        for line in Path('/proc/cpuinfo').read_text().splitlines():
            if line.startswith('model name'):
                return line.split(':', 1)[1].strip()
    return platform.machine()


def python_worker(config_path: str):
    import io
    import warnings
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt
    from matplotlib.font_manager import FontProperties
    import numpy as np
    sys.path.insert(0, str(ROOT / "python"))
    from laymesh import render_source
    cfg = json.loads(Path(config_path).read_text())
    font = FontProperties(family="DejaVu Sans", size=8)
    plt.rcParams.update({"path.simplify": False, "agg.path.chunksize": 0,
                         "font.size": 8, "axes.linewidth": .6, "svg.fonttype": "path"})
    samples, messages = [], []
    for _ in range(cfg["iterations"]):
        start = time.perf_counter()
        raw = json.loads(Path(cfg["data"]).read_text())
        # Match native plot frame: page 120x90 mm; chart at (10,10), size 100x70;
        # margins left/top/right/bottom = 18/6/8/18 mm.
        fig = plt.figure(figsize=(120 / 25.4, 90 / 25.4), facecolor="white")
        ax = fig.add_axes([28 / 120, 28 / 90, 74 / 120, 46 / 90])
        ax.set_xlim(0, 1); ax.set_ylim(0, 1)
        ticks = [0, .25, .5, .75, 1]
        ax.set_xticks(ticks, [f"{v:.2f}" for v in ticks], fontproperties=font)
        ax.set_yticks(ticks, [f"{v:.2f}" for v in ticks], fontproperties=font)
        ax.set_xlabel("x", fontproperties=font); ax.set_ylabel("y", fontproperties=font)
        ax.spines[["top", "right"]].set_visible(False)
        ax.tick_params(length=1.2 * 72 / 25.4, width=.6, pad=1.2 * 72 / 25.4)
        if cfg["kind"] == "line":
            ax.plot(raw["x"], raw["y"], color="#0072B2", linewidth=.6)
        elif cfg["kind"] == "scatter":
            ax.scatter(raw["x"], raw["y"], color="#0072B2", s=9, linewidths=0)
        else:
            ax.imshow(np.asarray(raw), extent=(0, 1, 0, 1), origin="lower", aspect="auto",
                      cmap="viridis", vmin=0, vmax=1, interpolation="nearest")
        with warnings.catch_warnings(record=True) as caught:
            warnings.simplefilter("always")
            if cfg["workflow"] == "bridge":
                result = render_source('page=canvas(size=(120 mm,90 mm),background="#ffffff")\npicture=image(src={{fig}})\npage.add(picture,size=(120 mm, 90 mm))',
                                       namespace={"fig": fig}, base_dir=Path(cfg["output"]).parent,
                                       output=cfg["output"], dpi=254, plot_dpi=254)
                preview_bytes = len(result.preview_svg.encode())
            else:
                stream = io.StringIO(); fig.savefig(stream, format="svg")
                preview_bytes = len(stream.getvalue().encode())
                fig.savefig(cfg["output"], format="png", dpi=254)
            messages.extend(str(w.message) for w in caught)
        plt.close(fig)
        samples.append({"seconds": time.perf_counter() - start, "preview_bytes": preview_bytes,
                        "output_bytes": Path(cfg["output"]).stat().st_size})
    print(json.dumps({"samples": samples, "warnings": sorted(set(messages)),
                      "versions": {"python": platform.python_version(), "matplotlib": matplotlib.__version__, "numpy": np.__version__}}))


def native_worker(config_path):
    sys.path.insert(0, str(ROOT / "python"))
    from laymesh.bridge import _command
    cfg=json.loads(Path(config_path).read_text()); samples=[]; warnings=[]
    for _ in range(cfg["iterations"]):
        start=time.perf_counter();preview=Path(cfg["output"]).with_suffix('.svg')
        for output in [preview,Path(cfg["output"])]:
            cmd=[*_command(),'render',cfg['lay'],'-o',str(output)]
            if output.suffix=='.png':cmd+=['--dpi','254']
            result=subprocess.run(cmd,capture_output=True,text=True,check=True)
            warnings.extend(result.stderr.splitlines())
        samples.append({'seconds':time.perf_counter()-start,'preview_bytes':preview.stat().st_size,'output_bytes':Path(cfg['output']).stat().st_size})
    print(json.dumps({'samples':samples,'warnings':sorted(set(warnings)), 'versions':{'engine':subprocess.check_output([*_command(),'--version'],text=True).strip()}}))


def measured(command):
    import psutil
    # Temporary files avoid deadlock if a child writes a long warning or error.
    with tempfile.TemporaryFile() as out, tempfile.TemporaryFile() as err:
        start = time.perf_counter()
        process = subprocess.Popen(command, cwd=ROOT, stdout=out, stderr=err)
        peak = 0
        while process.poll() is None:
            try:
                parent = psutil.Process(process.pid)
                rss = 0
                for child in [parent, *parent.children(recursive=True)]:
                    try:
                        rss += child.memory_info().rss
                    except psutil.Error:
                        pass
                peak = max(peak, rss)
            except psutil.Error:
                pass
            time.sleep(.01)
        elapsed = time.perf_counter() - start
        out.seek(0); stdout = out.read().decode()
        err.seek(0); stderr = err.read().decode()
        if process.returncode:
            raise RuntimeError(f"Worker failed ({process.returncode}): {stderr[-4000:]}\n{stdout[-1000:]}")
        payload = json.loads(stdout.strip().splitlines()[-1])
        return {"wall_seconds": elapsed, "peak_tree_rss_mib": peak / 1024**2, **payload}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--worker")
    parser.add_argument("--native-worker")
    parser.add_argument("--repeats", type=int, default=3)
    parser.add_argument("--output", type=Path, default=ROOT / "docs/benchmarks/native-plots.json")
    args = parser.parse_args()
    if args.native_worker:
        native_worker(args.native_worker); return
    if args.worker:
        python_worker(args.worker); return
    if args.repeats < 2:
        parser.error("--repeats must be at least 2 (one warm-up plus measured repetitions)")
    import numpy as np
    from PIL import Image
    workloads = [(kind, n) for kind in ("line", "scatter") for n in (1000, 100000)] + [("heatmap", n) for n in (64, 512)]
    results = []
    with tempfile.TemporaryDirectory(prefix="laymesh-plot-benchmark-") as folder:
        temp = Path(folder)
        for kind, n in workloads:
            data = temp / f"{kind}-{n}.json"
            if kind == "heatmap":
                yy, xx = np.mgrid[0:n, 0:n] / max(1, n - 1)
                values = (.5 + .45 * np.sin(8 * xx) * np.cos(8 * yy)).tolist()
            else:
                x = np.linspace(0, 1, n)
                values = {"x": x.tolist(), "y": (.5 + .35 * np.sin(x * 20)).tolist()}
            data.write_text(json.dumps(values, separators=(",", ":")))
            lay = temp / f"{kind}-{n}.lay"
            quote = lambda v: json.dumps(str(v))
            source = f'''page=canvas(size=(120 mm,90 mm),background="#ffffff")
s=plot_style(font_family={quote('DejaVu Sans')},font_size=8 pt,line_width=0.6 pt)
p=plot(size=(100 mm,70 mm),x=axis(label="x",range=(0,1),ticks=[0,0.25,0.5,0.75,1],format=".2f"),y=axis(label="y",range=(0,1),ticks=[0,0.25,0.5,0.75,1],format=".2f"),style=s,margins=(18 mm,6 mm,8 mm,18 mm))
'''
            if kind == "heatmap":
                source += f'z=array(src={quote(data)})\np.heatmap(z=z,extent=(0,1,0,1),vmin=0,vmax=1)\n'
            else:
                source += f'd=table(src={quote(data)})\np.{kind}(x=d["x"],y=d["y"],color="#0072B2")\n'
            source += 'page.add(p,offset=(10 mm,10 mm))\n'
            lay.write_text(source)
            for workflow in ("native", "matplotlib", "bridge"):
                cfg = {"workflow": workflow, "kind": kind, "size": n, "lay": str(lay), "data": str(data),
                       "output": str(temp / f"{kind}-{n}-{workflow}.png")}
                config = temp / "worker.json"
                command = [sys.executable, __file__, "--native-worker", str(config)] if workflow == "native" else [sys.executable, __file__, "--worker", str(config)]
                config.write_text(json.dumps({**cfg, "iterations": 1}))
                cold = [measured(command) for _ in range(args.repeats)]
                config.write_text(json.dumps({**cfg, "iterations": args.repeats + 1}))
                warm = measured(command)
                with Image.open(cfg["output"]) as image:
                    if image.size != (1200, 900):
                        raise RuntimeError(f"Mismatched PNG size: {workflow} {image.size}")
                record = {"kind": kind, "size": n, "workflow": workflow, "cold": cold, "warm": warm,
                          "cold_median_seconds": statistics.median(r["wall_seconds"] for r in cold),
                          "warm_median_seconds": statistics.median(s["seconds"] for s in warm["samples"][1:]),
                          "peak_tree_rss_mib": max(warm["peak_tree_rss_mib"], *(r["peak_tree_rss_mib"] for r in cold))}
                results.append(record)
                print(f'{kind} {n} {workflow}: cold {record["cold_median_seconds"]:.3f}s, warm {record["warm_median_seconds"]:.3f}s, tree RSS {record["peak_tree_rss_mib"]:.1f} MiB', flush=True)
    report = {"environment": {"platform": platform.platform(), "cpu": cpu_name(), "logical_cpus": os.cpu_count(),
                              "python": platform.python_version(), "engine": "Rust native CLI (startup included in every export)"},
              "method": {"repeats": args.repeats, "png_pixels": [1200,900], "page_mm": [120,90], "dpi": 254,
                         "deliverables": "SVG preview in memory plus PNG on disk", "data_preparation_timed": False,
                         "input_loading_timed": True, "sampling": False, "matplotlib_path_simplify": False,
                         "warmup_discarded": 1, "rss": "10 ms sampled sum of worker and descendant process RSS; approximate, shared pages may be counted more than once",
                         "limits": "Typography/rasterizers differ; bridge may rasterize the complete Figure. This compares workflows, not isolated rasterizer throughput. PNG file size is compression-dependent. Preview text embedding differs."},
              "results": results}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    print(f"Saved {args.output}")


if __name__ == "__main__":
    main()
