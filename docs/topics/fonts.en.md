# Body fonts and Page preview

<!-- walkthrough:start -->
## Purpose and concepts

Text size is physical. A width constraint enables wrapping; spans mix colors, fonts, weights and formulas within a paragraph. Text measurement defines layout bounds, without glyph-outline anchors.

## Minimal complete example

Run this file directly with `laymesh validate` or `laymesh render`; it contains its own canvas and required definitions.

<!-- example:examples/manual/text.lay -->

## Parameters and default behavior

Unitless geometry uses the canvas unit; unitless type and stroke sizes use pt. Explicit call parameters override inherited/theme defaults. The linked interface reference lists accepted types, choices and defaults per parameter.

## Composition

Reuse text with different placement widths to reflow each instance independently. font_family accepts names, file paths or ordered fallback lists. Include font resources when reproducing work across machines.

<!-- example:examples/gallery/typography/font.lay -->

## Common errors and limits

font_weight is an integer from 100 to 900. Missing glyphs emit W_FONT and measured squares. System fonts are not bundled with documents. font_style accepts normal/italic variables or equivalent strings.

## Individual functions

### text

Create text material, recognizing inline mathematics in ordinary strings. Configure font fallback, size, color and a background box. size can constrain wrapping; raw strings disable automatic mathematics.

Returns: Reusable text material.

Required inputs: `content / spans`.

[Minimal complete source](../../examples/manual/text.lay) · [Composition source](../../examples/gallery/typography/font.lay) · [All parameters](interface-reference.en.md#text)

### span

Define a run inside text(spans=[...]) with local font/color overrides. A span is not independently placed material.

Returns: value

Required inputs: `content`.

[Minimal complete source](../../examples/manual/span.lay) · [Composition source](../../examples/gallery/typography/inline-formula.lay) · [All parameters](interface-reference.en.md#span)

<!-- walkthrough:end -->

## Detailed behavior and further examples

`font_family` accepts a system family name, a font-file path, or an ordered list. TTF, OTF, TTC, and OTC files are supported; select collection faces with `path#face name`. Relative paths resolve from the defining `.lay` module. The default is the system sans-serif family.

On Windows, use `"C:/Fonts/Example.ttf"` or `r"C:\Fonts\Example.ttf"`; backslashes in ordinary strings must be escaped. Pass dynamic Python paths as strings through a `{{font}}` binding instead of interpolating them into DSL source.

```lay
page=canvas(size=(100,40))
page.add(text("My data", font_family="Arial", font_size=12))
```

Missing fonts, unmatched weights, and missing glyphs produce source-located `W_FONT` warnings. Missing characters are drawn as vector boxes. SVG, PNG, and PDF export continue even with no system fonts available. `--warnings hide` and `LAYMESH_WARNINGS=hide` hide warning messages only.

The browser reads local fonts only with your permission. Use the preview Fonts menu to authorize installed fonts or load font files; user-provided data stays in this page's memory. Other unavailable fonts can still display boxes. Access to installed fonts is limited on the current HTTP preview address and remains to be checked on the HTTPS GitHub Pages site.

Common names such as Times New Roman, Arial, Helvetica, SimSun, SimHei, and KaiTi remain usable when the user supplies or authorizes those fonts. The Page does not bundle them or relabel the open fonts as those families. Existing examples use DejaVu Sans for English and scientific plots, and Noto Sans CJK SC for Chinese text.

Exports can embed the fonts actually used, with PDF subsetting. This does not bundle body fonts in the application. For reproducible output across systems, provide the same font files and check their licenses.
