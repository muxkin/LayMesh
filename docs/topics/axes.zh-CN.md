# 坐标轴与刻度

<!-- walkthrough:start -->
## 用途与概念

axis 配置数据范围、映射、刻度和标签。默认主轴是 x/y；显式 range 保证不同图表使用同样数值范围。linear、log、symlog 是不同数据映射，不是页面旋转。

## 最小完整示例

此文件包含画布及必要定义，可直接运行 `laymesh validate` 或 `laymesh render`，不依赖前文未声明的变量。

<!-- example:examples/manual/axis.lay -->

## 参数与默认行为

裸几何长度使用画布单位，裸字号和描边宽度使用 pt。显式调用参数优先于继承和主题默认值；下面的接口参考逐项列出允许类型、可选值与默认来源。

## 组合用法

ticks 配 tick_text 可显示分类文字或公式；minor_ticks 和 grid 控制细分刻度与背景网格。通过 chart.axes["x"].spine.path 选轴线，通过完整组件 bounds 对齐标题区域。

<!-- example:examples/plot/multi-axes-breaks.lay -->

## 常见错误与限制

log 范围和有效数据必须为正。刻度文字与显式 ticks 长度要一致。axis.min/max 表示数值范围端点，反向轴也不改变它们的数值含义。

## 逐项功能说明

### axis

配置数据范围、linear/log/symlog 映射、刻度、标签和断轴。数据范围采用原始数值，文字与线宽采用物理单位。

返回：axis

[最小完整源码](../../examples/manual/axis.lay) · [组合源码](../../examples/plot/multi-axes-breaks.lay) · [全部参数](interface-reference.zh-CN.md#axis)

<!-- walkthrough:end -->

## 详细行为与补充示例

### 参数

| 参数 | 用途 | 默认或要求 |
| --- | --- | --- |
| `range` | 数据范围 | 可显式指定 |
| `minor_ticks` | 次刻度 | []；auto 显式启用 |
| `reverse` | 反向显示 | false |

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

### 限制与相关主题

[数据输入与缺失值](data.zh-CN.md) · [绘图区与物理尺寸](plot-area.zh-CN.md) · [标签与科学计数法](labels.zh-CN.md) · [图例与共享色标](legends.zh-CN.md) · [数据锚点与标注](annotations.zh-CN.md)
