# OpenType mathematical fonts

Branch: `codex/opentype-math-fonts`. This branch is not a published release.

`formula(..., math_font="XITS Math")` selects an installed OpenType math font.
A font file path works on native hosts and with fonts supplied through the
in-memory/WASM host. Paths are relative to the defining `.lay` module (or
`.lcss` stylesheet). `math_font` also inherits from the canvas, text, span,
plot, and plot style. Use raw DSL strings for LaTeX, especially when mhchem
contains embedded `$...$` expressions.

```lay
page=canvas(size=(160mm,60mm),math_font="XITS Math")
page.add(formula(r"\ce{A <=>[{forward}][{reverse}] B}",font_size=16pt))
```

The default `ratex-katex` backend is retained. The OpenType backend reuses the
RaTeX parser and LayMesh exporters, but obtains glyphs, advances, math
constants, italic corrections, accent attachments, script kerns, stretch
variants, and connector assemblies from the selected font. It never
substitutes a KaTeX face for an explicitly selected math font.

Covered constructions include fractions/binomials, continued fractions,
scripts, indexed roots, operators and limits, `left/right/middle`, matrices,
arrays with solid/dashed rules, aligned equations and tags, commutative
diagrams, labelled arrows, braces/brackets, accents, enclosures/cancellation,
phantoms, overlap/raise commands, line breaks, math alphabets, and inline math.
RaTeX's mhchem (`ce`/`pu`) and bussproofs (`prooftree`) use the same font-aware
constructions. Proof trees support unary through quinary inference, labels,
solid/dashed/no rules, nested branches, and changing root direction.

A font may lack a horizontal arrow assembly, and some installed STIX Two
2.02 arrow recipes contain invalid terminal connectors. For horizontal
arrows and equality only, the backend then extends the shaft region of that
font's native outline while preserving its end shapes and stroke thickness.
Other finite stretch constructions use their largest native variant when
no assembly exists. Missing glyphs, missing MATH tables, embedding
restrictions, or unavailable fonts fail explicitly.

`mathcal` and `mathscr` currently select the same Unicode script alphabet;
font-specific GSUB stylistic-set selection is not implemented. Math alphabet
commands follow the inner-command override used by
[KaTeX](https://github.com/KaTeX/KaTeX/blob/v0.16.22/src/Options.js):
`\mathbf{\mathcal A}` is script A, not bold script A, while
`\mathcal{\mathbf A}` is bold roman A. Use `\mathbf{A1}` for bold roman
letters/digits and `\bm{\alpha x}` for bold italic symbols. Native styled
Unicode letters also retain their selected-font glyphs. XITS Math Bold
is a supplementary face with incomplete stretch coverage: algebra works,
and missing radical constructions report an error. Regular XITS Math supports
`mathbf`, `boldsymbol`, and `bm` without selecting the standalone bold face.

## Fixed upstream corpus

Fixtures come from [RaTeX v0.1.14](https://github.com/erweixin/RaTeX/tree/ae391d727ac615437c63c308f4538d971a84bede),
the version used by LayMesh. The original source files, SHA-256 hashes, and
MIT license are retained under `tests/math/ratex-0.1.14`:

- Golden formulas: 1075 entries; parser comparison: 791; layout comparison: 29.
- Chemistry: 91; physics: 22; proof trees: 38.
- Additional editable domain cases: 32, under `domain-cases.json`.

After deduplicating the overlapping primary math suites, the audit runs 1931
case entries (1921 distinct formulas), four fonts, and inline/display styles:
**15,448 layout checks**. Expected parser/policy errors and real font-coverage
limits are recorded separately; they are not counted as successful renders.

| Backend | Rendered checks | Parser/policy rejection | Missing-glyph checks |
| --- | ---: | ---: | ---: |
| KaTeX default | 3742 | 120 | 0 |
| Latin Modern Math | 3646 | 120 | 96 |
| STIX Two Math | 3708 | 120 | 34 |
| XITS Math | 3708 | 120 | 34 |

All 91 chemistry, 22 physics, 38 proof-tree cases and 32 additional domain
cases render in every font and style. No unimplemented OpenType node remains
in the accepted corpus. Every successful OpenType render checks finite
geometry and exported outline bounds. The missing-glyph cases include CJK,
other scripts, emoji, and some symbols absent from Latin Modern Math; choosing
a math font does not give it glyphs that the file does not contain.

The focused regressions also verify MATH-constant fault injection, assembly
parts, alphabet codepoints, text/chemistry roman forms, relative paths, inline
baselines, plot labels, accent/operator placement, proof rules/root direction,
alignment/tag placement, 30-level delimiter nesting, zero-width spacing,
cache invalidation, embedding restrictions, and explicit errors.

OpenType partial bonds (`~`, `~-`, `~--`, `~=`, `-~-`) are native compound
constructions. A per-parser adapter retains the bond semantics from RaTeX
0.1.14's mhchem token productions before their KaTeX lap/raise/tiny-hyphen
recipes expand. It delegates to the original `ce` handler, preserving the
shared recursion limit. No formula text or font name selects a special case.
The native minus outline supplies the ink endpoints, advance, axis and
thickness; the dashed row uses three horizontally shortened copies of that
outline. Every layer inherits the current math style, size and color.
This adapter is tied to the pinned upstream productions, covered by tests;
ordinary overlays and the default RaTeX backend keep their existing behavior.

The bond tests measure actual outline endpoints and thickness in **1,280
cases**: four font files (including XITS Bold), four sizes (6–48 pt), both
formula styles, four mixed-bond spellings, and ten contexts including
superscripts, two levels of subscripts, fractions and explicit script styles.
Another 48 chain cases include the bare dashed bond, nested chemistry and
color; they verify the native axis, consistent atom/bond scale, layer spacing
and clearance from adjacent atoms. Separate parser tests preserve ordinary
ASTs and upstream recursion errors. Rendering checks cover these paths in
SVG, PDF, PNG and PPTX. Math minus and text hyphen remain distinct.

Nested math alphabets are tested in both command orders, independently of
the real bold-letter cases. `regressions.lay` provides an enlarged, editable
comparison; `chemical-bonds.lay` adds compound bonds in different math styles.
The existing default RaTeX backend does not select its sans italic face for
`\mathsfit`; the OpenType backend selects the correct Unicode alphabet.

## Reproduce

```sh
mkdir -p target/opentype-math-validation
LAYMESH_MATH_AUDIT="$PWD/target/opentype-math-validation/corpus.json" \
  cargo test -p laymesh-core --test opentype_math --test opentype_math_corpus
python3 experiments/opentype-math/summarize-corpus.py \
  target/opentype-math-validation/corpus.json
cargo test -p laymesh-render --test opentype_math
cargo build -p laymesh-cli
python3 experiments/opentype-math/verify.py
.venv/bin/python experiments/opentype-math/build-atlas.py
target/debug/laymesh render experiments/opentype-math/regressions.lay \
  -o target/opentype-math-validation/regressions.png --dpi 160
target/debug/laymesh render experiments/opentype-math/chemical-bonds.lay \
  -o target/opentype-math-validation/chemical-bonds.png --dpi 180
```

Outputs go to `target/opentype-math-validation`: the original comparison,
a four-page vector `domain-atlas.pdf`, editable `.lay` pages, SVG/PNG/PPTX
exports, PDF-derived previews, and machine-readable audit results. The atlas
builder needs PyMuPDF and Poppler. A release binary can be selected with
`--binary target/release/laymesh` for both render scripts.

The deterministic font fixtures are unmodified Latin Modern Math 1.959,
STIX Two Math 2.02 b142, and XITS Math 1.302 regular/bold, with license notices.
They are test assets and are not bundled into distribution packages.

## Browser corpus gallery

The gallery retains every nonempty, noncomment entry in all eleven formula
corpus files from the pinned RaTeX v0.1.14 checkout, including duplicates:
the six primary suites above, lexer comparison (791), website mathematics
(1048), website proof trees (38), the original chemistry text fixture (103),
and the Unicode smoke-test script (18). With the 32 domain examples this is
**4076 entries, 1938 distinct formulas, and 32,608 font/style results**.
This scope covers the original formula corpus files; it does not claim to
display every assertion from RaTeX's Rust, platform or application unit tests.

SVGs are exported by the actual LayMesh formula renderer, with KaTeX,
Latin Modern Math, STIX Two Math and XITS Math in inline and display styles.
The 15,368 distinct combinations already present in the fixed audit must
match its exact parser/policy/font-coverage outcomes. The 136 new combinations
from the Unicode script are rendered and checked for OpenType geometry,
without assigning them a pre-existing expected-error baseline.
Every original fixture and font asset is verified against its SHA-256.
The page exposes source-file positions, errors, text fallback, missing-glyph
placeholders and invalid dimensions. A successful export alone is not a
visual-quality assertion. Baseline guides, vector zoom, source search,
category/status/font filters, pagination and detail views support manual review.

```sh
# First generate corpus.json using the audit command above.
cargo build -p laymesh-render --example math_gallery --release
python3 experiments/opentype-math/build-gallery.py
python3 experiments/opentype-math/serve-gallery.py --port 8765
```

The server binds to the machine's Tailscale IPv4 address by default and serves
only `target/opentype-math-gallery`. It does not publish through Funnel.
On this development host the URL is `http://100.64.0.6:8765/`;
`?suite=chemistry` opens chemistry and `?suite=domain&q=bond&size=48`
opens the enlarged compound-bond example. `/health` reports the loaded count.
After frontend-only edits, `build-gallery.py --reuse` refreshes the page while
checking the renderer fingerprint, all prior outcomes and SVG hashes.
