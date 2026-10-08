# RaTeX 0.1.14 corpus fixtures

Source: https://github.com/erweixin/RaTeX/tree/ae391d727ac615437c63c308f4538d971a84bede

The original golden/parser/layout/proof text files and website chemistry/physics
JSON files are copied without changing their bytes. `provenance.json` records
paths, hashes, tag, and commit. The upstream MIT license is included. Preserve
the original fixtures when adding regressions; add LayMesh-specific cases to
`experiments/opentype-math/domain-cases.json` instead.

The harness deduplicates overlapping primary math suites, retains chemistry
and physics category entries, and removes a single outer `$...$` pair from
website examples. Literal `\n` sequences in parser fixtures are retained:
some are intentionally invalid inputs. No network access is needed for tests.

`expected-errors.json` freezes only the existing parser/policy rejections and
font glyph-coverage errors for the pinned test faces. A new rejection, changed
error, or unexpected success fails the test and needs review. Accepted OpenType
renders additionally check finite dimensions and outline bounds; default
KaTeX rendering is compared for command acceptance, not pixel dimensions.

Run `cargo test -p laymesh-core --test opentype_math_corpus`. Set the optional
`LAYMESH_MATH_AUDIT` environment variable to an absolute JSON output path to
retain every outcome. `summarize-corpus.py` verifies fixture hashes and separates
rendered, rejected, and missing-glyph results.
