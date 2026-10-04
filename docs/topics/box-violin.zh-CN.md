# 箱线图与小提琴图

<!-- walkthrough:start -->
## 用途与概念

boxplot 概括分位数、须和离群点；violin 用核密度估计展示分布。它们读取原始观测而不是已经汇总的五数值。

## 最小完整示例

此文件包含画布及必要定义，可直接运行 `laymesh validate` 或 `laymesh render`，不依赖前文未声明的变量。

<!-- example:examples/manual/plot-boxplot.lay -->

## 参数与默认行为

裸几何长度使用画布单位，裸字号和描边宽度使用 pt。显式调用参数优先于继承和主题默认值；下面的接口参考逐项列出允许类型、可选值与默认来源。

## 组合用法

把同一批观测放在相邻位置的 boxplot 和 violin 中，使用显式 data_width 保持可比较宽度。R-7 分位数、1.5 IQR 默认须和 Scott 带宽说明统计口径。

<!-- example:examples/plot/statistics.lay -->

## 常见错误与限制

单点或常量样本的小提琴不能形成常规密度轮廓，会显示中位数并告警。points 是估计采样数，不是原始样本量；不要用它改变数据。

## 逐项功能说明

### plot-boxplot

箱线图图层：向当前图表添加数据，通过物理样式参数控制外观，并使用命名轴参数选择数据映射。

返回：图层句柄，可用于共享图例。

[最小完整源码](../../examples/manual/plot-boxplot.lay) · [组合源码](../../examples/plot/statistics.lay) · [全部参数](interface-reference.zh-CN.md#plot-boxplot)

### plot-violin

小提琴图图层：向当前图表添加数据，通过物理样式参数控制外观，并使用命名轴参数选择数据映射。

返回：图层句柄，可用于共享图例。

[最小完整源码](../../examples/manual/plot-violin.lay) · [组合源码](../../examples/plot/statistics.lay) · [全部参数](interface-reference.zh-CN.md#plot-violin)

<!-- walkthrough:end -->

## 详细行为与补充示例

### 参数

| 参数 | 用途 | 默认或要求 |
| --- | --- | --- |
| `whisker` | 箱线须的 IQR 倍数 | 1.5 |
| `bandwidth` | 核密度带宽 | scott |
| `points` | 小提琴采样点数 | 128 |

### 常见用法

箱线图使用 R-7 分位数，默认须线到 1.5 IQR 内最远观测。小提琴图使用高斯核和 Scott 带宽；常量或单点样本仅显示中位数并警告。

[完整参数与规则](../cartesian-plots.zh-CN.md)

### 限制与相关主题

[直方图与 ECDF](hist-ecdf.zh-CN.md)
