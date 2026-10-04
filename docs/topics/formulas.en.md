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

In `text(spans=[span("Ratio: "), formula(r"\frac{a}{b}")], font_size=12, color="#245447")`, a formula without an explicit size or color inherits those of its text. An explicit span font list replaces its parent's list. Math uses the KaTeX formula fonts embedded by RaTeX; `font_family` controls ordinary text.

SVG/PDF formulas are vectors; SVG metadata retains LaTeX source. Formula errors carry source locations. RaTeX limitations apply: custom macro definitions and external-resource commands are unavailable. Python `{{name}}` binding applies only outside DSL strings and comments, so it does not replace these f-string contents.

[Fonts](fonts.en.md) · [Parameters](interface-reference.en.md) · [Editors](editors.en.md)

## workflow

Complete sources and executable verification fixtures for this workflow are listed in the [feature coverage map](feature-map.en.md). Follow this page’s input conditions and limits when composing features.
