# 热图与等高线

<!-- walkthrough:start -->
## 用途与概念

heatmap 表示单元格数值，contour 表示采样场等值线，contourf 填充等值区间。它们均要求规则的二维矩阵，但边缘坐标和采样坐标含义不同。

## 最小完整示例

此文件包含画布及必要定义，可直接运行 `laymesh validate` 或 `laymesh render`，不依赖前文未声明的变量。

<!-- example:examples/manual/plot-heatmap.lay -->

## 参数与默认行为

裸几何长度使用画布单位，裸字号和描边宽度使用 pt。显式调用参数优先于继承和主题默认值；下面的接口参考逐项列出允许类型、可选值与默认来源。

## 组合用法

用共享 color_scale 让热图与等高线采用相同颜色规则。x_edges/y_edges 定义热图边缘，contour 的 x/y 定义采样坐标；非均匀轴或断轴采用矢量单元格保证映射。

<!-- example:examples/plot/shared-colors.lay -->

## 常见错误与限制

levels 必须递增，等高线至少 2×2。矩阵缺失区域不自动插值，散点不自动变成网格。坐标列表与 extent 互斥；行长不一致报错。

## 逐项功能说明

### plot-heatmap

热图图层：向当前图表添加数据，通过物理样式参数控制外观，并使用命名轴参数选择数据映射。

返回：图层句柄，可用于共享图例。

[最小完整源码](../../examples/manual/plot-heatmap.lay) · [组合源码](../../examples/plot/shared-colors.lay) · [全部参数](interface-reference.zh-CN.md#plot-heatmap)

### plot-contour

等高线图层：向当前图表添加数据，通过物理样式参数控制外观，并使用命名轴参数选择数据映射。

返回：图层句柄，可用于共享图例。

[最小完整源码](../../examples/manual/plot-contour.lay) · [组合源码](../../examples/plot/shared-colors.lay) · [全部参数](interface-reference.zh-CN.md#plot-contour)

### plot-contourf

填充等高线图层：向当前图表添加数据，通过物理样式参数控制外观，并使用命名轴参数选择数据映射。

返回：图层句柄，可用于共享图例。

[最小完整源码](../../examples/manual/plot-contourf.lay) · [组合源码](../../examples/plot/shared-colors.lay) · [全部参数](interface-reference.zh-CN.md#plot-contourf)

<!-- walkthrough:end -->

## 详细行为与补充示例

### 参数

| 参数 | 用途 | 默认或要求 |
| --- | --- | --- |
| `z` | 二维数据矩阵 | 等长行 |
| `x_edges / y_edges` | 单元格边界 | 严格递增；与 extent 二选一 |
| `levels` | 等高线级别 | 严格递增 |

### 常见用法

`color_scale` 的连续模式为 `linear/log/symlog/centered`，必须指定有限递增的 `vmin/vmax`。log 的范围及有效颜色数据必须为正；symlog 接受正的 `constant`；centered 的 `center` 必须严格位于范围内。离散模式用 `norm="boundary",boundaries=[...]`，不用 vmin/vmax，边界值进入右侧区间，最终端点仍属于最后一段。

`cmap` 可为全部 [Matplotlib 3.11.2 预设](cmaps.zh-CN.md)、不可变的 `cmap(...)` 对象，或至少两种颜色组成的自定义序列。连续序列使用 RGB 线性插值，离散序列按分段取色；缺失透明，范围外默认钳制，可用 `under/over` 指定颜色。热图、散点、等高线和色标复用同一个标尺。旧热图 `cmap/vmin/vmax` 仍使用图层私有线性标尺，与 `color_scale` 同时指定报错。

散点 `c` 与 `marker_fill` 二选一，c 必须与 x/y 等长；`marker_size` 可为等长物理长度列表，opacity 可为等长的 0–1 数组。描边仍包含在每个标记最终外框内。逐点大小与透明度在图例中用图层的标量默认样本表示；数值颜色对应的含义用色标表达。

热图 `x_edges/y_edges` 是严格递增的单元格边界，长度分别为列数加一/行数加一，与 `extent` 冲突。未指定的方向仍采用默认均匀边界。非线性空间轴、独立断轴或非均匀边界使用逐格矢量几何保证准确映射；原有均匀线性热图保留 raster 默认输出路径。

`contour/contourf` 接受至少 2×2 的规则矩阵和显式递增 `levels`。x/y 是与列/行等长的采样点坐标；省略时默认 `0..列数-1`、`0..行数-1`，或使用 `extent` 的采样点端点。x/y 与 extent 二选一。采用 [D3 contour](https://d3js.org/d3-contour/contour) 并把 `(i+0.5,j+0.5)` 校正到实际采样坐标，支持非均匀采样间距。含缺失顶点的网格单元留空，不对散乱点插值。

填充级别定义区间的下限：小于首级别的区域空缺，最高级别向上延伸。相邻超水平集合相减形成互不叠加的色带，透明度不会因嵌套区域重复累积。contourf 未给标尺时按有限数据范围创建私有 viridis 标尺；contour 未给标尺时用图层单色。

### 限制与相关主题

[折线与散点](line-scatter.zh-CN.md) · [误差棒与填充带](uncertainty.zh-CN.md) · [柱图与阶梯](bars.zh-CN.md) · [多轴与断轴](multi-axes.zh-CN.md)
