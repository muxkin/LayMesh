# OpenType mathematical font experiment

Branch: `codex/opentype-math-fonts`.

`formula(..., math_font="XITS Math")` selects an installed OpenType math font.
A font file path works on native hosts and with fonts supplied through the
in-memory/WASM host. Paths are relative to the defining `.lay` module (or
`.lcss` stylesheet). `math_font` can also be inherited from the canvas, text,
span, plot, or plot style.

The default `ratex-katex` backend is retained. The experimental backend reuses
the RaTeX parser and LayMesh exporters, but obtains glyphs, advances, math
constants, italic corrections, accent attachments, script kerns, stretch
variants, and connector assemblies from the selected OpenType font.
It never substitutes a KaTeX face for an explicitly selected math font.

Covered: ordinary symbols, Greek, fractions and binomials, scripts, indexed
roots, sums/integrals, `left/right`, matrices, common accents, over/underlines,
math alphabets, explicit spaces, and inline math. `mathcal` and `mathscr`
currently select the same Unicode script alphabet; stylistic-set selection is
not implemented.

The backend is an experiment, not full RaTeX command parity. Unimplemented
commands/environments (including extensible labelled arrows, `middle`,
aligned equations, array rules, and tags) return `E_FORMULA`. Missing glyphs,
missing MATH tables, embedding restrictions, or unavailable fonts fail
explicitly. Fonts without an assembly recipe use their largest native
variant when the requested size exceeds their finite variants.

XITS Math regular covers the complete test corpus. XITS Math Bold is a
supplementary face with incomplete stretch coverage: algebra works, and
unsupported radical constructions report an error. It is not advertised
as a complete replacement for the regular math font.

Reproduce:

```sh
cargo test -p laymesh-core --test opentype_math -p laymesh-render --test opentype_math
cargo build -p laymesh-cli
python3 experiments/opentype-math/verify.py
```

Outputs go to `target/opentype-math-validation`: a four-column SVG/PDF/PNG/PPTX
comparison, a bold XITS sample, and a JSON record of font hashes and exports.
The `.lay` source is directly editable. The test fixtures include unmodified
Latin Modern Math 1.959, STIX Two Math 2.02 b142, and XITS Math 1.302
regular/bold with their license notices; they are not distribution assets.

The regression suite changes only MATH constants while keeping glyph outlines
identical, checks outline bounds, verifies assembly glyphs, tests relative
paths, inherited inline baselines, plot labels, accent placement, operator limits,
cache invalidation, embedding restrictions, and explicit errors.
