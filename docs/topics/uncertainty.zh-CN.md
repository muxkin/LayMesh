# 误差棒与填充带

<!-- walkthrough:start -->
## 用途与概念

errorbar 在采样点绘制 xerr/yerr 不确定度，band 在 x 对应的 lower/upper 之间填充。误差是数据单位，cap_size 和线宽是物理长度。

## 最小完整示例

此文件包含画布及必要定义，可直接运行 `laymesh validate` 或 `laymesh render`，不依赖前文未声明的变量。

<!-- example:examples/manual/plot-errorbar.lay -->

## 参数与默认行为

裸几何长度使用画布单位，裸字号和描边宽度使用 pt。显式调用参数优先于继承和主题默认值；下面的接口参考逐项列出允许类型、可选值与默认来源。

## 组合用法

同一实验数据可组合半透明 band、中心折线及点误差。误差可对称或分别指定上下误差；上下界须与采样位置对应。

<!-- example:examples/plot/scientific.lay -->

## 常见错误与限制

误差不能为负，数组长度必须相同。断轴或投影裁剪不会制造新的误差端帽；缺失输入保留诊断。不要把上下界误传成误差幅度。

## 逐项功能说明

### plot-errorbar

向图表添加误差棒。xerr、yerr 支持对称误差或分别提供上下界误差；cap_size 设置端帽的物理长度。

返回：图层句柄，可用于共享图例。

[最小完整源码](../../examples/manual/plot-errorbar.lay) · [组合源码](../../examples/plot/scientific.lay) · [全部参数](interface-reference.zh-CN.md#plot-errorbar)

### plot-band

填充带图层：向当前图表添加数据，通过物理样式参数控制外观，并使用命名轴参数选择数据映射。

返回：图层句柄，可用于共享图例。

[最小完整源码](../../examples/manual/plot-band.lay) · [组合源码](../../examples/plot/scientific.lay) · [全部参数](interface-reference.zh-CN.md#plot-band)

<!-- walkthrough:end -->

## 详细行为与补充示例

### 参数

| 参数 | 用途 | 默认或要求 |
| --- | --- | --- |
| `yerr / xerr` | 误差范围 | 与观测匹配 |
| `lower / upper` | 填充上下界 | 与 x 等长 |

### 常见用法

误差棒表达每个观测的不确定度；band 用 lower/upper 表达连续区间。示例左侧同时包含折线、散点、填充带、误差棒和参考线，右侧展示热图。

[完整参数与规则](plot-reference.zh-CN.md)

### 限制与相关主题

[折线与散点](line-scatter.zh-CN.md) · [柱图与阶梯](bars.zh-CN.md) · [热图与等高线](heatmaps.zh-CN.md) · [多轴与断轴](multi-axes.zh-CN.md)
