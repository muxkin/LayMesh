# Formula text font fixtures

These eight **test-only, renamed subsets** derive from Noto Serif, Noto Serif
CJK SC, Noto Sans Arabic and Noto Sans Devanagari. They are used by Rust tests
and the experimental corpus gallery, never by the product's runtime font
catalog or distribution package.

`manifest.json` records each upstream URL, input version, collection face,
input SHA-256 and resulting subset SHA-256. The supplied copyright files retain
the original notices and SIL Open Font License. Family and PostScript names
were changed to `LayMesh Formula Text ...` for the derived subsets.

Subsetting retains GSUB/GPOS and their glyph closure, so ligatures, combining
marks, Arabic joining and Devanagari shaping are exercised. CJK regular/bold
come from face 2 of the original collections; Latin includes regular, bold,
italic and bold italic. No synthetic weight or slant is introduced.

Rebuild with the exact input files identified in the manifest:

```sh
.venv/bin/python tests/fonts/text/build-fixtures.py --font-root /usr/share/fonts
```

The script gathers characters from the pinned RaTeX corpus and local domain /
text cases, together with fixed Latin, combining, Arabic and Devanagari ranges.
It preserves input timestamps and shaping features. Rebuilding with other
upstream versions changes hashes and requires reviewing and updating the
manifest. `fixtures.rs` shares the same bytes with tests and the gallery.
