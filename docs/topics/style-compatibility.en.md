# Style compatibility and troubleshooting

[Style manual](lcss.en.md) · [Property reference](style-properties.en.md)

## Supported CSS

Complete upstream CSS loads, but only implemented appearance applies to a fixed canvas. Backgrounds, typography, borders, corners, opacity, padding, shadows and animation can work. Flex/grid, width/height/margin/position and interaction pseudo-classes do not lay out objects.

Unknown properties and invalid values recover per declaration; unsupported selectors recover per rule. Later valid declarations continue. One W_CSS per file summarizes ignored items with their locations. Many ignored Bootstrap declarations are expected.

## Media and supports

Static media defaults to screen. Size queries use the fixed canvas, not the browser viewport:

~~~lcss
@media screen and (min-width:100mm) {
  .caption { font-size:4mm; }
}
@supports (background:linear-gradient(90deg,red,blue)) {
  .card { background:linear-gradient(90deg,red,blue); }
}
~~~

Support includes min/max-width and min/max-height with and/or/not and comma alternatives. @supports is true only for actually supported properties and values. An unimplemented interpolation color space does not activate a branch.

Static reduced-motion conditions use no-preference. Animation previews follow the system preference; video defaults to screen/no-preference. See [Animation export and troubleshooting](animation-export.en.md).

## Diagnose the symptom

| Symptom | Check |
| --- | --- |
| Class has no effect | Library import, spelling, compatible target |
| Radius/font will not change | Explicit .lay parameter, specificity, importance |
| Open Props value is ineffective | Import, variable existence, supported expanded value |
| Fallback not used | Variable may be a valid empty value rather than missing |
| Blur override creates no shadow | First declare box-shadow or apply a shadow class |
| Shadow cropped | Object position, effect bounds and canvas size |
| CSS size does not resize an object | Use .lay size, offset and anchors |
| Export DPI leaves shadow size unchanged | Physical length is preserved; layout_dpi controls layout px |

## Warnings and errors

W_CSS reports ignored stylesheet rules/declarations. W_FONT reports font/glyph issues. W_ANIMATION reports missing animation definitions or effective duration. Missing files, import cycles and invalid .lay parameters remain errors.

Dimensional errors keep E_UNIT. 160*100/90mm produces inverse length; use 160mm*100/90 or (160*100/90)*1mm for a length. Diagnostics do not rewrite source.

~~~sh
laymesh validate examples/manual/style-project.lay
laymesh inspect examples/manual/style-project.lay --json
laymesh render examples/manual/style-project.lay -o style.pdf
~~~

A source error retains the marked last successful preview; that image is not the result of the failing edit.
