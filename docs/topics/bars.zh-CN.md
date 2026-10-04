# 柱图与阶梯

<!-- walkthrough:start -->
## 用途与概念

bar 的 positions、values、baseline 和 data_width 都是数据单位。step 使用 pre/mid/post 定义阶跃出现的位置；area 把曲线与基线之间的区域填充。

## 最小完整示例

此文件包含画布及必要定义，可直接运行 `laymesh validate` 或 `laymesh render`，不依赖前文未声明的变量。

<!-- example:examples/manual/plot-bar.lay -->

## 参数与默认行为

裸几何长度使用画布单位，裸字号和描边宽度使用 pt。显式调用参数优先于继承和主题默认值；下面的接口参考逐项列出允许类型、可选值与默认来源。

## 组合用法

用不同 positions 手工形成分组柱，使用逐点 baseline 形成堆叠柱。将 hline 的零线、step 曲线和带透明度 area 组合，可比较离散与连续表达。

<!-- example:examples/plot/statistics.lay -->

## 常见错误与限制

柱宽不是 mm；不得与物理线宽混淆。不会自动决定类别位置、排序或堆叠。负 values 可以落在基线另一侧，log 轴仍受正值要求约束。

## 逐项功能说明

### plot-bar

柱图图层：向当前图表添加数据，通过物理样式参数控制外观，并使用命名轴参数选择数据映射。

返回：图层句柄，可用于共享图例。

[最小完整源码](../../examples/manual/plot-bar.lay) · [组合源码](../../examples/plot/statistics.lay) · [全部参数](interface-reference.zh-CN.md#plot-bar)

### plot-step

阶梯图图层：向当前图表添加数据，通过物理样式参数控制外观，并使用命名轴参数选择数据映射。

返回：图层句柄，可用于共享图例。

[最小完整源码](../../examples/manual/plot-step.lay) · [组合源码](../../examples/plot/statistics.lay) · [全部参数](interface-reference.zh-CN.md#plot-step)

### plot-area

面积图图层：向当前图表添加数据，通过物理样式参数控制外观，并使用命名轴参数选择数据映射。

返回：图层句柄，可用于共享图例。

[最小完整源码](../../examples/manual/plot-area.lay) · [组合源码](../../examples/plot/polar-data.lay) · [全部参数](interface-reference.zh-CN.md#plot-area)

<!-- walkthrough:end -->

## 详细行为与补充示例

### 参数

| 参数 | 用途 | 默认或要求 |
| --- | --- | --- |
| `data_width` | 柱宽，数据单位 | 0.8 |
| `baseline` | 柱基线 | 0 |
| `where` | 阶梯位置 | post |

### 常见用法

柱位置、宽度和 baseline 使用数据单位。分组通过显式位置实现，堆叠通过 baseline 实现。step 的 pre/mid/post 控制阶梯位置，保留输入顺序。

[完整参数与规则](../cartesian-plots.zh-CN.md)

### 限制与相关主题

[折线与散点](line-scatter.zh-CN.md) · [误差棒与填充带](uncertainty.zh-CN.md) · [热图与等高线](heatmaps.zh-CN.md) · [多轴与断轴](multi-axes.zh-CN.md)
