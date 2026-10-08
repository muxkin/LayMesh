#!/usr/bin/env python3
"""Reproduce the font comparison and verify its SVG/PDF/PNG exports."""
import argparse
import hashlib
import json
import subprocess
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT=Path(__file__).resolve().parents[2]

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument("--binary",type=Path,default=ROOT/"target/debug/laymesh")
    p.add_argument("--output",type=Path,default=ROOT/"target/opentype-math-validation")
    a=p.parse_args()
    a.binary=a.binary.resolve()
    a.output=a.output.resolve()
    a.output.mkdir(parents=True,exist_ok=True)
    fixture=ROOT/"tests/fonts/math"
    manifest=json.loads((fixture/"manifest.json").read_text())
    for font in manifest:
        assert hashlib.sha256((fixture/font["file"]).read_bytes()).hexdigest()==font["sha256"]
    source=Path(__file__).with_name("comparison.lay")
    outputs={}
    for fmt in ["svg","pdf","png","pptx"]:
        target=a.output/f"comparison.{fmt}"
        command=[str(a.binary),"render",str(source),"-o",str(target)]
        if fmt=="png":command+=["--dpi","180"]
        run=subprocess.run(command,capture_output=True,text=True,check=True)
        warnings=run.stderr.strip().splitlines()
        assert not warnings or (fmt=="pptx" and all("W_PPTX_FONT" in line for line in warnings)),run.stderr
        outputs[fmt]={"file":str(target.relative_to(ROOT)) if target.is_relative_to(ROOT) else str(target),
                      "sha256":hashlib.sha256(target.read_bytes()).hexdigest(),"bytes":target.stat().st_size,
                      "warnings":warnings}
    root=ET.parse(a.output/"comparison.svg").getroot()
    paths=[n for n in root.iter() if n.tag.endswith("}path")]
    assert len(paths)>200
    assert (a.output/"comparison.pdf").read_bytes().startswith(b"%PDF-")
    assert (a.output/"comparison.png").read_bytes().startswith(b"\x89PNG")
    assert (a.output/"comparison.pptx").read_bytes().startswith(b"PK")
    bold=a.output/"xits-bold.lay"
    bold.write_text('page=canvas(size=(90mm,30mm),background="#ffffff")\n'
                    'page.add(formula(r"E=mc^2",math_font="'+str(fixture/"XITSMath-Bold.otf")
                    +'",font_size=22pt,style=display),offset=(5mm,5mm))\n')
    subprocess.run([str(a.binary),"render",str(bold),"-o",str(bold.with_suffix(".pdf"))],check=True,capture_output=True)
    report={"fonts":manifest,"comparison_columns":4,"formula_cases_per_column":11,
            "svg_paths":len(paths),"outputs":outputs}
    (a.output/"verification.json").write_text(json.dumps(report,indent=2)+"\n")
    print(json.dumps(report,indent=2))

if __name__=="__main__":
    main()
