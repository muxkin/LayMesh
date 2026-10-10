#!/usr/bin/env python3
"""Execute the shipped Notebook twice and exercise the real kernel/CLI bridge.

Requires nbclient, nbformat, ipykernel and the normal Python package dependencies.
Use --installed after installing a locally built wheel in an isolated environment.
No user kernelspec, Python installation or Notebook is modified.
"""
from __future__ import annotations

import argparse
import base64
import hashlib
import io
import json
import os
from pathlib import Path
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]


EXERCISES = r"""
import hashlib, io, json, subprocess, zipfile
import laymesh
from IPython.display import display
from IPython.utils.capture import capture_output
from PIL import Image as PILImage
from laymesh import render_source, render_file, LayMeshBridgeError
from laymesh._runtime import bundled_command
from laymesh.bridge import _command

command = _command()
version = subprocess.check_output([*command, "--version"], text=True).strip()
if installed_check:
    assert command == bundled_command(), "Installed test must use its bundled engine"
    # Source/JS/PATH fallback cannot accidentally make an installed test pass.
    for name in ("LAYMESH_CLI", "PYTHONPATH", "NODE_PATH", "NODE_OPTIONS"):
        os.environ.pop(name, None)
    os.environ["PATH"] = str(Path.cwd() / "empty-path")

# NumPy, pandas, inline LaTeX, CJK, ligatures and bidi text travel through the
# kernel namespace. Every update replaces the generated source/data assets.
samples = pd.DataFrame({"x": np.arange(4), "y": [1.0, 3.0, 2.0, 4.0]})
caption = "Notebook initial 中文 fi العربية $\\frac{a}{b}$"
source = '''page=canvas(size=(64mm,48mm),background="none")
data={{samples}}
p=plot(size=(50mm,32mm))
p.line(x=data["x"],y=data["y"],marker=circle)
page.add(p,offset=(7mm,5mm))
page.add(text(content={{caption}},font_size=5pt),offset=(2mm,40mm))
'''
with capture_output() as first:
    get_ipython().run_cell_magic("laymesh", "--warnings hide -o native.pdf --save-source native.lay", source)
first_png = first.outputs[-1].data["image/png"]
first_data = next(Path("native.assets").glob("*.json")).read_bytes()
samples.loc[1, "y"] = 7.0
caption = "Notebook changed 中文 fi العربية $\\sqrt{x^2+1}$"
with capture_output() as second:
    get_ipython().run_cell_magic("laymesh", "--warnings hide -o native.pptx --save-source native.lay", source)
second_png = second.outputs[-1].data["image/png"]
assert first_png != second_png
assert "Notebook changed" in Path("native.lay").read_text()
assert len(list(Path("native.assets").glob("*.json"))) == 2
assert any(p.read_bytes() != first_data for p in Path("native.assets").glob("*.json"))
assert Path("native.pdf").read_bytes().startswith(b"%PDF")
with zipfile.ZipFile("native.pptx") as deck:
    assert b"Notebook changed" in deck.read("ppt/slides/slide1.xml")
with capture_output() as reopened:
    get_ipython().run_line_magic("laymesh", "native.lay --warnings hide")
assert reopened.outputs[-1].data["image/png"] == second_png
result = render_file("native.lay", output="native-replay.png", dpi=192, show_warnings=False)
assert result.preview_svg.startswith("<svg")
assert base64.b64decode(second_png) == Path("native-replay.png").read_bytes()
with capture_output() as api_display:
    display(result)
assert len(api_display.outputs[-1].data["text/plain"].encode()) < 1024

# A mutable Figure remains vector, then reloads its changed asset in the same
# kernel. The pyplot figure manager must not leak on repeated Notebook runs.
assert not plt.get_fignums()
fig, ax = plt.subplots(figsize=(2.5, 1.5))
ax.plot([0, 1, 2], [1, 2, 1])
fig.tight_layout()
plt.close(fig)
figure_source = 'page=canvas(size=(75mm,50mm))\npage.add(image(src={{fig}}),size=(65mm,auto),offset=(5mm,5mm))'
with capture_output() as vector_first:
    get_ipython().run_cell_magic("laymesh", "--warnings hide --save-source mutable.lay", figure_source)
with capture_output() as vector_same:
    get_ipython().run_cell_magic("laymesh", "--warnings hide --save-source mutable.lay", figure_source)
assert vector_first.outputs[-1].data["image/png"] == vector_same.outputs[-1].data["image/png"]
assert len(list(Path("mutable.assets").glob("*.svg"))) == 1
ax.lines[0].set_ydata([3, 1, 4])
with capture_output() as vector_second:
    get_ipython().run_cell_magic("laymesh", "--warnings hide --save-source mutable.lay", figure_source)
assert vector_first.outputs[-1].data["image/png"] != vector_second.outputs[-1].data["image/png"]
assert len(list(Path("mutable.assets").glob("*.svg"))) == 2

# File magic resolves modules and transparent raster assets relative to the
# source, even with spaces/CJK in paths and a different kernel working folder.
layout = Path("相对素材 # case")
layout.mkdir(exist_ok=True)
PILImage.new("RGBA", (16, 16), (0, 128, 255, 128)).save(layout / "图片.png")
(layout / "materials.lay").write_text('export photo=image(src="图片.png")\n', encoding="utf-8")
(layout / "figure.lay").write_text('import { photo } from "./materials.lay"\npage=canvas(size=(25.4mm,12.7mm),background="none")\npage.add(photo,size=(8mm,8mm),offset=(1mm,1mm))', encoding="utf-8")
with capture_output() as relative:
    get_ipython().run_line_magic("laymesh", '"相对素材 # case/figure.lay" -o "relative export.png" --dpi 96')
with PILImage.open("relative export.png") as image:
    assert image.size == (96, 48)
    assert image.convert("RGBA").getpixel((10, 10)) == (0, 128, 255, 128)
preview_bytes = base64.b64decode(relative.outputs[-1].data["image/png"])
with PILImage.open(io.BytesIO(preview_bytes)) as image:
    assert image.size == (192, 96)
assert relative.outputs[-1].metadata["image/png"] == {"width": 96, "height": 48}

# Preview resolution stays independent of the 1200-DPI default and settings.
Path("export.json").write_text('{"export":{"dpi":72,"png":{"compression":"fast"}}}')
with capture_output() as configured:
    get_ipython().run_cell_magic("laymesh", "-o configured.png --config export.json", 'page=canvas(size=(25.4mm,12.7mm))')
with PILImage.open("configured.png") as image:
    assert image.size == (72, 36)
with PILImage.open(io.BytesIO(base64.b64decode(configured.outputs[-1].data["image/png"]))) as image:
    assert image.size == (192, 96)
with capture_output() as defaults:
    get_ipython().run_cell_magic("laymesh", "-o default.tif", 'page=canvas(size=(2.54mm,1.27mm))')
with PILImage.open("default.tif") as image:
    assert image.size == (120, 60)
    assert image.tag_v2[259] == 5

# Errors are actionable and leave the kernel/next magic usable; no SVG fallback.
for invalid, message in [('page=unknown()', None), ('page=canvas(size=(10mm,10mm))\nvalue={{np.mean(x)}}', "Python 表达式")]:
    try:
        get_ipython().run_cell_magic("laymesh", "--warnings hide", invalid)
    except LayMeshBridgeError as error:
        assert message is None or message in str(error)
    else:
        raise AssertionError("Invalid input was accepted")
with capture_output() as recovered:
    get_ipython().run_cell_magic("laymesh", "--warnings hide", 'page=canvas(size=(10mm,10mm))\npage.add(text("$E=mc^2$"))')
assert "image/png" in recovered.outputs[-1].data
assert "image/svg+xml" not in recovered.outputs[-1].data
assert not list(Path.cwd().rglob(".laymesh-*.lay"))
assert not plt.get_fignums()
kernel_evidence = dict(engine_version=version,
    engine_sha256=hashlib.sha256(Path(command[0]).read_bytes()).hexdigest(),
    python_package_sha256={p.name: hashlib.sha256(p.read_bytes()).hexdigest()
                           for p in sorted(Path(laymesh.__file__).parent.glob("*.py"))},
    data_updates=True, figure_updates=True, unchanged_figure_asset_reused=True,
    standalone_replay=True, relative_modules_and_alpha=True, config_dpi=True,
    default_dpi_1200_and_tiff_lzw=True, errors_recover=True, svg_api_compatible=True,
    png_exact_native_match=True, compact_result_display=True, installed_offline=installed_check)
print(json.dumps(kernel_evidence))
"""


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--installed", action="store_true")
    parser.add_argument("--binary", type=Path, default=ROOT / "target/release/laymesh")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    import nbformat
    from nbclient import NotebookClient
    from PIL import Image

    destination = args.output.resolve()
    destination.mkdir(parents=True, exist_ok=True)
    environment = os.environ.copy()
    if args.installed:
        environment.pop("PYTHONPATH", None)
        environment.pop("LAYMESH_CLI", None)
    else:
        environment["PYTHONPATH"] = str(ROOT / "python")
        environment["LAYMESH_CLI"] = str(args.binary.resolve())
    environment["MPLBACKEND"] = "Agg"
    with tempfile.TemporaryDirectory(prefix="laymesh-jupyter-") as directory:
        work = Path(directory)
        kernel = work / "jupyter/kernels/laymesh-check"
        kernel.mkdir(parents=True)
        (kernel / "kernel.json").write_text(json.dumps({
            "argv": [sys.executable, "-m", "ipykernel_launcher", "-f", "{connection_file}"],
            "display_name": "LayMesh isolated check", "language": "python",
        }))
        environment["JUPYTER_PATH"] = str(work / "jupyter")
        # Kernelspec discovery is performed in the controller process too.
        os.environ["JUPYTER_PATH"] = environment["JUPYTER_PATH"]
        notebook = nbformat.read(ROOT / "examples/jupyter-integration.ipynb", as_version=4)
        nbformat.validate(notebook)
        client = NotebookClient(notebook, timeout=180, kernel_name="laymesh-check",
                                resources={"metadata": {"path": str(work)}})
        repeats = []
        with client.setup_kernel(env=environment):
            for repeat in range(2):
                start = time.perf_counter()
                client.reset_execution_trackers()
                for index, cell in enumerate(notebook.cells):
                    client.execute_cell(cell, index)
                previews = []
                for cell in notebook.cells:
                    for output in cell.get("outputs", []):
                        if "image/png" not in output.get("data", {}):
                            continue
                        assert "image/svg+xml" not in output.data
                        data = base64.b64decode(output.data["image/png"])
                        with Image.open(io.BytesIO(data)) as image:
                            image.load()
                            previews.append({"pixels": list(image.size), "css": output.metadata["image/png"],
                                             "sha256": hashlib.sha256(data).hexdigest()})
                assert len(previews) == 3, previews
                expected = work / "output/laymesh-jupyter/figure.pdf"
                assert list(work.rglob("figure.pdf")) == [expected]
                repeats.append({"run": repeat + 1, "code_cells": 10,
                                "elapsed_ms": (time.perf_counter() - start) * 1000, "previews": previews})
                nbformat.write(notebook, destination / "jupyter-integration-executed.ipynb")
            checks = nbformat.v4.new_code_cell("installed_check = " + repr(args.installed) + "\nimport base64\n" + EXERCISES)
            notebook.cells.append(checks)
            client.execute_cell(checks, len(notebook.cells) - 1)
            lines = [output.text for output in checks.outputs if output.output_type == "stream" and output.name == "stdout"]
            evidence = json.loads("".join(lines).strip().splitlines()[-1])
        binary = args.binary.resolve()
        assert evidence["engine_sha256"] == hashlib.sha256(binary.read_bytes()).hexdigest()
        expected_package = {p.name: hashlib.sha256(p.read_bytes()).hexdigest()
                            for p in sorted((ROOT / "python/laymesh").glob("*.py"))}
        assert evidence["python_package_sha256"] == expected_package
        result = {"mode": "installed-wheel" if args.installed else "source-checkout", "python": sys.version.split()[0],
                  "engine_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
                  "source_sha256": {str(path.relative_to(ROOT)): hashlib.sha256(path.read_bytes()).hexdigest()
                                    for path in [Path(__file__).resolve(), ROOT / "examples/jupyter-integration.ipynb", *sorted((ROOT / "python/laymesh").glob("*.py"))]},
                  "repeats": repeats, "kernel_checks": evidence}
        (destination / "verification.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
        print(json.dumps({"mode": result["mode"], "repeated_code_cells": 20, "cpu_previews": 6,
                          "kernel_checks": evidence}, ensure_ascii=False))


if __name__ == "__main__":
    main()
