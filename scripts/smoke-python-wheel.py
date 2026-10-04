#!/usr/bin/env python3
"""Exercise an installed wheel from outside the checkout, without Node/npm on PATH."""
from __future__ import annotations
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import xml.etree.ElementTree as ET


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--examples", type=Path, help="Also validate every repository .lay example")
    args = parser.parse_args()
    import laymesh
    from laymesh import render_source, render_file
    from laymesh._runtime import bundled_command
    command = bundled_command()
    assert command, "A source checkout is not an installed release wheel"
    vendor = Path(laymesh.__file__).parent / "_vendor"
    manifest = json.loads((vendor / "manifest.json").read_text(encoding="utf-8"))
    assert subprocess.check_output([command[0], "--version"], text=True).strip() == "laymesh " + manifest["rust_version"]
    examples = args.examples.resolve() if args.examples else None
    previous_directory = Path.cwd()
    with tempfile.TemporaryDirectory(prefix="laymesh-installed-test-") as tmp:
        os.chdir(tmp)
        # These changes are process-local. Pip installation happens before this test.
        for name in ("LAYMESH_CLI", "PYTHONPATH", "NODE_PATH", "NODE_OPTIONS"):
            os.environ.pop(name, None)
        os.environ["PATH"] = str(Path(tmp) / "empty-path")
        assert not shutil.which("node") and not shutil.which("npm")
        source = '''page = canvas(size=(120, 90), background="#ffffff")
p = plot(size=(110, 80), x=axis(label="Time (s)"), y=axis(label="Signal"))
p.line(x=[0,1,2,3], y=[1,3,2,4], label="Experiment")
p.legend(position="top_left")
page.add(p, offset=(5,5))
page.add(text("中文 $E=mc^2$ $\\mathbb{R} \\alpha \\sum_{i=1}^n i$", font_size=9), offset=(15,5))
'''
        render_source(source, output="figure.svg", save_source="figure.lay")
        ET.parse("figure.svg")
        for ext, magic in [("pdf", b"%PDF"), ("png", b"\x89PNG\r\n\x1a\n")]:
            render_file("figure.lay", output=f"figure.{ext}", **({"dpi": 150} if ext == "png" else {}))
            assert Path(f"figure.{ext}").read_bytes().startswith(magic)
        scripts = Path(sys.executable).parent
        cli = scripts / ("laymesh.exe" if os.name == "nt" else "laymesh")
        subprocess.run([str(cli), "validate", "figure.lay"], check=True)
        result = subprocess.run([sys.executable, "-m", "laymesh", "inspect", "figure.lay", "--json"], check=True, capture_output=True, text=True)
        json.loads(result.stdout)
        # Reopen a saved data-bound source without the Python namespace.
        import numpy as np
        import pandas as pd
        table = pd.DataFrame({"x": np.arange(4), "y": [1, 3, 2, 4]})
        bound = '''page=canvas(size=(100,80))
d={{values}}
p=plot(size=(90,70))
p.line(x=d["x"],y=d["y"])
page.add(p,offset=(5,5))'''
        render_source(bound, namespace={"values": table}, output="data.svg", save_source="data.lay")
        subprocess.run([str(cli), "render", "data.lay", "-o", "data.pdf"], check=True)
        # Import a Matplotlib Figure, retain the vector material, rerender independently.
        import matplotlib
        matplotlib.use("Agg")
        import matplotlib.pyplot as plt
        fig, ax = plt.subplots()
        ax.plot([0, 1], [1, 2])
        render_source('page=canvas(size=(100,80))\npage.add(image(src={{fig}}),size=(90,auto))', namespace={"fig": fig}, output="mpl.pdf", save_source="mpl.lay")
        plt.close(fig)
        subprocess.run([str(cli), "render", "mpl.lay", "-o", "mpl.png"], check=True)
        # Execute both notebook magics through an actual IPython shell.
        os.environ["IPYTHONDIR"] = str(Path(tmp) / "ipython")
        from IPython.core.interactiveshell import InteractiveShell
        from laymesh.ipython import load_ipython_extension
        from unittest.mock import patch
        from traitlets.config import Config
        shell = InteractiveShell.instance(config=Config({"HistoryManager": {"enabled": False}}))
        load_ipython_extension(shell)
        with patch("laymesh.ipython.display") as shown:
            shell.run_cell_magic("laymesh", "--warnings hide --save-source notebook.lay -o notebook.svg", 'page=canvas(size=(40,30))\npage.add(text("Notebook $E=mc^2$"),offset=(2,2))')
            shell.run_line_magic("laymesh", "notebook.lay --warnings hide -o notebook.pdf")
            assert shown.call_count == 2
        assert Path("notebook.svg").is_file() and Path("notebook.pdf").read_bytes().startswith(b"%PDF")
        if examples:
            # Modules without a page are not standalone entry points.
            entries = sorted(p for p in examples.rglob("*.lay") if "canvas(" in p.read_text(encoding="utf-8"))
            for entry in entries:
                subprocess.run([str(cli), "validate", str(entry)], check=True, stdout=subprocess.DEVNULL)
            print(f"Validated {len(entries)} native example entry points with bundled runtime")
        print(json.dumps({"wheel_version": manifest["version"], "target": manifest["target"], "engine": "rust", "exports": ["SVG", "PDF", "PNG"], "system_node": False, "saved_data": True, "matplotlib": True, "notebook_magics": True}))
        os.chdir(previous_directory)


if __name__ == "__main__":
    main()
