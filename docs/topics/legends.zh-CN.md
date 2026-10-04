# 图例与共享色标

<!-- walkthrough:start -->
## 用途与概念

图例解释离散系列，色标解释数值颜色映射。图层 label 生成图例项目；color_scale 把同一数值规则共享给散点、热图、等高线和独立 colorbar。

## 最小完整示例

此文件包含画布及必要定义，可直接运行 `laymesh validate` 或 `laymesh render`，不依赖前文未声明的变量。

<!-- example:examples/manual/color_scale.lay -->

## 参数与默认行为

裸几何长度使用画布单位，裸字号和描边宽度使用 pt。显式调用参数优先于继承和主题默认值；下面的接口参考逐项列出允许类型、可选值与默认来源。

## 组合用法

p.legend/p.colorbar 是图表内部装饰；legend(layers=...) 和 colorbar(scale=...) 是可独立放置的素材。共享色标需要显式范围与同一配置对象，不能只选同名配色。

<!-- example:examples/plot/shared-colors.lay -->

## 常见错误与限制

color_scale 与图层私有 cmap/vmin/vmax 不能同时指定。连续色标按 RGB 插值并处理 alpha。独立图例中的图层应来自当前作品的有效图表。

## 逐项功能说明

### color_scale

定义数值到颜色的共享映射。连续 norm 需要明确范围，boundary 使用分段边界；各图层和独立色标引用同一配置。

返回：color_scale

[最小完整源码](../../examples/manual/color_scale.lay) · [组合源码](../../examples/plot/shared-colors.lay) · [全部参数](interface-reference.zh-CN.md#color_scale)

### legend

由已有图层列表创建可独立放置的图例素材。label 决定文字，样例来自图层样式；用 add 在页面或组中定位。

返回：material

必需输入：`layers`.

[最小完整源码](../../examples/manual/legend.lay) · [组合源码](../../examples/plot/complete-data.lay) · [全部参数](interface-reference.zh-CN.md#legend)

### colorbar

由 color_scale 创建独立色标素材。length/thickness 是物理尺寸，刻度数值与配色映射一致；用 add 定位。

返回：material

必需输入：`scale`.

[最小完整源码](../../examples/manual/colorbar.lay) · [组合源码](../../examples/plot/shared-colors.lay) · [全部参数](interface-reference.zh-CN.md#colorbar)

### plot-legend

在尚未放置的图表中添加图例装饰，默认使用带 label 的图层。position 属于图表局部坐标；独立图例使用 legend(layers=...)。

返回：decoration

[最小完整源码](../../examples/manual/plot-legend.lay) · [组合源码](../../examples/plot/first-plot.lay) · [全部参数](interface-reference.zh-CN.md#plot-legend)

### plot-colorbar

为图表图层添加内部色标装饰。传入图层引用确定映射，位置与长度在图表局部确定；共享独立色标使用 colorbar(scale=...)。

返回：decoration

[最小完整源码](../../examples/manual/plot-colorbar.lay) · [组合源码](../../examples/plot/colorbars.lay) · [全部参数](interface-reference.zh-CN.md#plot-colorbar)

<!-- walkthrough:end -->

## 详细行为与补充示例

### 参数

| 参数 | 用途 | 默认或要求 |
| --- | --- | --- |
| `color_scale` | 共享颜色映射 | 显式复用同一对象 |
| `orientation` | 色标方向 | vertical / horizontal |
| `length` | 色标物理长度 | 正长度 |

### 常见用法

`color_scale` 的连续模式为 `linear/log/symlog/centered`，必须指定有限递增的 `vmin/vmax`。log 的范围及有效颜色数据必须为正；symlog 接受正的 `constant`；centered 的 `center` 必须严格位于范围内。离散模式用 `norm="boundary",boundaries=[...]`，不用 vmin/vmax，边界值进入右侧区间，最终端点仍属于最后一段。

`cmap` 可为现有七种配色或至少两种 `#RGB/#RRGGBB` 组成的序列。连续序列使用 RGB 线性插值，离散序列按分段取色；缺失透明，范围外默认钳制，可用 `under/over` 指定颜色。热图、散点、等高线和色标复用同一个标尺。旧热图 `cmap/vmin/vmax` 仍使用图层私有线性标尺，与 `color_scale` 同时指定报错。

独立装饰是可重复放置的素材，支持 page/group.add 的锚点、物理偏移、旋转和缩放；不预留图表空间、不移动面板。图例按显式图层顺序排列，重复标签不合并，空标签及热图不列入图例。支持富文本 title、columns、font_size、background、frame、sample_width、sample_gap、gap、padding；可给 `style=plot_style(...)`，默认继承第一个图层的图表样式。

独立色标必须给 scale 和物理 length，默认 vertical、thickness=3 mm；支持 ticks/format/notation/exponent/exponent_offset/label/label_offset/style。其素材外框包含完整文字，add 的 top_left 对齐的是测量后的素材边界。普通字符串继承样式，明确设置的文字或公式样式保持不变。

局部 `p.legend(...)` 使用同样图例选项；`p.colorbar(layer,...)` 可以绑定本图热图或任何带颜色标尺的图层，每图仍最多一个。其四侧/手动位置规则保留，额外支持独立 `label_offset`。共享色标无需绑定图表；横向从左到右、纵向从下到上增加，刻度与颜色使用相同归一化。

### 限制与相关主题

[数据输入与缺失值](data.zh-CN.md) · [绘图区与物理尺寸](plot-area.zh-CN.md) · [坐标轴与刻度](axes.zh-CN.md) · [标签与科学计数法](labels.zh-CN.md) · [数据锚点与标注](annotations.zh-CN.md)
