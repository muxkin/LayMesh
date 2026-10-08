# Pinned RaTeX layout with host text metrics

Source: [RaTeX v0.1.14](https://github.com/erweixin/RaTeX/tree/ae391d727ac615437c63c308f4538d971a84bede/crates/ratex-layout),
commit `ae391d727ac615437c63c308f4538d971a84bede`. The original MIT notice is
retained in `LICENSE`. This workspace uses a Cargo patch for this crate.

Only `src/engine.rs` and `src/layout_options.rs` change layout behavior:

- An optional borrowed `TextLayout` hook returns a complete measured text box
  before scripts, arrows, fractions, proof rules or neighboring math are placed.
- Adjacent text-mode leaves and groups are offered together, preserving shaping
  across characters. This also applies inside explicit fonts and proof cells.
- Text weight and italic state propagate through derived layout options and
  nested text commands. Math alphabets keep the upstream font path.
- With no hook, or when it returns `None`, the bundled-font path is retained.

The hook contains no font lookup, outlines, Unicode shaping or formula-specific
substitutions. These belong to LayMesh's shared formula text implementation.
The crate manifest replaces upstream workspace dependencies with pinned 0.1.14
dependencies, so it builds from this standalone vendor directory.

This is an experimental workspace patch. A release must ship or upstream this
crate change; publishing an unpatched core crate alone would lose the hook.
