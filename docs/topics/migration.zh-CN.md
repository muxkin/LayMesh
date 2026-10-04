# 统一 API 迁移

本次语言版本为 0.2，输出 Scene 的 `schemaVersion` 为 8。旧参数直接给出替代诊断；不保留长期重复入口。

| 旧写法 | 统一写法 |
| --- | --- |
| `width=80, height=40` | `size=(80, 40)` |
| `width=80` | `size=(80, auto)` |
| 文字 `size=10pt` | `font_size=10pt` |
| `font`, `weight`, `italic` | `font_family`, `font_weight`, `font_style="italic"` |
| `classes` | `class="paper annotation"` |
| `stroke`, `stroke_width` | 封闭对象 `border_color`, `border_width`；开放线条 `line_color`, `line_width` |
| `dash`, `stroke_cap`, `stroke_join` | `border_dash/cap/join` 或 `line_dash/cap/join` |
| `outline(...)` | 对象边框/线条参数；复用规则写入 `.lcss` |
| `marker_stroke` | `marker_border_color` |
| 柱图 `width` | `data_width`；极坐标角宽用 `angle_width` |
| `plot_area=(x,y,w,h)` | `plot_area=box(offset=(x,y), size=(w,h))` |

裸几何长度改为默认 mm，字号和线宽类裸值默认 pt。纯数字数据保持原值。普通字符串内 `$…$` 渲染公式，需要原样显示时加 `r` 前缀。

仓库提供 `scripts/migrate-language.mjs` 作为可审阅的源文件迁移工具。它迁移参数和内联旧边框预设；复杂跨模块样式应按组件主题重新整理。迁移后运行 `validate`、`inspect` 并对照导出。

原 Node 编译和渲染入口由 Rust/WASM 接替；Python `render`/`render_file`、CLI 命令和字体回退顺序保持原调用方式；传入的 DSL 使用本页统一语义。历史基准记录保留当时版本和测试背景，不代表本次测量。
