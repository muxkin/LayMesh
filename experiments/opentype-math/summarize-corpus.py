#!/usr/bin/env python3
"""Summarize the fixed RaTeX corpus audit and verify fixture provenance."""
import argparse
import hashlib
import json
from collections import Counter, defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("audit", type=Path)
    parser.add_argument("--output", type=Path, default=ROOT / "target/opentype-math-validation/corpus-summary.json")
    args = parser.parse_args()
    provenance = json.loads((ROOT / "tests/math/ratex-0.1.14/provenance.json").read_text())
    for entry in provenance["files"]:
        data = (ROOT / "tests/math/ratex-0.1.14" / entry["file"]).read_bytes()
        assert hashlib.sha256(data).hexdigest() == entry["sha256"]
    fonts = json.loads((ROOT / "tests/fonts/math/manifest.json").read_text())
    for entry in fonts:
        assert hashlib.sha256((ROOT / "tests/fonts/math" / entry["file"]).read_bytes()).hexdigest() == entry["sha256"]
    rows = json.loads(args.audit.read_text())
    counters = defaultdict(Counter)
    categories = defaultdict(Counter)
    missing = defaultdict(list)
    for row in rows:
        error = row.get("error", "")
        if not error:
            status = "rendered"
            if row["font"] != "ratex-katex":
                assert not row["bounds_errors"], row
        elif error.startswith("公式包含"):
            status = "policy_rejected"
        elif error.startswith("RaTeX"):
            status = "parse_rejected"
        elif "缺少字形" in error or "缺少伸缩符号" in error:
            status = "font_glyph_missing"
            if row["style"] == "display":
                missing[row["font"]].append({key: row[key] for key in ["suite", "line", "source", "error"]})
        else:
            raise AssertionError(row)
        counters[row["font"]][status] += 1
        categories[row["suite"]][status] += 1
    report = {"upstream": provenance, "font_fixtures": fonts, "layout_checks": len(rows),
              "case_entries": len(rows) // 8, "distinct_formulas":len({row["source"]for row in rows}), "styles": ["inline", "display"],
              "outcomes_by_font": dict(counters), "outcomes_by_suite": dict(categories),
              "font_coverage_limits": dict(missing)}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps({key: report[key] for key in ["case_entries", "distinct_formulas", "layout_checks", "outcomes_by_font", "outcomes_by_suite"]}, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
