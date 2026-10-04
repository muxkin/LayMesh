#!/usr/bin/env python3
"""Check reader-facing Markdown links, images and gallery layout."""
from pathlib import Path
import json
import re

ROOT = Path(__file__).resolve().parents[1]
pages = [ROOT / "README.md", ROOT / "README.en.md", *sorted((ROOT / "docs").rglob("*.md")), ROOT / "site/README.md", ROOT / "site/README.en.md"]
problems = []
image_count = 0
for page in pages:
    source = page.read_text(encoding="utf-8")
    # Schema fields belong in the programming API, not reader-facing examples.
    is_api_reference = page.name in ("api-reference.en.md", "api-reference.zh-CN.md", "node-reference.en.md", "node-reference.zh-CN.md", "node.en.md", "node.zh-CN.md", "migration.en.md", "migration.zh-CN.md")
    if not is_api_reference and re.search(r"Scene v\d+|schemaVersion|Scene schema is version|scene: 3\b", source, re.I):
        problems.append(f"{page}: internal version number")
    for is_image, target in re.findall(r"(!?)\[[^\]]*\]\(([^)]+)\)", source):
        if target.startswith(("https://", "http://", "mailto:")) or target.startswith("#"):
            continue
        local = target.split("#", 1)[0]
        if not (page.parent / local).exists():
            problems.append(f"{page}: missing {target}")
        image_count += bool(is_image)

gallery = ROOT / "docs/gallery"
catalog = json.loads((ROOT / "examples/gallery/catalog.json").read_text(encoding="utf-8"))
entries_checked = 0
for language in ("zh-CN", "en"):
    for category in (c["slug"] for c in catalog["categories"]):
        page = gallery / f"{category}.{language}.md"
        content = page.read_text(encoding="utf-8")
        entries = re.split(r"(?=^### )", content, flags=re.M)[1:]
        expected = next(item for item in catalog["categories"] if item["slug"] == category)
        if len(entries) != len(expected["items"]):
            problems.append(f"{page}: expected {len(expected['items'])} examples, found {len(entries)}")
        entries_checked += len(entries)
        for entry in entries:
            if not re.search(r"```\n\n!\[[^\]]+\]\([^\n]+\)", entry):
                problems.append(f"{page}: image is not directly below code in {entry.splitlines()[0]}")
            labels = ("Source:", "Reproduce:", "Observed CLI output:") if language == "en" else ("源码：", "复现命令：", "实测：")
            if not all(piece in entry for piece in labels):
                problems.append(f"{page}: incomplete entry {entry.splitlines()[0]}")

for language in ("zh-CN", "en"):
    for kind, expected in (("notebook", 2), ("comprehensive", 8)):
        page = gallery / f"{kind}.{language}.md"
        content = page.read_text(encoding="utf-8")
        actual = len(re.findall(r"```\n\n!\[[^\]]+\]\([^\n]+\)", content))
        if actual != expected:
            problems.append(f"{page}: expected {expected} images directly below code, found {actual}")

if problems:
    raise SystemExit("\n".join(problems))
print(f"Checked {len(pages)} Markdown pages, {image_count} inline images, {entries_checked} focused and 20 extended examples across two languages, and local links.")
