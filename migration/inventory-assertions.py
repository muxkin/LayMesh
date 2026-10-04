#!/usr/bin/env python3
"""Extract the baseline test/ assertion inventory without executing JavaScript.

Regeneration only: tree-sitter==0.25.2, tree-sitter-javascript==0.25.0.
The generated inventory and the coverage gate need no parser dependency.
"""
import hashlib
import json
import subprocess
from pathlib import Path
from tree_sitter import Language, Parser
import tree_sitter_javascript

ROOT = Path(__file__).resolve().parents[1]
BASELINE = "78db22d"


def walk(node):
    yield node
    for child in node.named_children:
        yield from walk(child)


def generate():
    parser = Parser(Language(tree_sitter_javascript.language()))
    files = subprocess.check_output(
        ["git", "ls-tree", "--name-only", BASELINE, "tests/"], cwd=ROOT, text=True
    ).splitlines()
    assertions, tests, sources = [], [], {}
    for file in files:
        if not file.endswith(".test.mjs"):
            continue
        source = subprocess.check_output(["git", "show", f"{BASELINE}:{file}"], cwd=ROOT)
        tree = parser.parse(source)
        if tree.root_node.has_error:
            raise ValueError(f"Cannot parse complete baseline source: {file}")
        sources[file] = hashlib.sha256(source).hexdigest()
        nodes = list(walk(tree.root_node))
        text = lambda node: source[node.start_byte:node.end_byte].decode() if node else ""
        calls = [n for n in nodes if n.type == "call_expression"]
        test_ranges = []
        for call in calls:
            if text(call.child_by_field_name("function")) != "test":
                continue
            args = call.child_by_field_name("arguments").named_children
            title = text(args[0])[1:-1]
            test_id = f"{file}:{call.start_point.row + 1}"
            test_ranges.append((call.start_byte, call.end_byte, test_id))
            tests.append({"id": test_id, "file": file, "name": title,
                          "line": call.start_point.row + 1})
        functions = [n for n in nodes if n.type in ("function_declaration", "function_expression", "arrow_function")]
        for call in calls:
            function = text(call.child_by_field_name("function"))
            if function != "assert" and not function.startswith("assert."):
                continue
            owner = next((tid for lo, hi, tid in test_ranges if lo <= call.start_byte < hi), None)
            enclosing = [n for n in functions if n.start_byte <= call.start_byte < n.end_byte]
            named = [n for n in enclosing if n.child_by_field_name("name")]
            helper = text(min(named, key=lambda n: n.end_byte - n.start_byte).child_by_field_name("name")) if named else None
            ancestors, parent = [], call.parent
            while parent and parent.type != "program":
                if parent.type in ("for_statement", "for_in_statement", "while_statement"):
                    body = parent.child_by_field_name("body")
                    ancestors.append(source[parent.start_byte:body.start_byte].decode() if body else text(parent))
                parent = parent.parent
            invocations = []
            if not owner and helper:
                invocations = [{"line": c.start_point.row + 1,
                                "test": next((tid for lo, hi, tid in test_ranges if lo <= c.start_byte < hi), None),
                                "expression": text(c)}
                               for c in calls if text(c.child_by_field_name("function")) == helper]
            assertions.append({"id": f"{file}:{call.start_point.row + 1}:{call.start_point.column + 1}",
                "file": file, "line": call.start_point.row + 1, "column": call.start_point.column + 1,
                "test": owner, "helper": helper if not owner else None,
                "assertion": text(call), "parameter_context": ancestors, "helper_invocations": invocations})
    assert len(tests) == 183, len(tests)
    assert len({a['id'] for a in assertions}) == len(assertions)
    inventory = {"baseline_commit": BASELINE, "source_sha256": sources,
        "test_count": len(tests), "assertion_call_sites": len(assertions),
        "note": "Static assertion call sites, including helper assertions. Loops and helper invocations are retained as context; this is not a count of executed parameter instances or a coverage claim.",
        "tests": tests, "assertions": assertions}
    dest = ROOT / "migration/legacy-assertions.json"
    dest.write_text(json.dumps(inventory, ensure_ascii=False, indent=2) + "\n")
    print(f"{len(tests)} tests; {len(assertions)} assertion sites -> {dest}")


if __name__ == "__main__":
    generate()
