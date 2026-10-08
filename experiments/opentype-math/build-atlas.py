#!/usr/bin/env python3
"""Render editable domain comparisons and merge them into a vector PDF atlas."""
import argparse
import json
import subprocess
from pathlib import Path

import pymupdf as fitz

ROOT = Path(__file__).resolve().parents[2]
FONTS = [
    ("KaTeX", "ratex-katex"),
    ("Latin Modern Math", str(ROOT / "tests/fonts/math/latinmodern-math.otf")),
    ("STIX Two Math", str(ROOT / "tests/fonts/math/STIX2Math.otf")),
    ("XITS Math", str(ROOT / "tests/fonts/math/XITSMath-Regular.otf")),
]


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--binary", type=Path, default=ROOT / "target/debug/laymesh")
    p.add_argument("--output", type=Path, default=ROOT / "target/opentype-math-validation")
    args = p.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    cases = json.loads(Path(__file__).with_name("domain-cases.json").read_text())
    atlas = fitz.open()
    records = []
    for index, (title, rows) in enumerate(cases.items(), 1):
        lines = [
            'page=canvas(size=(520mm,385mm),background="#ffffff",'
            'font_family=' + json.dumps(str(ROOT / "tests/fonts/DejaVuSans.ttf")) + ')',
            f'page.add(text({json.dumps(title)},font_size=22pt),offset=(10mm,7mm))',
        ]
        for col, (label, font) in enumerate(FONTS):
            x = 10 + col * 128
            lines.append(f'page.add(text({json.dumps(label)},font_size=13pt,color="#27364a"),offset=({x}mm,23mm))')
            for row, (label, source) in enumerate(rows):
                y = 40 + row * 40
                lines.append(f'page.add(text({json.dumps(label)},font_size=9pt,color="#687385"),offset=({x}mm,{y}mm))')
                assert '"""' not in source
                # Raw strings also preserve backslashes inside mhchem's $...$ islands.
                lines.append(f'page.add(formula(r"""{source}""",math_font={json.dumps(font)},style=display,font_size=14pt),offset=({x}mm,{y+7}mm))')
        lines.append(f'page.add(text("OpenType MATH / selected-font geometry / page {index}",font_size=9pt,color="#687385"),offset=(10mm,373mm))')
        source = output / f"domain-{index}.lay"
        source.write_text("\n".join(lines) + "\n")
        for fmt in ["pdf", "svg", "png", "pptx"]:
            target = source.with_suffix("." + fmt)
            command=[str(args.binary.resolve()), "render", str(source), "-o", str(target)]
            if fmt == "png": command += ["--dpi","180"]
            run = subprocess.run(command, capture_output=True, text=True, check=True)
            warnings = run.stderr.strip().splitlines()
            assert not warnings or (fmt == "pptx" and all("W_PPTX_FONT" in line for line in warnings)), run.stderr
        with fitz.open(source.with_suffix(".pdf")) as page:
            atlas.insert_pdf(page)
        records.append({"page": index, "category": title, "cases": len(rows), "columns": len(FONTS)})
    target = output / "domain-atlas.pdf"
    atlas.save(target, deflate=True)
    assert len(atlas) == len(cases)
    atlas.close()
    subprocess.run(["pdftoppm", "-scale-to", "2200", "-png", str(target), str(output / "domain-preview")], check=True, capture_output=True)
    (output / "domain-atlas.json").write_text(json.dumps(records, indent=2) + "\n")
    print(json.dumps({"atlas": str(target), "pages": records}, indent=2))


if __name__ == "__main__":
    main()
