# LCSS 主题与区域填充

LCSS 是 LayMesh 的样式语言，独立文件使用 `.lcss`。它采用 CSS 风格的选择器和声明，但不是浏览器 CSS；几何尺寸、数据、范围和定位仍写在 `.lay`。

<!-- example:examples/unified.lay -->

## 加载与覆盖顺序

`canvas(stylesheet="language/paper.lcss")` 加载本地样式表，也可传入有序文件列表。入口内嵌 `style { … }` 在外部文件之后生效；同一层按具体程度、声明顺序覆盖，对象显式参数优先。模块样式仅作用于模块创建的对象，作为组件默认值，入口主题可以覆盖。

```lcss
@import "base.lcss";
plot::axis(x) { line-color: #444444; line-width: 0.6pt; }
.annotation {
  --paper: #ffffff;
  background: var(--paper);
  border: 0.5pt solid #b9d5cb;
  border-radius: 1mm;
  padding: 1mm;
}
```

`.lay` 使用 `class="paper annotation"`。支持类型、类、类型与类组合、逗号分组、所属关系及 `>` 直接所属选择器；图形部件包括 `plot::area`、`plot::axis(x)`、`plot::tick-label`、`plot::axis-label`、`plot::legend` 和 `plot::colorbar`。

## 填充与边框

`fill` 控制形状内部；`background` 控制区域底色。支持 `none`、颜色、`linear-gradient(...)`、`radial-gradient(...)`、`hatch(slash, #666666, 2mm, 0.5pt)`、`hatch(cross, …)`、`hatch(dots, …)` 和 `url("texture.png")`。DSL 对应 `linear_gradient`、`radial_gradient`、`hatch`、`image_fill` 构造器。

画布、图表外框、绘图区、轴及其文字区域、文字/公式、图例/色标和导入图片底框分别设置。绘图区填充遵循极坐标轮廓及断轴可见分段，纯填充不改变数据映射。`padding` 是内部留白并参与边界测量。

封闭轮廓使用 `border_*`，开放线条使用 `line_*`。LCSS 对应连字符写法，例如 `border-width` 和 `line-width`。文字与字体属性继承，背景和定位不继承；透明度按父子层级相乘。同一素材的实例可以通过 `class` 独立设置样式。

## 资源和限制

所有 LCSS 长度必须显式带单位（零可以省略），主题不会随画布默认单位变化。图片和字体路径相对于声明它的 `.lcss` 解析。变量 `var(--name)` 支持默认值，变量或导入循环报错。

不支持网页布局、媒体查询、动画、交互伪类及 `!important`；这些写法会明确报错，不会静默忽略。[完整参数](interface-reference.zh-CN.md) · [迁移](migration.zh-CN.md)

## style-properties

每个属性映射到以下 DSL 参数；类型、单位和继承与参数悬停使用同一份元数据。

| LCSS | DSL |
| --- | --- |
| `font-family` | `font_family` |
| `font-size` | `font_size` |
| `font-weight` | `font_weight` |
| `font-style` | `font_style` |
| `color` | `color` |
| `line-height` | `line_height` |
| `background` | `background` |
| `padding` | `padding` |
| `border-radius` | `border_radius` |
| `border-color` | `border_color` |
| `border-width` | `border_width` |
| `border-style` | `border_style` |
| `border-dash` | `border_dash` |
| `border-dash-offset` | `border_dash_offset` |
| `border-cap` | `border_cap` |
| `border-join` | `border_join` |
| `border-miter-limit` | `border_miter_limit` |
| `border-opacity` | `border_opacity` |
| `line-color` | `line_color` |
| `line-width` | `line_width` |
| `line-style` | `line_style` |
| `line-dash` | `line_dash` |
| `line-dash-offset` | `line_dash_offset` |
| `line-cap` | `line_cap` |
| `line-join` | `line_join` |
| `line-miter-limit` | `line_miter_limit` |
| `line-opacity` | `line_opacity` |
| `fill` | `fill` |
| `opacity` | `opacity` |
| `tick-font-size` | `tick_font_size` |
| `label-font-size` | `label_font_size` |
| `tick-color` | `tick_color` |
| `grid-color` | `grid_color` |
| `grid-line-width` | `grid_line_width` |
| `grid-line-dash` | `grid_line_dash` |
| `start-head` | `start_head` |
| `end-head` | `end_head` |
| `start-cap` | `start_cap` |
| `end-cap` | `end_cap` |

端部属性接受 `head(...)`，例如 `end-head: head(shape=triangle,size=(5mm,4mm));`。LCSS 标识符是样式值，不读取 DSL 的变量作用域。

## workflow

本流程的完整源码与可执行验证文件列在[功能覆盖清单](feature-map.zh-CN.md)。组合使用时请遵循本页的输入条件与限制。
