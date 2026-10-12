# Selectors, variables and cascade

[Style manual](lcss.en.md) · Next: [Appearance properties and units](style-properties.en.md)

## Selecting objects

| Selector | Meaning |
| --- | --- |
| rect | Rectangle material |
| .card | Object with the card class |
| rect.card, .card.emphasis | Type plus class, or multiple classes |
| text, formula | Selector grouping |
| .panel text | Text descendants of panel |
| .panel > text | Directly owned text |
| [class~="card"] | Class list contains the card token |
| :root, html, :where(html) | Root canvas |

Class attribute selectors support equality, token, prefix, suffix and substring comparisons against the actual class string. Do not assume support for IDs, interaction pseudo-classes or arbitrary DOM selectors. Ownership is the LayMesh container hierarchy.

Chart parts include plot::area, plot::axis(x), plot::tick-label, plot::axis-label, plot::legend and plot::colorbar.

~~~lcss
plot::area { background:#f8fafc; }
plot::axis(x) { line-color:#334155; line-width:0.6pt; }
plot::tick-label { font-size:8pt; color:#334155; }
.annotation { font-size:9pt; background:#ffffff; padding:1mm; }
~~~

## When both sources set a radius

Explicit .lay parameters take priority even over important style declarations:

~~~lay
page = canvas(size=(80mm,50mm))
style { .card { border-radius:2mm !important; background:#dbeafe; } }
page.add(rect(size=(30mm,20mm),class="card",border_radius=5mm),anchor=center,target=page.center)
~~~

The resulting radius is 5mm. Omit border_radius or its radius alias when the theme should control it; retain explicit parameters for deliberate individual exceptions.

## Comparing style declarations

Module styles provide defaults only for objects created by that module. The entry theme can override these defaults. Within theme declarations, compare:

1. !important before ordinary declarations.
2. Specificity: class and attribute conditions outrank a type selector; additional matching conditions add specificity. Root :where selectors add none.
3. Source order when the first two comparisons tie.

Importing a library first does not let an ordinary declaration beat an important one. Bootstrap shadows require an important, more specific override:

~~~lcss
@import "@laymesh/bootstrap.css";
.shadow.custom { box-shadow-blur:2mm !important; }
~~~

Use class="shadow custom". Static and animation styles share the cascade; stylesheet and @import do not create different priorities.

## Variables, fallback and empty values

~~~lcss
:root { --accent:#0d6efd; --gap:1mm; }
.card { --edge:var(--accent); border:0.5pt solid var(--edge); padding:var(--gap); }
.caption { color:var(--caption,var(--accent,#334155)); }
~~~

Custom properties begin with -- and inherit through ownership. var(--name,fallback) uses fallback for missing or unresolvable variables; fallbacks can nest. A declared empty value, --optional:;, is valid and differs from a missing variable. var(--optional,red) expands to empty instead of red; a color declaration using it then becomes invalid. Cycles can invalidate the declaration or activate an available fallback without stopping later declarations.

Style variables do not read .lay bindings. inherit takes the parent's value; initial takes the initial value; unset selects inheritance for inherited properties and the initial value otherwise.

## Inheritance and reuse

Font family, math font, math-text fallback, font size, weight, style, color, line height and text shadows inherit. Background, borders, corners and box shadows do not. Opacity is not inherited, but ancestor and child opacity multiply.

Place a material repeatedly and assign instance classes separately. Put repeated appearance in the theme instead of duplicating declarations.
