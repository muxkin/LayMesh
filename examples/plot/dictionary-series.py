"""Python dictionary binding; run from the repository root with PYTHONPATH=python.
Outputs and the reproducible .lay source are written to dictionary-output/.
"""
from pathlib import Path
from laymesh import render_source

HERE = Path(__file__).resolve().parent
series = {
    "Baseline": [1, 2, 3, 3.5],
    "Annealed": [1.2, 2.8, 3.1, 4],
    "Cold worked": [2, 2.2, 2.8, 3.1],
}
source = "\n".join(
    "series = {{series}}" if line.startswith("series=") else line
    for line in HERE.joinpath("dictionary-series.lay").read_text().splitlines()
)
output = HERE / "dictionary-output"
output.mkdir(exist_ok=True)
result = render_source(
    source, namespace={"series": series}, base_dir=output,
    output=output / "series.pdf", save_source="series.lay",
)
print(result.saved_source)
