# 直方图与 ECDF

<!-- walkthrough:start -->
## 用途与概念

hist 把原始观测按 bins 分箱；ECDF 在排序观测处计算累计比例。统计基于原始数据，与轴的显示变换无关。stat=count/probability/density 决定柱高含义。

## 最小完整示例

此文件包含画布及必要定义，可直接运行 `laymesh validate` 或 `laymesh render`，不依赖前文未声明的变量。

<!-- example:examples/manual/plot-hist.lay -->

## 参数与默认行为

裸几何长度使用画布单位，裸字号和描边宽度使用 pt。显式调用参数优先于继承和主题默认值；下面的接口参考逐项列出允许类型、可选值与默认来源。

## 组合用法

显式 bins 可处理不等宽区间，density 的归一化按区间宽度计算。将 ECDF 绑定到范围为 0–1 的副轴，可以与频数或密度柱图同页比较。

<!-- example:examples/plot/statistics.lay -->

## 常见错误与限制

weights 必须非负且与观测数量匹配。密度柱高并非单个区间概率，要乘区间宽度。自动分箱和加权统计规则见详细参数说明。

## 逐项功能说明

### plot-hist

直方图图层：向当前图表添加数据，通过物理样式参数控制外观，并使用命名轴参数选择数据映射。

返回：图层句柄，可用于共享图例。

[最小完整源码](../../examples/manual/plot-hist.lay) · [组合源码](../../examples/plot/statistics.lay) · [全部参数](interface-reference.zh-CN.md#plot-hist)

### plot-ecdf

经验累积分布图层：向当前图表添加数据，通过物理样式参数控制外观，并使用命名轴参数选择数据映射。

返回：图层句柄，可用于共享图例。

[最小完整源码](../../examples/manual/plot-ecdf.lay) · [组合源码](../../examples/plot/statistics.lay) · [全部参数](interface-reference.zh-CN.md#plot-ecdf)

<!-- walkthrough:end -->

## 详细行为与补充示例

### 参数

| 参数 | 用途 | 默认或要求 |
| --- | --- | --- |
| `bins` | 箱数或递增边界 | 10 |
| `stat` | 统计量 | count |
| `weights` | 非负等长权重 | 可选 |

### 常见用法

直方图支持 count、probability 和 density；箱边界严格递增，最后一箱含右端点。ECDF 排序并合并重复值，使用右连续曲线。统计在原始数据单位中计算。

[完整参数与规则](../cartesian-plots.zh-CN.md)

### 限制与相关主题

[箱线图与小提琴图](box-violin.zh-CN.md)
