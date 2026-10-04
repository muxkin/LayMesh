# 折线与散点

<!-- walkthrough:start -->
## 用途与概念

line 连续连接有效点，scatter 只绘制标记。二者保留输入顺序，line 的 marker 可同时显示采样点；样式尺寸使用物理单位，与数据范围独立。

## 最小完整示例

此文件包含画布及必要定义，可直接运行 `laymesh validate` 或 `laymesh render`，不依赖前文未声明的变量。

<!-- example:examples/manual/plot-line.lay -->

## 参数与默认行为

裸几何长度使用画布单位，裸字号和描边宽度使用 pt。显式调用参数优先于继承和主题默认值；下面的接口参考逐项列出允许类型、可选值与默认来源。

## 组合用法

同一 x/y 可用浅色线连接并用不同标记强调点。scatter 的 c 配 color_scale 显示第三变量，marker_size 和 opacity 可以是与数据长度相同的列表。

<!-- example:examples/plot/line-scatter.lay -->

## 常见错误与限制

不自动排序 x。c 与 marker_fill 互斥，逐点数组长度须匹配；缺失点分断折线或跳过标记。marker 可用 circle/square/triangle/triangle_down/diamond/none。

## 逐项功能说明

### plot-line

向图表添加折线图层，依次连接数据点；可同时显示标记。使用 x_axis、y_axis 选择命名坐标轴；极坐标使用 theta、r，雷达图使用 values。

返回：图层句柄，可用于共享图例。

[最小完整源码](../../examples/manual/plot-line.lay) · [组合源码](../../examples/plot/line-scatter.lay) · [全部参数](interface-reference.zh-CN.md#plot-line)

### plot-scatter

向图表添加散点图层；可设置标记形状、物理尺寸及边框，或使用 c 与 color_scale 按数值映射颜色。

返回：图层句柄，可用于共享图例。

[最小完整源码](../../examples/manual/plot-scatter.lay) · [组合源码](../../examples/plot/line-scatter.lay) · [全部参数](interface-reference.zh-CN.md#plot-scatter)

<!-- walkthrough:end -->

## 详细行为与补充示例

### 参数

| 参数 | 用途 | 默认或要求 |
| --- | --- | --- |
| `x / y` | 等长数据序列 | 必填 |
| `marker_size` | 标记物理尺寸 | 示例：2.4 mm |
| `label` | 图例文字 | 可选 |

### 常见用法

line 按输入顺序连接数据，不自动排序；scatter 单独绘制每个观测。两者可以叠加，使用相同坐标轴和颜色；缺失处折线断开，散点跳过。

[完整参数与规则](plot-reference.zh-CN.md)

### 限制与相关主题

[误差棒与填充带](uncertainty.zh-CN.md) · [柱图与阶梯](bars.zh-CN.md) · [热图与等高线](heatmaps.zh-CN.md) · [多轴与断轴](multi-axes.zh-CN.md)
