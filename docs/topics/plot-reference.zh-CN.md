# 绘图对象与方法

## 操作流程

尺寸分为三层，内容不会自动撑大页面或图表外框：

| 层级 | 控制方式 | 含义 |
| --- | --- | --- |
| 整页画布 | `canvas(size=(宽,高))` | 最终导出页面的固定尺寸 |
| 单个图表外框 | `plot(size=(宽,高))` | 包含绘图区、刻度、轴标题、倍率和色标 |
| 内部绘图区 | `margins` 或 `plot_area` | 数据实际映射到的矩形区域 |

图表内部的 padding 参数名是 `margins=(左,上,右,下)`：绘图区宽度为外框宽度减去左右留白，高度同理。`margins` 与 `plot_area` 二选一；两者都省略时才使用自动留白。

| 调用 | 参数与规则 |
| --- | --- |
| `plot(size=(w,h), ...)` | `size` 必填，为包含所有轴、标签和色标的外框；`x`、`y` 接受 `axis(...)`；`style` 接受 `plot_style(...)` |
| `plot(..., margins=(left,top,right,bottom))` | 可选，四个非负长度；省略时测量文字并自动留白；固定留白不足给出 `W_PLOT_LAYOUT` 并继续输出 |
| `plot(..., plot_area=box(offset=(left, top), size=(width, height)))` | 可选，锁定绘图区相对外框的位置和物理尺寸；与 `margins` 互斥；周围空间不足或超出外框给出 `W_PLOT_LAYOUT` |
| `axis(label="", scale="linear", ...)` | `scale` 为 `linear`、正值 `log` 或支持负数和零的 `symlog`；`range=(min,max)` 显式递增范围；`ticks=[...]` 为范围内严格递增刻度 |
| `axis(..., format=".2f")` | 可选 D3 数字格式，如 `.2f`、`.1e`、`.0%`；默认根据刻度选择格式 |
| `axis(..., label_offset=(dx,dy))` | 仅微调该轴标题，默认 `(0 mm,0 mm)`；有限物理长度，可为负值，适用于所有计数法 |
| `plot_style(...)` | `font_family` 为系统字体名称、文件路径或有序列表；`font_size=8 pt`；`line_width=0.6 pt`；轴/文字 `color="#222222"`；`colors=[...]` 自定义图层颜色循环 |

布局长度用 `mm/cm/in/pt/px`；数据用无单位数值，科学单位写在标签中。标记 `marker_size` 是包围框直径/边长，**不是面积**。自动线性范围在数据两侧加 5% 留白；含热图时自动范围取单元格最外侧边界。常量数据自动展开。对数范围在 log10 空间扩展。

`page.add(p,size=(..., auto))` 会重新计算图表布局，保持字号、线宽和标记尺寸；未覆盖的高度保留定义值。图表首次成功放置后封存，之后可再次放置，但不能继续添加图层、图例或色标。将图表放进 group 后再整体缩放 group，仍会缩放全部子内容。多面板的精确排版由用户指定各自的 `plot_area` 和绘图区锚点；没有自动对齐、等宽化或跨面板调整。

网格先绘制，图层按添加顺序绘制，随后绘制坐标轴和标签，最后绘制图例。网格和数据层分别裁剪到绘图区。图例支持四角定位或物理坐标定位，不执行自动避让。长标签、最终刻度仍重叠或图例放不下时给出 `W_PLOT_LAYOUT`，保留完整内容并继续输出。自动刻度仍会尝试减少数量，显式刻度保持原样；字号和固定绘图区不会自动缩小。

布局警告包含放置语句的源码位置；留白不足时还报告各侧大约缺少多少 mm。CLI 的 `validate` 和 `render` 将警告写入 stderr，仍可成功返回；Rust/WASM API 可读取 `scene.warnings`，Notebook 显示 Python warning。超出图表外框的内容允许绘制，但超出整张页面的部分仍会被导出页面边界裁剪。非正绘图区、非法轴范围、无效数据等无法正确计算的情况仍报错。

共同可选参数：`color="#0072B2"`、`line_width=0.6 pt`、`opacity=1`、`label=""`。填充带默认 `opacity=0.2`；其他图层为 1。颜色使用 `#RGB/#RRGGBB`。

| 方法 | 数据与选项 |
| --- | --- |
| `p.line(x=..., y=...)` | 等长一维数组；按输入顺序连线；`dash=[2 pt,2 pt]` 可设置成对的正长度虚线 |
| `p.scatter(x=..., y=...)` | `circle/square/triangle/triangle_down/diamond`；`marker_size=3 pt` 为包含描边的最终外框尺寸；重叠点逐点累积透明度 |
| `p.errorbar(x=..., y=..., yerr=...)` | 至少给 `xerr` 或 `yerr`；每个误差可以是非负标量、等长数组，或 `[下误差数组,上误差数组]`；`cap_size=3 pt` 为端帽全长 |
| `p.band(x=..., lower=..., upper=...)` | 三个等长数组；每个有效点满足 `lower <= upper`；不计算置信区间 |
| `p.hline(y=...)` / `p.vline(x=...)` | 参考线覆盖绘图区；支持 `dash`；其数值参与相应轴的自动范围 |
| `p.legend(position="top_right")` | 显示有非空 `label` 的非热图图层；位置为四角字符串或相对绘图区的物理坐标，支持多列及独立字号，详见上节 |

原生路径不经过 DSL 的 10,000 条路径命令限制，也不进行几何简化或数据降采样。输入数组在添加图层时复制，后续修改原列表不会追溯修改图层。

`line`、`scatter` 和 `errorbar` 共用以下选项。折线和误差棒的标记默认 `none`，散点默认 `circle`；省略新选项仍为原有实心样式。

| 选项 | 含义与默认值 |
| --- | --- |
| `marker` | `circle/square/triangle/triangle_down/diamond/none`；`triangle` 朝上，`triangle_down` 朝下 |
| `marker_size=3 pt` | 正物理长度，最终标记外框尺寸；不是面积。描边在此尺寸内预留空间 |
| `marker_fill` | 默认图层 `color`；十六进制颜色或 `none`。`none` 保留透明内部，白色填充会遮住背景 |
| `marker_border_color="none"` | 独立描边颜色或 `none` |
| `marker_border_width` | 默认 `plot_style.line_width`；非负物理长度，启用描边时须小于 `marker_size` |

数据点与图例共用几何、填充、描边和逐点透明度。带标记的折线显示线与标记组合；误差棒图例按实际 `xerr/yerr` 显示相应杆线、端帽及可选标记。图例保留图层顺序，相同 `label` 不会自动合并。

```text
p.line(x=[0,1,2], y=[1,3,2], marker="diamond", marker_size=2 mm,
       marker_fill="none", marker_border_color="#222222", marker_border_width=0.5 pt,
       label="Measured")
```

![黑白标记与折线、误差棒组合图例](../../site/media/plot-markers-1920.webp)

[可编辑源码](../../examples/plot/markers.lay) · [SVG](../../examples/plot/markers.svg) · [PDF](../../examples/plot/markers.pdf)。可分别修改每个系列的标记、填充、线型与误差方向。

轴、图层图例和色标的 `label` 接受字符串、`text(...)` 或 `formula(...)`。字符串继承图表颜色和对应字号；文字／公式素材使用自身样式，保留指定字号、宽度和换行，不自动缩放。混排使用现有 `text(spans=[...])`；相对字体路径仍按定义素材的 `.lay` 文件解析。

```text
label = text(spans=[span("Energy "), formula(source=r"E_f", font_size=9 pt), span(" (eV)")],
             font_family="DejaVu Sans", font_size=9 pt)
p = plot(size=(100 mm,80 mm), x=axis(label=label),
         y=axis(label=formula(source=r"I-I_0", font_size=10 pt), notation="offset"))
```

坐标轴和 `colorbar(...)` 共用下列参数：

| 选项 | 显示规则 |
| --- | --- |
| `notation="plain"` | 默认，保持现有数字格式；如需原来的 `1.00e+6` 文本，可用 `format=".2e"` |
| `notation="scientific"` | 每个非零刻度显示 `a×10ⁿ`，指数为数学上标；默认精度 `.2e`，显式 `format` 必须为指数类型 `e`。零始终显示 `0`，进位后同步更新指数 |
| `notation="offset"` | 全轴共用 `×10ⁿ`；自动指数由原始范围最大绝对值的十进制数量级确定；指数为零时省略倍率 |
| `exponent=6` | 仅用于 `offset`，显式指定整数指数；支持可表示的倍率范围 `-323…308`，刻度换算溢出时报告错误 |
| `format=".1f"` | 在 `offset` 中作用于除以倍率后的刻度；例如原值 `1500000`、指数 `6` 显示 `1.5` |
| `exponent_offset=(dx,dy)` | 仅用于 `offset`，默认 `(0 mm,0 mm)`；倍率的物理位移，x 向右、y 向下，可为负值 |

横轴倍率默认放在刻度下方靠右，纵轴倍率在上方靠左；`tick_labels=false` 也隐藏倍率。倍率和标签的实际尺寸参与外围空间测量。`notation/format/exponent` 只改变显示，数据及 `chart.data(...)` 均使用原始数值，线性／对数映射不变。固定留白或 `plot_area` 不足时只警告，不重排数据区域；手动多面板的位置保持不变。

横轴标题默认居中，与横轴倍率共用刻度下方的一行。只有两者在默认位置的实测边界重叠时，才把标题放到倍率下方，间隔 **1.2 mm**；公式上下标、文字宽度及换行均参与测量。没有横轴统一倍率时保留原有默认标题位置。

`axis(label_offset=(dx,dy))` 独立微调轴标题，`exponent_offset` 独立微调倍率。先完成默认定位和上述碰撞处理，再施加偏移；移动倍率不会带动标题，移动标题也不会带动倍率。偏移沿**图表局部物理坐标**，x 向右、y 向下，随后随图表旋转或所在组缩放。纵轴标题旋转 90° 后仍按这两个局部方向平移。偏移造成标题与倍率重叠或空间不足时报告 `W_PLOT_LAYOUT`，保留用户设置。自动留白会测量所需空间；原有贴外框边缘的标题若被手动移到外框之外，也会保留位置并警告，可用 `plot_area` 明确预留空间。色标继续使用原有布局规则，也支持独立 `label_offset`。

```text
x = axis(label="Time (s)", notation="offset", exponent=-6,
         exponent_offset=(0 mm, -0.5 mm),
         label_offset=(0 mm, 0.5 mm))
```

更复杂的标题布局可使用独立的 `page.add(text(...))` 或 `group.add(text(...))`，并以绘图区锚点定位。

![公式、混合标签、逐刻度科学计数法和统一倍率](../../site/media/plot-scientific-labels-1920.webp)

[可编辑源码](../../examples/plot/scientific-labels.lay) · [SVG](../../examples/plot/scientific-labels.svg) · [PDF](../../examples/plot/scientific-labels.pdf)。两个绘图区明确设为 **76 × 52 mm**，页面左上角分别是 **(30,32) mm** 和 **(134,32) mm**；右图横轴标题向下 0.5 mm、倍率向上 0.5 mm，数据区域保持不变。

```text
h = plot(size=(80 mm,65 mm), x=axis(label="x"), y=axis(label="y"))
heat = h.heatmap(z=[[1,2,3],[4,5,6]], extent=(0,3,0,2),
                origin="lower", cmap="viridis", vmin=0, vmax=6)
h.colorbar(heat, label="Intensity", format=".1f")
page.add(h)
```

- `z` 是非空、等宽的二维数值矩阵。支持 x_edges/y_edges 非均匀边界、命名轴和独立断轴。
- `extent=(xmin,xmax,ymin,ymax)` 指最外侧单元格边界；默认 `(0,列数,0,行数)`。x/y 范围必须递增。
- `origin="lower"` 将矩阵首行放在底部；`"upper"` 将首行放在顶部。默认 lower。
- 配色支持 `viridis/magma/inferno/plasma/cividis/rdbu/gray`。`vmin/vmax` 默认取有效数据的最小/最大值，常量自动展开；超出颜色范围的值钳制到端点颜色。
- `mode="raster"` 为默认：每个单元格对应源图像一个像素，采用最近邻显示；不插值、不平滑、不随输出 DPI 重算数值。非均匀网格或高级空间坐标会使用矢量单元格保证映射准确；`mode="vector"` 输出逐格矩形，适合小矩阵，大矩阵可能产生很大的文件。
- `h.colorbar(heat, label="...", format="...")` 绑定该图表内特定热图的颜色范围。每个图表最多一个色标，默认位于右侧；旧调用保留线性归一化；color_scale 支持共享非线性标尺。轴、刻度、普通文字及色标保持矢量。四侧和手动定位见下节。

`p.colorbar(heat, ...)` 可绑定本图的热图或带颜色标尺的图层，且每图最多一个。省略新选项保持右侧默认外观。

| 选项 | 含义与默认值 |
| --- | --- |
| `position="right"` | `right/left/top/bottom`；左右为竖直色标，上下为水平色标 |
| `position=(x,y)` | 色带左上角相对绘图区左上角的物理坐标，可为负值；不触发自动留白 |
| `orientation` | 手动坐标可选 `vertical/horizontal`，默认竖直；侧边位置自动决定方向，显式冲突报错 |
| `length` | 正物理长度；省略时竖直匹配绘图区高度，水平匹配宽度。侧边色带沿对应边居中 |
| `thickness=3 mm` | 色带厚度，正物理长度 |
| `gap=2.4 mm` | 侧边间隔，非负；左、上、下侧避开已测量的轴装饰，右侧保持原有间隔。手动坐标不能同时指定 `gap` |
| `ticks=[...]` | 显式有限数值，严格递增且位于热图颜色范围 `[vmin,vmax]`；空列表隐藏刻度。省略时自动生成 |
| `label`, `format`, `notation`, `exponent`, `exponent_offset` | 复用上面的富文本标签与计数法规则 |

```text
p.colorbar(heat, position="bottom", length=45 mm, thickness=3 mm,
           ticks=[0,2500,5000], notation="offset", exponent=3, format=".1f",
           label=formula(source=r"I", font_size=9 pt))
# 手动位置的替代调用（同一图表只调用一次 colorbar）：
# p.colorbar(heat, position=(3.5 mm,51 mm), orientation="horizontal", length=45 mm)
```

竖直色标从下到上增大，水平色标从左到右增大；颜色映射与绑定图层完全相同，包括共享非线性归一化。左右侧刻度朝外，上下侧刻度朝外；手动竖直色标刻度在右，手动水平色标刻度在下。色标倍率在竖直色带上方靠左，水平刻度外侧靠右；可用 `exponent_offset` 调整。

自动留白会测量侧边色标，固定留白和固定绘图区保持用户设置。手动色标不改变留白；溢出外框、长标签或重叠刻度会给出 `W_PLOT_LAYOUT`，仍保留长度、厚度、字号、位置和显式刻度。局部 `p.colorbar(...)` 绑定本图图层；跨图共享使用独立的 `colorbar(scale=..., length=...)` 素材，并让各图层使用同一个 `color_scale`，详见[共享装饰](../cartesian-plots.zh-CN.md#独立图例与色标)。两种形式均不自动避让其他面板或手动装饰。

![四侧色标与科学标签](../../site/media/plot-colorbars-1920.webp)

[可编辑源码](../../examples/plot/colorbars.lay) · [SVG](../../examples/plot/colorbars.svg) · [PDF](../../examples/plot/colorbars.pdf)。四幅热图的绘图区均为 **52 × 36 mm**，每幅分别定位；源码也提供手动水平色标替代参数。

```text
d = table(src="results.csv")
p.line(x=d["time"], y=d["signal"])
z = array(src="matrix.json")
```

`table` 接受带唯一非空表头的 CSV，或按列组织的 JSON：`{"x":[0,1],"y":[2,3],"sample":["A","B"]}`。各列必须等长且非空；允许保留字符串列，但传给绘图方法的列必须为数值。CSV 支持引号、列名中的逗号和 UTF-8 BOM；空单元格视为缺失，文字 `NaN` 不会自动当作缺失。

`array` 接受一维或二维数值 JSON。外部 JSON 使用 `null` 表示缺失；不接受 JSON `NaN`、`Infinity`、复数或布尔数组。数据采用 JavaScript number；超出安全整数范围的整数会拒绝。相对路径依据**定义该数据的 `.lay` 文件**解析，可用于组件模块。文件按单次编译缓存，下次编译会重新读取修改后的内容。

缺失数据触发带源码位置及数量的 `W_PLOT_MISSING`：折线和填充带在缺失行断开，散点和误差棒跳过缺失行，热图缺失单元格透明。长度不匹配、无有效点、负误差、非法对数值、不规则矩阵和非有限范围报错。普通数据图层不排序或补点；统计方法按明确规则计算描述统计，不修改输入数组。

## 限制与相关主题

[dsl-reference](../language-reference.zh-CN.md) · [Rust/WASM API](node-reference.zh-CN.md) · [Python API](python-reference.zh-CN.md) · [CLI 参数](cli-reference.zh-CN.md)

## 固定布局与科研样式


`plot_area=box(offset=(左,上), size=(宽,高))` 的四个值都是物理长度，相对于图表外框。左、上非负，宽、高为正，边界须为有限值；超出 `size` 时给出警告并保留设置。它与固定四边 `margins` 的区别是：覆盖外框 `size=(width, height)` 时，绘图区位置和尺寸仍固定，只有外围空间改变。字号、刻度、轴标签、图例或色标的修改也不会挤压绘图区；空间不足会报带源码位置的 `W_PLOT_LAYOUT` 并继续出图。锁定模式下，轴标签和色标标签随各自坐标轴定位，不会漂到扩大的外框边缘。

```text
page = canvas(size=(120 mm, 90 mm), background="#ffffff")
p = plot(size=(100 mm, 70 mm), plot_area=box(offset=(20 mm, 10 mm), size=(60 mm, 40 mm)),
         x=axis(label="Time (s)", range=(0, 2)),
         y=axis(label="Signal", range=(0, 4)))
p.line(x=[0, 1, 2], y=[0, 3, 1])
chart = page.add(p, anchor="plot_top_left", offset=(30 mm, 20 mm))
peak = chart.data(x=1, y=3)
pointer = page.add(line(end_head=head(), dx=-10 mm, dy=8 mm, line_width=0.5 pt),
                   anchor="end", target=peak)
page.add(text(content="Peak", font_size=8 pt), anchor=bottom_left,
         target=pointer.start, offset=(1 mm, -1 mm))
```

- `chart.data(x=..., y=...)` 返回已放置图表实例的数据锚点。x/y 是有限、无单位的数值，支持线性轴、正值对数轴和自动范围。锚点按该实例最终绘图区计算，不改变数据范围；修改轴范围并重新编译后，标注仍对应同一数据值。端点允许使用，超出轴范围报错，不静默钳制。
- 标注的 `offset` 是同一容器中的物理位移。可以把文字移到绘图区外；文字、箭头等是独立页面元素，不受数据层裁剪，也不自动避让或计入图表外框。
- 图表实例有九个绘图区锚点：`plot_top_left/plot_top_center/plot_top_right`、`plot_middle_left/plot_center/plot_middle_right`、`plot_bottom_left/plot_bottom_center/plot_bottom_right`。可作为 `target=chart.plot_top_left`，也可用字符串 `anchor="plot_top_left"` 将数据区域直接定位到页面。
- `anchor="start"/"end"` 可定位线段和箭头端点；`pointer.start/end` 可继续放置文字。绘图区和端点锚点随实例旋转；原有九个外框锚点仍取旋转后的轴对齐外接框。新增锚点名须在 `anchor` 中加引号，没有新增保留字。
- 目标与标注必须位于同一画布或组内。图表及标注一同放入 group 后，整体缩放和旋转会同步作用于两者；此时物理尺寸也按已有 group 规则缩放。每个图表实例均可单独定位和微调。

数据值、绘图区锚点和物理偏移可以直接组合。`anchor` 指所放元素自身的对齐点，`target` 指目标位置；下例的位移沿页面坐标方向，而非图表旋转后的局部方向：

```text
page.add(text(content="Peak", font_size=8 pt), anchor=bottom_left,
         target=chart.data(x=1, y=3), offset=(2 mm, -2 mm))
page.add(text(content="(a)", font_size=9 pt),
         target=chart.plot_top_left, offset=(2 mm, 2 mm))
```

![固定绘图区与数据坐标标注的实际输出](../../site/media/plot-annotations-1920.webp)

[可编辑源码](../../examples/plot/annotations.lay) · [SVG](../../examples/plot/annotations.svg) · [PDF](../../examples/plot/annotations.pdf)。示例绘图区为 **88 × 55 mm**，左上角固定在页面 **(26, 20) mm**；峰值箭头、阈值文字和标题分别放置。



新增选项均显式启用，省略时保持原有样式。它们不会修改 `plot_area`、数据范围、图表放置锚点或其他面板。自动留白模式仍会根据实际轴文字测量留白；需要固定数据区域时使用 `plot_area`。固定留白或固定绘图区周围空间不足时继续输出并报告 `W_PLOT_LAYOUT`，不缩小字号、不平移或压缩绘图区。

| 参数 | 行为与默认值 |
| --- | --- |
| `axis(minor_ticks=[...])` | 显式次刻度，默认空；有限、严格递增且位于轴范围内，对数轴须为正值。与主刻度重合时只画主刻度；不生成文字、不扩展范围、不自动补点。`ticks=[]` 可关闭主刻度 |
| `axis(tick_direction="out")` | `in/out/inout`；默认向外；`inout` 将刻度总长平分到轴内外 |
| `axis(tick_length=1.2 mm, minor_tick_length=0.6 mm)` | 主、次刻度的物理总长，允许零，不允许负数 |
| `axis(tick_labels=false)` | 隐藏该轴刻度文字，保留刻度、网格和轴标签；默认 `true` |
| `axis(grid="none")` | `none/major/both`；主网格对应最终主刻度，`both` 额外显示显式次刻度网格；次网格为 50% 不透明度 |
| `plot(frame="box")` | 加上方及右侧边框，不添加上方或右侧刻度；默认 `axes` 仅左、下轴线 |
| `plot_style(tick_font_size=7 pt, label_font_size=9 pt)` | 分别控制轴/色标刻度和轴/色标标签字号；省略时各自继承 `font_size` |
| `plot_style(grid_color="#dddddd", grid_line_width=0.3 pt, grid_line_dash=[])` | 网格颜色、正线宽及成对正长度虚线；空虚线数组表示实线 |
| `p.legend(columns=2, font_size=7 pt)` | 按图层顺序逐行排列；`columns` 为正整数，默认 1；字号默认继承 `font_size` |
| `p.legend(position=(0 mm, -9 mm))` | 图例外框左上角相对绘图区左上角的物理位置；x 向右、y 向下，可为负值，也继续支持四角字符串 |
| `p.legend(background="none", frame=true)` | 背景为 `none` 或十六进制颜色，默认白色；边框默认关闭，开启时使用图表轴线颜色和线宽 |

手动图例可以放在绘图区外，不会为它自动增加留白或改变外框尺寸。超出图表外框时给出警告并保留坐标。图例不会检测与轴标签或数据的遮挡；使用者应保留所需空间。网格在数据后方，可能被不透明热图遮住。组缩放仍按现有规则缩放其中的文字、刻度和图例。

![显式定位的双面板、主次刻度和外置多列图例](../../site/media/plot-publication-1920.webp)

[可编辑源码](../../examples/plot/publication.lay) · [SVG](../../examples/plot/publication.svg) · [PDF](../../examples/plot/publication.pdf)。两个绘图区分别明确设为 **64 × 45 mm**，左上角由使用者指定在页面 **(20, 26) mm** 和 **(115, 26) mm**；这个示例没有调用自动对齐。



## 扩展图层与投影

[多轴与断轴](multi-axes.zh-CN.md) · [柱图与阶梯](bars.zh-CN.md) · [直方图与 ECDF](hist-ecdf.zh-CN.md) · [箱线图与小提琴](box-violin.zh-CN.md) · [热图与等高线](heatmaps.zh-CN.md) · [极坐标投影](polar-projection.zh-CN.md) · [极坐标图层](polar-layers.zh-CN.md) · [雷达图](radar.zh-CN.md)
