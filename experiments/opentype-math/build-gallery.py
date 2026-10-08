#!/usr/bin/env python3
"""Build a self-contained gallery retaining every original RaTeX fixture entry."""
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[2]
HERE = Path(__file__).resolve().parent
FIXTURES = ROOT / "tests/math/ratex-0.1.14"
FONT_LABELS = {"ratex-katex":"KaTeX", "latinmodern":"Latin Modern Math", "stix":"STIX Two Math", "xits":"XITS Math"}
SUITES = {"golden":"基础数学", "parser":"解析器", "layout":"布局", "chemistry":"化学", "physics":"物理", "proofs":"证明树",
    "lexer":"词法器", "website-math":"官网数学", "website-proofs":"官网证明树", "chemistry-golden":"化学原始测试", "unicode":"多语言脚本", "domain":"领域补充", "text":"文本回退测试"}


def cases(provenance):
    result = []
    files = {f["file"]:f for f in provenance["files"]}
    for suite in SUITES:
        if suite == "text":
            rows=[(i,source,title) for i,(title,source) in enumerate(json.loads((HERE/"text-fallback-cases.json").read_text()),1)]
            kind,fixture,upstream="text-test","text-fallback-cases.json",None
        elif suite == "domain":
            domain = json.loads((HERE/"domain-cases.json").read_text())
            rows = [(i, source, title+" / "+label) for i, (title,label,source) in enumerate(
                [(title,label,source) for title,items in domain.items() for label,source in items],1)]
            kind, fixture, upstream = "domain", "domain-cases.json", None
        elif suite in ("chemistry","physics","website-math","website-proofs"):
            fixture = suite+".json"
            rows = []
            for i, source in enumerate(json.loads((FIXTURES/fixture).read_text())["formulas"],1):
                source = source[1:-1] if source.startswith("$") and source.endswith("$") else source
                rows.append((i,source,""))
            kind, upstream = "json", files[fixture]["path"]
        else:
            fixture = suite+".txt"
            rows = []
            section = ""
            for line, raw in enumerate((FIXTURES/fixture).read_text().splitlines(),1):
                source = raw.strip()
                if source.startswith("#"):
                    section=source.lstrip("# ")
                elif source:
                    rows.append((line,source,section))
            kind, upstream = "text", files[fixture]["path"]
        for ordinal, (position,source,label) in enumerate(rows,1):
            # Preserve the fixture bytes but remove the optional outer math
            # delimiters when passing a formula to the renderer.
            source = source[1:-1] if source.startswith("$") and source.endswith("$") else source
            result.append({"id":f"{suite}-{ordinal:04}","suite":suite,"ordinal":ordinal,
                "position":position,"position_kind":kind,"fixture":fixture,"upstream_path":upstream,
                "source":source,"label":label})
    return result


def classify(result):
    error = result.get("error", "")
    if not error: return "rendered"
    if error.startswith("公式包含"): return "policy_rejected"
    if error.startswith("RaTeX"): return "parse_rejected"
    if "缺少字形" in error or "缺少伸缩符号" in error: return "font_glyph_missing"
    raise ValueError(f"Unclassified error: {result}")


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output",type=Path,default=ROOT/"target/opentype-math-gallery")
    parser.add_argument("--audit",type=Path,default=ROOT/"target/opentype-math-validation/corpus.json")
    parser.add_argument("--binary",type=Path,default=ROOT/"target/release/examples/math_gallery")
    parser.add_argument("--reuse",action="store_true",help="Reuse verified renders to update only the page")
    args=parser.parse_args()
    output=args.output.resolve()
    output.mkdir(parents=True,exist_ok=True)
    provenance=json.loads((FIXTURES/"provenance.json").read_text())
    fonts=json.loads((ROOT/"tests/fonts/math/manifest.json").read_text())
    text_fonts=json.loads((ROOT/"tests/fonts/text/manifest.json").read_text())
    for directory,entries in [(FIXTURES,provenance["files"]),(ROOT/"tests/fonts/math",fonts),(ROOT/"tests/fonts/text",text_fonts)]:
        for entry in entries:
            assert hashlib.sha256((directory/entry["file"]).read_bytes()).hexdigest()==entry["sha256"]
    entries=cases(provenance)
    unique=list(dict.fromkeys(c["source"] for c in entries))
    inputs=output/"inputs.json"
    inputs.write_text(json.dumps(unique,ensure_ascii=False))
    engine=[ROOT/"Cargo.lock",ROOT/"Cargo.toml",ROOT/"rust-toolchain.toml"]+sorted(
        p for p in (ROOT/"crates").rglob("*") if p.is_file() and p.suffix in {".rs",".toml",".json"})+[ROOT/"tests/fonts/text/fixtures.rs",ROOT/"tests/fonts/text/manifest.json",ROOT/"tests/fonts/math/manifest.json"]
    fingerprint=hashlib.sha256()
    for path in engine:
        fingerprint.update(str(path.relative_to(ROOT)).encode())
        fingerprint.update(path.read_bytes())
    digest=fingerprint.hexdigest()
    if args.reuse:
        previous=json.loads((output/"data.json").read_text())
        assert previous["meta"]["renderer_fingerprint"]==digest,"Renderer changed; rebuild SVGs"
    else:
        subprocess.run([str(args.binary.resolve()),str(inputs),str(output)],check=True)
    rendered=json.loads((output/"renders.json").read_text())
    assert [r["source"] for r in rendered]==unique
    references={}
    for row in json.loads(args.audit.read_text()):
        key=(row["source"],row["font"],row["style"])
        if key in references: assert references[key].get("error")==row.get("error")
        references[key]=row
    checked={policy:Counter() for policy in ("variants","text_variants")}
    audited_checks=0
    exported=0
    preview_errors=0
    for record in rendered:
      for policy in checked:
        for font,styles in record[policy].items():
            for style,result in styles.items():
                ref=references.get((record["source"],font,style)) if policy=="variants" else None
                if ref is not None:
                    assert (ref.get("code"),ref.get("error"))==(result.get("code"),result.get("error")),(record["source"],font,style,result)
                    audited_checks+=1
                result["status"]=classify(result)
                result["audit_checked"]=ref is not None
                result["expected"]=ref is not None and bool(result.get("error"))
                checked[policy][result["status"]]+=1
                if "svg" in result:
                    exported+=1
                    file=output/result["svg"]
                    assert file.is_file() and file.stat().st_size>100
                    result["sha256"]=hashlib.sha256(file.read_bytes()).hexdigest()
                elif result["status"]=="rendered":
                    assert result.get("preview_error") and (ref is None or ref["bounds_errors"]),result
                    preview_errors+=1
    # Adding fixtures must not silently drop any formula from the fixed audit.
    assert audited_checks==len(references),"Fixed audit coverage was lost"
    lookup={r["source"]:i for i,r in enumerate(rendered)}
    for c in entries: c["render_index"]=lookup[c["source"]]
    expanded={policy:Counter() for policy in checked}
    for c in entries:
      for policy in checked:
        for font in FONT_LABELS:
            for style in ("display","inline"):
                expanded[policy][rendered[c["render_index"]][policy][font][style]["status"]]+=1
    metadata={"generated_at":datetime.now(timezone.utc).isoformat(),"renderer_fingerprint":digest,
        "git_revision":subprocess.check_output(["git","rev-parse","HEAD"],cwd=ROOT,text=True).strip(),
        "upstream":provenance,"font_fixtures":fonts,"text_font_fixtures":text_fonts,"suites":[{"id":suite,"label":label,"count":sum(c["suite"]==suite for c in entries)} for suite,label in SUITES.items()],
        "fonts":[{"id":id,"label":label} for id,label in FONT_LABELS.items()],
        "case_entries":len(entries),"upstream_entries":sum(c["upstream_path"] is not None for c in entries),
        "distinct_formulas":len(unique),"styles":["display","inline"],"text_policies":["strict","text"],"layout_checks":len(entries)*16,
        "distinct_layout_checks":len(unique)*16,"outcomes":dict(expanded["text_variants"]),"distinct_outcomes":dict(checked["text_variants"]),
        "policy_outcomes":{policy:dict(counts) for policy,counts in expanded.items()},
        "duplicate_entries_retained":len(entries)-len(unique),"all_audited_outcomes_match":True,
        "audit_checks":audited_checks,"supplemental_checks":len(unique)*16-audited_checks,
        "svg_files":exported,"preview_errors":preview_errors}
    data={"meta":metadata,"cases":entries,"renders":rendered}
    (output/"data.json").write_text(json.dumps(data,ensure_ascii=False,separators=(",",":"))+"\n")
    (output/"summary.json").write_text(json.dumps(metadata,ensure_ascii=False,indent=2)+"\n")
    public=output/"sources"
    public.mkdir(exist_ok=True)
    for file in ["provenance.json","LICENSE"]+[f["file"] for f in provenance["files"]]:
        shutil.copyfile(FIXTURES/file,public/file)
    shutil.copyfile(HERE/"domain-cases.json",public/"domain-cases.json")
    shutil.copyfile(HERE/"text-fallback-cases.json",public/"text-fallback-cases.json")
    shutil.copyfile(ROOT/"tests/fonts/text/manifest.json",public/"text-fonts-manifest.json")
    for file in (ROOT/"tests/fonts/text").glob("*.txt"):
        shutil.copyfile(file,public/file.name)
    for file in (ROOT/"tests/fonts/math").glob("*.txt"):
        shutil.copyfile(file,public/file.name)
    for file in (HERE/"gallery").iterdir():
        if file.is_file(): shutil.copyfile(file,output/file.name)
    print(json.dumps({"directory":str(output),"cases":len(entries),"upstream_cases":metadata["upstream_entries"],
        "distinct_formulas":len(unique),"checks":len(entries)*16,"svg_files":exported,"preview_errors":preview_errors,"outcomes":{policy:dict(counts) for policy,counts in expanded.items()}},ensure_ascii=False,indent=2))


if __name__=="__main__": main()
