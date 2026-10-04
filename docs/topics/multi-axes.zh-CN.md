# 多轴与断轴

<!-- walkthrough:start -->
## 用途与概念

命名轴允许图层选择自己的数值映射。主轴是 x/y；add_axis 给新增轴唯一名称和 left/right/top/bottom 位置，图层通过 x_axis/y_axis 绑定。

## 最小完整示例

此文件包含画布及必要定义，可直接运行 `laymesh validate` 或 `laymesh render`，不依赖前文未声明的变量。

<!-- example:examples/manual/plot-add_axis.lay -->

## 参数与默认行为

裸几何长度使用画布单位，裸字号和描边宽度使用 pt。显式调用参数优先于继承和主题默认值；下面的接口参考逐项列出允许类型、可选值与默认来源。

## 组合用法

一条副轴曲线可与主轴柱图叠放。各轴独立设置 breaks 和 segment_lengths；数据锚点必须使用同一轴名称，才能与相应图层保持一致。

<!-- example:examples/plot/multi-axes-breaks.lay -->

## 常见错误与限制

不会自动为同侧多个轴留出间距。断轴区间必须在显式范围内递增且不重叠；segment_lengths 加间隙必须等于数据区边长，不会自动缩放。

## 逐项功能说明

### plot-add_axis

给尚未放置图表添加唯一命名的轴。side 决定方向与位置，图层用 x_axis/y_axis 选择此轴；同侧间距需显式 offset。

返回：axis

必需输入：`name`, `side`, `axis`.

[最小完整源码](../../examples/manual/plot-add_axis.lay) · [组合源码](../../examples/plot/multi-axes-breaks.lay) · [全部参数](interface-reference.zh-CN.md#plot-add_axis)

<!-- walkthrough:end -->

## 详细行为与补充示例

### 参数

| 参数 | 用途 | 默认或要求 |
| --- | --- | --- |
| `x_axis / y_axis` | 图层绑定轴 | x / y |
| `break_gap` | 断口物理间距 | 2 mm |
| `segment_lengths` | 可见段物理长度 | 省略时按变换后跨度分配 |

### 常见用法

`x/y` 是主轴名称。每个新名称须非空且唯一；`side="left|right|top|bottom"`，默认 `right`。`offset=0 mm` 从绘图区对应边向外量，允许负值。同侧多轴不会自动推开；根据文字宽度自行留空间。每个图层都接受 `x_axis/y_axis`，默认 `x/y`；方向不匹配或名称不存在时报错。自动范围只统计绑定该轴的数据，未使用的轴应给出显式 `range`。

`chart.data(...)` 使用原始数值，遵循所选轴的变换、反向和分段。`chart.axis(name=...,anchor="start|center|end")` 的 start/end 对应范围最小/最大值；reverse 不改变这个含义，center 是轴的物理中点，可能位于断口。两类锚点都随实例旋转、所在组缩放。`page.add` 的 offset 在页面坐标中，`group.add` 的 offset 在组坐标中。

| 轴参数 | 规则 |
| --- | --- |
| `reverse=false` | 翻转显示方向，不修改数据或统计结果 |
| `scale="symlog", constant=1` | 使用 `sign(x) × log(1+abs(x)/constant)`；constant 是正的原始数据单位数值，支持负数和零 |
| `tick_text=[...]` | 与显式 `ticks` 等长；字符串、`text(...)`、`formula(...)`，输入顺序保留 |
| `tick_rotation=0 deg` | 围绕刻度文字中心旋转；先测量旋转后的边界，再定位 |
| `tick_offset=(0 mm,0 mm)` | 图表局部坐标中的文字位移，x 向右、y 向下 |
| `tick_font_size=...`, `tick_color="#..."` | 刻度文字及倍率的独立样式；刻度杆也使用 tick_color，素材明确指定的文字/公式样式保留 |
| `line_color="#..."`, `line_width=...` | 该轴的线条、默认文字颜色和线宽；tick_color 可单独覆盖刻度颜色 |
| `spine=false` | 隐藏该轴轴线及断口符号，保留刻度与标签 |
| `minor_ticks="auto"` | 显式启用自动次刻度；线性/对称对数显示的数值刻度区间五等分，对数轴生成每个十进制数量级的 2–9 次刻度；原默认仍为 `[]` |
| `grid="none|major|both"` | 每条轴分别启用；附加轴默认无网格。网格位于数据图层后方 |

标题、倍率的 `label_offset/exponent_offset` 继续独立生效。显示顺序、轴名和类别不是自动推断的；分类图请明确给出数值位置及 `tick_text`。

`breaks` 隐藏开区间，边界值仍显示；必须递增、不重叠且严格位于显式 `range` 内。`break_gap` 默认 2 mm，可为统一长度或与断口数等长的物理长度列表，均须为正。`break_mark_size` 控制每个斜线符号的横纵外框尺寸。

省略 `segment_lengths` 时，先扣除全部断口间距，再按各可见段**经过轴变换后的跨度**分配剩余长度。显式段长必须为正、数量为断口数加一，且段长与间距之和等于绘图区对应边长，容差 `1e-6 mm`。不匹配报错，不缩放用户指定的长度；精确分段建议同时指定 `plot_area`。

每个图层按自己绑定的两条轴建立可见区间组合并裁剪。其他轴可以具有不同的断区、断口位置、段长，或完全不断轴。折线、误差棒、柱、填充和热图均在各窗口中绘制；裁剪边界不新增误差端帽，也没有覆盖其他图层的白带。落在隐藏区间的显式刻度不绘制；落在隐藏区间的数据锚点报错。

检查输出含绘图区、各轴数据范围与变换、可见段及长度、断口及宽度、装饰边界，单位均为局部 mm。`page_transform=[a,b,c,d,e,f]` 将局部坐标映射到页面：`X=a*x+c*y+e`，`Y=b*x+d*y+f`，包括图表旋转与上层组缩放。

### 限制与相关主题

[折线与散点](line-scatter.zh-CN.md) · [误差棒与填充带](uncertainty.zh-CN.md) · [柱图与阶梯](bars.zh-CN.md) · [热图与等高线](heatmaps.zh-CN.md)
