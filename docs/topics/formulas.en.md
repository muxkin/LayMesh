# Strings and mathematical formulas

<!-- walkthrough:start -->
## Purpose and concepts

formula accepts LaTeX without dollar delimiters. Raw strings preserve backslashes. inline fits text lines; display uses standalone layout. math_font=ratex_katex is a predefined string alias.

## Minimal complete example

Run this file directly with `laymesh validate` or `laymesh render`; it contains its own canvas and required definitions.

<!-- example:examples/manual/formula.lay -->

## Parameters and default behavior

Unitless geometry uses the canvas unit; unitless type and stroke sizes use pt. Explicit call parameters override inherited/theme defaults. The linked interface reference lists accepted types, choices and defaults per parameter.

## Composition

text(spans=[...]) combines spans and formulas. Embedded $...$ in ordinary text differs from explicit formula input; use raw/formatted strings deliberately when backslashes or substitutions are involved.

<!-- example:examples/gallery/typography/display-formula.lay -->

## Common errors and limits

The math engine rejects external resources, require and custom macros. Legacy mathjax font names warn and map to the current default. Formula colors support the same spaces as ordinary text.

## Individual functions

### formula

Create formula material from LaTeX without dollar delimiters, or a run in text. Raw strings preserve backslashes; style selects inline/display layout.

Returns: material

Required inputs: `source`.

[Minimal complete source](../../examples/manual/formula.lay) · [Composition source](../../examples/gallery/typography/display-formula.lay) · [All parameters](interface-reference.en.md#formula)

<!-- walkthrough:end -->

## Detailed behavior and further examples

### Ordinary and raw text

```lay
page = canvas(size=(150, 100))
page.add(text("Energy $E=mc^2$", font_size=10), offset=(5, 5))
page.add(text(r"Literal $E=mc^2$"), offset=(5, 20))
page.add(formula(r"\frac{a}{b}", font_size=12), offset=(5, 35))
```

Write LaTeX backslashes directly inside math, for example `axis(label="$\frac{\Delta E}{k_B T}$")`. Single, double and triple-quoted multiline strings are supported. The `r` prefix preserves raw text and disables automatic math; explicit `formula(r"…")` still renders math. Resource paths never undergo math detection.

### Interpolation

```lay
score = 0.975
label = f"Fit: $R^2={score:.3f}$"
literal = rf"Score {score:.2%}; literal $R^2$"
```

Braces accept DSL expressions. Numeric formatting supports common `f`, `e`, `g`, `%`, `d`, precision, signs and alignment. `{{` and `}}` produce literal braces. `rf` interpolates without automatic math. Interpolation executes neither Python nor JavaScript.

### Mixed content and inheritance

In `text(spans=[span("Ratio: "), formula(r"\frac{a}{b}")], font_size=12, color="#245447")`, a formula without an explicit size or color inherits those of its text. An explicit span font list replaces its parent's list. By default, math uses the KaTeX formula fonts embedded by RaTeX; `font_family` controls ordinary text.

SVG/PDF formulas are vectors; SVG metadata retains LaTeX source. Formula errors carry source locations. RaTeX limitations apply: custom macro definitions and external-resource commands are unavailable. Python `{{name}}` binding applies only outside DSL strings and comments, so it does not replace these f-string contents.

[Fonts](fonts.en.md) · [Parameters](interface-reference.en.md) · [Editors](editors.en.md)

## workflow

Complete sources and executable verification fixtures for this workflow are listed in the [feature coverage map](feature-map.en.md). Follow this page’s input conditions and limits when composing features.

## OpenType math fonts (experimental)

`math_font` also accepts an installed OpenType math font name or a font file path:

```lay
page=canvas(size=(100mm,50mm))
page.add(formula(r"\frac{a}{b}+\bm{\alpha}",math_font="XITS Math",font_size=16pt))
```

For reproducibility, use a file such as `math_font="./fonts/XITSMath-Regular.otf"`. Paths belong to the defining module or LCSS stylesheet. The property inherits through canvas, text, span, and plot styles, including automatic `$...$` math. Browser hosts must load the font bytes first. The backend reads glyph metrics, math constants, italic corrections, and stretch constructions from the selected font and exports vector outlines.

This branch covers mhchem `ce/pu`, `prooftree`, labelled arrows, `middle`, aligned equations/tags, array rules, overlap, braces, cancellation, and the ordinary math constructions. The fixed RaTeX 0.1.14 corpus and domain cases run 15,448 layout checks; all 91 chemistry, 22 physics, and 38 proof-tree examples render with four fonts and both styles. Parser/policy rejections and missing font glyphs are recorded separately. Unavailable fonts or missing MATH tables return `E_MATH_FONT`; selected math faces never silently substitute another font. `mathcal` and `mathscr` currently share the Unicode script alphabet. The standalone XITS bold face has incomplete stretch coverage; regular XITS Math supports `mathbf`, `boldsymbol`, and `bm`. Horizontal arrows without valid assemblies extend the shaft region of the selected font's native outline; other finite constructions use their largest native variant. Use raw DSL strings for LaTeX, especially when chemistry contains embedded `$...$` math.

Inner math alphabet commands override outer ones: `\mathbf{\mathcal A}` selects script A, while `\mathcal{\mathbf A}` selects bold roman A. Use `\mathbf{A1}` for bold letters/digits and `\bm{\alpha x}` for bold italic symbols. Script shapes and stroke weights vary with the selected font.

OpenType chemical bonds `~`, `~-`, `~--`, `~=`, and `-~-` use a compound construction based on the selected font's minus outline. Solid and dashed rows share their ink endpoints and thickness, and inherit the surrounding math size, script style and color. The geometry tests check endpoints directly, including bonds inside superscripts, nested subscripts and fractions.

See the [editable comparison and verification instructions](../../experiments/opentype-math/README.md). The default remains `ratex-katex`; legacy `mathjax-*` names retain their compatibility mapping.
