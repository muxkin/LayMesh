# Unified API migration

The language version is 0.2; output Scene `schemaVersion` is 8. Removed parameters produce replacement diagnostics rather than permanent duplicate entry points.

| Previous spelling | Unified spelling |
| --- | --- |
| `width=80, height=40` | `size=(80, 40)` |
| `width=80` | `size=(80, auto)` |
| Text `size=10pt` | `font_size=10pt` |
| `font`, `weight`, `italic` | `font_family`, `font_weight`, `font_style="italic"` |
| `classes` | `class="paper annotation"` |
| `stroke`, `stroke_width` | Closed objects: `border_color`, `border_width`; open lines: `line_color`, `line_width` |
| `dash`, `stroke_cap`, `stroke_join` | `border_dash/cap/join` or `line_dash/cap/join` |
| `outline(...)` | Object border/line parameters; reusable rules in `.lcss` |
| `marker_stroke` | `marker_border_color` |
| Bar `width` | `data_width`; polar angular width uses `angle_width` |
| `plot_area=(x,y,w,h)` | `plot_area=box(offset=(x,y), size=(w,h))` |

Bare geometry now defaults to mm; bare typography and line widths default to pt. Numeric data is unchanged. Ordinary strings render `$…$` math; use the `r` prefix for literal display.

`scripts/migrate-language.mjs` provides reviewable source migration. It renames parameters and inlines old outline presets; complex cross-module styles should be reorganized as component themes. Run `validate`, `inspect` and compare exports after migration.

Rust/WASM replace the Node compilation/rendering entry points. Python `render`/`render_file`, CLI commands and font fallback order retain their calling conventions; embedded DSL follows the new semantics. Historical benchmarks retain their original version and measurement context.

`arrow()` now creates a filled shape; use dedicated entries such as `arrow.arc()` for other templates. Migration rewrites only identifiable legacy calls with line-only arguments (`line_*`, `start_head/end_head`, or `start_cap/end_cap`) whose remaining arguments also belong to `line()`. It does not replace the function name globally. Old calls containing only shared geometry (`dx/dy` or `length/angle`) are ambiguous: change them manually to `line(...,end_head=head())` when a line arrow was intended. User-defined bindings retain the existing shadowing rules. See [shape arrows](arrows.en.md).
