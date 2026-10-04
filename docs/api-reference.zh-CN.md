# API 参考

<!-- Generated from crates/laymesh-core/api.json and docs/feature-coverage.json. -->

## 预定义字符串变量

下列名称是无需声明的普通字符串变量。用户绑定优先，对应字符串写法继续合法。场景含义放在参数说明中，而不是变量悬停中。

| 变量 | 类型 | 字符串值 |
| --- | --- | --- |
| `auto` | `string` | `"auto"` |
| `axes` | `string` | `"axes"` |
| `bar` | `string` | `"bar"` |
| `bevel` | `string` | `"bevel"` |
| `both` | `string` | `"both"` |
| `bottom` | `string` | `"bottom"` |
| `bottom_center` | `string` | `"bottom_center"` |
| `bottom_left` | `string` | `"bottom_left"` |
| `bottom_right` | `string` | `"bottom_right"` |
| `boundary` | `string` | `"boundary"` |
| `box` | `string` | `"box"` |
| `butt` | `string` | `"butt"` |
| `cartesian` | `string` | `"cartesian"` |
| `center` | `string` | `"center"` |
| `centered` | `string` | `"centered"` |
| `chord` | `string` | `"chord"` |
| `circle` | `string` | `"circle"` |
| `cm` | `string` | `"cm"` |
| `contain` | `string` | `"contain"` |
| `container` | `string` | `"container"` |
| `count` | `string` | `"count"` |
| `cover` | `string` | `"cover"` |
| `cross` | `string` | `"cross"` |
| `dash` | `string` | `"dash"` |
| `dash_dot` | `string` | `"dash_dot"` |
| `dashed` | `string` | `"dashed"` |
| `deg` | `string` | `"deg"` |
| `density` | `string` | `"density"` |
| `diamond` | `string` | `"diamond"` |
| `display` | `string` | `"display"` |
| `dot` | `string` | `"dot"` |
| `dots` | `string` | `"dots"` |
| `dotted` | `string` | `"dotted"` |
| `double` | `string` | `"double"` |
| `end` | `string` | `"end"` |
| `evenodd` | `string` | `"evenodd"` |
| `horizontal` | `string` | `"horizontal"` |
| `in` | `string` | `"in"` |
| `inch` | `string` | `"inch"` |
| `incoming` | `string` | `"incoming"` |
| `inline` | `string` | `"inline"` |
| `inout` | `string` | `"inout"` |
| `italic` | `string` | `"italic"` |
| `justify` | `string` | `"justify"` |
| `left` | `string` | `"left"` |
| `linear` | `string` | `"linear"` |
| `local` | `string` | `"local"` |
| `log` | `string` | `"log"` |
| `long_dash` | `string` | `"long_dash"` |
| `lower` | `string` | `"lower"` |
| `major` | `string` | `"major"` |
| `mathjax_newcm` | `string` | `"mathjax-newcm"` |
| `mathjax_tex` | `string` | `"mathjax-tex"` |
| `mid` | `string` | `"mid"` |
| `middle_left` | `string` | `"middle_left"` |
| `middle_right` | `string` | `"middle_right"` |
| `miter` | `string` | `"miter"` |
| `mm` | `string` | `"mm"` |
| `none` | `string` | `"none"` |
| `nonzero` | `string` | `"nonzero"` |
| `normal` | `string` | `"normal"` |
| `offset` | `string` | `"offset"` |
| `open` | `string` | `"open"` |
| `out` | `string` | `"out"` |
| `outgoing` | `string` | `"outgoing"` |
| `parent` | `string` | `"parent"` |
| `plain` | `string` | `"plain"` |
| `plot_bottom_center` | `string` | `"plot_bottom_center"` |
| `plot_bottom_left` | `string` | `"plot_bottom_left"` |
| `plot_bottom_right` | `string` | `"plot_bottom_right"` |
| `plot_center` | `string` | `"plot_center"` |
| `plot_middle_left` | `string` | `"plot_middle_left"` |
| `plot_middle_right` | `string` | `"plot_middle_right"` |
| `plot_top_center` | `string` | `"plot_top_center"` |
| `plot_top_left` | `string` | `"plot_top_left"` |
| `plot_top_right` | `string` | `"plot_top_right"` |
| `polar` | `string` | `"polar"` |
| `post` | `string` | `"post"` |
| `pre` | `string` | `"pre"` |
| `probability` | `string` | `"probability"` |
| `pt` | `string` | `"pt"` |
| `px` | `string` | `"px"` |
| `rad` | `string` | `"rad"` |
| `radar` | `string` | `"radar"` |
| `raster` | `string` | `"raster"` |
| `ratex_katex` | `string` | `"ratex-katex"` |
| `raw` | `string` | `"raw"` |
| `right` | `string` | `"right"` |
| `round` | `string` | `"round"` |
| `scientific` | `string` | `"scientific"` |
| `shortest` | `string` | `"shortest"` |
| `single` | `string` | `"single"` |
| `slash` | `string` | `"slash"` |
| `solid` | `string` | `"solid"` |
| `square` | `string` | `"square"` |
| `start` | `string` | `"start"` |
| `stealth` | `string` | `"stealth"` |
| `stretch` | `string` | `"stretch"` |
| `symlog` | `string` | `"symlog"` |
| `target` | `string` | `"target"` |
| `top` | `string` | `"top"` |
| `top_center` | `string` | `"top_center"` |
| `top_left` | `string` | `"top_left"` |
| `top_right` | `string` | `"top_right"` |
| `triangle` | `string` | `"triangle"` |
| `triangle_down` | `string` | `"triangle_down"` |
| `triple` | `string` | `"triple"` |
| `upper` | `string` | `"upper"` |
| `vector` | `string` | `"vector"` |
| `vertical` | `string` | `"vertical"` |
| `x` | `string` | `"x"` |
| `y` | `string` | `"y"` |

## canvas

创建页面并设置页面物理尺寸、背景和默认几何单位。通过 add 放置素材；不带单位的几何长度默认使用 mm，字号和线宽默认使用 pt。

返回：用于接收素材的页面。

必需：`size`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `name` | string | 画布标题或命名坐标轴的唯一名称 | "Untitled" |
| `size` | (length, length) / 画布单位 | 页面 (宽,高)，两个长度必须为正；不支持 auto | — |
| `unit` | "mm" \| "cm" \| "in" \| "inch" \| "pt" \| "px" | 裸几何长度的默认单位；不影响数据或字号<br>`mm`: 毫米<br>`cm`: 厘米<br>`in`: 英寸<br>`inch`: 英寸<br>`pt`: 印刷点，1/72 英寸<br>`px`: 布局像素，按 layout_dpi 换算 | mm |
| `layout_dpi` | number | px 与物理长度的换算率，独立于导出 DPI | 96 |
| `stylesheet` | string \| string[] | .lcss 文件路径或有序路径列表 | — |
| `class` | string | 空格分隔的 LCSS 类名 | "" |
| `background` | paint | 对象区域的底色或填充，不是文字或轴线颜色 | none |
| `font_family` | string \| string[] | 系统字体族名、字体文件路径或有序列表；不内置正文字体；缺字警告并显示方框。 | system sans-serif |
| `font_size` | length / pt | 字号；省略单位时为 pt；未指定时继承 | 继承 / inherit |
| `font_weight` | integer | 字体字重，100–900 | 400 |
| `font_style` | "normal" \| "italic" | 正体或斜体<br>`normal`: 正常字形<br>`italic`: 斜体字形 | normal |
| `color` | color | 文字颜色或数据系列基础配色 | — |
| `line_height` | length / pt | 文字行高；裸值为 pt | 1.2 × font_size |

### 最小完整示例

```lay
# Minimal complete example: canvas
page=canvas(size=(100mm,75mm),background="#ffffff")
page.add(text("Physical page",font_size=12pt),offset=(5mm,5mm))
```

[概念与常见错误](topics/concepts.zh-CN.md#canvas) · [组合源码](../examples/hello.lay)

## image

从本地文件定义可重复放置的图片素材。尺寸、crop 与 contain/cover/stretch 适配在 add 中确定，路径相对于素材定义文件。

返回：material

位置参数：`src`.

必需：`src`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `src` | string | 相对于定义文件的本地资源路径 | — |
| `size` | (length \| auto, length \| auto) / 画布单位 | 二维物理尺寸 (宽,高)；auto 保持比例或自然排版 | — |
| `class` | string | 空格分隔的 LCSS 类名 | "" |
| `background` | paint | 对象区域的底色或填充，不是文字或轴线颜色 | none |
| `padding` | length \| length[] / 画布单位 | 内部留白；一个值或上、右、下、左四个值 | 0 |
| `border_radius` | length / 画布单位 | 底框圆角的物理半径 | 0 |
| `border_color` | color | 封闭轮廓颜色，none 表示无边框 | none |
| `border_width` | length / pt | 封闭轮廓线宽；裸值为 pt | — |
| `border_style` | "none" \| "solid" \| "dashed" \| "dotted" \| "dash_dot" \| "double" \| "triple" | 边框线型；与填充独立<br>`none`: 不绘制此项<br>`solid`: 连续实线<br>`dashed`: 虚线；可用 dash 参数覆盖长度<br>`dotted`: 点线<br>`dash_dot`: 点划线<br>`double`: 双线描边<br>`triple`: 三线描边 | solid |
| `border_dash` | length[] / pt | 交替的边框线段和间隔长度列表；须成对 | — |
| `border_dash_offset` | length / pt | 虚线相位；裸值为 pt | 0 |
| `border_cap` | "butt" \| "round" \| "square" | 轮廓虚线端点形状<br>`butt`: 平端帽，不向端点外延伸<br>`round`: 圆端帽，向外延伸半个线宽<br>`square`: 方端帽，向外延伸半个线宽 | butt |
| `border_join` | "miter" \| "round" \| "bevel" | 轮廓交点形状<br>`miter`: 延长边缘形成尖角，受尖角限制约束<br>`round`: 以圆弧连接相邻描边边缘<br>`bevel`: 以平边截去连接尖角 | miter |
| `border_miter_limit` | value | 尖角的最大延伸与线宽之比 | 4 |
| `border_opacity` | value | 边框独立透明度 0–1 | 1 |

### 最小完整示例

```lay
# Minimal complete example: image
page=canvas(size=(100mm,75mm),background="#ffffff")
page.add(image(src="../assets/photo.png"),size=(55mm,auto),offset=(8mm,8mm))
```

[概念与常见错误](topics/images.zh-CN.md#image) · [组合源码](../examples/gallery/images/crop.lay)

## text

创建文字素材，自动识别普通字符串中的行内公式；可设置字体回退、字号、颜色与底框。size 可限定换行宽度，原始字符串关闭自动公式解析。

返回：可重复放置的文字素材。

位置参数：`content`.

必需：`content / spans`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `content` | string | 文字内容；支持 $…$ 公式，r 字符串禁用自动解析 | — |
| `spans` | (span \| formula)[] | 按顺序排版的 span(...) 与 formula(...) 列表 | — |
| `size` | (length \| auto, length \| auto) / 画布单位 | 二维物理尺寸 (宽,高)；auto 保持比例或自然排版 | — |
| `class` | string | 空格分隔的 LCSS 类名 | "" |
| `font_family` | string \| string[] | 系统字体族名、字体文件路径或有序列表；不内置正文字体；缺字警告并显示方框。 | system sans-serif |
| `font_size` | length / pt | 字号；省略单位时为 pt；未指定时继承 | 10pt |
| `font_weight` | integer | 字体字重，100–900 | 400 |
| `font_style` | "normal" \| "italic" | 正体或斜体<br>`normal`: 正常字形<br>`italic`: 斜体字形 | normal |
| `color` | color | 文字颜色或数据系列基础配色 | — |
| `line_height` | length / pt | 文字行高；裸值为 pt | 1.2 × font_size |
| `align` | "left" \| "center" \| "right" \| "justify" | 文字在排版框中的水平对齐方式<br>`left`: 左侧或左对齐，依参数用途<br>`center`: 中心对齐<br>`right`: 右侧或右对齐，依参数用途<br>`justify`: 按可用宽度两端对齐 | left |
| `background` | paint | 对象区域的底色或填充，不是文字或轴线颜色 | none |
| `padding` | length \| length[] / 画布单位 | 内部留白；一个值或上、右、下、左四个值 | 0 |
| `border_radius` | length / 画布单位 | 底框圆角的物理半径 | 0 |
| `border_color` | color | 封闭轮廓颜色，none 表示无边框 | none |
| `border_width` | length / pt | 封闭轮廓线宽；裸值为 pt | — |
| `border_style` | "none" \| "solid" \| "dashed" \| "dotted" \| "dash_dot" \| "double" \| "triple" | 边框线型；与填充独立<br>`none`: 不绘制此项<br>`solid`: 连续实线<br>`dashed`: 虚线；可用 dash 参数覆盖长度<br>`dotted`: 点线<br>`dash_dot`: 点划线<br>`double`: 双线描边<br>`triple`: 三线描边 | solid |
| `border_dash` | length[] / pt | 交替的边框线段和间隔长度列表；须成对 | — |
| `border_dash_offset` | length / pt | 虚线相位；裸值为 pt | 0 |
| `border_cap` | "butt" \| "round" \| "square" | 轮廓虚线端点形状<br>`butt`: 平端帽，不向端点外延伸<br>`round`: 圆端帽，向外延伸半个线宽<br>`square`: 方端帽，向外延伸半个线宽 | butt |
| `border_join` | "miter" \| "round" \| "bevel" | 轮廓交点形状<br>`miter`: 延长边缘形成尖角，受尖角限制约束<br>`round`: 以圆弧连接相邻描边边缘<br>`bevel`: 以平边截去连接尖角 | miter |
| `border_miter_limit` | value | 尖角的最大延伸与线宽之比 | 4 |
| `border_opacity` | value | 边框独立透明度 0–1 | 1 |

### 最小完整示例

```lay
# Minimal complete example: text
page=canvas(size=(100mm,75mm),background="#ffffff")
page.add(text("Wrapping preserves physical type size.",size=(45mm,auto),font_size=12pt),offset=(8mm,8mm))
```

[概念与常见错误](topics/fonts.zh-CN.md#text) · [组合源码](../examples/gallery/typography/font.lay)

## span

定义 text(spans=[...]) 内的一段文字及局部字体或颜色覆盖。span 本身不是可以独立放置的素材。

返回：value

位置参数：`content`.

必需：`content`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `content` | string | 文字内容；支持 $…$ 公式，r 字符串禁用自动解析 | — |
| `class` | string | 空格分隔的 LCSS 类名 | "" |
| `font_family` | string \| string[] | 系统字体族名、字体文件路径或有序列表；不内置正文字体；缺字警告并显示方框。 | system sans-serif |
| `font_size` | length / pt | 字号；省略单位时为 pt；未指定时继承 | 继承 / inherit |
| `font_weight` | integer | 字体字重，100–900 | 400 |
| `font_style` | "normal" \| "italic" | 正体或斜体<br>`normal`: 正常字形<br>`italic`: 斜体字形 | normal |
| `color` | color | 文字颜色或数据系列基础配色 | — |

### 最小完整示例

```lay
# Minimal complete example: span
page=canvas(size=(100mm,75mm),background="#ffffff")
page.add(text(spans=[span("Red ",color="#e36b70"),span("bold",font_weight=700)],font_size=12pt),offset=(8mm,8mm))
```

[概念与常见错误](topics/fonts.zh-CN.md#span) · [组合源码](../examples/gallery/typography/inline-formula.lay)

## formula

用不带美元定界符的 LaTeX 创建公式素材，或作为 text 的行内片段。raw 字符串保留反斜杠，style 控制 inline/display 排版。

返回：material

位置参数：`source`.

必需：`source`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `source` | string | 公式 LaTeX 源码；不需要美元定界符 | — |
| `class` | string | 空格分隔的 LCSS 类名 | "" |
| `font_size` | length / pt | 字号；省略单位时为 pt；未指定时继承 | 10pt (inherit in text) |
| `color` | color | 文字颜色或数据系列基础配色 | #000000 |
| `math_font` | "ratex-katex" \| "mathjax-newcm" \| "mathjax-tex" | RaTeX 内置 KaTeX 公式字体；旧 mathjax-* 名称仅用于兼容，产生警告后映射至 ratex-katex。<br>`ratex-katex`: 当前默认的 RaTeX 数学字体<br>`mathjax-newcm`: 兼容旧名称；会映射到当前默认字体<br>`mathjax-tex`: 兼容旧名称；会映射到当前默认字体 | ratex-katex |
| `style` | "inline" \| "display" | 对象的样式配置；plot 使用 plot_style，公式使用 inline/display<br>`inline`: 行内公式排版<br>`display`: 独立公式排版 | — |
| `background` | paint | 对象区域的底色或填充，不是文字或轴线颜色 | none |
| `padding` | length \| length[] / 画布单位 | 内部留白；一个值或上、右、下、左四个值 | 0 |
| `border_radius` | length / 画布单位 | 底框圆角的物理半径 | 0 |
| `border_color` | color | 封闭轮廓颜色，none 表示无边框 | none |
| `border_width` | length / pt | 封闭轮廓线宽；裸值为 pt | — |
| `border_style` | "none" \| "solid" \| "dashed" \| "dotted" \| "dash_dot" \| "double" \| "triple" | 边框线型；与填充独立<br>`none`: 不绘制此项<br>`solid`: 连续实线<br>`dashed`: 虚线；可用 dash 参数覆盖长度<br>`dotted`: 点线<br>`dash_dot`: 点划线<br>`double`: 双线描边<br>`triple`: 三线描边 | solid |
| `border_dash` | length[] / pt | 交替的边框线段和间隔长度列表；须成对 | — |
| `border_dash_offset` | length / pt | 虚线相位；裸值为 pt | 0 |
| `border_cap` | "butt" \| "round" \| "square" | 轮廓虚线端点形状<br>`butt`: 平端帽，不向端点外延伸<br>`round`: 圆端帽，向外延伸半个线宽<br>`square`: 方端帽，向外延伸半个线宽 | butt |
| `border_join` | "miter" \| "round" \| "bevel" | 轮廓交点形状<br>`miter`: 延长边缘形成尖角，受尖角限制约束<br>`round`: 以圆弧连接相邻描边边缘<br>`bevel`: 以平边截去连接尖角 | miter |
| `border_miter_limit` | value | 尖角的最大延伸与线宽之比 | 4 |
| `border_opacity` | value | 边框独立透明度 0–1 | 1 |

### 最小完整示例

```lay
# Minimal complete example: formula
page=canvas(size=(100mm,75mm),background="#ffffff")
page.add(formula(r"E=mc^2",style=display,font_size=16pt,math_font=ratex_katex),offset=(8mm,8mm))
```

[概念与常见错误](topics/formulas.zh-CN.md#formula) · [组合源码](../examples/gallery/typography/display-formula.lay)

## rect

创建矩形素材；size 指定物理尺寸，fill 设置内部填充，border_* 设置边框，border_radius 设置圆角。调用 add 后才放置到页面。

返回：可重复放置的矩形素材。

必需：`size`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `size` | (length \| auto, length \| auto) / 画布单位 | 二维物理尺寸 (宽,高)；auto 保持比例或自然排版 | — |
| `class` | string | 空格分隔的 LCSS 类名 | "" |
| `fill` | paint | 形状内部的颜色、渐变、纹理或图片 | none |
| `fill_rule` | value | 自交和孔洞的填充规则 nonzero/evenodd | nonzero |
| `background` | paint | 对象区域的底色或填充，不是文字或轴线颜色 | none |
| `padding` | length \| length[] / 画布单位 | 内部留白；一个值或上、右、下、左四个值 | 0 |
| `border_radius` | length / 画布单位 | 底框圆角的物理半径 | 0 |
| `border_color` | color | 封闭轮廓颜色，none 表示无边框 | none |
| `border_width` | length / pt | 封闭轮廓线宽；裸值为 pt | 0.3mm (if border_color is set) |
| `border_style` | "none" \| "solid" \| "dashed" \| "dotted" \| "dash_dot" \| "double" \| "triple" | 边框线型；与填充独立<br>`none`: 不绘制此项<br>`solid`: 连续实线<br>`dashed`: 虚线；可用 dash 参数覆盖长度<br>`dotted`: 点线<br>`dash_dot`: 点划线<br>`double`: 双线描边<br>`triple`: 三线描边 | solid |
| `border_dash` | length[] / pt | 交替的边框线段和间隔长度列表；须成对 | — |
| `border_dash_offset` | length / pt | 虚线相位；裸值为 pt | 0 |
| `border_cap` | "butt" \| "round" \| "square" | 轮廓虚线端点形状<br>`butt`: 平端帽，不向端点外延伸<br>`round`: 圆端帽，向外延伸半个线宽<br>`square`: 方端帽，向外延伸半个线宽 | butt |
| `border_join` | "miter" \| "round" \| "bevel" | 轮廓交点形状<br>`miter`: 延长边缘形成尖角，受尖角限制约束<br>`round`: 以圆弧连接相邻描边边缘<br>`bevel`: 以平边截去连接尖角 | miter |
| `border_miter_limit` | value | 尖角的最大延伸与线宽之比 | 4 |
| `border_opacity` | value | 边框独立透明度 0–1 | 1 |

### 最小完整示例

```lay
# Minimal complete example: rect
page=canvas(size=(100mm,75mm),background="#ffffff")
page.add(rect(size=(40mm,25mm),border_radius=7mm,fill="#e6f2f3",border_color="#087f8c",border_width=1mm),offset=(10mm,10mm))
```

[概念与常见错误](topics/shapes.zh-CN.md#rect) · [组合源码](../examples/basic.lay)

## ellipse

由 size=(宽,高) 定义椭圆素材。fill 与 border_* 分别控制内部与描边；布局框角点不一定在椭圆上。

返回：material

必需：`size`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `size` | (length \| auto, length \| auto) / 画布单位 | 二维物理尺寸 (宽,高)；auto 保持比例或自然排版 | — |
| `class` | string | 空格分隔的 LCSS 类名 | "" |
| `fill` | paint | 形状内部的颜色、渐变、纹理或图片 | none |
| `fill_rule` | value | 自交和孔洞的填充规则 nonzero/evenodd | nonzero |
| `border_color` | color | 封闭轮廓颜色，none 表示无边框 | none |
| `border_width` | length / pt | 封闭轮廓线宽；裸值为 pt | 0.3mm (if border_color is set) |
| `border_style` | "none" \| "solid" \| "dashed" \| "dotted" \| "dash_dot" \| "double" \| "triple" | 边框线型；与填充独立<br>`none`: 不绘制此项<br>`solid`: 连续实线<br>`dashed`: 虚线；可用 dash 参数覆盖长度<br>`dotted`: 点线<br>`dash_dot`: 点划线<br>`double`: 双线描边<br>`triple`: 三线描边 | solid |
| `border_dash` | length[] / pt | 交替的边框线段和间隔长度列表；须成对 | — |
| `border_dash_offset` | length / pt | 虚线相位；裸值为 pt | 0 |
| `border_cap` | "butt" \| "round" \| "square" | 轮廓虚线端点形状<br>`butt`: 平端帽，不向端点外延伸<br>`round`: 圆端帽，向外延伸半个线宽<br>`square`: 方端帽，向外延伸半个线宽 | butt |
| `border_join` | "miter" \| "round" \| "bevel" | 轮廓交点形状<br>`miter`: 延长边缘形成尖角，受尖角限制约束<br>`round`: 以圆弧连接相邻描边边缘<br>`bevel`: 以平边截去连接尖角 | miter |
| `border_miter_limit` | value | 尖角的最大延伸与线宽之比 | 4 |
| `border_opacity` | value | 边框独立透明度 0–1 | 1 |

### 最小完整示例

```lay
# Minimal complete example: ellipse
page=canvas(size=(100mm,75mm),background="#ffffff")
page.add(ellipse(size=(45mm,25mm),fill="#e6f2f3",border_color="#087f8c",border_width=1mm),offset=(10mm,10mm))
```

[概念与常见错误](topics/shapes.zh-CN.md#ellipse) · [组合源码](../examples/basic.lay)

## line

创建线段；使用 dx/dy 或 length/angle，支持独立端帽和头部。零长度需要显式 angle，头部大小不随线长缩小。

返回：material

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `length` | length / 画布单位 | 线段非负长度，与 dx/dy 互斥 | — |
| `angle` | angle / deg | 局部方向；0 朝右，90 朝下；零长度必须显式指定 | 0deg |
| `dx` | length / 画布单位 | 线段水平位移；与 dy 同时指定，与 length/angle 互斥 | — |
| `dy` | length / 画布单位 | 线段竖直位移；与 dx 同时指定，与 length/angle 互斥 | — |
| `class` | string | 空格分隔的 LCSS 类名 | "" |
| `line_color` | color | 开放线条颜色；默认采用数据系列颜色或主题颜色 | #000000 |
| `line_width` | length / pt | 开放线条宽度；裸值为 pt | 0.3pt |
| `line_style` | "none" \| "solid" \| "dashed" \| "dotted" \| "dash_dot" \| "double" \| "triple" | 线型；double/triple 保留透明间隔<br>`none`: 不绘制此项<br>`solid`: 连续实线<br>`dashed`: 虚线；可用 dash 参数覆盖长度<br>`dotted`: 点线<br>`dash_dot`: 点划线<br>`double`: 双线描边<br>`triple`: 三线描边 | solid |
| `line_dash` | length[] / pt | 交替的实线段和间隔长度列表；裸值为 pt | — |
| `line_dash_offset` | length / pt | 虚线相位；裸值为 pt | 0 |
| `line_cap` | "butt" \| "round" \| "square" | 开放端点形状<br>`butt`: 平端帽，不向端点外延伸<br>`round`: 圆端帽，向外延伸半个线宽<br>`square`: 方端帽，向外延伸半个线宽 | butt |
| `line_join` | "miter" \| "round" \| "bevel" | 线段交点形状<br>`miter`: 延长边缘形成尖角，受尖角限制约束<br>`round`: 以圆弧连接相邻描边边缘<br>`bevel`: 以平边截去连接尖角 | miter |
| `line_miter_limit` | value | 尖角的最大延伸与线宽之比 | 4 |
| `line_opacity` | value | 线条独立透明度 0–1 | 1 |
| `start_head` | head | 起始端头部配置，默认无 | — |
| `end_head` | head | 结束端头部配置，默认无 | — |
| `start_cap` | "butt" \| "round" \| "square" | 独立端帽，默认继承 line_cap<br>`butt`: 平端帽，不向端点外延伸<br>`round`: 圆端帽，向外延伸半个线宽<br>`square`: 方端帽，向外延伸半个线宽 | 继承 line_cap |
| `end_cap` | "butt" \| "round" \| "square" | 独立端帽，默认继承 line_cap<br>`butt`: 平端帽，不向端点外延伸<br>`round`: 圆端帽，向外延伸半个线宽<br>`square`: 方端帽，向外延伸半个线宽 | 继承 line_cap |

### 最小完整示例

```lay
# Minimal complete example: line
page=canvas(size=(100mm,75mm),background="#ffffff")
page.add(line(length=55mm,angle=20deg,start_cap=round,end_head=head(shape=triangle,size=(6mm,5mm)),line_width=2mm),offset=(12mm,15mm))
```

[概念与常见错误](topics/shapes.zh-CN.md#line) · [组合源码](../examples/basic.lay)

## group

创建带局部坐标的复用容器。先添加子实例再放置整个组；第一次放置封存内容，重复放置重放内部定位关系。

返回：material

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `class` | string | 空格分隔的 LCSS 类名 | "" |
| `font_family` | string \| string[] | 系统字体族名、字体文件路径或有序列表；不内置正文字体；缺字警告并显示方框。 | system sans-serif |
| `font_size` | length / pt | 字号；省略单位时为 pt；未指定时继承 | 继承 / inherit |
| `font_weight` | integer | 字体字重，100–900 | 400 |
| `font_style` | "normal" \| "italic" | 正体或斜体<br>`normal`: 正常字形<br>`italic`: 斜体字形 | normal |
| `color` | color | 文字颜色或数据系列基础配色 | — |
| `line_height` | length / pt | 文字行高；裸值为 pt | 1.2 × font_size |
| `background` | paint | 对象区域的底色或填充，不是文字或轴线颜色 | none |
| `padding` | length \| length[] / 画布单位 | 内部留白；一个值或上、右、下、左四个值 | 0 |
| `border_radius` | length / 画布单位 | 底框圆角的物理半径 | 0 |
| `border_color` | color | 封闭轮廓颜色，none 表示无边框 | none |
| `border_width` | length / pt | 封闭轮廓线宽；裸值为 pt | — |
| `border_style` | "none" \| "solid" \| "dashed" \| "dotted" \| "dash_dot" \| "double" \| "triple" | 边框线型；与填充独立<br>`none`: 不绘制此项<br>`solid`: 连续实线<br>`dashed`: 虚线；可用 dash 参数覆盖长度<br>`dotted`: 点线<br>`dash_dot`: 点划线<br>`double`: 双线描边<br>`triple`: 三线描边 | solid |
| `border_dash` | length[] / pt | 交替的边框线段和间隔长度列表；须成对 | — |
| `border_dash_offset` | length / pt | 虚线相位；裸值为 pt | 0 |
| `border_cap` | "butt" \| "round" \| "square" | 轮廓虚线端点形状<br>`butt`: 平端帽，不向端点外延伸<br>`round`: 圆端帽，向外延伸半个线宽<br>`square`: 方端帽，向外延伸半个线宽 | butt |
| `border_join` | "miter" \| "round" \| "bevel" | 轮廓交点形状<br>`miter`: 延长边缘形成尖角，受尖角限制约束<br>`round`: 以圆弧连接相邻描边边缘<br>`bevel`: 以平边截去连接尖角 | miter |
| `border_miter_limit` | value | 尖角的最大延伸与线宽之比 | 4 |
| `border_opacity` | value | 边框独立透明度 0–1 | 1 |

### 最小完整示例

```lay
# Minimal complete example: group
page=canvas(size=(100mm,75mm),background="#ffffff")
g=group()
a=g.add(rect(size=(35mm,20mm),fill="#e6f2f3"))
g.add(text("Reused"),target=a.center,anchor=center)
page.add(g,offset=(8mm,8mm))
page.add(g,offset=(55mm,38mm),rotation=15deg)
```

[概念与常见错误](topics/groups.zh-CN.md#group) · [组合源码](../examples/gallery/containers/group-reuse.lay)

## path

由原始 move_to/line_to/quad_to/cubic_to/arc_to/close 指令定义路径。可含多个子路径；开放子路径支持端帽与头部。

返回：material

必需：`commands`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `commands` | path-command[] | 从 move_to 开始的有序路径命令列表 | — |
| `class` | string | 空格分隔的 LCSS 类名 | "" |
| `fill` | paint | 形状内部的颜色、渐变、纹理或图片 | none |
| `fill_rule` | value | 自交和孔洞的填充规则 nonzero/evenodd | nonzero |
| `border_color` | color | 封闭轮廓颜色，none 表示无边框 | none |
| `border_width` | length / pt | 封闭轮廓线宽；裸值为 pt | — |
| `border_style` | "none" \| "solid" \| "dashed" \| "dotted" \| "dash_dot" \| "double" \| "triple" | 边框线型；与填充独立<br>`none`: 不绘制此项<br>`solid`: 连续实线<br>`dashed`: 虚线；可用 dash 参数覆盖长度<br>`dotted`: 点线<br>`dash_dot`: 点划线<br>`double`: 双线描边<br>`triple`: 三线描边 | solid |
| `border_dash` | length[] / pt | 交替的边框线段和间隔长度列表；须成对 | — |
| `border_dash_offset` | length / pt | 虚线相位；裸值为 pt | 0 |
| `border_cap` | "butt" \| "round" \| "square" | 轮廓虚线端点形状<br>`butt`: 平端帽，不向端点外延伸<br>`round`: 圆端帽，向外延伸半个线宽<br>`square`: 方端帽，向外延伸半个线宽 | butt |
| `border_join` | "miter" \| "round" \| "bevel" | 轮廓交点形状<br>`miter`: 延长边缘形成尖角，受尖角限制约束<br>`round`: 以圆弧连接相邻描边边缘<br>`bevel`: 以平边截去连接尖角 | miter |
| `border_miter_limit` | value | 尖角的最大延伸与线宽之比 | 4 |
| `border_opacity` | value | 边框独立透明度 0–1 | 1 |
| `start_head` | head | 起始端头部配置，默认无 | — |
| `end_head` | head | 结束端头部配置，默认无 | — |
| `start_cap` | "butt" \| "round" \| "square" | 路径独立端帽，默认继承 border_cap；虚线内部端帽不受此项影响<br>`butt`: 平端帽，不向端点外延伸<br>`round`: 圆端帽，向外延伸半个线宽<br>`square`: 方端帽，向外延伸半个线宽 | 继承 border_cap |
| `end_cap` | "butt" \| "round" \| "square" | 路径独立端帽，默认继承 border_cap；虚线内部端帽不受此项影响<br>`butt`: 平端帽，不向端点外延伸<br>`round`: 圆端帽，向外延伸半个线宽<br>`square`: 方端帽，向外延伸半个线宽 | 继承 border_cap |

### 最小完整示例

```lay
# Minimal complete example: path
page=canvas(size=(100mm,75mm),background="#ffffff")
page.add(path(commands=[move_to(0mm,20mm),quad_to(20mm,0mm,40mm,20mm),line_to(40mm,35mm),close()],fill="#e6f2f3",border_color="#087f8c",border_width=1mm),offset=(10mm,10mm))
```

[概念与常见错误](topics/shapes.zh-CN.md#path) · [组合源码](../examples/gallery/shapes/path-commands.lay)

## polygon

用至少三个 points 定义自动闭合的多边形。节点顺序决定边界遍历方向，fill_rule 控制自交填充。

返回：material

必需：`points`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `points` | value | 点列表；star 为整数角数，violin 为 KDE 采样点数 | — |
| `class` | string | 空格分隔的 LCSS 类名 | "" |
| `fill` | paint | 形状内部的颜色、渐变、纹理或图片 | none |
| `fill_rule` | value | 自交和孔洞的填充规则 nonzero/evenodd | nonzero |
| `border_color` | color | 封闭轮廓颜色，none 表示无边框 | none |
| `border_width` | length / pt | 封闭轮廓线宽；裸值为 pt | — |
| `border_style` | "none" \| "solid" \| "dashed" \| "dotted" \| "dash_dot" \| "double" \| "triple" | 边框线型；与填充独立<br>`none`: 不绘制此项<br>`solid`: 连续实线<br>`dashed`: 虚线；可用 dash 参数覆盖长度<br>`dotted`: 点线<br>`dash_dot`: 点划线<br>`double`: 双线描边<br>`triple`: 三线描边 | solid |
| `border_dash` | length[] / pt | 交替的边框线段和间隔长度列表；须成对 | — |
| `border_dash_offset` | length / pt | 虚线相位；裸值为 pt | 0 |
| `border_cap` | "butt" \| "round" \| "square" | 轮廓虚线端点形状<br>`butt`: 平端帽，不向端点外延伸<br>`round`: 圆端帽，向外延伸半个线宽<br>`square`: 方端帽，向外延伸半个线宽 | butt |
| `border_join` | "miter" \| "round" \| "bevel" | 轮廓交点形状<br>`miter`: 延长边缘形成尖角，受尖角限制约束<br>`round`: 以圆弧连接相邻描边边缘<br>`bevel`: 以平边截去连接尖角 | miter |
| `border_miter_limit` | value | 尖角的最大延伸与线宽之比 | 4 |
| `border_opacity` | value | 边框独立透明度 0–1 | 1 |

### 最小完整示例

```lay
# Minimal complete example: polygon
page=canvas(size=(100mm,75mm),background="#ffffff")
page.add(polygon(points=[(0mm,0mm),(40mm,0mm),(30mm,30mm)],fill="#e6f2f3"),offset=(10mm,10mm))
```

[概念与常见错误](topics/shapes.zh-CN.md#polygon) · [组合源码](../examples/gallery/shapes/polygon-polyline.lay)

## polyline

用至少两个 points 定义开放折线。line_* 控制描边，start_head/end_head 在逻辑端点定位，转角由 line_join 决定。

返回：material

必需：`points`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `points` | value | 点列表；star 为整数角数，violin 为 KDE 采样点数 | — |
| `class` | string | 空格分隔的 LCSS 类名 | "" |
| `fill` | paint | 形状内部的颜色、渐变、纹理或图片 | none |
| `fill_rule` | value | 自交和孔洞的填充规则 nonzero/evenodd | nonzero |
| `line_color` | color | 开放线条颜色；默认采用数据系列颜色或主题颜色 | — |
| `line_width` | length / pt | 开放线条宽度；裸值为 pt | — |
| `line_style` | "none" \| "solid" \| "dashed" \| "dotted" \| "dash_dot" \| "double" \| "triple" | 线型；double/triple 保留透明间隔<br>`none`: 不绘制此项<br>`solid`: 连续实线<br>`dashed`: 虚线；可用 dash 参数覆盖长度<br>`dotted`: 点线<br>`dash_dot`: 点划线<br>`double`: 双线描边<br>`triple`: 三线描边 | solid |
| `line_dash` | length[] / pt | 交替的实线段和间隔长度列表；裸值为 pt | — |
| `line_dash_offset` | length / pt | 虚线相位；裸值为 pt | 0 |
| `line_cap` | "butt" \| "round" \| "square" | 开放端点形状<br>`butt`: 平端帽，不向端点外延伸<br>`round`: 圆端帽，向外延伸半个线宽<br>`square`: 方端帽，向外延伸半个线宽 | butt |
| `line_join` | "miter" \| "round" \| "bevel" | 线段交点形状<br>`miter`: 延长边缘形成尖角，受尖角限制约束<br>`round`: 以圆弧连接相邻描边边缘<br>`bevel`: 以平边截去连接尖角 | miter |
| `line_miter_limit` | value | 尖角的最大延伸与线宽之比 | 4 |
| `line_opacity` | value | 线条独立透明度 0–1 | 1 |
| `start_head` | head | 起始端头部配置，默认无 | — |
| `end_head` | head | 结束端头部配置，默认无 | — |
| `start_cap` | "butt" \| "round" \| "square" | 独立端帽，默认继承 line_cap<br>`butt`: 平端帽，不向端点外延伸<br>`round`: 圆端帽，向外延伸半个线宽<br>`square`: 方端帽，向外延伸半个线宽 | 继承 line_cap |
| `end_cap` | "butt" \| "round" \| "square" | 独立端帽，默认继承 line_cap<br>`butt`: 平端帽，不向端点外延伸<br>`round`: 圆端帽，向外延伸半个线宽<br>`square`: 方端帽，向外延伸半个线宽 | 继承 line_cap |

### 最小完整示例

```lay
# Minimal complete example: polyline
page=canvas(size=(100mm,75mm),background="#ffffff")
page.add(polyline(points=[(0mm,20mm),(20mm,0mm),(45mm,20mm)],line_color="#087f8c",line_width=1.5mm,line_join=round),offset=(10mm,10mm))
```

[概念与常见错误](topics/shapes.zh-CN.md#polyline) · [组合源码](../examples/gallery/shapes/polygon-polyline.lay)

## arc

定义 radius、start、end 指定的开放圆弧。页面角度 0 朝右、90 朝下；可设置端帽和头部，不自动封闭成扇形。

返回：material

必需：`radius`, `start`, `end`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `radius` | length / 画布单位 | 几何半径；radial_gradient 中为归一化标量 | — |
| `start` | value | 圆弧起始角；线性渐变中为归一化起点 | — |
| `end` | value | 圆弧终止角；线性渐变中为归一化终点 | — |
| `class` | string | 空格分隔的 LCSS 类名 | "" |
| `fill` | paint | 形状内部的颜色、渐变、纹理或图片 | none |
| `fill_rule` | value | 自交和孔洞的填充规则 nonzero/evenodd | nonzero |
| `line_color` | color | 开放线条颜色；默认采用数据系列颜色或主题颜色 | — |
| `line_width` | length / pt | 开放线条宽度；裸值为 pt | — |
| `line_style` | "none" \| "solid" \| "dashed" \| "dotted" \| "dash_dot" \| "double" \| "triple" | 线型；double/triple 保留透明间隔<br>`none`: 不绘制此项<br>`solid`: 连续实线<br>`dashed`: 虚线；可用 dash 参数覆盖长度<br>`dotted`: 点线<br>`dash_dot`: 点划线<br>`double`: 双线描边<br>`triple`: 三线描边 | solid |
| `line_dash` | length[] / pt | 交替的实线段和间隔长度列表；裸值为 pt | — |
| `line_dash_offset` | length / pt | 虚线相位；裸值为 pt | 0 |
| `line_cap` | "butt" \| "round" \| "square" | 开放端点形状<br>`butt`: 平端帽，不向端点外延伸<br>`round`: 圆端帽，向外延伸半个线宽<br>`square`: 方端帽，向外延伸半个线宽 | butt |
| `line_join` | "miter" \| "round" \| "bevel" | 线段交点形状<br>`miter`: 延长边缘形成尖角，受尖角限制约束<br>`round`: 以圆弧连接相邻描边边缘<br>`bevel`: 以平边截去连接尖角 | miter |
| `line_miter_limit` | value | 尖角的最大延伸与线宽之比 | 4 |
| `line_opacity` | value | 线条独立透明度 0–1 | 1 |
| `start_head` | head | 起始端头部配置，默认无 | — |
| `end_head` | head | 结束端头部配置，默认无 | — |
| `start_cap` | "butt" \| "round" \| "square" | 独立端帽，默认继承 line_cap<br>`butt`: 平端帽，不向端点外延伸<br>`round`: 圆端帽，向外延伸半个线宽<br>`square`: 方端帽，向外延伸半个线宽 | 继承 line_cap |
| `end_cap` | "butt" \| "round" \| "square" | 独立端帽，默认继承 line_cap<br>`butt`: 平端帽，不向端点外延伸<br>`round`: 圆端帽，向外延伸半个线宽<br>`square`: 方端帽，向外延伸半个线宽 | 继承 line_cap |

### 最小完整示例

```lay
# Minimal complete example: arc
page=canvas(size=(100mm,75mm),background="#ffffff")
page.add(arc(radius=20mm,start=0deg,end=120deg,line_width=1mm,end_head=head(shape=open)),offset=(15mm,15mm))
```

[概念与常见错误](topics/shapes.zh-CN.md#arc) · [组合源码](../examples/gallery/shapes/arc-sector.lay)

## sector

由半径和起止角定义闭合扇形。填充包括圆心与圆弧之间的区域，描边使用 border_*。

返回：material

必需：`radius`, `start`, `end`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `radius` | length / 画布单位 | 几何半径；radial_gradient 中为归一化标量 | — |
| `start` | value | 圆弧起始角；线性渐变中为归一化起点 | — |
| `end` | value | 圆弧终止角；线性渐变中为归一化终点 | — |
| `class` | string | 空格分隔的 LCSS 类名 | "" |
| `fill` | paint | 形状内部的颜色、渐变、纹理或图片 | none |
| `fill_rule` | value | 自交和孔洞的填充规则 nonzero/evenodd | nonzero |
| `border_color` | color | 封闭轮廓颜色，none 表示无边框 | none |
| `border_width` | length / pt | 封闭轮廓线宽；裸值为 pt | — |
| `border_style` | "none" \| "solid" \| "dashed" \| "dotted" \| "dash_dot" \| "double" \| "triple" | 边框线型；与填充独立<br>`none`: 不绘制此项<br>`solid`: 连续实线<br>`dashed`: 虚线；可用 dash 参数覆盖长度<br>`dotted`: 点线<br>`dash_dot`: 点划线<br>`double`: 双线描边<br>`triple`: 三线描边 | solid |
| `border_dash` | length[] / pt | 交替的边框线段和间隔长度列表；须成对 | — |
| `border_dash_offset` | length / pt | 虚线相位；裸值为 pt | 0 |
| `border_cap` | "butt" \| "round" \| "square" | 轮廓虚线端点形状<br>`butt`: 平端帽，不向端点外延伸<br>`round`: 圆端帽，向外延伸半个线宽<br>`square`: 方端帽，向外延伸半个线宽 | butt |
| `border_join` | "miter" \| "round" \| "bevel" | 轮廓交点形状<br>`miter`: 延长边缘形成尖角，受尖角限制约束<br>`round`: 以圆弧连接相邻描边边缘<br>`bevel`: 以平边截去连接尖角 | miter |
| `border_miter_limit` | value | 尖角的最大延伸与线宽之比 | 4 |
| `border_opacity` | value | 边框独立透明度 0–1 | 1 |

### 最小完整示例

```lay
# Minimal complete example: sector
page=canvas(size=(100mm,75mm),background="#ffffff")
page.add(sector(radius=20mm,start=0deg,end=120deg,fill="#087f8c"),offset=(15mm,15mm))
```

[概念与常见错误](topics/shapes.zh-CN.md#sector) · [组合源码](../examples/gallery/shapes/arc-sector.lay)

## star

由尖数、外半径和内半径定义闭合星形。inner_radius 小于 outer_radius，rotation 决定初始方向。

返回：material

必需：`points`, `outer_radius`, `inner_radius`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `points` | value | 点列表；star 为整数角数，violin 为 KDE 采样点数 | — |
| `outer_radius` | length / 画布单位 | 外轮廓的几何半径 | — |
| `inner_radius` | length / 画布单位 | 内轮廓几何半径；极坐标中为物理内孔半径 | 0 |
| `rotation` | value | 围绕实例中心旋转的角度 | 0deg |
| `class` | string | 空格分隔的 LCSS 类名 | "" |
| `fill` | paint | 形状内部的颜色、渐变、纹理或图片 | none |
| `fill_rule` | value | 自交和孔洞的填充规则 nonzero/evenodd | nonzero |
| `border_color` | color | 封闭轮廓颜色，none 表示无边框 | none |
| `border_width` | length / pt | 封闭轮廓线宽；裸值为 pt | — |
| `border_style` | "none" \| "solid" \| "dashed" \| "dotted" \| "dash_dot" \| "double" \| "triple" | 边框线型；与填充独立<br>`none`: 不绘制此项<br>`solid`: 连续实线<br>`dashed`: 虚线；可用 dash 参数覆盖长度<br>`dotted`: 点线<br>`dash_dot`: 点划线<br>`double`: 双线描边<br>`triple`: 三线描边 | solid |
| `border_dash` | length[] / pt | 交替的边框线段和间隔长度列表；须成对 | — |
| `border_dash_offset` | length / pt | 虚线相位；裸值为 pt | 0 |
| `border_cap` | "butt" \| "round" \| "square" | 轮廓虚线端点形状<br>`butt`: 平端帽，不向端点外延伸<br>`round`: 圆端帽，向外延伸半个线宽<br>`square`: 方端帽，向外延伸半个线宽 | butt |
| `border_join` | "miter" \| "round" \| "bevel" | 轮廓交点形状<br>`miter`: 延长边缘形成尖角，受尖角限制约束<br>`round`: 以圆弧连接相邻描边边缘<br>`bevel`: 以平边截去连接尖角 | miter |
| `border_miter_limit` | value | 尖角的最大延伸与线宽之比 | 4 |
| `border_opacity` | value | 边框独立透明度 0–1 | 1 |

### 最小完整示例

```lay
# Minimal complete example: star
page=canvas(size=(100mm,75mm),background="#ffffff")
page.add(star(points=5,outer_radius=20mm,inner_radius=9mm,fill="#e6f2f3",border_color="#087f8c"),offset=(15mm,15mm))
```

[概念与常见错误](topics/shapes.zh-CN.md#star) · [组合源码](../examples/gallery/shapes/star-ring.lay)

## ring

定义带透明内孔的圆环。inner_radius 小于 outer_radius，孔不以背景色遮盖，可透出底下的绘制。

返回：material

必需：`outer_radius`, `inner_radius`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `outer_radius` | length / 画布单位 | 外轮廓的几何半径 | — |
| `inner_radius` | length / 画布单位 | 内轮廓几何半径；极坐标中为物理内孔半径 | 0 |
| `class` | string | 空格分隔的 LCSS 类名 | "" |
| `fill` | paint | 形状内部的颜色、渐变、纹理或图片 | none |
| `fill_rule` | value | 自交和孔洞的填充规则 nonzero/evenodd | nonzero |
| `border_color` | color | 封闭轮廓颜色，none 表示无边框 | none |
| `border_width` | length / pt | 封闭轮廓线宽；裸值为 pt | — |
| `border_style` | "none" \| "solid" \| "dashed" \| "dotted" \| "dash_dot" \| "double" \| "triple" | 边框线型；与填充独立<br>`none`: 不绘制此项<br>`solid`: 连续实线<br>`dashed`: 虚线；可用 dash 参数覆盖长度<br>`dotted`: 点线<br>`dash_dot`: 点划线<br>`double`: 双线描边<br>`triple`: 三线描边 | solid |
| `border_dash` | length[] / pt | 交替的边框线段和间隔长度列表；须成对 | — |
| `border_dash_offset` | length / pt | 虚线相位；裸值为 pt | 0 |
| `border_cap` | "butt" \| "round" \| "square" | 轮廓虚线端点形状<br>`butt`: 平端帽，不向端点外延伸<br>`round`: 圆端帽，向外延伸半个线宽<br>`square`: 方端帽，向外延伸半个线宽 | butt |
| `border_join` | "miter" \| "round" \| "bevel" | 轮廓交点形状<br>`miter`: 延长边缘形成尖角，受尖角限制约束<br>`round`: 以圆弧连接相邻描边边缘<br>`bevel`: 以平边截去连接尖角 | miter |
| `border_miter_limit` | value | 尖角的最大延伸与线宽之比 | 4 |
| `border_opacity` | value | 边框独立透明度 0–1 | 1 |

### 最小完整示例

```lay
# Minimal complete example: ring
page=canvas(size=(100mm,75mm),background="#ffffff")
page.add(ring(outer_radius=20mm,inner_radius=13mm,fill="#087f8c"),offset=(15mm,15mm))
```

[概念与常见错误](topics/shapes.zh-CN.md#ring) · [组合源码](../examples/gallery/shapes/star-ring.lay)

## box

建立矩形区域配置，供 plot_area 或 crop 使用。box 自身不绘制；单位由使用场景决定，crop 为 0–1 归一化坐标。

返回：box

必需：`size`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `offset` | (length, length) / 画布单位 | 相对目标的 (横向, 纵向) 偏移 | (0, 0) |
| `size` | (length \| auto, length \| auto) / 画布单位 | 二维物理尺寸 (宽,高)；auto 保持比例或自然排版 | — |

### 最小完整示例

```lay
# Minimal complete example: box
page=canvas(size=(100mm,75mm),background="#ffffff")
page.add(image(src="../assets/photo.png"),size=(60mm,auto),crop=box(offset=(0.25,0),size=(0.75,1)),offset=(10mm,10mm))
```

[概念与常见错误](topics/plot-area.zh-CN.md#box) · [组合源码](../examples/plot/publication.lay)

## instance.data

在已放置图表中选择原始数据坐标点。轴变换、断轴、实例变换及组重放都重新解析此引用；点必须处于有效可见数据域。

返回：anchor

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `x` | axis \| number[] | 直角坐标横轴配置或图层横坐标数据 | — |
| `y` | axis \| number[] | 直角坐标纵轴配置或图层纵坐标数据 | — |
| `x_axis` | value | 绑定的横轴名称 | x |
| `y_axis` | value | 绑定的纵轴名称 | y |
| `theta` | axis \| number[] | 极坐标角轴配置或图层角坐标数据 | — |
| `r` | axis \| number[] | 极坐标径向轴配置或半径数据 | — |
| `category` | value | 雷达图类别名称 | — |
| `value` | value | 待处理的标量、字符串或列表元素 | — |

### 最小完整示例

```lay
# Minimal complete example: instance.data
page=canvas(size=(100mm,75mm),background="#ffffff")
p=plot(size=(86mm,62mm),x=axis(range=(0,4)),y=axis(range=(0,5)))
p.scatter(x=[1,2,3],y=[1,3,2])
chart=page.add(p,offset=(7mm,6mm))
page.add(text("Peak"),target=chart.data(x=2,y=3),anchor=bottom_center,offset=(0mm,-2mm))
```

[概念与常见错误](topics/anchors.zh-CN.md#instance-data) · [组合源码](../examples/gallery/positioning/chart-parts.lay)

## instance.axis

按名称选择图表轴的数值起端、物理中点或数值终端。保留旧变换语义；新几何查询可通过 axes[name].spine.path 选轴线。

返回：anchor

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `name` | string | 画布标题或命名坐标轴的唯一名称 | — |
| `anchor` | "start" \| "center" \| "end" | 当前实例用于对齐的锚点<br>`start`: 轴线遍历起端<br>`center`: 轴线中点<br>`end`: 轴线遍历末端 | top_left |

### 最小完整示例

```lay
# Minimal complete example: instance.axis
page=canvas(size=(100mm,75mm),background="#ffffff")
p=plot(size=(86mm,62mm),x=axis(range=(0,4)),y=axis(range=(0,5)))
p.line(x=[0,1,2,3],y=[1,2,4,3])
chart=page.add(p,offset=(7mm,6mm))
page.add(ellipse(size=(2mm,2mm),fill="#e36b70"),anchor=center,target=chart.axis(name="x",anchor=center))
```

[概念与常见错误](topics/anchors.zh-CN.md#instance-axis) · [组合源码](../examples/manual/geometry-composition.lay)

## add

将素材放置到画布或组合中，使用 anchor 对齐素材锚点、target 指定目标、offset 设置偏移。同一素材可以重复放置，各实例可独立设置尺寸与样式。

返回：已放置实例；可读取测量尺寸并引用其锚点继续定位。

位置参数：`material`.

必需：`material`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `size` | (length \| auto, length \| auto) / 画布单位 | 二维物理尺寸 (宽,高)；auto 保持比例或自然排版 | — |
| `fit` | "contain" \| "cover" \| "stretch" | contain 完整显示，cover 填满裁剪，stretch 拉伸<br>`contain`: 保持比例完整显示，容器可能留空<br>`cover`: 保持比例填满，超出容器的部分裁剪<br>`stretch`: 独立缩放宽高以填满目标尺寸 | contain |
| `crop` | box (normalized 0–1) | 原图中的归一化裁剪区域；各坐标为 0–1 | — |
| `anchor` | "top_left" \| "top_center" \| "top_right" \| "middle_left" \| "center" \| "middle_right" \| "bottom_left" \| "bottom_center" \| "bottom_right" \| "start" \| "end" \| "plot_top_left" \| "plot_top_center" \| "plot_top_right" \| "plot_middle_left" \| "plot_center" \| "plot_middle_right" \| "plot_bottom_left" \| "plot_bottom_center" \| "plot_bottom_right" \| self selector | 自身布局框九点名称、线段 start/end、图表 plot_*，或 self 几何选择器；target 必须来自同一容器中已放置实例<br>`top_left`: 左上角，对应布局框<br>`top_center`: 上边中点，对应布局框<br>`top_right`: 右上角，对应布局框<br>`middle_left`: 左边中点，对应布局框<br>`center`: 中心，对应布局框<br>`middle_right`: 右边中点，对应布局框<br>`bottom_left`: 左下角，对应布局框<br>`bottom_center`: 下边中点，对应布局框<br>`bottom_right`: 右下角，对应布局框<br>`start`: 旧端点锚点：只适用于具有端点的素材；保留旧变换语义<br>`end`: 旧端点锚点：只适用于具有端点的素材；保留旧变换语义<br>`plot_top_left`: 图表绘图区九点锚点；只适用于图表素材，随实例变换<br>`plot_top_center`: 图表绘图区九点锚点；只适用于图表素材，随实例变换<br>`plot_top_right`: 图表绘图区九点锚点；只适用于图表素材，随实例变换<br>`plot_middle_left`: 图表绘图区九点锚点；只适用于图表素材，随实例变换<br>`plot_center`: 图表绘图区九点锚点；只适用于图表素材，随实例变换<br>`plot_middle_right`: 图表绘图区九点锚点；只适用于图表素材，随实例变换<br>`plot_bottom_left`: 图表绘图区九点锚点；只适用于图表素材，随实例变换<br>`plot_bottom_center`: 图表绘图区九点锚点；只适用于图表素材，随实例变换<br>`plot_bottom_right`: 图表绘图区九点锚点；只适用于图表素材，随实例变换 | top_left |
| `target` | anchor | 同一容器中已放置实例的锚点；候选集合须显式索引 | parent.top_left |
| `offset` | (length, length) / 画布单位 | 相对目标的 (横向, 纵向) 偏移 | (0, 0) |
| `rotation` | angle | 围绕实例中心旋转，可使用路径锚点的 tangent_angle | 0deg |
| `opacity` | number \| number[] | 对象透明度，0–1；与父级透明度相乘 | 1 |
| `class` | string | 空格分隔的 LCSS 类名 | "" |
| `font_family` | string \| string[] | 系统字体族名、字体文件路径或有序列表；不内置正文字体；缺字警告并显示方框。 | system sans-serif |
| `font_size` | length / pt | 字号；省略单位时为 pt；未指定时继承 | 继承 / inherit |
| `font_weight` | integer | 字体字重，100–900 | 400 |
| `font_style` | "normal" \| "italic" | 正体或斜体<br>`normal`: 正常字形<br>`italic`: 斜体字形 | normal |
| `color` | color | 文字颜色或数据系列基础配色 | — |
| `line_height` | length / pt | 文字行高；裸值为 pt | 1.2 × font_size |
| `background` | paint | 对象区域的底色或填充，不是文字或轴线颜色 | none |
| `padding` | length \| length[] / 画布单位 | 内部留白；一个值或上、右、下、左四个值 | 0 |
| `border_radius` | length / 画布单位 | 底框圆角的物理半径 | 0 |
| `border_color` | color | 封闭轮廓颜色，none 表示无边框 | none |
| `border_width` | length / pt | 封闭轮廓线宽；裸值为 pt | — |
| `border_style` | "none" \| "solid" \| "dashed" \| "dotted" \| "dash_dot" \| "double" \| "triple" | 边框线型；与填充独立<br>`none`: 不绘制此项<br>`solid`: 连续实线<br>`dashed`: 虚线；可用 dash 参数覆盖长度<br>`dotted`: 点线<br>`dash_dot`: 点划线<br>`double`: 双线描边<br>`triple`: 三线描边 | solid |
| `border_dash` | length[] / pt | 交替的边框线段和间隔长度列表；须成对 | — |
| `border_dash_offset` | length / pt | 虚线相位；裸值为 pt | 0 |
| `border_cap` | "butt" \| "round" \| "square" | 轮廓虚线端点形状<br>`butt`: 平端帽，不向端点外延伸<br>`round`: 圆端帽，向外延伸半个线宽<br>`square`: 方端帽，向外延伸半个线宽 | butt |
| `border_join` | "miter" \| "round" \| "bevel" | 轮廓交点形状<br>`miter`: 延长边缘形成尖角，受尖角限制约束<br>`round`: 以圆弧连接相邻描边边缘<br>`bevel`: 以平边截去连接尖角 | miter |
| `border_miter_limit` | value | 尖角的最大延伸与线宽之比 | 4 |
| `border_opacity` | value | 边框独立透明度 0–1 | 1 |
| `line_color` | color | 开放线条颜色；默认采用数据系列颜色或主题颜色 | — |
| `line_width` | length / pt | 开放线条宽度；裸值为 pt | — |
| `line_style` | "none" \| "solid" \| "dashed" \| "dotted" \| "dash_dot" \| "double" \| "triple" | 线型；double/triple 保留透明间隔<br>`none`: 不绘制此项<br>`solid`: 连续实线<br>`dashed`: 虚线；可用 dash 参数覆盖长度<br>`dotted`: 点线<br>`dash_dot`: 点划线<br>`double`: 双线描边<br>`triple`: 三线描边 | solid |
| `line_dash` | length[] / pt | 交替的实线段和间隔长度列表；裸值为 pt | — |
| `line_dash_offset` | length / pt | 虚线相位；裸值为 pt | 0 |
| `line_cap` | "butt" \| "round" \| "square" | 开放端点形状<br>`butt`: 平端帽，不向端点外延伸<br>`round`: 圆端帽，向外延伸半个线宽<br>`square`: 方端帽，向外延伸半个线宽 | butt |
| `line_join` | "miter" \| "round" \| "bevel" | 线段交点形状<br>`miter`: 延长边缘形成尖角，受尖角限制约束<br>`round`: 以圆弧连接相邻描边边缘<br>`bevel`: 以平边截去连接尖角 | miter |
| `line_miter_limit` | value | 尖角的最大延伸与线宽之比 | 4 |
| `line_opacity` | value | 线条独立透明度 0–1 | 1 |
| `offset_space` | "container" \| "target" | offset 的方向：容器坐标，或目标路径的切线与左法线<br>`container`: 偏移沿当前容器的横纵方向<br>`target`: 偏移沿目标切线与左法线方向 | container |

### 最小完整示例

```lay
# Minimal complete example: add
page=canvas(size=(100mm,75mm),background="#ffffff")
material=rect(size=(25mm,18mm),fill="#087f8c")
a=page.add(material,offset=(10mm,10mm))
page.add(material,anchor=top_left,target=a.bottom_right,offset=(3mm,3mm))
```

[概念与常见错误](topics/anchors.zh-CN.md#add) · [组合源码](../examples/gallery/positioning/anchors.lay)

## fuse

融合同容器中两个已放置矢量实例的可见轮廓，原实例退出绘制与引用。间隔连接需要正 bridge_width；images/text/groups 不支持。

返回：instance

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `points` | value | 点列表；star 为整数角数，violin 为 KDE 采样点数 | — |
| `bridge_width` | length / 画布单位 | 连接桥的物理宽度 | — |
| `junction` | value | 融合连接方式 | — |
| `radius` | length / 画布单位 | 几何半径；radial_gradient 中为归一化标量 | — |
| `fill` | paint | 形状内部的颜色、渐变、纹理或图片 | none |
| `fill_rule` | value | 自交和孔洞的填充规则 nonzero/evenodd | nonzero |
| `opacity` | number \| number[] | 对象透明度，0–1；与父级透明度相乘 | 1 |
| `border_color` | color | 封闭轮廓颜色，none 表示无边框 | none |
| `border_width` | length / pt | 封闭轮廓线宽；裸值为 pt | — |
| `border_style` | "none" \| "solid" \| "dashed" \| "dotted" \| "dash_dot" \| "double" \| "triple" | 边框线型；与填充独立<br>`none`: 不绘制此项<br>`solid`: 连续实线<br>`dashed`: 虚线；可用 dash 参数覆盖长度<br>`dotted`: 点线<br>`dash_dot`: 点划线<br>`double`: 双线描边<br>`triple`: 三线描边 | solid |
| `border_dash` | length[] / pt | 交替的边框线段和间隔长度列表；须成对 | — |
| `border_dash_offset` | length / pt | 虚线相位；裸值为 pt | 0 |
| `border_cap` | "butt" \| "round" \| "square" | 轮廓虚线端点形状<br>`butt`: 平端帽，不向端点外延伸<br>`round`: 圆端帽，向外延伸半个线宽<br>`square`: 方端帽，向外延伸半个线宽 | butt |
| `border_join` | "miter" \| "round" \| "bevel" | 轮廓交点形状<br>`miter`: 延长边缘形成尖角，受尖角限制约束<br>`round`: 以圆弧连接相邻描边边缘<br>`bevel`: 以平边截去连接尖角 | miter |
| `border_miter_limit` | value | 尖角的最大延伸与线宽之比 | 4 |
| `border_opacity` | value | 边框独立透明度 0–1 | 1 |

### 最小完整示例

```lay
# Minimal complete example: fuse
page=canvas(size=(100mm,75mm),background="#ffffff")
a=page.add(rect(size=(25mm,20mm),fill="#087f8c"),offset=(10mm,10mm))
b=page.add(rect(size=(25mm,20mm),fill="#087f8c"),offset=(30mm,10mm))
page.fuse(a,b,fill="#087f8c")
```

[概念与常见错误](topics/groups.zh-CN.md#fuse) · [组合源码](../examples/gallery/shapes/fuse-angled.lay)

## linear_gradient

创建线性渐变填充，stops 的位置在 0–1；start/end 为素材布局框中的归一化坐标。颜色 alpha 与 stop 透明度相乘。

返回：paint

必需：`stops`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `start` | value / scalar | 圆弧起始角；线性渐变中为归一化起点 | — |
| `end` | value / scalar | 圆弧终止角；线性渐变中为归一化终点 | — |
| `stops` | (number, color, number?)[] / scalar | 至少两个递增色标：(位置 0–1, 颜色[, 透明度]) | — |

### 最小完整示例

```lay
# Minimal complete example: linear_gradient
page=canvas(size=(100mm,75mm),background="#ffffff")
paint=linear_gradient(stops=[(0,"#0072b2"),(1,"#ffffff90")])
page.add(rect(size=(60mm,30mm),fill=paint),offset=(10mm,10mm))
```

[概念与常见错误](topics/fills.zh-CN.md#linear_gradient) · [组合源码](../examples/gallery/shapes/linear-gradient.lay)

## radial_gradient

创建径向渐变填充，center 和 radius 相对于布局框。可复用同一填充在不同大小的形状中，最终绘制由实例尺寸决定。

返回：paint

必需：`stops`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `center` | value / scalar | 渐变中心坐标或颜色归一化的数据中心 | — |
| `radius` | value / scalar | 几何半径；radial_gradient 中为归一化标量 | — |
| `stops` | (number, color, number?)[] / scalar | 至少两个递增色标：(位置 0–1, 颜色[, 透明度]) | — |

### 最小完整示例

```lay
# Minimal complete example: radial_gradient
page=canvas(size=(100mm,75mm),background="#ffffff")
paint=radial_gradient(stops=[(0,"#ffffff"),(1,"#087f8c")])
page.add(ellipse(size=(60mm,35mm),fill=paint),offset=(10mm,10mm))
```

[概念与常见错误](topics/fills.zh-CN.md#radial_gradient) · [组合源码](../examples/gallery/shapes/radial-gradient.lay)

## hatch

创建 slash/cross/dots 重复图案填充。spacing 和 line_width 为物理尺寸，图案限制在素材填充区域内。

返回：paint

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `pattern` | "slash" \| "cross" \| "dots" | 重复纹理类型 slash/cross/dots<br>`slash`: 斜线填充图案<br>`cross`: 交叉斜线填充图案<br>`dots`: 圆点填充图案 | slash |
| `color` | color | 文字颜色或数据系列基础配色 | — |
| `background` | paint | 对象区域的底色或填充，不是文字或轴线颜色 | none |
| `spacing` | length / 画布单位 | 纹理间距的物理长度 | 2mm |
| `line_width` | length / pt | 开放线条宽度；裸值为 pt | — |
| `angle` | angle | 纹理旋转角度 | 0deg |

### 最小完整示例

```lay
# Minimal complete example: hatch
page=canvas(size=(100mm,75mm),background="#ffffff")
page.add(rect(size=(60mm,30mm),fill=hatch(pattern=cross,color="#087f8c",spacing=3mm,line_width=0.5pt)),offset=(10mm,10mm))
```

[概念与常见错误](topics/fills.zh-CN.md#hatch) · [组合源码](../examples/unified.lay)

## image_fill

将本地图片作为形状内部填充，fit 决定适配方式。与 image 素材不同，填充受形状轮廓限制，可用于圆角矩形。

返回：paint

必需：`src`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `src` | string | 相对于定义文件的本地资源路径 | — |
| `fit` | "contain" \| "cover" \| "stretch" | contain 完整显示，cover 填满裁剪，stretch 拉伸<br>`contain`: 保持比例完整显示，容器可能留空<br>`cover`: 保持比例填满，超出容器的部分裁剪<br>`stretch`: 独立缩放宽高以填满目标尺寸 | contain |

### 最小完整示例

```lay
# Minimal complete example: image_fill
page=canvas(size=(100mm,75mm),background="#ffffff")
page.add(rect(size=(60mm,35mm),fill=image_fill(src="../assets/photo.png",fit=cover),border_radius=8mm),offset=(10mm,10mm))
```

[概念与常见错误](topics/fills.zh-CN.md#image_fill) · [组合源码](../examples/manual/paint-composition.lay)

## table

读取 CSV 或列式 JSON 表格，列名须非空且唯一，列长相同；d["列名"] 选择一列，null 或空 CSV 单元格保留缺失。

返回：table

位置参数：`src`.

必需：`src`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `src` | string | 相对于定义文件的本地资源路径 | — |

### 最小完整示例

```lay
# Minimal complete example: table
page=canvas(size=(100mm,75mm),background="#ffffff")
d=table(src="../plot/first-plot.csv")
p=plot(size=(86mm,62mm),x=axis(range=(0,4)),y=axis(range=(0,5)))
p.line(x=d["time"],y=d["signal"])
page.add(p,offset=(7mm,6mm))
```

[概念与常见错误](topics/data.zh-CN.md#table) · [组合源码](../examples/plot/first-plot.lay)

## array

从 JSON 读取非空数值向量或矩阵。矩阵须为矩形，null 表示缺失；内联数组直接使用列表而非 array(values=...)。

返回：number[] | number[][]

位置参数：`src`.

必需：`src`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `src` | string | 相对于定义文件的本地资源路径 | — |

### 最小完整示例

```lay
# Minimal complete example: array
page=canvas(size=(100mm,75mm),background="#ffffff")
x=array(src="observations.json")
p=plot(size=(86mm,62mm),x=axis(range=(0,4)),y=axis(range=(0,5)))
p.scatter(x=x,y=[1,2,4,3],marker=circle)
page.add(p,offset=(7mm,6mm))
```

[概念与常见错误](topics/data.zh-CN.md#array) · [组合源码](../examples/plot/complete-data.lay)

## plot

创建固定物理尺寸的原生图表，用图层方法添加数据。x、y 设置坐标轴，plot_area 固定绘图区；通过 add 将整个图表放入页面。

返回：可添加数据图层并放入页面的图表。

必需：`size`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `size` | (length \| auto, length \| auto) / 画布单位 | 二维物理尺寸 (宽,高)；auto 保持比例或自然排版 | — |
| `x` | axis \| number[] | 直角坐标横轴配置或图层横坐标数据 | — |
| `y` | axis \| number[] | 直角坐标纵轴配置或图层纵坐标数据 | — |
| `style` | plot_style | plot_style(...) 配置对象；仅提供未被显式参数覆盖的默认值 | — |
| `margins` | (length, length, length, length) / 画布单位 | 绘图区四侧留白 (左,上,右,下)；与 plot_area 互斥 | — |
| `plot_area` | box / 画布单位 | 固定绘图区 box(offset=(左,上), size=(宽,高)) | — |
| `frame` | "axes" \| "box" \| "none" \| boolean | 图表外框 axes/box/none；图例为布尔开关<br>`axes`: 仅绘制启用的坐标轴线<br>`box`: 绘制完整矩形图框<br>`none`: 不绘制此项 | axes |
| `projection` | "cartesian" \| "polar" \| "radar" | 直角坐标、极坐标或雷达投影<br>`cartesian`: 笛卡尔横纵坐标<br>`polar`: 角度与半径坐标<br>`radar`: 按类别角度排列的雷达坐标 | cartesian |
| `theta` | axis \| number[] | 极坐标角轴配置或图层角坐标数据 | — |
| `r` | axis \| number[] | 极坐标径向轴配置或半径数据 | — |
| `angle_unit` | "deg" \| "rad" | 极坐标数据角度单位<br>`deg`: 角度使用度<br>`rad`: 角度使用弧度 | deg |
| `theta_zero` | value | 零角方向 east/north/west/south | east |
| `theta_direction` | value | 角度递增方向 ccw/cw | ccw |
| `r_label_angle` | value | 径向刻度标签所在角度 | — |
| `inner_radius` | length / 画布单位 | 内轮廓几何半径；极坐标中为物理内孔半径 | 0 |
| `categories` | string[] | 雷达图有序类别名称 | — |
| `category_labels` | (string \| text \| formula)[] | 雷达类别显示标签，支持公式 | — |
| `ranges` | (number, number)[] | 雷达图每个类别的独立数据范围 | — |
| `radar_frame` | value | 雷达框形状 polygon/circle | polygon |
| `class` | string | 空格分隔的 LCSS 类名 | "" |
| `font_family` | string \| string[] | 系统字体族名、字体文件路径或有序列表；不内置正文字体；缺字警告并显示方框。 | system sans-serif |
| `font_size` | length / pt | 字号；省略单位时为 pt；未指定时继承 | 继承 / inherit |
| `font_weight` | integer | 字体字重，100–900 | 400 |
| `font_style` | "normal" \| "italic" | 正体或斜体<br>`normal`: 正常字形<br>`italic`: 斜体字形 | normal |
| `color` | color | 文字颜色或数据系列基础配色 | — |
| `line_height` | length / pt | 文字行高；裸值为 pt | 1.2 × font_size |
| `background` | paint | 对象区域的底色或填充，不是文字或轴线颜色 | none |
| `padding` | length \| length[] / 画布单位 | 内部留白；一个值或上、右、下、左四个值 | 0 |
| `border_radius` | length / 画布单位 | 底框圆角的物理半径 | 0 |
| `border_color` | color | 封闭轮廓颜色，none 表示无边框 | none |
| `border_width` | length / pt | 封闭轮廓线宽；裸值为 pt | — |
| `border_style` | "none" \| "solid" \| "dashed" \| "dotted" \| "dash_dot" \| "double" \| "triple" | 边框线型；与填充独立<br>`none`: 不绘制此项<br>`solid`: 连续实线<br>`dashed`: 虚线；可用 dash 参数覆盖长度<br>`dotted`: 点线<br>`dash_dot`: 点划线<br>`double`: 双线描边<br>`triple`: 三线描边 | solid |
| `border_dash` | length[] / pt | 交替的边框线段和间隔长度列表；须成对 | — |
| `border_dash_offset` | length / pt | 虚线相位；裸值为 pt | 0 |
| `border_cap` | "butt" \| "round" \| "square" | 轮廓虚线端点形状<br>`butt`: 平端帽，不向端点外延伸<br>`round`: 圆端帽，向外延伸半个线宽<br>`square`: 方端帽，向外延伸半个线宽 | butt |
| `border_join` | "miter" \| "round" \| "bevel" | 轮廓交点形状<br>`miter`: 延长边缘形成尖角，受尖角限制约束<br>`round`: 以圆弧连接相邻描边边缘<br>`bevel`: 以平边截去连接尖角 | miter |
| `border_miter_limit` | value | 尖角的最大延伸与线宽之比 | 4 |
| `border_opacity` | value | 边框独立透明度 0–1 | 1 |

### 最小完整示例

```lay
# Minimal complete example: plot
page=canvas(size=(100mm,75mm),background="#ffffff")
p=plot(size=(86mm,62mm),x=axis(range=(0,4)),y=axis(range=(0,5)))
p.line(x=[0,1,2,3],y=[1,2,4,3],label="Signal")
page.add(p,offset=(7mm,6mm))
```

[概念与常见错误](topics/plot-area.zh-CN.md#plot) · [组合源码](../examples/plot/publication.lay)

## axis

配置数据范围、linear/log/symlog 映射、刻度、标签和断轴。数据范围采用原始数值，文字与线宽采用物理单位。

返回：axis

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `font_family` | string \| string[] | 系统字体族名、字体文件路径或有序列表；不内置正文字体；缺字警告并显示方框。 | system sans-serif |
| `font_size` | length / pt | 字号；省略单位时为 pt；未指定时继承 | 继承 / inherit |
| `font_weight` | integer | 字体字重，100–900 | 400 |
| `font_style` | "normal" \| "italic" | 正体或斜体<br>`normal`: 正常字形<br>`italic`: 斜体字形 | normal |
| `color` | color | 文字颜色或数据系列基础配色 | — |
| `line_height` | length / pt | 文字行高；裸值为 pt | 1.2 × font_size |
| `notation` | "plain" \| "scientific" \| "offset" | 数字显示 plain/scientific/offset，不改变数据坐标<br>`plain`: 直接显示刻度数字<br>`scientific`: 使用科学计数法显示刻度<br>`offset`: 使用公共偏移量显示刻度 | plain |
| `format` | value | D3 数字格式，例如 .2f 或 .2e | — |
| `exponent` | value | offset 记数法的十进制指数；整数 | — |
| `exponent_offset` | length / 画布单位 | 倍率公式相对默认位置的物理偏移 | (0,0) |
| `label` | string \| text \| formula | 标签内容；支持普通文字、混合公式或文字素材 | — |
| `label_offset` | length / 画布单位 | 轴或色标标题相对默认位置的物理偏移 | (0,0) |
| `scale` | "linear" \| "log" \| "symlog" | 坐标变换 linear/log/symlog，或共享色标对象<br>`linear`: 线性映射<br>`log`: 对数映射；数值必须为正<br>`symlog`: 零附近线性、远离零时对数的映射 | linear |
| `range` | (number, number) | 坐标轴的显式数据范围 (最小, 最大) | — |
| `ticks` | number[] | 递增且无重复的主刻度数据值 | — |
| `minor_ticks` | number[] \| auto | 次刻度数据值，或 auto 自动生成 | — |
| `tick_direction` | "in" \| "out" \| "inout" | 刻度指向内、外或两侧<br>`in`: 刻度向绘图区内侧<br>`out`: 刻度向绘图区外侧<br>`inout`: 刻度同时向内向外 | out |
| `tick_length` | length / 画布单位 | 主刻度线的物理长度 | 1.2mm |
| `minor_tick_length` | length / 画布单位 | 次刻度线的物理长度 | 0.6mm |
| `tick_labels` | boolean | 是否显示刻度标签 | true |
| `tick_rotation` | value | 刻度文字的旋转角 | 0deg |
| `tick_offset` | length / 画布单位 | 刻度文字相对刻度的物理偏移 | (0,0) |
| `tick_font_size` | length / pt | 刻度字号；裸值为 pt | 继承字号 / inherit font_size |
| `tick_color` | color | 刻度线与文字的颜色 | — |
| `grid` | "none" \| "major" \| "both" | 显示主刻度网格、所有网格或关闭<br>`none`: 不绘制网格<br>`major`: 仅主要刻度网格<br>`both`: 主次刻度网格 | none |
| `reverse` | boolean | 反转坐标轴的映射方向 | false |
| `constant` | value | symlog 线性区域的正阈值 | 1 |
| `breaks` | value | 轴数据范围内须排除的有序区间 | — |
| `break_gap` | length / 画布单位 | 断口的物理长度；标量或每断口一个值 | 2mm |
| `segment_lengths` | value | 各可见段的固定物理长度 | — |
| `spine` | boolean | 是否显示轴线 | true |
| `class` | string | 空格分隔的 LCSS 类名 | "" |
| `line_color` | color | 开放线条颜色；默认采用数据系列颜色或主题颜色 | — |
| `line_width` | length / pt | 开放线条宽度；裸值为 pt | — |
| `line_style` | "none" \| "solid" \| "dashed" \| "dotted" \| "dash_dot" \| "double" \| "triple" | 线型；double/triple 保留透明间隔<br>`none`: 不绘制此项<br>`solid`: 连续实线<br>`dashed`: 虚线；可用 dash 参数覆盖长度<br>`dotted`: 点线<br>`dash_dot`: 点划线<br>`double`: 双线描边<br>`triple`: 三线描边 | solid |
| `line_dash` | length[] / pt | 交替的实线段和间隔长度列表；裸值为 pt | — |
| `line_dash_offset` | length / pt | 虚线相位；裸值为 pt | 0 |
| `line_cap` | "butt" \| "round" \| "square" | 开放端点形状<br>`butt`: 平端帽，不向端点外延伸<br>`round`: 圆端帽，向外延伸半个线宽<br>`square`: 方端帽，向外延伸半个线宽 | butt |
| `line_join` | "miter" \| "round" \| "bevel" | 线段交点形状<br>`miter`: 延长边缘形成尖角，受尖角限制约束<br>`round`: 以圆弧连接相邻描边边缘<br>`bevel`: 以平边截去连接尖角 | miter |
| `line_miter_limit` | value | 尖角的最大延伸与线宽之比 | 4 |
| `line_opacity` | value | 线条独立透明度 0–1 | 1 |
| `background` | paint | 对象区域的底色或填充，不是文字或轴线颜色 | none |
| `padding` | length \| length[] / 画布单位 | 内部留白；一个值或上、右、下、左四个值 | 0 |
| `border_radius` | length / 画布单位 | 底框圆角的物理半径 | 0 |
| `border_color` | color | 封闭轮廓颜色，none 表示无边框 | none |
| `border_width` | length / pt | 封闭轮廓线宽；裸值为 pt | — |
| `border_style` | "none" \| "solid" \| "dashed" \| "dotted" \| "dash_dot" \| "double" \| "triple" | 边框线型；与填充独立<br>`none`: 不绘制此项<br>`solid`: 连续实线<br>`dashed`: 虚线；可用 dash 参数覆盖长度<br>`dotted`: 点线<br>`dash_dot`: 点划线<br>`double`: 双线描边<br>`triple`: 三线描边 | solid |
| `border_dash` | length[] / pt | 交替的边框线段和间隔长度列表；须成对 | — |
| `border_dash_offset` | length / pt | 虚线相位；裸值为 pt | 0 |
| `border_cap` | "butt" \| "round" \| "square" | 轮廓虚线端点形状<br>`butt`: 平端帽，不向端点外延伸<br>`round`: 圆端帽，向外延伸半个线宽<br>`square`: 方端帽，向外延伸半个线宽 | butt |
| `border_join` | "miter" \| "round" \| "bevel" | 轮廓交点形状<br>`miter`: 延长边缘形成尖角，受尖角限制约束<br>`round`: 以圆弧连接相邻描边边缘<br>`bevel`: 以平边截去连接尖角 | miter |
| `border_miter_limit` | value | 尖角的最大延伸与线宽之比 | 4 |
| `border_opacity` | value | 边框独立透明度 0–1 | 1 |
| `tick_text` | (string \| text \| formula)[] | 与 ticks 等长的文字或公式标签列表 | — |
| `break_mark_size` | value | 断轴斜线记号的物理长度 | — |

### 最小完整示例

```lay
# Minimal complete example: axis
page=canvas(size=(100mm,75mm),background="#ffffff")
p=plot(size=(86mm,62mm),x=axis(range=(0,4),ticks=[0,2,4],label="Time"),y=axis(range=(0,5),grid=major))
p.line(x=[0,1,2,3],y=[1,2,4,3])
page.add(p,offset=(7mm,6mm))
```

[概念与常见错误](topics/axes.zh-CN.md#axis) · [组合源码](../examples/plot/multi-axes-breaks.lay)

## plot_style

定义可共享的图表、图层与装饰默认样式。显式调用参数优先于此配置；它不是 inline/display 字符串选项。

返回：plot_style

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `font_family` | string \| string[] | 系统字体族名、字体文件路径或有序列表；不内置正文字体；缺字警告并显示方框。 | system sans-serif |
| `font_size` | length / pt | 字号；省略单位时为 pt；未指定时继承 | 8pt |
| `font_weight` | integer | 字体字重，100–900 | 400 |
| `font_style` | "normal" \| "italic" | 正体或斜体<br>`normal`: 正常字形<br>`italic`: 斜体字形 | normal |
| `color` | color | 文字颜色或数据系列基础配色 | #222222 |
| `line_height` | length / pt | 文字行高；裸值为 pt | 1.2 × font_size |
| `tick_font_size` | length / pt | 刻度字号；裸值为 pt | 继承字号 / inherit font_size |
| `label_font_size` | length / pt | 轴标题字号；裸值为 pt | 继承字号 / inherit font_size |
| `line_color` | color | 开放线条颜色；默认采用数据系列颜色或主题颜色 | — |
| `line_width` | length / pt | 开放线条宽度；裸值为 pt | 0.6pt |
| `line_style` | "none" \| "solid" \| "dashed" \| "dotted" \| "dash_dot" \| "double" \| "triple" | 线型；double/triple 保留透明间隔<br>`none`: 不绘制此项<br>`solid`: 连续实线<br>`dashed`: 虚线；可用 dash 参数覆盖长度<br>`dotted`: 点线<br>`dash_dot`: 点划线<br>`double`: 双线描边<br>`triple`: 三线描边 | solid |
| `line_dash` | length[] / pt | 交替的实线段和间隔长度列表；裸值为 pt | — |
| `line_dash_offset` | length / pt | 虚线相位；裸值为 pt | 0 |
| `line_cap` | "butt" \| "round" \| "square" | 开放端点形状<br>`butt`: 平端帽，不向端点外延伸<br>`round`: 圆端帽，向外延伸半个线宽<br>`square`: 方端帽，向外延伸半个线宽 | butt |
| `line_join` | "miter" \| "round" \| "bevel" | 线段交点形状<br>`miter`: 延长边缘形成尖角，受尖角限制约束<br>`round`: 以圆弧连接相邻描边边缘<br>`bevel`: 以平边截去连接尖角 | miter |
| `line_miter_limit` | value | 尖角的最大延伸与线宽之比 | 4 |
| `line_opacity` | value | 线条独立透明度 0–1 | 1 |
| `colors` | color[] | 非空循环配色列表，可用 palette(...) 生成 | — |
| `grid_color` | color | 网格线颜色 | #dddddd |
| `grid_line_width` | length / pt | 网格线宽；裸值为 pt | 0.3pt |
| `grid_line_dash` | value | 网格线虚线长度列表；裸值为 pt | [] |

### 最小完整示例

```lay
# Minimal complete example: plot_style
page=canvas(size=(100mm,75mm),background="#ffffff")
s=plot_style(font_size=9pt,line_width=1pt)
p=plot(size=(86mm,62mm),style=s)
p.line(x=[0,1,2,3],y=[1,2,4,3])
page.add(p,offset=(7mm,6mm))
```

[概念与常见错误](topics/plot-area.zh-CN.md#plot_style) · [组合源码](../examples/plot/publication.lay)

## color_scale

定义数值到颜色的共享映射。连续 norm 需要明确范围，boundary 使用分段边界；各图层和独立色标引用同一配置。

返回：color_scale

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `norm` | "linear" \| "log" \| "symlog" \| "centered" \| "boundary" | 颜色归一化 linear/log/symlog/centered/boundary<br>`linear`: 线性映射<br>`log`: 对数映射；数值必须为正<br>`symlog`: 零附近线性、远离零时对数的映射<br>`centered`: 以 center 为分界的连续颜色归一化<br>`boundary`: 按 boundaries 分段的离散颜色归一化 | linear |
| `vmin` | value | 颜色映射最小数据值 | — |
| `vmax` | value | 颜色映射最大数据值 | — |
| `center` | value | 渐变中心坐标或颜色归一化的数据中心 | — |
| `cmap` | string \| color[] \| cmap | 预设名称、颜色列表或 cmap(...) 配色对象 | viridis |
| `constant` | value | symlog 线性区域的正阈值 | 1 |
| `boundaries` | value | boundary 颜色映射的递增区间边界 | — |
| `under` | value | 低于色标下限的数据颜色 | — |
| `over` | value | 高于色标上限的数据颜色 | — |

### 最小完整示例

```lay
# Minimal complete example: color_scale
page=canvas(size=(100mm,75mm),background="#ffffff")
s=color_scale(norm=linear,vmin=0,vmax=5,cmap=["#ffffff90","#087f8c"])
p=plot(size=(86mm,62mm),x=axis(range=(0,4)),y=axis(range=(0,5)))
p.scatter(x=[0,1,2,3],y=[1,2,4,3],c=[0,2,4,5],color_scale=s)
page.add(p,offset=(7mm,6mm))
```

[概念与常见错误](topics/legends.zh-CN.md#color_scale) · [组合源码](../examples/plot/shared-colors.lay)

## legend

由已有图层列表创建可独立放置的图例素材。label 决定文字，样例来自图层样式；用 add 在页面或组中定位。

返回：material

必需：`layers`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `layers` | plot-layer[] | 需要共享图例的已创建图层列表 | — |
| `style` | plot_style | plot_style(...) 默认样式对象；显式参数优先 | — |
| `position` | "top_left" \| "top_center" \| "top_right" \| "middle_left" \| "center" \| "middle_right" \| "bottom_left" \| "bottom_center" \| "bottom_right" \| (length,length) | 图例位置、统计组数据位置或色标位置；由所属对象决定<br>`top_left`: 左上角，对应布局框<br>`top_center`: 上边中点，对应布局框<br>`top_right`: 右上角，对应布局框<br>`middle_left`: 左边中点，对应布局框<br>`center`: 中心对齐<br>`middle_right`: 右边中点，对应布局框<br>`bottom_left`: 左下角，对应布局框<br>`bottom_center`: 下边中点，对应布局框<br>`bottom_right`: 右下角，对应布局框 | top_right |
| `columns` | integer | 图例列数，正整数，按行排列 | 1 |
| `font_size` | length / pt | 字号；省略单位时为 pt；未指定时继承 | 继承 / inherit |
| `background` | paint | 对象区域的底色或填充，不是文字或轴线颜色 | #ffffff |
| `frame` | enum \| boolean | 图表外框 axes/box/none；图例为布尔开关 | false |
| `title` | string \| text \| formula | 图例标题，支持文字或公式 | — |
| `sample_width` | length / 画布单位 | 图例符号的物理宽度 | 6mm |
| `gap` | length / 画布单位 | 图例条目间距或色标与绘图区的物理间隔 | — |
| `padding` | length \| length[] / 画布单位 | 内部留白；一个值或上、右、下、左四个值 | 1.2mm |
| `sample_gap` | length / 画布单位 | 图例符号和文字的物理间隔 | — |
| `class` | string | 空格分隔的 LCSS 类名 | "" |
| `border_color` | color | 封闭轮廓颜色，none 表示无边框 | none |
| `border_width` | length / pt | 封闭轮廓线宽；裸值为 pt | — |
| `border_style` | "none" \| "solid" \| "dashed" \| "dotted" \| "dash_dot" \| "double" \| "triple" | 边框线型；与填充独立<br>`none`: 不绘制此项<br>`solid`: 连续实线<br>`dashed`: 虚线；可用 dash 参数覆盖长度<br>`dotted`: 点线<br>`dash_dot`: 点划线<br>`double`: 双线描边<br>`triple`: 三线描边 | solid |
| `border_dash` | length[] / pt | 交替的边框线段和间隔长度列表；须成对 | — |
| `border_dash_offset` | length / pt | 虚线相位；裸值为 pt | 0 |
| `border_cap` | "butt" \| "round" \| "square" | 轮廓虚线端点形状<br>`butt`: 平端帽，不向端点外延伸<br>`round`: 圆端帽，向外延伸半个线宽<br>`square`: 方端帽，向外延伸半个线宽 | butt |
| `border_join` | "miter" \| "round" \| "bevel" | 轮廓交点形状<br>`miter`: 延长边缘形成尖角，受尖角限制约束<br>`round`: 以圆弧连接相邻描边边缘<br>`bevel`: 以平边截去连接尖角 | miter |
| `border_miter_limit` | value | 尖角的最大延伸与线宽之比 | 4 |
| `border_opacity` | value | 边框独立透明度 0–1 | 1 |

### 最小完整示例

```lay
# Minimal complete example: legend
page=canvas(size=(100mm,75mm),background="#ffffff")
p=plot(size=(86mm,62mm),x=axis(range=(0,4)),y=axis(range=(0,5)))
layer=p.line(x=[0,1,2,3],y=[1,2,4,3],label="Signal")
page.add(p,offset=(7mm,6mm))
page.add(legend(layers=[layer]),offset=(10mm,5mm))
```

[概念与常见错误](topics/legends.zh-CN.md#legend) · [组合源码](../examples/plot/complete-data.lay)

## colorbar

由 color_scale 创建独立色标素材。length/thickness 是物理尺寸，刻度数值与配色映射一致；用 add 定位。

返回：material

必需：`scale`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `scale` | "linear" \| "log" \| "symlog" | 坐标变换 linear/log/symlog，或共享色标对象<br>`linear`: 线性映射<br>`log`: 对数映射；数值必须为正<br>`symlog`: 零附近线性、远离零时对数的映射 | — |
| `style` | plot_style | plot_style(...) 默认样式对象；显式参数优先 | — |
| `notation` | "plain" \| "scientific" \| "offset" | 数字显示 plain/scientific/offset，不改变数据坐标<br>`plain`: 直接显示刻度数字<br>`scientific`: 使用科学计数法显示刻度<br>`offset`: 使用公共偏移量显示刻度 | plain |
| `format` | value | D3 数字格式，例如 .2f 或 .2e | — |
| `exponent` | value | offset 记数法的十进制指数；整数 | — |
| `exponent_offset` | length / 画布单位 | 倍率公式相对默认位置的物理偏移 | (0,0) |
| `label` | string \| text \| formula | 标签内容；支持普通文字、混合公式或文字素材 | — |
| `label_offset` | length / 画布单位 | 轴或色标标题相对默认位置的物理偏移 | (0,0) |
| `position` | "left" \| "right" \| "top" \| "bottom" \| (length,length) | 图例位置、统计组数据位置或色标位置；由所属对象决定<br>`left`: 左侧或左对齐，依参数用途<br>`right`: 右侧或右对齐，依参数用途<br>`top`: 上侧<br>`bottom`: 下侧 | — |
| `orientation` | "vertical" \| "horizontal" | vertical/horizontal 方向<br>`vertical`: 纵向<br>`horizontal`: 横向 | vertical |
| `length` | length / 画布单位 | 色条的物理长度 | — |
| `thickness` | length / 画布单位 | 色条的物理厚度 | 3mm |
| `gap` | length / 画布单位 | 图例条目间距或色标与绘图区的物理间隔 | — |
| `ticks` | number[] | 递增且无重复的主刻度数据值 | — |
| `class` | string | 空格分隔的 LCSS 类名 | "" |
| `background` | paint | 对象区域的底色或填充，不是文字或轴线颜色 | none |
| `padding` | length \| length[] / 画布单位 | 内部留白；一个值或上、右、下、左四个值 | 0 |
| `border_radius` | length / 画布单位 | 底框圆角的物理半径 | 0 |
| `border_color` | color | 封闭轮廓颜色，none 表示无边框 | none |
| `border_width` | length / pt | 封闭轮廓线宽；裸值为 pt | — |
| `border_style` | "none" \| "solid" \| "dashed" \| "dotted" \| "dash_dot" \| "double" \| "triple" | 边框线型；与填充独立<br>`none`: 不绘制此项<br>`solid`: 连续实线<br>`dashed`: 虚线；可用 dash 参数覆盖长度<br>`dotted`: 点线<br>`dash_dot`: 点划线<br>`double`: 双线描边<br>`triple`: 三线描边 | solid |
| `border_dash` | length[] / pt | 交替的边框线段和间隔长度列表；须成对 | — |
| `border_dash_offset` | length / pt | 虚线相位；裸值为 pt | 0 |
| `border_cap` | "butt" \| "round" \| "square" | 轮廓虚线端点形状<br>`butt`: 平端帽，不向端点外延伸<br>`round`: 圆端帽，向外延伸半个线宽<br>`square`: 方端帽，向外延伸半个线宽 | butt |
| `border_join` | "miter" \| "round" \| "bevel" | 轮廓交点形状<br>`miter`: 延长边缘形成尖角，受尖角限制约束<br>`round`: 以圆弧连接相邻描边边缘<br>`bevel`: 以平边截去连接尖角 | miter |
| `border_miter_limit` | value | 尖角的最大延伸与线宽之比 | 4 |
| `border_opacity` | value | 边框独立透明度 0–1 | 1 |

### 最小完整示例

```lay
# Minimal complete example: colorbar
page=canvas(size=(100mm,75mm),background="#ffffff")
s=color_scale(vmin=0,vmax=5,cmap="viridis")
page.add(colorbar(scale=s,length=45mm,label="Value"),offset=(20mm,12mm))
```

[概念与常见错误](topics/legends.zh-CN.md#colorbar) · [组合源码](../examples/plot/shared-colors.lay)

## plot.add_axis

给尚未放置图表添加唯一命名的轴。side 决定方向与位置，图层用 x_axis/y_axis 选择此轴；同侧间距需显式 offset。

返回：axis

必需：`name`, `side`, `axis`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `name` | string | 画布标题或命名坐标轴的唯一名称 | — |
| `side` | "left" \| "right" \| "top" \| "bottom" | 命名轴放置于 left/right/top/bottom<br>`left`: 左侧或左对齐，依参数用途<br>`right`: 右侧或右对齐，依参数用途<br>`top`: 上侧<br>`bottom`: 下侧 | — |
| `offset` | (length, length) / 画布单位 | 相对目标的 (横向, 纵向) 偏移 | (0, 0) |
| `axis` | value | axis(...) 坐标轴配置 | — |

### 最小完整示例

```lay
# Minimal complete example: plot.add_axis
page=canvas(size=(100mm,75mm),background="#ffffff")
p=plot(size=(86mm,62mm),x=axis(range=(0,4)),y=axis(range=(0,5)))
p.add_axis(name="right_y",side=right,axis=axis(range=(0,10),label="Second scale"))
p.line(x=[0,1,2,3],y=[1,2,4,3],y_axis="right_y")
page.add(p,offset=(7mm,6mm))
```

[概念与常见错误](topics/multi-axes.zh-CN.md#plot-add_axis) · [组合源码](../examples/plot/multi-axes-breaks.lay)

## plot.line

向图表添加折线图层，依次连接数据点；可同时显示标记。使用 x_axis、y_axis 选择命名坐标轴；极坐标使用 theta、r，雷达图使用 values。

返回：图层句柄，可用于共享图例。

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `x` | axis \| number[] | 直角坐标横轴配置或图层横坐标数据 | — |
| `y` | axis \| number[] | 直角坐标纵轴配置或图层纵坐标数据 | — |
| `x_axis` | value | 绑定的横轴名称 | x |
| `y_axis` | value | 绑定的纵轴名称 | y |
| `color` | color | 文字颜色或数据系列基础配色 | — |
| `opacity` | number \| number[] | 对象透明度，0–1；与父级透明度相乘 | 1 |
| `label` | string \| text \| formula | 标签内容；支持普通文字、混合公式或文字素材 | — |
| `class` | string | 空格分隔的 LCSS 类名 | "" |
| `line_color` | color | 开放线条颜色；默认采用数据系列颜色或主题颜色 | — |
| `line_width` | length / pt | 开放线条宽度；裸值为 pt | — |
| `line_style` | "none" \| "solid" \| "dashed" \| "dotted" \| "dash_dot" \| "double" \| "triple" | 线型；double/triple 保留透明间隔<br>`none`: 不绘制此项<br>`solid`: 连续实线<br>`dashed`: 虚线；可用 dash 参数覆盖长度<br>`dotted`: 点线<br>`dash_dot`: 点划线<br>`double`: 双线描边<br>`triple`: 三线描边 | solid |
| `line_dash` | length[] / pt | 交替的实线段和间隔长度列表；裸值为 pt | — |
| `line_dash_offset` | length / pt | 虚线相位；裸值为 pt | 0 |
| `line_cap` | "butt" \| "round" \| "square" | 开放端点形状<br>`butt`: 平端帽，不向端点外延伸<br>`round`: 圆端帽，向外延伸半个线宽<br>`square`: 方端帽，向外延伸半个线宽 | butt |
| `line_join` | "miter" \| "round" \| "bevel" | 线段交点形状<br>`miter`: 延长边缘形成尖角，受尖角限制约束<br>`round`: 以圆弧连接相邻描边边缘<br>`bevel`: 以平边截去连接尖角 | miter |
| `line_miter_limit` | value | 尖角的最大延伸与线宽之比 | 4 |
| `line_opacity` | value | 线条独立透明度 0–1 | 1 |
| `marker` | "none" \| "circle" \| "square" \| "triangle" \| "triangle_down" \| "diamond" | 散点或折线标记形状<br>`none`: 无标记<br>`circle`: 圆形标记<br>`square`: 方形标记<br>`triangle`: 上三角标记<br>`triangle_down`: 下三角标记<br>`diamond`: 菱形标记 | circle |
| `marker_size` | length / 画布单位 | 包含边框的标记外框物理尺寸 | — |
| `marker_fill` | value | 标记内部颜色或 none | — |
| `marker_border_color` | color | 标记边框颜色或 none | none |
| `marker_border_width` | length / pt | 标记边框宽度；裸值为 pt | — |
| `fill` | paint | 形状内部的颜色、渐变、纹理或图片 | none |
| `closed` | boolean | 连接首尾形成闭合曲线 | false |
| `interpolation` | "polar" \| "chord" | 极坐标线段沿 polar 插值或 chord 直连<br>`polar`: 在极坐标数据中插值，再投影到页面<br>`chord`: 在页面中用直线弦连接投影点 | polar |
| `wrap` | "shortest" \| "raw" | 角度跨越边界时按 shortest 或 raw 连接<br>`shortest`: 选择相邻角度之间的最短跨周期路线<br>`raw`: 保留原始角度差，可跨越多个周期 | shortest |
| `zero_baseline` | value | 对数轴上基线位于可见范围边缘 | — |
| `theta` | axis \| number[] | 极坐标角轴配置或图层角坐标数据 | — |
| `r` | axis \| number[] | 极坐标径向轴配置或半径数据 | — |
| `values` | number[] | 统计计算的观测值或柱高数据 | — |
| `theta_edges` | value | 极坐标单元角边界 | — |
| `r_edges` | value | 极坐标单元径向边界 | — |
| `thetaerr` | value | 角方向误差 | — |
| `rerr` | value | 径向误差 | — |

### 最小完整示例

```lay
# Minimal complete example: plot.line
page=canvas(size=(100mm,75mm),background="#ffffff")
p=plot(size=(86mm,62mm),x=axis(range=(0,4)),y=axis(range=(0,5)))
p.line(x=[0,1,2,3],y=[1,2,4,3],marker=circle)
page.add(p,offset=(7mm,6mm))
```

[概念与常见错误](topics/line-scatter.zh-CN.md#plot-line) · [组合源码](../examples/plot/line-scatter.lay)

## plot.scatter

向图表添加散点图层；可设置标记形状、物理尺寸及边框，或使用 c 与 color_scale 按数值映射颜色。

返回：图层句柄，可用于共享图例。

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `x` | axis \| number[] | 直角坐标横轴配置或图层横坐标数据 | — |
| `y` | axis \| number[] | 直角坐标纵轴配置或图层纵坐标数据 | — |
| `x_axis` | value | 绑定的横轴名称 | x |
| `y_axis` | value | 绑定的纵轴名称 | y |
| `color` | color | 文字颜色或数据系列基础配色 | — |
| `opacity` | number \| number[] | 对象透明度，0–1；与父级透明度相乘 | 1 |
| `label` | string \| text \| formula | 标签内容；支持普通文字、混合公式或文字素材 | — |
| `class` | string | 空格分隔的 LCSS 类名 | "" |
| `marker` | "none" \| "circle" \| "square" \| "triangle" \| "triangle_down" \| "diamond" | 散点或折线标记形状<br>`none`: 无标记<br>`circle`: 圆形标记<br>`square`: 方形标记<br>`triangle`: 上三角标记<br>`triangle_down`: 下三角标记<br>`diamond`: 菱形标记 | circle |
| `marker_size` | length / 画布单位 | 包含边框的标记外框物理尺寸 | — |
| `marker_fill` | value | 标记内部颜色或 none | — |
| `marker_border_color` | color | 标记边框颜色或 none | none |
| `marker_border_width` | length / pt | 标记边框宽度；裸值为 pt | — |
| `c` | value | 散点颜色的数值数据 | — |
| `color_scale` | color | 共享 color_scale(...) 对象 | — |
| `theta` | axis \| number[] | 极坐标角轴配置或图层角坐标数据 | — |
| `r` | axis \| number[] | 极坐标径向轴配置或半径数据 | — |
| `values` | number[] | 统计计算的观测值或柱高数据 | — |
| `theta_edges` | value | 极坐标单元角边界 | — |
| `r_edges` | value | 极坐标单元径向边界 | — |
| `thetaerr` | value | 角方向误差 | — |
| `rerr` | value | 径向误差 | — |
| `wrap` | "shortest" \| "raw" | 角度跨越边界时按 shortest 或 raw 连接<br>`shortest`: 选择相邻角度之间的最短跨周期路线<br>`raw`: 保留原始角度差，可跨越多个周期 | shortest |
| `interpolation` | "polar" \| "chord" | 极坐标线段沿 polar 插值或 chord 直连<br>`polar`: 在极坐标数据中插值，再投影到页面<br>`chord`: 在页面中用直线弦连接投影点 | polar |
| `closed` | boolean | 连接首尾形成闭合曲线 | false |

### 最小完整示例

```lay
# Minimal complete example: plot.scatter
page=canvas(size=(100mm,75mm),background="#ffffff")
p=plot(size=(86mm,62mm),x=axis(range=(0,4)),y=axis(range=(0,5)))
p.scatter(x=[0,1,2,3],y=[1,2,4,3],marker=diamond)
page.add(p,offset=(7mm,6mm))
```

[概念与常见错误](topics/line-scatter.zh-CN.md#plot-scatter) · [组合源码](../examples/plot/line-scatter.lay)

## plot.errorbar

向图表添加误差棒。xerr、yerr 支持对称误差或分别提供上下界误差；cap_size 设置端帽的物理长度。

返回：图层句柄，可用于共享图例。

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `x` | axis \| number[] | 直角坐标横轴配置或图层横坐标数据 | — |
| `y` | axis \| number[] | 直角坐标纵轴配置或图层纵坐标数据 | — |
| `x_axis` | value | 绑定的横轴名称 | x |
| `y_axis` | value | 绑定的纵轴名称 | y |
| `color` | color | 文字颜色或数据系列基础配色 | — |
| `opacity` | number \| number[] | 对象透明度，0–1；与父级透明度相乘 | 1 |
| `label` | string \| text \| formula | 标签内容；支持普通文字、混合公式或文字素材 | — |
| `class` | string | 空格分隔的 LCSS 类名 | "" |
| `line_color` | color | 开放线条颜色；默认采用数据系列颜色或主题颜色 | — |
| `line_width` | length / pt | 开放线条宽度；裸值为 pt | — |
| `line_style` | "none" \| "solid" \| "dashed" \| "dotted" \| "dash_dot" \| "double" \| "triple" | 线型；double/triple 保留透明间隔<br>`none`: 不绘制此项<br>`solid`: 连续实线<br>`dashed`: 虚线；可用 dash 参数覆盖长度<br>`dotted`: 点线<br>`dash_dot`: 点划线<br>`double`: 双线描边<br>`triple`: 三线描边 | solid |
| `line_dash` | length[] / pt | 交替的实线段和间隔长度列表；裸值为 pt | — |
| `line_dash_offset` | length / pt | 虚线相位；裸值为 pt | 0 |
| `line_cap` | "butt" \| "round" \| "square" | 开放端点形状<br>`butt`: 平端帽，不向端点外延伸<br>`round`: 圆端帽，向外延伸半个线宽<br>`square`: 方端帽，向外延伸半个线宽 | butt |
| `line_join` | "miter" \| "round" \| "bevel" | 线段交点形状<br>`miter`: 延长边缘形成尖角，受尖角限制约束<br>`round`: 以圆弧连接相邻描边边缘<br>`bevel`: 以平边截去连接尖角 | miter |
| `line_miter_limit` | value | 尖角的最大延伸与线宽之比 | 4 |
| `line_opacity` | value | 线条独立透明度 0–1 | 1 |
| `marker` | "none" \| "circle" \| "square" \| "triangle" \| "triangle_down" \| "diamond" | 散点或折线标记形状<br>`none`: 无标记<br>`circle`: 圆形标记<br>`square`: 方形标记<br>`triangle`: 上三角标记<br>`triangle_down`: 下三角标记<br>`diamond`: 菱形标记 | circle |
| `marker_size` | length / 画布单位 | 包含边框的标记外框物理尺寸 | — |
| `marker_fill` | value | 标记内部颜色或 none | — |
| `marker_border_color` | color | 标记边框颜色或 none | none |
| `marker_border_width` | length / pt | 标记边框宽度；裸值为 pt | — |
| `yerr` | number \| number[] \| (number[], number[]) | 纵向误差；标量、对称数组或上下误差 | — |
| `xerr` | number \| number[] \| (number[], number[]) | 横向误差；标量、对称数组或左右误差 | — |
| `cap_size` | length / 画布单位 | 误差棒端帽的物理长度 | — |
| `theta` | axis \| number[] | 极坐标角轴配置或图层角坐标数据 | — |
| `r` | axis \| number[] | 极坐标径向轴配置或半径数据 | — |
| `values` | number[] | 统计计算的观测值或柱高数据 | — |
| `theta_edges` | value | 极坐标单元角边界 | — |
| `r_edges` | value | 极坐标单元径向边界 | — |
| `thetaerr` | value | 角方向误差 | — |
| `rerr` | value | 径向误差 | — |
| `wrap` | "shortest" \| "raw" | 角度跨越边界时按 shortest 或 raw 连接<br>`shortest`: 选择相邻角度之间的最短跨周期路线<br>`raw`: 保留原始角度差，可跨越多个周期 | shortest |
| `interpolation` | "polar" \| "chord" | 极坐标线段沿 polar 插值或 chord 直连<br>`polar`: 在极坐标数据中插值，再投影到页面<br>`chord`: 在页面中用直线弦连接投影点 | polar |
| `closed` | boolean | 连接首尾形成闭合曲线 | false |

### 最小完整示例

```lay
# Minimal complete example: plot.errorbar
page=canvas(size=(100mm,75mm),background="#ffffff")
p=plot(size=(86mm,62mm),x=axis(range=(0,4)),y=axis(range=(0,5)))
p.errorbar(x=[1,2,3],y=[1,3,2],yerr=[0.2,0.4,0.3])
page.add(p,offset=(7mm,6mm))
```

[概念与常见错误](topics/uncertainty.zh-CN.md#plot-errorbar) · [组合源码](../examples/plot/scientific.lay)

## plot.band

填充带图层：向当前图表添加数据，通过物理样式参数控制外观，并使用命名轴参数选择数据映射。

返回：图层句柄，可用于共享图例。

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `x` | axis \| number[] | 直角坐标横轴配置或图层横坐标数据 | — |
| `y` | axis \| number[] | 直角坐标纵轴配置或图层纵坐标数据 | — |
| `x_axis` | value | 绑定的横轴名称 | x |
| `y_axis` | value | 绑定的纵轴名称 | y |
| `color` | color | 文字颜色或数据系列基础配色 | — |
| `opacity` | number \| number[] | 对象透明度，0–1；与父级透明度相乘 | 1 |
| `label` | string \| text \| formula | 标签内容；支持普通文字、混合公式或文字素材 | — |
| `class` | string | 空格分隔的 LCSS 类名 | "" |
| `border_color` | color | 封闭轮廓颜色，none 表示无边框 | none |
| `border_width` | length / pt | 封闭轮廓线宽；裸值为 pt | — |
| `border_style` | "none" \| "solid" \| "dashed" \| "dotted" \| "dash_dot" \| "double" \| "triple" | 边框线型；与填充独立<br>`none`: 不绘制此项<br>`solid`: 连续实线<br>`dashed`: 虚线；可用 dash 参数覆盖长度<br>`dotted`: 点线<br>`dash_dot`: 点划线<br>`double`: 双线描边<br>`triple`: 三线描边 | solid |
| `border_dash` | length[] / pt | 交替的边框线段和间隔长度列表；须成对 | — |
| `border_dash_offset` | length / pt | 虚线相位；裸值为 pt | 0 |
| `border_cap` | "butt" \| "round" \| "square" | 轮廓虚线端点形状<br>`butt`: 平端帽，不向端点外延伸<br>`round`: 圆端帽，向外延伸半个线宽<br>`square`: 方端帽，向外延伸半个线宽 | butt |
| `border_join` | "miter" \| "round" \| "bevel" | 轮廓交点形状<br>`miter`: 延长边缘形成尖角，受尖角限制约束<br>`round`: 以圆弧连接相邻描边边缘<br>`bevel`: 以平边截去连接尖角 | miter |
| `border_miter_limit` | value | 尖角的最大延伸与线宽之比 | 4 |
| `border_opacity` | value | 边框独立透明度 0–1 | 1 |
| `lower` | value | 填充带下边界数据 | — |
| `upper` | value | 填充带上边界数据 | — |
| `fill` | paint | 形状内部的颜色、渐变、纹理或图片 | none |
| `hatch` | value | 旧图层纹理 slash/cross；新绘制可用 fill=hatch(...) | — |
| `hatch_spacing` | value | 图层纹理线间距 | 1.5mm |
| `hatch_width` | length / pt | 图层纹理线宽；裸值为 pt | — |
| `theta` | axis \| number[] | 极坐标角轴配置或图层角坐标数据 | — |
| `r` | axis \| number[] | 极坐标径向轴配置或半径数据 | — |
| `values` | number[] | 统计计算的观测值或柱高数据 | — |
| `theta_edges` | value | 极坐标单元角边界 | — |
| `r_edges` | value | 极坐标单元径向边界 | — |
| `thetaerr` | value | 角方向误差 | — |
| `rerr` | value | 径向误差 | — |
| `wrap` | "shortest" \| "raw" | 角度跨越边界时按 shortest 或 raw 连接<br>`shortest`: 选择相邻角度之间的最短跨周期路线<br>`raw`: 保留原始角度差，可跨越多个周期 | shortest |
| `interpolation` | "polar" \| "chord" | 极坐标线段沿 polar 插值或 chord 直连<br>`polar`: 在极坐标数据中插值，再投影到页面<br>`chord`: 在页面中用直线弦连接投影点 | polar |
| `closed` | boolean | 连接首尾形成闭合曲线 | false |

### 最小完整示例

```lay
# Minimal complete example: plot.band
page=canvas(size=(100mm,75mm),background="#ffffff")
p=plot(size=(86mm,62mm),x=axis(range=(0,4)),y=axis(range=(0,5)))
p.band(x=[0,1,2,3],lower=[0.5,1,2,1],upper=[1.5,3,4,3])
page.add(p,offset=(7mm,6mm))
```

[概念与常见错误](topics/uncertainty.zh-CN.md#plot-band) · [组合源码](../examples/plot/scientific.lay)

## plot.hline

水平参考线图层：向当前图表添加数据，通过物理样式参数控制外观，并使用命名轴参数选择数据映射。

返回：图层句柄，可用于共享图例。

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `x` | axis \| number[] | 直角坐标横轴配置或图层横坐标数据 | — |
| `y` | axis \| number[] | 直角坐标纵轴配置或图层纵坐标数据 | — |
| `x_axis` | value | 绑定的横轴名称 | x |
| `y_axis` | value | 绑定的纵轴名称 | y |
| `color` | color | 文字颜色或数据系列基础配色 | — |
| `opacity` | number \| number[] | 对象透明度，0–1；与父级透明度相乘 | 1 |
| `label` | string \| text \| formula | 标签内容；支持普通文字、混合公式或文字素材 | — |
| `class` | string | 空格分隔的 LCSS 类名 | "" |
| `line_color` | color | 开放线条颜色；默认采用数据系列颜色或主题颜色 | — |
| `line_width` | length / pt | 开放线条宽度；裸值为 pt | — |
| `line_style` | "none" \| "solid" \| "dashed" \| "dotted" \| "dash_dot" \| "double" \| "triple" | 线型；double/triple 保留透明间隔<br>`none`: 不绘制此项<br>`solid`: 连续实线<br>`dashed`: 虚线；可用 dash 参数覆盖长度<br>`dotted`: 点线<br>`dash_dot`: 点划线<br>`double`: 双线描边<br>`triple`: 三线描边 | solid |
| `line_dash` | length[] / pt | 交替的实线段和间隔长度列表；裸值为 pt | — |
| `line_dash_offset` | length / pt | 虚线相位；裸值为 pt | 0 |
| `line_cap` | "butt" \| "round" \| "square" | 开放端点形状<br>`butt`: 平端帽，不向端点外延伸<br>`round`: 圆端帽，向外延伸半个线宽<br>`square`: 方端帽，向外延伸半个线宽 | butt |
| `line_join` | "miter" \| "round" \| "bevel" | 线段交点形状<br>`miter`: 延长边缘形成尖角，受尖角限制约束<br>`round`: 以圆弧连接相邻描边边缘<br>`bevel`: 以平边截去连接尖角 | miter |
| `line_miter_limit` | value | 尖角的最大延伸与线宽之比 | 4 |
| `line_opacity` | value | 线条独立透明度 0–1 | 1 |
| `theta` | axis \| number[] | 极坐标角轴配置或图层角坐标数据 | — |
| `r` | axis \| number[] | 极坐标径向轴配置或半径数据 | — |
| `values` | number[] | 统计计算的观测值或柱高数据 | — |
| `theta_edges` | value | 极坐标单元角边界 | — |
| `r_edges` | value | 极坐标单元径向边界 | — |
| `thetaerr` | value | 角方向误差 | — |
| `rerr` | value | 径向误差 | — |
| `wrap` | "shortest" \| "raw" | 角度跨越边界时按 shortest 或 raw 连接<br>`shortest`: 选择相邻角度之间的最短跨周期路线<br>`raw`: 保留原始角度差，可跨越多个周期 | shortest |
| `interpolation` | "polar" \| "chord" | 极坐标线段沿 polar 插值或 chord 直连<br>`polar`: 在极坐标数据中插值，再投影到页面<br>`chord`: 在页面中用直线弦连接投影点 | polar |
| `closed` | boolean | 连接首尾形成闭合曲线 | false |

### 最小完整示例

```lay
# Minimal complete example: plot.hline
page=canvas(size=(100mm,75mm),background="#ffffff")
p=plot(size=(86mm,62mm),x=axis(range=(0,4)),y=axis(range=(0,5)))
p.hline(y=2)
page.add(p,offset=(7mm,6mm))
```

[概念与常见错误](topics/annotations.zh-CN.md#plot-hline) · [组合源码](../examples/plot/annotations.lay)

## plot.vline

垂直参考线图层：向当前图表添加数据，通过物理样式参数控制外观，并使用命名轴参数选择数据映射。

返回：图层句柄，可用于共享图例。

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `x` | axis \| number[] | 直角坐标横轴配置或图层横坐标数据 | — |
| `y` | axis \| number[] | 直角坐标纵轴配置或图层纵坐标数据 | — |
| `x_axis` | value | 绑定的横轴名称 | x |
| `y_axis` | value | 绑定的纵轴名称 | y |
| `color` | color | 文字颜色或数据系列基础配色 | — |
| `opacity` | number \| number[] | 对象透明度，0–1；与父级透明度相乘 | 1 |
| `label` | string \| text \| formula | 标签内容；支持普通文字、混合公式或文字素材 | — |
| `class` | string | 空格分隔的 LCSS 类名 | "" |
| `line_color` | color | 开放线条颜色；默认采用数据系列颜色或主题颜色 | — |
| `line_width` | length / pt | 开放线条宽度；裸值为 pt | — |
| `line_style` | "none" \| "solid" \| "dashed" \| "dotted" \| "dash_dot" \| "double" \| "triple" | 线型；double/triple 保留透明间隔<br>`none`: 不绘制此项<br>`solid`: 连续实线<br>`dashed`: 虚线；可用 dash 参数覆盖长度<br>`dotted`: 点线<br>`dash_dot`: 点划线<br>`double`: 双线描边<br>`triple`: 三线描边 | solid |
| `line_dash` | length[] / pt | 交替的实线段和间隔长度列表；裸值为 pt | — |
| `line_dash_offset` | length / pt | 虚线相位；裸值为 pt | 0 |
| `line_cap` | "butt" \| "round" \| "square" | 开放端点形状<br>`butt`: 平端帽，不向端点外延伸<br>`round`: 圆端帽，向外延伸半个线宽<br>`square`: 方端帽，向外延伸半个线宽 | butt |
| `line_join` | "miter" \| "round" \| "bevel" | 线段交点形状<br>`miter`: 延长边缘形成尖角，受尖角限制约束<br>`round`: 以圆弧连接相邻描边边缘<br>`bevel`: 以平边截去连接尖角 | miter |
| `line_miter_limit` | value | 尖角的最大延伸与线宽之比 | 4 |
| `line_opacity` | value | 线条独立透明度 0–1 | 1 |
| `theta` | axis \| number[] | 极坐标角轴配置或图层角坐标数据 | — |
| `r` | axis \| number[] | 极坐标径向轴配置或半径数据 | — |
| `values` | number[] | 统计计算的观测值或柱高数据 | — |
| `theta_edges` | value | 极坐标单元角边界 | — |
| `r_edges` | value | 极坐标单元径向边界 | — |
| `thetaerr` | value | 角方向误差 | — |
| `rerr` | value | 径向误差 | — |
| `wrap` | "shortest" \| "raw" | 角度跨越边界时按 shortest 或 raw 连接<br>`shortest`: 选择相邻角度之间的最短跨周期路线<br>`raw`: 保留原始角度差，可跨越多个周期 | shortest |
| `interpolation` | "polar" \| "chord" | 极坐标线段沿 polar 插值或 chord 直连<br>`polar`: 在极坐标数据中插值，再投影到页面<br>`chord`: 在页面中用直线弦连接投影点 | polar |
| `closed` | boolean | 连接首尾形成闭合曲线 | false |

### 最小完整示例

```lay
# Minimal complete example: plot.vline
page=canvas(size=(100mm,75mm),background="#ffffff")
p=plot(size=(86mm,62mm),x=axis(range=(0,4)),y=axis(range=(0,5)))
p.vline(x=2)
page.add(p,offset=(7mm,6mm))
```

[概念与常见错误](topics/annotations.zh-CN.md#plot-vline) · [组合源码](../examples/manual/reference-lines-composition.lay)

## plot.bar

柱图图层：向当前图表添加数据，通过物理样式参数控制外观，并使用命名轴参数选择数据映射。

返回：图层句柄，可用于共享图例。

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `x` | axis \| number[] | 直角坐标横轴配置或图层横坐标数据 | — |
| `y` | axis \| number[] | 直角坐标纵轴配置或图层纵坐标数据 | — |
| `x_axis` | value | 绑定的横轴名称 | x |
| `y_axis` | value | 绑定的纵轴名称 | y |
| `color` | color | 文字颜色或数据系列基础配色 | — |
| `opacity` | number \| number[] | 对象透明度，0–1；与父级透明度相乘 | 1 |
| `label` | string \| text \| formula | 标签内容；支持普通文字、混合公式或文字素材 | — |
| `class` | string | 空格分隔的 LCSS 类名 | "" |
| `border_color` | color | 封闭轮廓颜色，none 表示无边框 | none |
| `border_width` | length / pt | 封闭轮廓线宽；裸值为 pt | — |
| `border_style` | "none" \| "solid" \| "dashed" \| "dotted" \| "dash_dot" \| "double" \| "triple" | 边框线型；与填充独立<br>`none`: 不绘制此项<br>`solid`: 连续实线<br>`dashed`: 虚线；可用 dash 参数覆盖长度<br>`dotted`: 点线<br>`dash_dot`: 点划线<br>`double`: 双线描边<br>`triple`: 三线描边 | solid |
| `border_dash` | length[] / pt | 交替的边框线段和间隔长度列表；须成对 | — |
| `border_dash_offset` | length / pt | 虚线相位；裸值为 pt | 0 |
| `border_cap` | "butt" \| "round" \| "square" | 轮廓虚线端点形状<br>`butt`: 平端帽，不向端点外延伸<br>`round`: 圆端帽，向外延伸半个线宽<br>`square`: 方端帽，向外延伸半个线宽 | butt |
| `border_join` | "miter" \| "round" \| "bevel" | 轮廓交点形状<br>`miter`: 延长边缘形成尖角，受尖角限制约束<br>`round`: 以圆弧连接相邻描边边缘<br>`bevel`: 以平边截去连接尖角 | miter |
| `border_miter_limit` | value | 尖角的最大延伸与线宽之比 | 4 |
| `border_opacity` | value | 边框独立透明度 0–1 | 1 |
| `data_width` | number / data | 数据坐标中的宽度，不使用物理默认单位 | — |
| `angle_width` | number / angle | 极坐标柱的角宽，遵循 angle_unit | — |
| `fill` | paint | 形状内部的颜色、渐变、纹理或图片 | none |
| `hatch` | value | 旧图层纹理 slash/cross；新绘制可用 fill=hatch(...) | — |
| `hatch_spacing` | value | 图层纹理线间距 | 1.5mm |
| `hatch_width` | length / pt | 图层纹理线宽；裸值为 pt | — |
| `positions` | number[] | 柱子中心位置的数据坐标 | — |
| `values` | number[] | 统计计算的观测值或柱高数据 | — |
| `orientation` | "vertical" \| "horizontal" | vertical/horizontal 方向<br>`vertical`: 纵向<br>`horizontal`: 横向 | vertical |
| `baseline` | value | 面积或柱图基线的数据坐标 | 0 |
| `theta` | axis \| number[] | 极坐标角轴配置或图层角坐标数据 | — |
| `r` | axis \| number[] | 极坐标径向轴配置或半径数据 | — |
| `width_unit` | value | 极坐标柱宽使用 deg 或 rad | — |
| `theta_edges` | value | 极坐标单元角边界 | — |
| `r_edges` | value | 极坐标单元径向边界 | — |
| `thetaerr` | value | 角方向误差 | — |
| `rerr` | value | 径向误差 | — |
| `wrap` | "shortest" \| "raw" | 角度跨越边界时按 shortest 或 raw 连接<br>`shortest`: 选择相邻角度之间的最短跨周期路线<br>`raw`: 保留原始角度差，可跨越多个周期 | shortest |
| `interpolation` | "polar" \| "chord" | 极坐标线段沿 polar 插值或 chord 直连<br>`polar`: 在极坐标数据中插值，再投影到页面<br>`chord`: 在页面中用直线弦连接投影点 | polar |
| `closed` | boolean | 连接首尾形成闭合曲线 | false |

### 最小完整示例

```lay
# Minimal complete example: plot.bar
page=canvas(size=(100mm,75mm),background="#ffffff")
p=plot(size=(86mm,62mm),x=axis(range=(0,4)),y=axis(range=(0,5)))
p.bar(positions=[1,2,3],values=[1,4,2],data_width=0.6)
page.add(p,offset=(7mm,6mm))
```

[概念与常见错误](topics/bars.zh-CN.md#plot-bar) · [组合源码](../examples/plot/statistics.lay)

## plot.step

阶梯图图层：向当前图表添加数据，通过物理样式参数控制外观，并使用命名轴参数选择数据映射。

返回：图层句柄，可用于共享图例。

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `x` | axis \| number[] | 直角坐标横轴配置或图层横坐标数据 | — |
| `y` | axis \| number[] | 直角坐标纵轴配置或图层纵坐标数据 | — |
| `x_axis` | value | 绑定的横轴名称 | x |
| `y_axis` | value | 绑定的纵轴名称 | y |
| `color` | color | 文字颜色或数据系列基础配色 | — |
| `opacity` | number \| number[] | 对象透明度，0–1；与父级透明度相乘 | 1 |
| `label` | string \| text \| formula | 标签内容；支持普通文字、混合公式或文字素材 | — |
| `class` | string | 空格分隔的 LCSS 类名 | "" |
| `line_color` | color | 开放线条颜色；默认采用数据系列颜色或主题颜色 | — |
| `line_width` | length / pt | 开放线条宽度；裸值为 pt | — |
| `line_style` | "none" \| "solid" \| "dashed" \| "dotted" \| "dash_dot" \| "double" \| "triple" | 线型；double/triple 保留透明间隔<br>`none`: 不绘制此项<br>`solid`: 连续实线<br>`dashed`: 虚线；可用 dash 参数覆盖长度<br>`dotted`: 点线<br>`dash_dot`: 点划线<br>`double`: 双线描边<br>`triple`: 三线描边 | solid |
| `line_dash` | length[] / pt | 交替的实线段和间隔长度列表；裸值为 pt | — |
| `line_dash_offset` | length / pt | 虚线相位；裸值为 pt | 0 |
| `line_cap` | "butt" \| "round" \| "square" | 开放端点形状<br>`butt`: 平端帽，不向端点外延伸<br>`round`: 圆端帽，向外延伸半个线宽<br>`square`: 方端帽，向外延伸半个线宽 | butt |
| `line_join` | "miter" \| "round" \| "bevel" | 线段交点形状<br>`miter`: 延长边缘形成尖角，受尖角限制约束<br>`round`: 以圆弧连接相邻描边边缘<br>`bevel`: 以平边截去连接尖角 | miter |
| `line_miter_limit` | value | 尖角的最大延伸与线宽之比 | 4 |
| `line_opacity` | value | 线条独立透明度 0–1 | 1 |
| `where` | "pre" \| "mid" \| "post" | 阶梯跳变位置 pre/mid/post<br>`pre`: 在左侧延伸阶梯<br>`mid`: 在区间中间跳变<br>`post`: 在右侧延伸阶梯 | post |
| `fill` | paint | 形状内部的颜色、渐变、纹理或图片 | none |
| `theta` | axis \| number[] | 极坐标角轴配置或图层角坐标数据 | — |
| `r` | axis \| number[] | 极坐标径向轴配置或半径数据 | — |
| `values` | number[] | 统计计算的观测值或柱高数据 | — |
| `theta_edges` | value | 极坐标单元角边界 | — |
| `r_edges` | value | 极坐标单元径向边界 | — |
| `thetaerr` | value | 角方向误差 | — |
| `rerr` | value | 径向误差 | — |
| `wrap` | "shortest" \| "raw" | 角度跨越边界时按 shortest 或 raw 连接<br>`shortest`: 选择相邻角度之间的最短跨周期路线<br>`raw`: 保留原始角度差，可跨越多个周期 | shortest |
| `interpolation` | "polar" \| "chord" | 极坐标线段沿 polar 插值或 chord 直连<br>`polar`: 在极坐标数据中插值，再投影到页面<br>`chord`: 在页面中用直线弦连接投影点 | polar |
| `closed` | boolean | 连接首尾形成闭合曲线 | false |

### 最小完整示例

```lay
# Minimal complete example: plot.step
page=canvas(size=(100mm,75mm),background="#ffffff")
p=plot(size=(86mm,62mm),x=axis(range=(0,4)),y=axis(range=(0,5)))
p.step(x=[0,1,2,3],y=[1,2,4,3],where=post)
page.add(p,offset=(7mm,6mm))
```

[概念与常见错误](topics/bars.zh-CN.md#plot-step) · [组合源码](../examples/plot/statistics.lay)

## plot.area

面积图图层：向当前图表添加数据，通过物理样式参数控制外观，并使用命名轴参数选择数据映射。

返回：图层句柄，可用于共享图例。

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `x` | axis \| number[] | 直角坐标横轴配置或图层横坐标数据 | — |
| `y` | axis \| number[] | 直角坐标纵轴配置或图层纵坐标数据 | — |
| `x_axis` | value | 绑定的横轴名称 | x |
| `y_axis` | value | 绑定的纵轴名称 | y |
| `color` | color | 文字颜色或数据系列基础配色 | — |
| `opacity` | number \| number[] | 对象透明度，0–1；与父级透明度相乘 | 1 |
| `label` | string \| text \| formula | 标签内容；支持普通文字、混合公式或文字素材 | — |
| `class` | string | 空格分隔的 LCSS 类名 | "" |
| `border_color` | color | 封闭轮廓颜色，none 表示无边框 | none |
| `border_width` | length / pt | 封闭轮廓线宽；裸值为 pt | — |
| `border_style` | "none" \| "solid" \| "dashed" \| "dotted" \| "dash_dot" \| "double" \| "triple" | 边框线型；与填充独立<br>`none`: 不绘制此项<br>`solid`: 连续实线<br>`dashed`: 虚线；可用 dash 参数覆盖长度<br>`dotted`: 点线<br>`dash_dot`: 点划线<br>`double`: 双线描边<br>`triple`: 三线描边 | solid |
| `border_dash` | length[] / pt | 交替的边框线段和间隔长度列表；须成对 | — |
| `border_dash_offset` | length / pt | 虚线相位；裸值为 pt | 0 |
| `border_cap` | "butt" \| "round" \| "square" | 轮廓虚线端点形状<br>`butt`: 平端帽，不向端点外延伸<br>`round`: 圆端帽，向外延伸半个线宽<br>`square`: 方端帽，向外延伸半个线宽 | butt |
| `border_join` | "miter" \| "round" \| "bevel" | 轮廓交点形状<br>`miter`: 延长边缘形成尖角，受尖角限制约束<br>`round`: 以圆弧连接相邻描边边缘<br>`bevel`: 以平边截去连接尖角 | miter |
| `border_miter_limit` | value | 尖角的最大延伸与线宽之比 | 4 |
| `border_opacity` | value | 边框独立透明度 0–1 | 1 |
| `baseline` | value | 面积或柱图基线的数据坐标 | 0 |
| `fill` | paint | 形状内部的颜色、渐变、纹理或图片 | none |
| `hatch` | value | 旧图层纹理 slash/cross；新绘制可用 fill=hatch(...) | — |
| `hatch_spacing` | value | 图层纹理线间距 | 1.5mm |
| `hatch_width` | length / pt | 图层纹理线宽；裸值为 pt | — |
| `theta` | axis \| number[] | 极坐标角轴配置或图层角坐标数据 | — |
| `r` | axis \| number[] | 极坐标径向轴配置或半径数据 | — |
| `values` | number[] | 统计计算的观测值或柱高数据 | — |
| `theta_edges` | value | 极坐标单元角边界 | — |
| `r_edges` | value | 极坐标单元径向边界 | — |
| `thetaerr` | value | 角方向误差 | — |
| `rerr` | value | 径向误差 | — |
| `wrap` | "shortest" \| "raw" | 角度跨越边界时按 shortest 或 raw 连接<br>`shortest`: 选择相邻角度之间的最短跨周期路线<br>`raw`: 保留原始角度差，可跨越多个周期 | shortest |
| `interpolation` | "polar" \| "chord" | 极坐标线段沿 polar 插值或 chord 直连<br>`polar`: 在极坐标数据中插值，再投影到页面<br>`chord`: 在页面中用直线弦连接投影点 | polar |
| `closed` | boolean | 连接首尾形成闭合曲线 | false |

### 最小完整示例

```lay
# Minimal complete example: plot.area
page=canvas(size=(100mm,75mm),background="#ffffff")
p=plot(size=(86mm,62mm),x=axis(range=(0,4)),y=axis(range=(0,5)))
p.area(x=[0,1,2,3],y=[1,2,4,3])
page.add(p,offset=(7mm,6mm))
```

[概念与常见错误](topics/bars.zh-CN.md#plot-area) · [组合源码](../examples/plot/polar-data.lay)

## plot.hist

直方图图层：向当前图表添加数据，通过物理样式参数控制外观，并使用命名轴参数选择数据映射。

返回：图层句柄，可用于共享图例。

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `x` | axis \| number[] | 直角坐标横轴配置或图层横坐标数据 | — |
| `y` | axis \| number[] | 直角坐标纵轴配置或图层纵坐标数据 | — |
| `x_axis` | value | 绑定的横轴名称 | x |
| `y_axis` | value | 绑定的纵轴名称 | y |
| `color` | color | 文字颜色或数据系列基础配色 | — |
| `opacity` | number \| number[] | 对象透明度，0–1；与父级透明度相乘 | 1 |
| `label` | string \| text \| formula | 标签内容；支持普通文字、混合公式或文字素材 | — |
| `class` | string | 空格分隔的 LCSS 类名 | "" |
| `border_color` | color | 封闭轮廓颜色，none 表示无边框 | none |
| `border_width` | length / pt | 封闭轮廓线宽；裸值为 pt | — |
| `border_style` | "none" \| "solid" \| "dashed" \| "dotted" \| "dash_dot" \| "double" \| "triple" | 边框线型；与填充独立<br>`none`: 不绘制此项<br>`solid`: 连续实线<br>`dashed`: 虚线；可用 dash 参数覆盖长度<br>`dotted`: 点线<br>`dash_dot`: 点划线<br>`double`: 双线描边<br>`triple`: 三线描边 | solid |
| `border_dash` | length[] / pt | 交替的边框线段和间隔长度列表；须成对 | — |
| `border_dash_offset` | length / pt | 虚线相位；裸值为 pt | 0 |
| `border_cap` | "butt" \| "round" \| "square" | 轮廓虚线端点形状<br>`butt`: 平端帽，不向端点外延伸<br>`round`: 圆端帽，向外延伸半个线宽<br>`square`: 方端帽，向外延伸半个线宽 | butt |
| `border_join` | "miter" \| "round" \| "bevel" | 轮廓交点形状<br>`miter`: 延长边缘形成尖角，受尖角限制约束<br>`round`: 以圆弧连接相邻描边边缘<br>`bevel`: 以平边截去连接尖角 | miter |
| `border_miter_limit` | value | 尖角的最大延伸与线宽之比 | 4 |
| `border_opacity` | value | 边框独立透明度 0–1 | 1 |
| `values` | number[] | 统计计算的观测值或柱高数据 | — |
| `bins` | integer \| number[] | 直方图正整数箱数或严格递增边界 | — |
| `weights` | value | 每个观测值的非负权重 | — |
| `orientation` | "vertical" \| "horizontal" | vertical/horizontal 方向<br>`vertical`: 纵向<br>`horizontal`: 横向 | vertical |
| `fill` | paint | 形状内部的颜色、渐变、纹理或图片 | none |
| `hatch` | value | 旧图层纹理 slash/cross；新绘制可用 fill=hatch(...) | — |
| `hatch_spacing` | value | 图层纹理线间距 | 1.5mm |
| `hatch_width` | length / pt | 图层纹理线宽；裸值为 pt | — |
| `stat` | "count" \| "probability" \| "density" | 直方图 count/probability/density 统计量<br>`count`: 每箱的样本数量<br>`probability`: 每箱的概率，所有箱之和为 1<br>`density`: 按箱宽归一化的概率密度 | count |
| `theta` | axis \| number[] | 极坐标角轴配置或图层角坐标数据 | — |
| `r` | axis \| number[] | 极坐标径向轴配置或半径数据 | — |
| `theta_edges` | value | 极坐标单元角边界 | — |
| `r_edges` | value | 极坐标单元径向边界 | — |
| `thetaerr` | value | 角方向误差 | — |
| `rerr` | value | 径向误差 | — |
| `wrap` | "shortest" \| "raw" | 角度跨越边界时按 shortest 或 raw 连接<br>`shortest`: 选择相邻角度之间的最短跨周期路线<br>`raw`: 保留原始角度差，可跨越多个周期 | shortest |
| `interpolation` | "polar" \| "chord" | 极坐标线段沿 polar 插值或 chord 直连<br>`polar`: 在极坐标数据中插值，再投影到页面<br>`chord`: 在页面中用直线弦连接投影点 | polar |
| `closed` | boolean | 连接首尾形成闭合曲线 | false |

### 最小完整示例

```lay
# Minimal complete example: plot.hist
page=canvas(size=(100mm,75mm),background="#ffffff")
p=plot(size=(86mm,62mm),x=axis(range=(0,4)),y=axis(range=(0,5)))
p.hist(values=[0.2,0.8,1.1,1.2,2.4,3.1],bins=[0,1,2,3,4])
page.add(p,offset=(7mm,6mm))
```

[概念与常见错误](topics/hist-ecdf.zh-CN.md#plot-hist) · [组合源码](../examples/plot/statistics.lay)

## plot.boxplot

箱线图图层：向当前图表添加数据，通过物理样式参数控制外观，并使用命名轴参数选择数据映射。

返回：图层句柄，可用于共享图例。

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `x` | axis \| number[] | 直角坐标横轴配置或图层横坐标数据 | — |
| `y` | axis \| number[] | 直角坐标纵轴配置或图层纵坐标数据 | — |
| `x_axis` | value | 绑定的横轴名称 | x |
| `y_axis` | value | 绑定的纵轴名称 | y |
| `color` | color | 文字颜色或数据系列基础配色 | — |
| `opacity` | number \| number[] | 对象透明度，0–1；与父级透明度相乘 | 1 |
| `label` | string \| text \| formula | 标签内容；支持普通文字、混合公式或文字素材 | — |
| `class` | string | 空格分隔的 LCSS 类名 | "" |
| `border_color` | color | 封闭轮廓颜色，none 表示无边框 | none |
| `border_width` | length / pt | 封闭轮廓线宽；裸值为 pt | — |
| `border_style` | "none" \| "solid" \| "dashed" \| "dotted" \| "dash_dot" \| "double" \| "triple" | 边框线型；与填充独立<br>`none`: 不绘制此项<br>`solid`: 连续实线<br>`dashed`: 虚线；可用 dash 参数覆盖长度<br>`dotted`: 点线<br>`dash_dot`: 点划线<br>`double`: 双线描边<br>`triple`: 三线描边 | solid |
| `border_dash` | length[] / pt | 交替的边框线段和间隔长度列表；须成对 | — |
| `border_dash_offset` | length / pt | 虚线相位；裸值为 pt | 0 |
| `border_cap` | "butt" \| "round" \| "square" | 轮廓虚线端点形状<br>`butt`: 平端帽，不向端点外延伸<br>`round`: 圆端帽，向外延伸半个线宽<br>`square`: 方端帽，向外延伸半个线宽 | butt |
| `border_join` | "miter" \| "round" \| "bevel" | 轮廓交点形状<br>`miter`: 延长边缘形成尖角，受尖角限制约束<br>`round`: 以圆弧连接相邻描边边缘<br>`bevel`: 以平边截去连接尖角 | miter |
| `border_miter_limit` | value | 尖角的最大延伸与线宽之比 | 4 |
| `border_opacity` | value | 边框独立透明度 0–1 | 1 |
| `values` | number[] | 统计计算的观测值或柱高数据 | — |
| `position` | value | 图例位置、统计组数据位置或色标位置；由所属对象决定 | — |
| `data_width` | number / data | 数据坐标中的宽度，不使用物理默认单位 | — |
| `outlier_size` | length / 画布单位 | 离群点物理大小 | — |
| `fill` | paint | 形状内部的颜色、渐变、纹理或图片 | none |
| `orientation` | "vertical" \| "horizontal" | vertical/horizontal 方向<br>`vertical`: 纵向<br>`horizontal`: 横向 | vertical |
| `whisker` | value | 箱线图须长的四分位距倍数 | 1.5 |
| `outliers` | boolean | 是否显示箱线图离群点 | true |
| `median_color` | color | 中位数线颜色 | — |
| `median_line_width` | length / pt | 中位数线宽，裸值 pt | — |
| `whisker_color` | color | 箱须颜色 | — |
| `whisker_line_width` | length / pt | 箱须线宽，裸值 pt | — |
| `outlier_marker` | "none" \| "circle" \| "square" \| "triangle" \| "triangle_down" \| "diamond" | 离群点标记形状<br>`none`: 无标记<br>`circle`: 圆形标记<br>`square`: 方形标记<br>`triangle`: 上三角标记<br>`triangle_down`: 下三角标记<br>`diamond`: 菱形标记 | — |
| `outlier_fill` | value | 离群点内部颜色 | — |
| `outlier_border_color` | color | 离群点边框颜色 | — |
| `outlier_border_width` | length / pt | 离群点边框宽度，裸值 pt | — |
| `theta` | axis \| number[] | 极坐标角轴配置或图层角坐标数据 | — |
| `r` | axis \| number[] | 极坐标径向轴配置或半径数据 | — |
| `theta_edges` | value | 极坐标单元角边界 | — |
| `r_edges` | value | 极坐标单元径向边界 | — |
| `thetaerr` | value | 角方向误差 | — |
| `rerr` | value | 径向误差 | — |
| `wrap` | "shortest" \| "raw" | 角度跨越边界时按 shortest 或 raw 连接<br>`shortest`: 选择相邻角度之间的最短跨周期路线<br>`raw`: 保留原始角度差，可跨越多个周期 | shortest |
| `interpolation` | "polar" \| "chord" | 极坐标线段沿 polar 插值或 chord 直连<br>`polar`: 在极坐标数据中插值，再投影到页面<br>`chord`: 在页面中用直线弦连接投影点 | polar |
| `closed` | boolean | 连接首尾形成闭合曲线 | false |

### 最小完整示例

```lay
# Minimal complete example: plot.boxplot
page=canvas(size=(100mm,75mm),background="#ffffff")
p=plot(size=(86mm,62mm),x=axis(range=(0,4)),y=axis(range=(0,5)))
p.boxplot(values=[1,2,2,3,4,4],position=2)
page.add(p,offset=(7mm,6mm))
```

[概念与常见错误](topics/box-violin.zh-CN.md#plot-boxplot) · [组合源码](../examples/plot/statistics.lay)

## plot.violin

小提琴图图层：向当前图表添加数据，通过物理样式参数控制外观，并使用命名轴参数选择数据映射。

返回：图层句柄，可用于共享图例。

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `x` | axis \| number[] | 直角坐标横轴配置或图层横坐标数据 | — |
| `y` | axis \| number[] | 直角坐标纵轴配置或图层纵坐标数据 | — |
| `x_axis` | value | 绑定的横轴名称 | x |
| `y_axis` | value | 绑定的纵轴名称 | y |
| `color` | color | 文字颜色或数据系列基础配色 | — |
| `opacity` | number \| number[] | 对象透明度，0–1；与父级透明度相乘 | 1 |
| `label` | string \| text \| formula | 标签内容；支持普通文字、混合公式或文字素材 | — |
| `class` | string | 空格分隔的 LCSS 类名 | "" |
| `border_color` | color | 封闭轮廓颜色，none 表示无边框 | none |
| `border_width` | length / pt | 封闭轮廓线宽；裸值为 pt | — |
| `border_style` | "none" \| "solid" \| "dashed" \| "dotted" \| "dash_dot" \| "double" \| "triple" | 边框线型；与填充独立<br>`none`: 不绘制此项<br>`solid`: 连续实线<br>`dashed`: 虚线；可用 dash 参数覆盖长度<br>`dotted`: 点线<br>`dash_dot`: 点划线<br>`double`: 双线描边<br>`triple`: 三线描边 | solid |
| `border_dash` | length[] / pt | 交替的边框线段和间隔长度列表；须成对 | — |
| `border_dash_offset` | length / pt | 虚线相位；裸值为 pt | 0 |
| `border_cap` | "butt" \| "round" \| "square" | 轮廓虚线端点形状<br>`butt`: 平端帽，不向端点外延伸<br>`round`: 圆端帽，向外延伸半个线宽<br>`square`: 方端帽，向外延伸半个线宽 | butt |
| `border_join` | "miter" \| "round" \| "bevel" | 轮廓交点形状<br>`miter`: 延长边缘形成尖角，受尖角限制约束<br>`round`: 以圆弧连接相邻描边边缘<br>`bevel`: 以平边截去连接尖角 | miter |
| `border_miter_limit` | value | 尖角的最大延伸与线宽之比 | 4 |
| `border_opacity` | value | 边框独立透明度 0–1 | 1 |
| `values` | number[] | 统计计算的观测值或柱高数据 | — |
| `position` | value | 图例位置、统计组数据位置或色标位置；由所属对象决定 | — |
| `data_width` | number / data | 数据坐标中的宽度，不使用物理默认单位 | — |
| `bandwidth` | value | 核密度估计的正带宽 | — |
| `points` | value | 点列表；star 为整数角数，violin 为 KDE 采样点数 | — |
| `fill` | paint | 形状内部的颜色、渐变、纹理或图片 | none |
| `orientation` | "vertical" \| "horizontal" | vertical/horizontal 方向<br>`vertical`: 纵向<br>`horizontal`: 横向 | vertical |
| `theta` | axis \| number[] | 极坐标角轴配置或图层角坐标数据 | — |
| `r` | axis \| number[] | 极坐标径向轴配置或半径数据 | — |
| `theta_edges` | value | 极坐标单元角边界 | — |
| `r_edges` | value | 极坐标单元径向边界 | — |
| `thetaerr` | value | 角方向误差 | — |
| `rerr` | value | 径向误差 | — |
| `wrap` | "shortest" \| "raw" | 角度跨越边界时按 shortest 或 raw 连接<br>`shortest`: 选择相邻角度之间的最短跨周期路线<br>`raw`: 保留原始角度差，可跨越多个周期 | shortest |
| `interpolation` | "polar" \| "chord" | 极坐标线段沿 polar 插值或 chord 直连<br>`polar`: 在极坐标数据中插值，再投影到页面<br>`chord`: 在页面中用直线弦连接投影点 | polar |
| `closed` | boolean | 连接首尾形成闭合曲线 | false |

### 最小完整示例

```lay
# Minimal complete example: plot.violin
page=canvas(size=(100mm,75mm),background="#ffffff")
p=plot(size=(86mm,62mm),x=axis(range=(0,4)),y=axis(range=(0,5)))
p.violin(values=[1,1.5,2,2.2,3,4],position=2)
page.add(p,offset=(7mm,6mm))
```

[概念与常见错误](topics/box-violin.zh-CN.md#plot-violin) · [组合源码](../examples/plot/statistics.lay)

## plot.ecdf

经验累积分布图层：向当前图表添加数据，通过物理样式参数控制外观，并使用命名轴参数选择数据映射。

返回：图层句柄，可用于共享图例。

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `x` | axis \| number[] | 直角坐标横轴配置或图层横坐标数据 | — |
| `y` | axis \| number[] | 直角坐标纵轴配置或图层纵坐标数据 | — |
| `x_axis` | value | 绑定的横轴名称 | x |
| `y_axis` | value | 绑定的纵轴名称 | y |
| `color` | color | 文字颜色或数据系列基础配色 | — |
| `opacity` | number \| number[] | 对象透明度，0–1；与父级透明度相乘 | 1 |
| `label` | string \| text \| formula | 标签内容；支持普通文字、混合公式或文字素材 | — |
| `class` | string | 空格分隔的 LCSS 类名 | "" |
| `line_color` | color | 开放线条颜色；默认采用数据系列颜色或主题颜色 | — |
| `line_width` | length / pt | 开放线条宽度；裸值为 pt | — |
| `line_style` | "none" \| "solid" \| "dashed" \| "dotted" \| "dash_dot" \| "double" \| "triple" | 线型；double/triple 保留透明间隔<br>`none`: 不绘制此项<br>`solid`: 连续实线<br>`dashed`: 虚线；可用 dash 参数覆盖长度<br>`dotted`: 点线<br>`dash_dot`: 点划线<br>`double`: 双线描边<br>`triple`: 三线描边 | solid |
| `line_dash` | length[] / pt | 交替的实线段和间隔长度列表；裸值为 pt | — |
| `line_dash_offset` | length / pt | 虚线相位；裸值为 pt | 0 |
| `line_cap` | "butt" \| "round" \| "square" | 开放端点形状<br>`butt`: 平端帽，不向端点外延伸<br>`round`: 圆端帽，向外延伸半个线宽<br>`square`: 方端帽，向外延伸半个线宽 | butt |
| `line_join` | "miter" \| "round" \| "bevel" | 线段交点形状<br>`miter`: 延长边缘形成尖角，受尖角限制约束<br>`round`: 以圆弧连接相邻描边边缘<br>`bevel`: 以平边截去连接尖角 | miter |
| `line_miter_limit` | value | 尖角的最大延伸与线宽之比 | 4 |
| `line_opacity` | value | 线条独立透明度 0–1 | 1 |
| `values` | number[] | 统计计算的观测值或柱高数据 | — |
| `complementary` | value | 绘制 1−ECDF | false |
| `theta` | axis \| number[] | 极坐标角轴配置或图层角坐标数据 | — |
| `r` | axis \| number[] | 极坐标径向轴配置或半径数据 | — |
| `theta_edges` | value | 极坐标单元角边界 | — |
| `r_edges` | value | 极坐标单元径向边界 | — |
| `thetaerr` | value | 角方向误差 | — |
| `rerr` | value | 径向误差 | — |
| `wrap` | "shortest" \| "raw" | 角度跨越边界时按 shortest 或 raw 连接<br>`shortest`: 选择相邻角度之间的最短跨周期路线<br>`raw`: 保留原始角度差，可跨越多个周期 | shortest |
| `interpolation` | "polar" \| "chord" | 极坐标线段沿 polar 插值或 chord 直连<br>`polar`: 在极坐标数据中插值，再投影到页面<br>`chord`: 在页面中用直线弦连接投影点 | polar |
| `closed` | boolean | 连接首尾形成闭合曲线 | false |

### 最小完整示例

```lay
# Minimal complete example: plot.ecdf
page=canvas(size=(100mm,75mm),background="#ffffff")
p=plot(size=(86mm,62mm),x=axis(range=(0,4)),y=axis(range=(0,5)))
p.ecdf(values=[0.2,0.8,1.1,1.2,2.4,3.1])
page.add(p,offset=(7mm,6mm))
```

[概念与常见错误](topics/hist-ecdf.zh-CN.md#plot-ecdf) · [组合源码](../examples/plot/statistics.lay)

## plot.heatmap

热图图层：向当前图表添加数据，通过物理样式参数控制外观，并使用命名轴参数选择数据映射。

返回：图层句柄，可用于共享图例。

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `x` | axis \| number[] | 直角坐标横轴配置或图层横坐标数据 | — |
| `y` | axis \| number[] | 直角坐标纵轴配置或图层纵坐标数据 | — |
| `x_axis` | value | 绑定的横轴名称 | x |
| `y_axis` | value | 绑定的纵轴名称 | y |
| `color` | color | 文字颜色或数据系列基础配色 | — |
| `opacity` | number \| number[] | 对象透明度，0–1；与父级透明度相乘 | 1 |
| `label` | string \| text \| formula | 标签内容；支持普通文字、混合公式或文字素材 | — |
| `class` | string | 空格分隔的 LCSS 类名 | "" |
| `z` | number[][] | 矩形二维网格；行对应 y，列对应 x | — |
| `x_edges` | value | 热图列的递增数据边界，长度为列数加一 | — |
| `y_edges` | value | 热图行的递增数据边界，长度为行数加一 | — |
| `cmap` | string \| color[] \| cmap | 预设名称、颜色列表或 cmap(...) 配色对象 | viridis |
| `vmin` | value | 颜色映射最小数据值 | — |
| `vmax` | value | 颜色映射最大数据值 | — |
| `color_scale` | color | 共享 color_scale(...) 对象 | — |
| `extent` | value | 数据网格的 (xmin,xmax,ymin,ymax) | — |
| `origin` | "lower" \| "upper" | 热图第一行位于下方或上方<br>`lower`: 第一行在下方<br>`upper`: 第一行在上方 | lower |
| `mode` | "raster" \| "vector" | 热图输出为栅格单元或矢量单元<br>`raster`: 以栅格方式生成热图单元<br>`vector`: 以矢量方式生成热图单元 | raster |
| `theta` | axis \| number[] | 极坐标角轴配置或图层角坐标数据 | — |
| `r` | axis \| number[] | 极坐标径向轴配置或半径数据 | — |
| `values` | number[] | 统计计算的观测值或柱高数据 | — |
| `theta_edges` | value | 极坐标单元角边界 | — |
| `r_edges` | value | 极坐标单元径向边界 | — |
| `thetaerr` | value | 角方向误差 | — |
| `rerr` | value | 径向误差 | — |
| `wrap` | "shortest" \| "raw" | 角度跨越边界时按 shortest 或 raw 连接<br>`shortest`: 选择相邻角度之间的最短跨周期路线<br>`raw`: 保留原始角度差，可跨越多个周期 | shortest |
| `interpolation` | "polar" \| "chord" | 极坐标线段沿 polar 插值或 chord 直连<br>`polar`: 在极坐标数据中插值，再投影到页面<br>`chord`: 在页面中用直线弦连接投影点 | polar |
| `closed` | boolean | 连接首尾形成闭合曲线 | false |

### 最小完整示例

```lay
# Minimal complete example: plot.heatmap
page=canvas(size=(100mm,75mm),background="#ffffff")
p=plot(size=(86mm,62mm),x=axis(range=(0,4)),y=axis(range=(0,5)))
p.heatmap(z=[[1,2,3],[2,4,2],[1,3,1]],extent=(0,4,0,5),origin=lower)
page.add(p,offset=(7mm,6mm))
```

[概念与常见错误](topics/heatmaps.zh-CN.md#plot-heatmap) · [组合源码](../examples/plot/shared-colors.lay)

## plot.contour

等高线图层：向当前图表添加数据，通过物理样式参数控制外观，并使用命名轴参数选择数据映射。

返回：图层句柄，可用于共享图例。

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `x` | axis \| number[] | 直角坐标横轴配置或图层横坐标数据 | — |
| `y` | axis \| number[] | 直角坐标纵轴配置或图层纵坐标数据 | — |
| `x_axis` | value | 绑定的横轴名称 | x |
| `y_axis` | value | 绑定的纵轴名称 | y |
| `color` | color | 文字颜色或数据系列基础配色 | — |
| `opacity` | number \| number[] | 对象透明度，0–1；与父级透明度相乘 | 1 |
| `label` | string \| text \| formula | 标签内容；支持普通文字、混合公式或文字素材 | — |
| `class` | string | 空格分隔的 LCSS 类名 | "" |
| `line_color` | color | 开放线条颜色；默认采用数据系列颜色或主题颜色 | — |
| `line_width` | length / pt | 开放线条宽度；裸值为 pt | — |
| `line_style` | "none" \| "solid" \| "dashed" \| "dotted" \| "dash_dot" \| "double" \| "triple" | 线型；double/triple 保留透明间隔<br>`none`: 不绘制此项<br>`solid`: 连续实线<br>`dashed`: 虚线；可用 dash 参数覆盖长度<br>`dotted`: 点线<br>`dash_dot`: 点划线<br>`double`: 双线描边<br>`triple`: 三线描边 | solid |
| `line_dash` | length[] / pt | 交替的实线段和间隔长度列表；裸值为 pt | — |
| `line_dash_offset` | length / pt | 虚线相位；裸值为 pt | 0 |
| `line_cap` | "butt" \| "round" \| "square" | 开放端点形状<br>`butt`: 平端帽，不向端点外延伸<br>`round`: 圆端帽，向外延伸半个线宽<br>`square`: 方端帽，向外延伸半个线宽 | butt |
| `line_join` | "miter" \| "round" \| "bevel" | 线段交点形状<br>`miter`: 延长边缘形成尖角，受尖角限制约束<br>`round`: 以圆弧连接相邻描边边缘<br>`bevel`: 以平边截去连接尖角 | miter |
| `line_miter_limit` | value | 尖角的最大延伸与线宽之比 | 4 |
| `line_opacity` | value | 线条独立透明度 0–1 | 1 |
| `z` | number[][] | 矩形二维网格；行对应 y，列对应 x | — |
| `levels` | value | 递增的等高线数据值 | — |
| `color_scale` | color | 共享 color_scale(...) 对象 | — |
| `cmap` | string \| color[] \| cmap | 预设名称、颜色列表或 cmap(...) 配色对象 | viridis |
| `vmin` | value | 颜色映射最小数据值 | — |
| `vmax` | value | 颜色映射最大数据值 | — |
| `extent` | value | 数据网格的 (xmin,xmax,ymin,ymax) | — |
| `periodic` | value | 规则极坐标场是否周期闭合 | false |
| `theta` | axis \| number[] | 极坐标角轴配置或图层角坐标数据 | — |
| `r` | axis \| number[] | 极坐标径向轴配置或半径数据 | — |
| `values` | number[] | 统计计算的观测值或柱高数据 | — |
| `theta_edges` | value | 极坐标单元角边界 | — |
| `r_edges` | value | 极坐标单元径向边界 | — |
| `thetaerr` | value | 角方向误差 | — |
| `rerr` | value | 径向误差 | — |
| `wrap` | "shortest" \| "raw" | 角度跨越边界时按 shortest 或 raw 连接<br>`shortest`: 选择相邻角度之间的最短跨周期路线<br>`raw`: 保留原始角度差，可跨越多个周期 | shortest |
| `interpolation` | "polar" \| "chord" | 极坐标线段沿 polar 插值或 chord 直连<br>`polar`: 在极坐标数据中插值，再投影到页面<br>`chord`: 在页面中用直线弦连接投影点 | polar |
| `closed` | boolean | 连接首尾形成闭合曲线 | false |

### 最小完整示例

```lay
# Minimal complete example: plot.contour
page=canvas(size=(100mm,75mm),background="#ffffff")
p=plot(size=(86mm,62mm),x=axis(range=(0,4)),y=axis(range=(0,5)))
p.contour(z=[[1,2,3],[2,4,2],[1,3,1]],levels=[1.5,2.5,3.5],extent=(0,4,0,5))
page.add(p,offset=(7mm,6mm))
```

[概念与常见错误](topics/heatmaps.zh-CN.md#plot-contour) · [组合源码](../examples/plot/shared-colors.lay)

## plot.contourf

填充等高线图层：向当前图表添加数据，通过物理样式参数控制外观，并使用命名轴参数选择数据映射。

返回：图层句柄，可用于共享图例。

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `x` | axis \| number[] | 直角坐标横轴配置或图层横坐标数据 | — |
| `y` | axis \| number[] | 直角坐标纵轴配置或图层纵坐标数据 | — |
| `x_axis` | value | 绑定的横轴名称 | x |
| `y_axis` | value | 绑定的纵轴名称 | y |
| `color` | color | 文字颜色或数据系列基础配色 | — |
| `opacity` | number \| number[] | 对象透明度，0–1；与父级透明度相乘 | 1 |
| `label` | string \| text \| formula | 标签内容；支持普通文字、混合公式或文字素材 | — |
| `class` | string | 空格分隔的 LCSS 类名 | "" |
| `border_color` | color | 封闭轮廓颜色，none 表示无边框 | none |
| `border_width` | length / pt | 封闭轮廓线宽；裸值为 pt | — |
| `border_style` | "none" \| "solid" \| "dashed" \| "dotted" \| "dash_dot" \| "double" \| "triple" | 边框线型；与填充独立<br>`none`: 不绘制此项<br>`solid`: 连续实线<br>`dashed`: 虚线；可用 dash 参数覆盖长度<br>`dotted`: 点线<br>`dash_dot`: 点划线<br>`double`: 双线描边<br>`triple`: 三线描边 | solid |
| `border_dash` | length[] / pt | 交替的边框线段和间隔长度列表；须成对 | — |
| `border_dash_offset` | length / pt | 虚线相位；裸值为 pt | 0 |
| `border_cap` | "butt" \| "round" \| "square" | 轮廓虚线端点形状<br>`butt`: 平端帽，不向端点外延伸<br>`round`: 圆端帽，向外延伸半个线宽<br>`square`: 方端帽，向外延伸半个线宽 | butt |
| `border_join` | "miter" \| "round" \| "bevel" | 轮廓交点形状<br>`miter`: 延长边缘形成尖角，受尖角限制约束<br>`round`: 以圆弧连接相邻描边边缘<br>`bevel`: 以平边截去连接尖角 | miter |
| `border_miter_limit` | value | 尖角的最大延伸与线宽之比 | 4 |
| `border_opacity` | value | 边框独立透明度 0–1 | 1 |
| `z` | number[][] | 矩形二维网格；行对应 y，列对应 x | — |
| `levels` | value | 递增的等高线数据值 | — |
| `color_scale` | color | 共享 color_scale(...) 对象 | — |
| `cmap` | string \| color[] \| cmap | 预设名称、颜色列表或 cmap(...) 配色对象 | viridis |
| `vmin` | value | 颜色映射最小数据值 | — |
| `vmax` | value | 颜色映射最大数据值 | — |
| `fill` | paint | 形状内部的颜色、渐变、纹理或图片 | none |
| `extent` | value | 数据网格的 (xmin,xmax,ymin,ymax) | — |
| `periodic` | value | 规则极坐标场是否周期闭合 | false |
| `theta` | axis \| number[] | 极坐标角轴配置或图层角坐标数据 | — |
| `r` | axis \| number[] | 极坐标径向轴配置或半径数据 | — |
| `values` | number[] | 统计计算的观测值或柱高数据 | — |
| `theta_edges` | value | 极坐标单元角边界 | — |
| `r_edges` | value | 极坐标单元径向边界 | — |
| `thetaerr` | value | 角方向误差 | — |
| `rerr` | value | 径向误差 | — |
| `wrap` | "shortest" \| "raw" | 角度跨越边界时按 shortest 或 raw 连接<br>`shortest`: 选择相邻角度之间的最短跨周期路线<br>`raw`: 保留原始角度差，可跨越多个周期 | shortest |
| `interpolation` | "polar" \| "chord" | 极坐标线段沿 polar 插值或 chord 直连<br>`polar`: 在极坐标数据中插值，再投影到页面<br>`chord`: 在页面中用直线弦连接投影点 | polar |
| `closed` | boolean | 连接首尾形成闭合曲线 | false |

### 最小完整示例

```lay
# Minimal complete example: plot.contourf
page=canvas(size=(100mm,75mm),background="#ffffff")
p=plot(size=(86mm,62mm),x=axis(range=(0,4)),y=axis(range=(0,5)))
p.contourf(z=[[1,2,3],[2,4,2],[1,3,1]],levels=[1.5,2.5,3.5],extent=(0,4,0,5))
page.add(p,offset=(7mm,6mm))
```

[概念与常见错误](topics/heatmaps.zh-CN.md#plot-contourf) · [组合源码](../examples/plot/shared-colors.lay)

## range

生成不包含 stop 的有限整数列表；一个参数为 stop，两个为 start/stop，三个增加 step。step 非零，最多 10,000 项。

返回：integer[]

位置参数：`start`, `stop`, `step`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `start` | integer | 起值；单参数调用视作 stop，默认起值 0 | — |
| `stop` | integer | 不包含的终值 | — |
| `step` | integer | 步长，非零；默认 1 | 1 |

### 最小完整示例

```lay
# Minimal complete example: range
page=canvas(size=(100mm,75mm),background="#ffffff")
for i in range(4) { page.add(rect(size=(10mm,10mm),fill="#087f8c"),offset=(10mm+i*16mm,20mm)) }
```

[概念与常见错误](topics/values.zh-CN.md#range) · [组合源码](../examples/functions.lay)

## len

返回列表、字典、几何集合的项数或字符串长度。

返回：integer

位置参数：`value`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `value` | value | 待处理的标量、字符串或列表元素 | — |

### 最小完整示例

```lay
# Minimal complete example: len
page=canvas(size=(100mm,75mm),background="#ffffff")
values=[1,2,3]
page.add(text("Count: "+str(len(values))),offset=(10mm,10mm))
```

[概念与常见错误](topics/values.zh-CN.md#len) · [组合源码](../examples/gallery/scripting/units-builtins.lay)

## append

返回加入一个新项的列表副本，不原地修改输入列表。需要累积结果时将返回值重新赋给变量。

返回：list

位置参数：`list`, `value`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `list` | value | 输入列表 | — |
| `value` | value | 待处理的标量、字符串或列表元素 | — |

### 最小完整示例

```lay
# Minimal complete example: append
page=canvas(size=(100mm,75mm),background="#ffffff")
values=append([1,2],3)
page.add(text("New count: "+str(len(values))),offset=(10mm,10mm))
```

[概念与常见错误](topics/values.zh-CN.md#append) · [组合源码](../examples/gallery/scripting/units-builtins.lay)

## str

将支持的数值、布尔值或字符串转换为文字，长度包含单位。可拼接为标签，不执行任意格式表达式。

返回：string

位置参数：`value`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `value` | value | 待处理的标量、字符串或列表元素 | — |

### 最小完整示例

```lay
# Minimal complete example: str
page=canvas(size=(100mm,75mm),background="#ffffff")
page.add(text("Measured: "+str(20mm)),offset=(10mm,10mm))
```

[概念与常见错误](topics/values.zh-CN.md#str) · [组合源码](../examples/gallery/scripting/units-builtins.lay)

## abs

返回绝对值并保留数值的单位类型。适用于标量或物理长度，不接受素材与列表。

返回：number | length

位置参数：`value`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `value` | value | 待处理的标量、字符串或列表元素 | — |

### 最小完整示例

```lay
# Minimal complete example: abs
page=canvas(size=(100mm,75mm),background="#ffffff")
page.add(rect(size=(abs(-30mm),20mm),fill="#087f8c"),offset=(10mm,10mm))
```

[概念与常见错误](topics/values.zh-CN.md#abs) · [组合源码](../examples/gallery/scripting/units-builtins.lay)

## min

返回一个或多个兼容单位数值中的最小值。所有参与值须使用兼容类型，返回值保留单位。

返回：number | length

位置参数：`values`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `values` | number[] | 统计计算的观测值或柱高数据 | — |

### 最小完整示例

```lay
# Minimal complete example: min
page=canvas(size=(100mm,75mm),background="#ffffff")
page.add(rect(size=(min(30mm,45mm),20mm),fill="#087f8c"),offset=(10mm,10mm))
```

[概念与常见错误](topics/values.zh-CN.md#min) · [组合源码](../examples/gallery/scripting/units-builtins.lay)

## max

返回一个或多个兼容单位数值中的最大值。所有参与值须使用兼容类型，返回值保留单位。

返回：number | length

位置参数：`values`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `values` | number[] | 统计计算的观测值或柱高数据 | — |

### 最小完整示例

```lay
# Minimal complete example: max
page=canvas(size=(100mm,75mm),background="#ffffff")
page.add(rect(size=(max(30mm,45mm),20mm),fill="#087f8c"),offset=(10mm,10mm))
```

[概念与常见错误](topics/values.zh-CN.md#max) · [组合源码](../examples/gallery/scripting/units-builtins.lay)

## move_to

创建路径指令，在 x/y 开始一条新子路径，不绘制连接到上一子路径的线。只能放入 path(commands=...)。

返回：path-command

位置参数：`x`, `y`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `x` | length / 画布单位 | 终点水平坐标；采用当前几何单位 | — |
| `y` | length / 画布单位 | 终点竖直坐标；采用当前几何单位 | — |

### 最小完整示例

```lay
# Minimal complete example: move_to
page=canvas(size=(100mm,75mm),background="#ffffff")
page.add(path(commands=[move_to(0mm,10mm),line_to(30mm,10mm)],border_color="#087f8c",border_width=1mm),offset=(10mm,10mm))
```

[概念与常见错误](topics/shapes.zh-CN.md#move_to) · [组合源码](../examples/gallery/shapes/path-commands.lay)

## line_to

创建从当前节点到 x/y 的直线段指令。路径必须先有 move_to，节点与原始段身份保留。

返回：path-command

位置参数：`x`, `y`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `x` | length / 画布单位 | 终点水平坐标；采用当前几何单位 | — |
| `y` | length / 画布单位 | 终点竖直坐标；采用当前几何单位 | — |

### 最小完整示例

```lay
# Minimal complete example: line_to
page=canvas(size=(100mm,75mm),background="#ffffff")
page.add(path(commands=[move_to(0mm,10mm),line_to(30mm,10mm)],border_color="#087f8c",border_width=1mm),offset=(10mm,10mm))
```

[概念与常见错误](topics/shapes.zh-CN.md#line_to) · [组合源码](../examples/gallery/shapes/path-commands.lay)

## quad_to

创建二次贝塞尔段，cx/cy 是控制点，x/y 是终点。控制点通常不在实际曲线上，t 与弧长比例不同。

返回：path-command

位置参数：`cx`, `cy`, `x`, `y`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `cx` | length / 画布单位 | 原始控制点 cx；采用当前几何单位 | — |
| `cy` | length / 画布单位 | 原始控制点 cy；采用当前几何单位 | — |
| `x` | length / 画布单位 | 终点水平坐标；采用当前几何单位 | — |
| `y` | length / 画布单位 | 终点竖直坐标；采用当前几何单位 | — |

### 最小完整示例

```lay
# Minimal complete example: quad_to
page=canvas(size=(100mm,75mm),background="#ffffff")
page.add(path(commands=[move_to(0mm,20mm),quad_to(15mm,0mm,30mm,20mm)],border_color="#087f8c",border_width=1mm),offset=(10mm,10mm))
```

[概念与常见错误](topics/shapes.zh-CN.md#quad_to) · [组合源码](../examples/gallery/shapes/path-commands.lay)

## cubic_to

创建三次贝塞尔段，两个控制点与终点按位置参数给出。查询 controls 时保留原始控制点，不按渲染细分改变索引。

返回：path-command

位置参数：`c1x`, `c1y`, `c2x`, `c2y`, `x`, `y`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `c1x` | length / 画布单位 | 原始控制点 c1x；采用当前几何单位 | — |
| `c1y` | length / 画布单位 | 原始控制点 c1y；采用当前几何单位 | — |
| `c2x` | length / 画布单位 | 原始控制点 c2x；采用当前几何单位 | — |
| `c2y` | length / 画布单位 | 原始控制点 c2y；采用当前几何单位 | — |
| `x` | length / 画布单位 | 终点水平坐标；采用当前几何单位 | — |
| `y` | length / 画布单位 | 终点竖直坐标；采用当前几何单位 | — |

### 最小完整示例

```lay
# Minimal complete example: cubic_to
page=canvas(size=(100mm,75mm),background="#ffffff")
page.add(path(commands=[move_to(0mm,20mm),cubic_to(10mm,0mm,20mm,35mm,40mm,20mm)],border_color="#087f8c",border_width=1mm),offset=(10mm,10mm))
```

[概念与常见错误](topics/shapes.zh-CN.md#cubic_to) · [组合源码](../examples/gallery/shapes/path-commands.lay)

## arc_to

创建椭圆弧路径段，参数包含半径、旋转、large_arc/sweep 布尔标志和终点。一个指令保持一条原始逻辑曲线段。

返回：path-command

位置参数：`rx`, `ry`, `rotation`, `large_arc`, `sweep`, `x`, `y`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `rx` | length / 画布单位 | 椭圆水平半径；采用当前几何单位 | — |
| `ry` | length / 画布单位 | 椭圆竖直半径；采用当前几何单位 | — |
| `rotation` | angle / deg | 椭圆局部旋转；deg/rad，裸值按度 | 0deg |
| `large_arc` | boolean | 是否选择大弧 | — |
| `sweep` | boolean | 是否沿页面正角度方向遍历 | — |
| `x` | length / 画布单位 | 终点水平坐标；采用当前几何单位 | — |
| `y` | length / 画布单位 | 终点竖直坐标；采用当前几何单位 | — |

### 最小完整示例

```lay
# Minimal complete example: arc_to
page=canvas(size=(100mm,75mm),background="#ffffff")
page.add(path(commands=[move_to(0mm,20mm),arc_to(20mm,15mm,0deg,false,true,40mm,20mm)],border_color="#087f8c",border_width=1mm),offset=(10mm,10mm))
```

[概念与常见错误](topics/shapes.zh-CN.md#arc_to) · [组合源码](../examples/gallery/shapes/path-commands.lay)

## close

创建闭合当前子路径的指令，连接末节点到起点并保留遍历方向。闭合路径首尾可重合，跨接缝区间须显式 wrap=true。

返回：path-command

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |

### 最小完整示例

```lay
# Minimal complete example: close
page=canvas(size=(100mm,75mm),background="#ffffff")
page.add(path(commands=[move_to(0mm,0mm),line_to(30mm,0mm),line_to(15mm,20mm),close()],border_color="#087f8c",border_width=1mm),offset=(10mm,10mm))
```

[概念与常见错误](topics/shapes.zh-CN.md#close) · [组合源码](../examples/gallery/shapes/evenodd-hole.lay)

## plot.legend

在尚未放置的图表中添加图例装饰，默认使用带 label 的图层。position 属于图表局部坐标；独立图例使用 legend(layers=...)。

返回：decoration

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `layers` | plot-layer[] | 需要共享图例的已创建图层列表 | — |
| `style` | plot_style | plot_style(...) 配置对象；仅提供未被显式参数覆盖的默认值 | — |
| `position` | "top_left" \| "top_center" \| "top_right" \| "middle_left" \| "center" \| "middle_right" \| "bottom_left" \| "bottom_center" \| "bottom_right" \| (length,length) | 图例位置、统计组数据位置或色标位置；由所属对象决定<br>`top_left`: 左上角，对应布局框<br>`top_center`: 上边中点，对应布局框<br>`top_right`: 右上角，对应布局框<br>`middle_left`: 左边中点，对应布局框<br>`center`: 中心对齐<br>`middle_right`: 右边中点，对应布局框<br>`bottom_left`: 左下角，对应布局框<br>`bottom_center`: 下边中点，对应布局框<br>`bottom_right`: 右下角，对应布局框 | top_right |
| `columns` | integer | 图例列数，正整数，按行排列 | 1 |
| `font_size` | length / pt | 字号；省略单位时为 pt；未指定时继承 | 继承 / inherit |
| `background` | paint | 对象区域的底色或填充，不是文字或轴线颜色 | #ffffff |
| `frame` | enum \| boolean | 图表外框 axes/box/none；图例为布尔开关 | false |
| `title` | string \| text \| formula | 图例标题，支持文字或公式 | — |
| `sample_width` | length / 画布单位 | 图例符号的物理宽度 | 6mm |
| `gap` | length / 画布单位 | 图例条目间距或色标与绘图区的物理间隔 | — |
| `padding` | length \| length[] / 画布单位 | 内部留白；一个值或上、右、下、左四个值 | 1.2mm |
| `sample_gap` | length / 画布单位 | 图例符号和文字的物理间隔 | — |
| `class` | string | 空格分隔的 LCSS 类名 | "" |
| `border_color` | color | 封闭轮廓颜色，none 表示无边框 | none |
| `border_width` | length / pt | 封闭轮廓线宽；裸值为 pt | — |
| `border_style` | "none" \| "solid" \| "dashed" \| "dotted" \| "dash_dot" \| "double" \| "triple" | 边框线型；与填充独立<br>`none`: 不绘制此项<br>`solid`: 连续实线<br>`dashed`: 虚线；可用 dash 参数覆盖长度<br>`dotted`: 点线<br>`dash_dot`: 点划线<br>`double`: 双线描边<br>`triple`: 三线描边 | solid |
| `border_dash` | length[] / pt | 交替的边框线段和间隔长度列表；须成对 | — |
| `border_dash_offset` | length / pt | 虚线相位；裸值为 pt | 0 |
| `border_cap` | "butt" \| "round" \| "square" | 轮廓虚线端点形状<br>`butt`: 平端帽，不向端点外延伸<br>`round`: 圆端帽，向外延伸半个线宽<br>`square`: 方端帽，向外延伸半个线宽 | butt |
| `border_join` | "miter" \| "round" \| "bevel" | 轮廓交点形状<br>`miter`: 延长边缘形成尖角，受尖角限制约束<br>`round`: 以圆弧连接相邻描边边缘<br>`bevel`: 以平边截去连接尖角 | miter |
| `border_miter_limit` | value | 尖角的最大延伸与线宽之比 | 4 |
| `border_opacity` | value | 边框独立透明度 0–1 | 1 |

### 最小完整示例

```lay
# Minimal complete example: plot.legend
page=canvas(size=(100mm,75mm),background="#ffffff")
p=plot(size=(86mm,62mm),x=axis(range=(0,4)),y=axis(range=(0,5)))
p.line(x=[0,1,2,3],y=[1,2,4,3],label="Signal")
p.legend(position=top_left)
page.add(p,offset=(7mm,6mm))
```

[概念与常见错误](topics/legends.zh-CN.md#plot-legend) · [组合源码](../examples/plot/first-plot.lay)

## plot.colorbar

为图表图层添加内部色标装饰。传入图层引用确定映射，位置与长度在图表局部确定；共享独立色标使用 colorbar(scale=...)。

返回：decoration

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `scale` | "linear" \| "log" \| "symlog" | 坐标变换 linear/log/symlog，或共享色标对象<br>`linear`: 线性映射<br>`log`: 对数映射；数值必须为正<br>`symlog`: 零附近线性、远离零时对数的映射 | — |
| `style` | plot_style | plot_style(...) 配置对象；仅提供未被显式参数覆盖的默认值 | — |
| `notation` | "plain" \| "scientific" \| "offset" | 数字显示 plain/scientific/offset，不改变数据坐标<br>`plain`: 直接显示刻度数字<br>`scientific`: 使用科学计数法显示刻度<br>`offset`: 使用公共偏移量显示刻度 | plain |
| `format` | value | D3 数字格式，例如 .2f 或 .2e | — |
| `exponent` | value | offset 记数法的十进制指数；整数 | — |
| `exponent_offset` | length / 画布单位 | 倍率公式相对默认位置的物理偏移 | (0,0) |
| `label` | string \| text \| formula | 标签内容；支持普通文字、混合公式或文字素材 | — |
| `label_offset` | length / 画布单位 | 轴或色标标题相对默认位置的物理偏移 | (0,0) |
| `position` | "left" \| "right" \| "top" \| "bottom" \| (length,length) | 图例位置、统计组数据位置或色标位置；由所属对象决定<br>`left`: 左侧或左对齐，依参数用途<br>`right`: 右侧或右对齐，依参数用途<br>`top`: 上侧<br>`bottom`: 下侧 | — |
| `orientation` | "vertical" \| "horizontal" | vertical/horizontal 方向<br>`vertical`: 纵向<br>`horizontal`: 横向 | vertical |
| `length` | length / 画布单位 | 色条的物理长度 | — |
| `thickness` | length / 画布单位 | 色条的物理厚度 | 3mm |
| `gap` | length / 画布单位 | 图例条目间距或色标与绘图区的物理间隔 | — |
| `ticks` | number[] | 递增且无重复的主刻度数据值 | — |
| `class` | string | 空格分隔的 LCSS 类名 | "" |
| `background` | paint | 对象区域的底色或填充，不是文字或轴线颜色 | none |
| `padding` | length \| length[] / 画布单位 | 内部留白；一个值或上、右、下、左四个值 | 0 |
| `border_radius` | length / 画布单位 | 底框圆角的物理半径 | 0 |
| `border_color` | color | 封闭轮廓颜色，none 表示无边框 | none |
| `border_width` | length / pt | 封闭轮廓线宽；裸值为 pt | — |
| `border_style` | "none" \| "solid" \| "dashed" \| "dotted" \| "dash_dot" \| "double" \| "triple" | 边框线型；与填充独立<br>`none`: 不绘制此项<br>`solid`: 连续实线<br>`dashed`: 虚线；可用 dash 参数覆盖长度<br>`dotted`: 点线<br>`dash_dot`: 点划线<br>`double`: 双线描边<br>`triple`: 三线描边 | solid |
| `border_dash` | length[] / pt | 交替的边框线段和间隔长度列表；须成对 | — |
| `border_dash_offset` | length / pt | 虚线相位；裸值为 pt | 0 |
| `border_cap` | "butt" \| "round" \| "square" | 轮廓虚线端点形状<br>`butt`: 平端帽，不向端点外延伸<br>`round`: 圆端帽，向外延伸半个线宽<br>`square`: 方端帽，向外延伸半个线宽 | butt |
| `border_join` | "miter" \| "round" \| "bevel" | 轮廓交点形状<br>`miter`: 延长边缘形成尖角，受尖角限制约束<br>`round`: 以圆弧连接相邻描边边缘<br>`bevel`: 以平边截去连接尖角 | miter |
| `border_miter_limit` | value | 尖角的最大延伸与线宽之比 | 4 |
| `border_opacity` | value | 边框独立透明度 0–1 | 1 |

### 最小完整示例

```lay
# Minimal complete example: plot.colorbar
page=canvas(size=(100mm,75mm),background="#ffffff")
p=plot(size=(86mm,62mm),x=axis(range=(0,4)),y=axis(range=(0,5)))
layer=p.heatmap(z=[[1,2],[3,4]],extent=(0,4,0,5))
p.colorbar(layer,label="Value")
page.add(p,offset=(7mm,6mm))
```

[概念与常见错误](topics/legends.zh-CN.md#plot-colorbar) · [组合源码](../examples/plot/colorbars.lay)

## ray

创建有起点和非零方向的查询射线。origin 与目标路径必须在同一容器；direction 在路径所选测量空间中解释。

返回：ray

必需：`origin`, `direction`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `origin` | "lower" \| "upper" | 容器中的锚点或坐标<br>`lower`: 第一行在下方<br>`upper`: 第一行在上方 | — |
| `direction` | (number, number) | 所选测量空间中的非零方向 | — |

### 最小完整示例

```lay
# Minimal complete example: ray
page=canvas(size=(100mm,75mm),background="#ffffff")
curve=page.add(path(commands=[move_to(0mm,0mm),cubic_to(0mm,0mm,0mm,35mm,50mm,35mm),cubic_to(65mm,35mm,70mm,0mm,75mm,0mm),line_to(82mm,12mm)],border_color="#0072b2",border_width=0.7mm),offset=(8mm,15mm))
route=curve.path.subpaths[0]
dot=ellipse(size=(2mm,2mm),fill="#e36b70")
candidates=route.intersections(ray(origin=(0mm,30mm),direction=(1,0)))
for point in candidates { page.add(dot,anchor=center,target=point) }
page.add(text("Candidates: "+str(len(candidates))),offset=(8mm,65mm))
```

[概念与常见错误](topics/anchors.zh-CN.md#ray) · [组合源码](../examples/manual/geometry-composition.lay)

## path.at

按弧长比例或距离选点；先选 segments[i] 才能使用原始参数 t。默认按放置后的路径测量。

返回：path_anchor

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `fraction` | number | 弧长比例，0–1 | — |
| `distance` | length / 画布单位 | 从所选路径起点量出的非负弧长，不得超过路径长度；与 fraction/t 互斥 | — |
| `t` | number | 曲线段的原始参数，0–1 | — |

### 最小完整示例

```lay
# Minimal complete example: path.at
page=canvas(size=(100mm,75mm),background="#ffffff")
curve=page.add(path(commands=[move_to(0mm,0mm),cubic_to(0mm,0mm,0mm,35mm,50mm,35mm),cubic_to(65mm,35mm,70mm,0mm,75mm,0mm),line_to(82mm,12mm)],border_color="#0072b2",border_width=0.7mm),offset=(8mm,15mm))
route=curve.path.subpaths[0]
dot=ellipse(size=(2mm,2mm),fill="#e36b70")
point=route.at(fraction=0.5)
page.add(dot,anchor=center,target=point)
```

[概念与常见错误](topics/anchors.zh-CN.md#path-at) · [组合源码](../examples/manual/geometry-composition.lay)

## path.nearest

返回所有全局最近路径位置，按子路径、源段和参数排序。必须索引；无限多个最近点报错。

返回：anchor_collection

必需：`to`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `to` | anchor \| (length, length) | 同一容器中的参考锚点或坐标 | — |

### 最小完整示例

```lay
# Minimal complete example: path.nearest
page=canvas(size=(100mm,75mm),background="#ffffff")
curve=page.add(path(commands=[move_to(0mm,0mm),cubic_to(0mm,0mm,0mm,35mm,50mm,35mm),cubic_to(65mm,35mm,70mm,0mm,75mm,0mm),line_to(82mm,12mm)],border_color="#0072b2",border_width=0.7mm),offset=(8mm,15mm))
route=curve.path.subpaths[0]
dot=ellipse(size=(2mm,2mm),fill="#e36b70")
candidates=route.nearest(to=curve.bounds.top_left)
for point in candidates { page.add(dot,anchor=center,target=point) }
page.add(text("Candidates: "+str(len(candidates))),offset=(8mm,65mm))
```

[概念与常见错误](topics/anchors.zh-CN.md#path-nearest) · [组合源码](../examples/manual/geometry-composition.lay)

## path.extrema

返回所选空间中的局部坐标极值，不含普通端点或常量区间。

返回：anchor_collection

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `axis` | "x" \| "y" | 查询 x 或 y 极值<br>`x`: 所选空间的横坐标<br>`y`: 所选空间的纵坐标 | "y" |

### 最小完整示例

```lay
# Minimal complete example: path.extrema
page=canvas(size=(100mm,75mm),background="#ffffff")
curve=page.add(path(commands=[move_to(0mm,0mm),cubic_to(0mm,0mm,0mm,35mm,50mm,35mm),cubic_to(65mm,35mm,70mm,0mm,75mm,0mm),line_to(82mm,12mm)],border_color="#0072b2",border_width=0.7mm),offset=(8mm,15mm))
route=curve.path.subpaths[0]
dot=ellipse(size=(2mm,2mm),fill="#e36b70")
candidates=route.extrema(axis=y)
for point in candidates { page.add(dot,anchor=center,target=point) }
page.add(text("Candidates: "+str(len(candidates))),offset=(8mm,65mm))
```

[概念与常见错误](topics/anchors.zh-CN.md#path-extrema) · [组合源码](../examples/manual/geometry-composition.lay)

## path.inflections

返回曲率改变符号且方向平滑的拐点。

返回：anchor_collection

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |

### 最小完整示例

```lay
# Minimal complete example: path.inflections
page=canvas(size=(100mm,75mm),background="#ffffff")
curve=page.add(path(commands=[move_to(0mm,0mm),cubic_to(0mm,0mm,0mm,35mm,50mm,35mm),cubic_to(65mm,35mm,70mm,0mm,75mm,0mm),line_to(82mm,12mm)],border_color="#0072b2",border_width=0.7mm),offset=(8mm,15mm))
route=curve.path.subpaths[0]
dot=ellipse(size=(2mm,2mm),fill="#e36b70")
candidates=route.inflections()
for point in candidates { page.add(dot,anchor=center,target=point) }
page.add(text("Candidates: "+str(len(candidates))),offset=(8mm,65mm))
```

[概念与常见错误](topics/anchors.zh-CN.md#path-inflections) · [组合源码](../examples/manual/geometry-composition.lay)

## path.corners

返回方向不平滑的连接节点；方向需 with_side 明确选择。

返回：anchor_collection

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |

### 最小完整示例

```lay
# Minimal complete example: path.corners
page=canvas(size=(100mm,75mm),background="#ffffff")
curve=page.add(path(commands=[move_to(0mm,0mm),cubic_to(0mm,0mm,0mm,35mm,50mm,35mm),cubic_to(65mm,35mm,70mm,0mm,75mm,0mm),line_to(82mm,12mm)],border_color="#0072b2",border_width=0.7mm),offset=(8mm,15mm))
route=curve.path.subpaths[0]
dot=ellipse(size=(2mm,2mm),fill="#e36b70")
candidates=route.corners()
for point in candidates { page.add(dot,anchor=center,target=point) }
page.add(text("Candidates: "+str(len(candidates))),offset=(8mm,65mm))
```

[概念与常见错误](topics/anchors.zh-CN.md#path-corners) · [组合源码](../examples/manual/geometry-composition.lay)

## path.intersections

返回与射线相交的全部路径位置；自交处保留不同路径位置，连续重合报错。

返回：anchor_collection

位置参数：`ray`.

必需：`ray`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `ray` | ray | 使用 ray(...) 创建的射线 | — |

### 最小完整示例

```lay
# Minimal complete example: path.intersections
page=canvas(size=(100mm,75mm),background="#ffffff")
curve=page.add(path(commands=[move_to(0mm,0mm),cubic_to(0mm,0mm,0mm,35mm,50mm,35mm),cubic_to(65mm,35mm,70mm,0mm,75mm,0mm),line_to(82mm,12mm)],border_color="#0072b2",border_width=0.7mm),offset=(8mm,15mm))
route=curve.path.subpaths[0]
dot=ellipse(size=(2mm,2mm),fill="#e36b70")
candidates=route.intersections(ray(origin=(0mm,30mm),direction=(1,0)))
for point in candidates { page.add(dot,anchor=center,target=point) }
page.add(text("Candidates: "+str(len(candidates))),offset=(8mm,65mm))
```

[概念与常见错误](topics/anchors.zh-CN.md#path-intersections) · [组合源码](../examples/manual/geometry-composition.lay)

## path.between

沿原路径顺序选择两个路径锚点间的连续区间；端点必须来自同一实例和子路径。

返回：geometry_path

位置参数：`start`, `end`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `start` | path_anchor | 区间起始路径位置 | — |
| `end` | path_anchor | 区间结束路径位置 | — |
| `wrap` | boolean | 跨闭合接缝时显式设为 true | false |

### 最小完整示例

```lay
# Minimal complete example: path.between
page=canvas(size=(100mm,75mm),background="#ffffff")
curve=page.add(path(commands=[move_to(0mm,0mm),cubic_to(0mm,0mm,0mm,35mm,50mm,35mm),cubic_to(65mm,35mm,70mm,0mm,75mm,0mm),line_to(82mm,12mm)],border_color="#0072b2",border_width=0.7mm),offset=(8mm,15mm))
route=curve.path.subpaths[0]
dot=ellipse(size=(2mm,2mm),fill="#e36b70")
point=route.between(route.nodes[1],route.nodes[3]).at(fraction=0.5)
page.add(dot,anchor=center,target=point)
```

[概念与常见错误](topics/anchors.zh-CN.md#path-between) · [组合源码](../examples/manual/geometry-composition.lay)

## path.in_space

切换弧长、最近点和特征查询的测量空间；结果仍是当前容器中的锚点。

返回：geometry_path

位置参数：`space`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `space` | "local" \| "parent" | local 为原始局部路径，parent 为当前容器<br>`local`: 按原始局部几何测量，结果仍返回容器坐标<br>`parent`: 按放置后的容器坐标几何测量 | — |

### 最小完整示例

```lay
# Minimal complete example: path.in_space
page=canvas(size=(100mm,75mm),background="#ffffff")
curve=page.add(path(commands=[move_to(0mm,0mm),cubic_to(0mm,0mm,0mm,35mm,50mm,35mm),cubic_to(65mm,35mm,70mm,0mm,75mm,0mm),line_to(82mm,12mm)],border_color="#0072b2",border_width=0.7mm),offset=(8mm,15mm))
route=curve.path.subpaths[0]
dot=ellipse(size=(2mm,2mm),fill="#e36b70")
point=route.in_space(local).at(fraction=0.5)
page.add(dot,anchor=center,target=point)
```

[概念与常见错误](topics/anchors.zh-CN.md#path-in_space) · [组合源码](../examples/manual/geometry-composition.lay)

## anchor.with_side

选择尖角的 incoming 或 outgoing 方向；正法线为沿遍历方向的左侧。

返回：path_anchor

位置参数：`side`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `side` | "incoming" \| "outgoing" | 明确指定入射或出射方向<br>`incoming`: 使用路径位置的入射切线<br>`outgoing`: 使用路径位置的出射切线 | — |

### 最小完整示例

```lay
# Minimal complete example: anchor.with_side
page=canvas(size=(100mm,75mm),background="#ffffff")
curve=page.add(path(commands=[move_to(0mm,0mm),cubic_to(0mm,0mm,0mm,35mm,50mm,35mm),cubic_to(65mm,35mm,70mm,0mm,75mm,0mm),line_to(82mm,12mm)],border_color="#0072b2",border_width=0.7mm),offset=(8mm,15mm))
route=curve.path.subpaths[0]
dot=ellipse(size=(2mm,2mm),fill="#e36b70")
corner=route.corners()[0].with_side(incoming)
page.add(line(length=8mm,line_color="#e36b70"),anchor=self.path.start,target=corner,rotation=corner.tangent_angle)
```

[概念与常见错误](topics/anchors.zh-CN.md#anchor-with_side) · [组合源码](../examples/manual/geometry-composition.lay)

## head

创建可复用的端点头部配置，不能单独放置。size 是沿朝向与横向的物理尺寸。

返回：head

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `shape` | "triangle" \| "open" \| "stealth" \| "dot" \| "diamond" \| "bar" | 预置头部形状<br>`triangle`: 闭合三角头部，尖端与逻辑端点重合<br>`open`: 开放 V 形头部，内部保留线身<br>`stealth`: 闭合内凹头部，尖端与逻辑端点重合<br>`dot`: 以端点为中心的椭圆头部<br>`diamond`: 以端点为中心的菱形头部<br>`bar`: 以端点为中心的矩形头部 | triangle |
| `size` | (length,length) / 画布单位 | 头部尺寸，不按线长缩小；省略时根据线宽决定 | — |
| `fill` | color | 内部颜色；闭合头部默认继承线色，open 默认 none | — |
| `border_color` | color | 描边颜色；open 默认继承线色 | — |
| `border_width` | length / pt | 描边宽度；open 默认继承线宽 | — |
| `opacity` | value | 头部透明度，0–1 | 1 |

### 最小完整示例

```lay
# Minimal complete example: head
page=canvas(size=(100mm,75mm),background="#ffffff")
h=head(shape=diamond,size=(6mm,5mm),fill="#e36b70")
page.add(line(length=50mm,end_head=h,line_width=1.5mm),offset=(12mm,25mm))
```

[概念与常见错误](topics/shapes.zh-CN.md#head) · [组合源码](../examples/basic.lay)

## rgb

创建 RGB 颜色；支持 alpha=0–1。

返回：color

位置参数：`r`, `g`, `b`.

必需：`r`, `g`, `b`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `r` | number | sRGB r 通道，0–255；允许小数，透明度单独指定 | — |
| `g` | number | sRGB g 通道，0–255；允许小数，透明度单独指定 | — |
| `b` | number | sRGB b 通道，0–255；允许小数，透明度单独指定 | — |
| `alpha` | number | 颜色透明度，0–1 | 1 |

### 最小完整示例

```lay
# Minimal complete example: rgb
page=canvas(size=(100mm,75mm),background="#ffffff")
page.add(rect(size=(60mm,30mm),fill=rgb(255,80,40,alpha=0.6)),offset=(10mm,10mm))
```

[概念与常见错误](topics/fills.zh-CN.md#rgb) · [组合源码](../examples/gallery/shapes/colors-alpha.lay)

## hsv

创建 HSV 颜色；支持 alpha=0–1。

返回：color

位置参数：`h`, `s`, `v`.

必需：`h`, `s`, `v`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `h` | angle / deg | 色相；支持 deg/rad，裸值按度；按 360° 周期归一化 | — |
| `s` | number | 饱和度，0–1；面板显示为百分数 | — |
| `v` | number | 明度，0–1；面板显示为百分数 | — |
| `alpha` | number | 颜色透明度，0–1 | 1 |

### 最小完整示例

```lay
# Minimal complete example: hsv
page=canvas(size=(100mm,75mm),background="#ffffff")
page.add(rect(size=(60mm,30mm),fill=hsv(200deg,0.7,0.8,alpha=0.6)),offset=(10mm,10mm))
```

[概念与常见错误](topics/fills.zh-CN.md#hsv) · [组合源码](../examples/gallery/shapes/colors-alpha.lay)

## oklch

创建 OKLCH 颜色；支持 alpha=0–1。

返回：color

位置参数：`l`, `c`, `h`.

必需：`l`, `c`, `h`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `l` | number | 感知明度，0–1；面板显示为百分数 | — |
| `c` | number | 色度，必须非负；超出 sRGB 色域时保持明度与色相并压缩色度 | — |
| `h` | angle / deg | 色相；支持 deg/rad，裸值按度；按 360° 周期归一化 | — |
| `alpha` | number | 颜色透明度，0–1 | 1 |

### 最小完整示例

```lay
# Minimal complete example: oklch
page=canvas(size=(100mm,75mm),background="#ffffff")
page.add(rect(size=(60mm,30mm),fill=oklch(0.7,0.15,200deg,alpha=0.6)),offset=(10mm,10mm))
```

[概念与常见错误](topics/fills.zh-CN.md#oklch) · [组合源码](../examples/gallery/shapes/colors-alpha.lay)

## dict

创建空字典，或从 JSON 文件加载有序嵌套字典；不放宽 table 的列校验。

返回：dict

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `src` | string | JSON 对象文件路径 | — |

### 最小完整示例

```lay
# Ordered value dictionaries, safe updates and pair unpacking.
page=canvas(size=(100mm,70mm),background="#ffffff")
d=dict()
d["B"]={"label":"Beta","color":"#0072b2"}
d["A"]={"label":"Alpha","color":"#d55e00"}
copy=d
copy["B"]["label"]="Copy"
d=d.update({"C":{"label":"Gamma","color":"#009e73"}})
keys=d.keys()
values=d.values()
items=d.items()
fallback=d.get("missing",{"label":"Default"})
for i,(name,value) in enumerate(items) {
 if name in d {
  page.add(rect(size=(18mm,8mm),fill=value["color"]),offset=(8mm,8mm+i*18mm))
  page.add(text(content=f"{name}: {value['label']}",font_family="DejaVu Sans",font_size=10pt),offset=(30mm,8mm+i*18mm))
 }
}
```

[概念与常见错误](topics/values.zh-CN.md#dict) · [组合源码](../examples/gallery/scripting/dictionaries.lay)

## enumerate

生成索引和值的列表，支持循环解包；start 默认 0，最多 10,000 项。

返回：list

位置参数：`seq`, `start`.

必需：`seq`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `seq` | iterable | 列表、字典或几何集合 | — |
| `start` | integer | 起始编号 | 0 |

### 最小完整示例

```lay
# Insertion order controls the layout; assigning a dictionary copies its values.
page=canvas(size=(150mm,90mm),background="#ffffff")
labels=dict()
labels["B"]="Baseline"
labels["A"]="Annealed"
copy=labels
copy["B"]="Changed copy"
labels=labels.update({"C":"Cold worked"})
keys=labels.keys()
values=labels.values()
items=labels.items()
default=labels.get("missing","No data")
colors=["#0072b2","#d55e00","#009e73"]
for i,(key,value) in enumerate(items) {
 card=group()
 card.add(rect(size=(42mm,55mm),fill=colors[i]))
 card.add(text(content=key,font_family="DejaVu Sans",font_size=20pt,color="#ffffff"),offset=(5mm,7mm))
 card.add(text(content=value,font_family="DejaVu Sans",font_size=8pt,color="#ffffff"),offset=(5mm,35mm))
 page.add(card,offset=(5mm+i*48mm,10mm))
}
for i,(key,value) in enumerate(zip(keys,values)) {
 page.add(text(content=f"{i+1}. {key} = {value}",font_family="DejaVu Sans",font_size=8pt),offset=(5mm+i*48mm,72mm))
}
```

[概念与常见错误](topics/control-flow.zh-CN.md#enumerate) · [组合源码](../examples/gallery/scripting/dictionaries.lay)

## zip

将位置参数中的可迭代集合逐项组合，在最短输入结束时停止；零参数返回空列表。

返回：list

位置参数：`sequences`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `sequences` | iterable... | 多个列表、字典或几何集合 | — |

### 最小完整示例

```lay
# Insertion order controls the layout; assigning a dictionary copies its values.
page=canvas(size=(150mm,90mm),background="#ffffff")
labels=dict()
labels["B"]="Baseline"
labels["A"]="Annealed"
copy=labels
copy["B"]="Changed copy"
labels=labels.update({"C":"Cold worked"})
keys=labels.keys()
values=labels.values()
items=labels.items()
default=labels.get("missing","No data")
colors=["#0072b2","#d55e00","#009e73"]
for i,(key,value) in enumerate(items) {
 card=group()
 card.add(rect(size=(42mm,55mm),fill=colors[i]))
 card.add(text(content=key,font_family="DejaVu Sans",font_size=20pt,color="#ffffff"),offset=(5mm,7mm))
 card.add(text(content=value,font_family="DejaVu Sans",font_size=8pt,color="#ffffff"),offset=(5mm,35mm))
 page.add(card,offset=(5mm+i*48mm,10mm))
}
for i,(key,value) in enumerate(zip(keys,values)) {
 page.add(text(content=f"{i+1}. {key} = {value}",font_family="DejaVu Sans",font_size=8pt),offset=(5mm+i*48mm,72mm))
}
```

[概念与常见错误](topics/control-flow.zh-CN.md#zip) · [组合源码](../examples/gallery/scripting/dictionaries.lay)

## cmap

获取 Matplotlib 3.11.2 命名配色对象；不可变，可取色、获取颜色列表和反向。

返回：cmap

位置参数：`name`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `name` | string | 预设名称，支持 _r 和原生别名 | viridis |

### 最小完整示例

```lay
# Preset discovery, sampling, colors and immutable reversal.
page=canvas(size=(100mm,60mm),background="#ffffff")
cm=cmap("viridis")
names=cmap_names(category="sequential")
colors=cm.colors(8)
reverse=cm.reversed()
accent=cm.sample(0.5)
cycle=palette("tab10",8)
for i,color in enumerate(colors) {
 page.add(rect(size=(10mm,12mm),fill=color),offset=(10mm+i*10mm,8mm))
 page.add(rect(size=(10mm,12mm),fill=reverse.sample(i/7)),offset=(10mm+i*10mm,24mm))
 page.add(rect(size=(10mm,8mm),fill=cycle[i]),offset=(10mm+i*10mm,40mm))
}
```

[概念与常见错误](topics/cmaps.zh-CN.md#cmap) · [组合源码](../examples/plot/cmap-presets.lay)

## cmap_names

查询独立配色预设，可按类别过滤并包含反向名称；不重复列出别名。

返回：list

位置参数：`category`, `reversed`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `category` | string \| null | sequential/diverging/cyclic/qualitative/misc；null 为全部 | null |
| `reversed` | boolean | 是否同时列出 _r 名称 | false |

### 最小完整示例

```lay
# Preset discovery, sampling, colors and immutable reversal.
page=canvas(size=(100mm,60mm),background="#ffffff")
cm=cmap("viridis")
names=cmap_names(category="sequential")
colors=cm.colors(8)
reverse=cm.reversed()
accent=cm.sample(0.5)
cycle=palette("tab10",8)
for i,color in enumerate(colors) {
 page.add(rect(size=(10mm,12mm),fill=color),offset=(10mm+i*10mm,8mm))
 page.add(rect(size=(10mm,12mm),fill=reverse.sample(i/7)),offset=(10mm+i*10mm,24mm))
 page.add(rect(size=(10mm,8mm),fill=cycle[i]),offset=(10mm+i*10mm,40mm))
}
```

[概念与常见错误](topics/cmaps.zh-CN.md#cmap_names) · [组合源码](../examples/plot/cmap-presets.lay)

## palette

生成配色列表；分类颜色按顺序轮换，连续均匀采样，循环配色不重复端点。

返回：list

位置参数：`name`, `n`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `name` | string | 预设名称 | tab10 |
| `n` | integer \| null | 0–10,000；null 使用原生颜色数量 | null |

### 最小完整示例

```lay
# Preset discovery, sampling, colors and immutable reversal.
page=canvas(size=(100mm,60mm),background="#ffffff")
cm=cmap("viridis")
names=cmap_names(category="sequential")
colors=cm.colors(8)
reverse=cm.reversed()
accent=cm.sample(0.5)
cycle=palette("tab10",8)
for i,color in enumerate(colors) {
 page.add(rect(size=(10mm,12mm),fill=color),offset=(10mm+i*10mm,8mm))
 page.add(rect(size=(10mm,12mm),fill=reverse.sample(i/7)),offset=(10mm+i*10mm,24mm))
 page.add(rect(size=(10mm,8mm),fill=cycle[i]),offset=(10mm+i*10mm,40mm))
}
```

[概念与常见错误](topics/cmaps.zh-CN.md#palette) · [组合源码](../examples/plot/cmap-presets.lay)

## dict.keys

返回按插入顺序排列的键列表。

返回：list

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |

### 最小完整示例

```lay
# Ordered value dictionaries, safe updates and pair unpacking.
page=canvas(size=(100mm,70mm),background="#ffffff")
d=dict()
d["B"]={"label":"Beta","color":"#0072b2"}
d["A"]={"label":"Alpha","color":"#d55e00"}
copy=d
copy["B"]["label"]="Copy"
d=d.update({"C":{"label":"Gamma","color":"#009e73"}})
keys=d.keys()
values=d.values()
items=d.items()
fallback=d.get("missing",{"label":"Default"})
for i,(name,value) in enumerate(items) {
 if name in d {
  page.add(rect(size=(18mm,8mm),fill=value["color"]),offset=(8mm,8mm+i*18mm))
  page.add(text(content=f"{name}: {value['label']}",font_family="DejaVu Sans",font_size=10pt),offset=(30mm,8mm+i*18mm))
 }
}
```

[概念与常见错误](topics/values.zh-CN.md#dict-keys) · [组合源码](../examples/gallery/scripting/dictionaries.lay)

## dict.values

返回按插入顺序排列的值列表；保留单位和对象类型。

返回：list

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |

### 最小完整示例

```lay
# Ordered value dictionaries, safe updates and pair unpacking.
page=canvas(size=(100mm,70mm),background="#ffffff")
d=dict()
d["B"]={"label":"Beta","color":"#0072b2"}
d["A"]={"label":"Alpha","color":"#d55e00"}
copy=d
copy["B"]["label"]="Copy"
d=d.update({"C":{"label":"Gamma","color":"#009e73"}})
keys=d.keys()
values=d.values()
items=d.items()
fallback=d.get("missing",{"label":"Default"})
for i,(name,value) in enumerate(items) {
 if name in d {
  page.add(rect(size=(18mm,8mm),fill=value["color"]),offset=(8mm,8mm+i*18mm))
  page.add(text(content=f"{name}: {value['label']}",font_family="DejaVu Sans",font_size=10pt),offset=(30mm,8mm+i*18mm))
 }
}
```

[概念与常见错误](topics/values.zh-CN.md#dict-values) · [组合源码](../examples/gallery/scripting/dictionaries.lay)

## dict.items

返回按插入顺序排列的键值对列表，可用于循环解包。

返回：list

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |

### 最小完整示例

```lay
# Ordered value dictionaries, safe updates and pair unpacking.
page=canvas(size=(100mm,70mm),background="#ffffff")
d=dict()
d["B"]={"label":"Beta","color":"#0072b2"}
d["A"]={"label":"Alpha","color":"#d55e00"}
copy=d
copy["B"]["label"]="Copy"
d=d.update({"C":{"label":"Gamma","color":"#009e73"}})
keys=d.keys()
values=d.values()
items=d.items()
fallback=d.get("missing",{"label":"Default"})
for i,(name,value) in enumerate(items) {
 if name in d {
  page.add(rect(size=(18mm,8mm),fill=value["color"]),offset=(8mm,8mm+i*18mm))
  page.add(text(content=f"{name}: {value['label']}",font_family="DejaVu Sans",font_size=10pt),offset=(30mm,8mm+i*18mm))
 }
}
```

[概念与常见错误](topics/values.zh-CN.md#dict-items) · [组合源码](../examples/gallery/scripting/dictionaries.lay)

## dict.get

按字符串键取值；缺失时返回 default，默认 null。

返回：value

位置参数：`key`, `default`.

必需：`key`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `key` | string | 字典键 | — |
| `default` | value | 缺失键的默认值 | null |

### 最小完整示例

```lay
# Ordered value dictionaries, safe updates and pair unpacking.
page=canvas(size=(100mm,70mm),background="#ffffff")
d=dict()
d["B"]={"label":"Beta","color":"#0072b2"}
d["A"]={"label":"Alpha","color":"#d55e00"}
copy=d
copy["B"]["label"]="Copy"
d=d.update({"C":{"label":"Gamma","color":"#009e73"}})
keys=d.keys()
values=d.values()
items=d.items()
fallback=d.get("missing",{"label":"Default"})
for i,(name,value) in enumerate(items) {
 if name in d {
  page.add(rect(size=(18mm,8mm),fill=value["color"]),offset=(8mm,8mm+i*18mm))
  page.add(text(content=f"{name}: {value['label']}",font_family="DejaVu Sans",font_size=10pt),offset=(30mm,8mm+i*18mm))
 }
}
```

[概念与常见错误](topics/values.zh-CN.md#dict-get) · [组合源码](../examples/gallery/scripting/dictionaries.lay)

## dict.update

返回合并后的新字典；覆盖键保留位置，新键追加到末尾；原字典不变。

返回：dict

位置参数：`other`.

必需：`other`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `other` | dict | 需要合并的字典 | — |

### 最小完整示例

```lay
# Ordered value dictionaries, safe updates and pair unpacking.
page=canvas(size=(100mm,70mm),background="#ffffff")
d=dict()
d["B"]={"label":"Beta","color":"#0072b2"}
d["A"]={"label":"Alpha","color":"#d55e00"}
copy=d
copy["B"]["label"]="Copy"
d=d.update({"C":{"label":"Gamma","color":"#009e73"}})
keys=d.keys()
values=d.values()
items=d.items()
fallback=d.get("missing",{"label":"Default"})
for i,(name,value) in enumerate(items) {
 if name in d {
  page.add(rect(size=(18mm,8mm),fill=value["color"]),offset=(8mm,8mm+i*18mm))
  page.add(text(content=f"{name}: {value['label']}",font_family="DejaVu Sans",font_size=10pt),offset=(30mm,8mm+i*18mm))
 }
}
```

[概念与常见错误](topics/values.zh-CN.md#dict-update) · [组合源码](../examples/gallery/scripting/dictionaries.lay)

## cmap.sample

按有限归一化位置取色；范围外钳制到 0–1。

返回：color

位置参数：`t`.

必需：`t`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `t` | number | 有限归一化位置 | — |

### 最小完整示例

```lay
# Preset discovery, sampling, colors and immutable reversal.
page=canvas(size=(100mm,60mm),background="#ffffff")
cm=cmap("viridis")
names=cmap_names(category="sequential")
colors=cm.colors(8)
reverse=cm.reversed()
accent=cm.sample(0.5)
cycle=palette("tab10",8)
for i,color in enumerate(colors) {
 page.add(rect(size=(10mm,12mm),fill=color),offset=(10mm+i*10mm,8mm))
 page.add(rect(size=(10mm,12mm),fill=reverse.sample(i/7)),offset=(10mm+i*10mm,24mm))
 page.add(rect(size=(10mm,8mm),fill=cycle[i]),offset=(10mm+i*10mm,40mm))
}
```

[概念与常见错误](topics/cmaps.zh-CN.md#cmap-sample) · [组合源码](../examples/plot/cmap-presets.lay)

## cmap.colors

生成颜色列表；n 默认原生数量，分类轮换、连续均匀采样、循环不重复端点。

返回：list

位置参数：`n`.

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |
| `n` | integer \| null | 0–10,000；null 使用原生数量 | null |

### 最小完整示例

```lay
# Preset discovery, sampling, colors and immutable reversal.
page=canvas(size=(100mm,60mm),background="#ffffff")
cm=cmap("viridis")
names=cmap_names(category="sequential")
colors=cm.colors(8)
reverse=cm.reversed()
accent=cm.sample(0.5)
cycle=palette("tab10",8)
for i,color in enumerate(colors) {
 page.add(rect(size=(10mm,12mm),fill=color),offset=(10mm+i*10mm,8mm))
 page.add(rect(size=(10mm,12mm),fill=reverse.sample(i/7)),offset=(10mm+i*10mm,24mm))
 page.add(rect(size=(10mm,8mm),fill=cycle[i]),offset=(10mm+i*10mm,40mm))
}
```

[概念与常见错误](topics/cmaps.zh-CN.md#cmap-colors) · [组合源码](../examples/plot/cmap-presets.lay)

## cmap.reversed

返回反向配色对象，使用 Matplotlib 原生反向颜色表。

返回：cmap

| 参数 | 允许类型 / 单位 | 含义与逐项选值 | 默认 / 继承 |
| --- | --- | --- | --- |

### 最小完整示例

```lay
# Preset discovery, sampling, colors and immutable reversal.
page=canvas(size=(100mm,60mm),background="#ffffff")
cm=cmap("viridis")
names=cmap_names(category="sequential")
colors=cm.colors(8)
reverse=cm.reversed()
accent=cm.sample(0.5)
cycle=palette("tab10",8)
for i,color in enumerate(colors) {
 page.add(rect(size=(10mm,12mm),fill=color),offset=(10mm+i*10mm,8mm))
 page.add(rect(size=(10mm,12mm),fill=reverse.sample(i/7)),offset=(10mm+i*10mm,24mm))
 page.add(rect(size=(10mm,8mm),fill=cycle[i]),offset=(10mm+i*10mm,40mm))
}
```

[概念与常见错误](topics/cmaps.zh-CN.md#cmap-reversed) · [组合源码](../examples/plot/cmap-presets.lay)
