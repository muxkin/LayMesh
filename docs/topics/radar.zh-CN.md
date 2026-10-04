# 雷达图

<!-- walkthrough:start -->
## 用途与概念

radar 为每个类别指定角度和各自的数值范围，再将各项归一化到共同物理半径。它适合比较多指标形状，不保证不同量纲具有相同意义。

## 最小完整示例

此文件包含画布及必要定义，可直接运行 `laymesh validate` 或 `laymesh render`，不依赖前文未声明的变量。

<!-- example:examples/manual/plot.lay -->

## 参数与默认行为

裸几何长度使用画布单位，裸字号和描边宽度使用 pt。显式调用参数优先于继承和主题默认值；下面的接口参考逐项列出允许类型、可选值与默认来源。

## 组合用法

categories 与 ranges 显式匹配，使用 closed=true 的 line 或 area 连接首尾。原始数据锚点可选 category/value，标签用 categories 或独立文本。

<!-- example:examples/plot/radar.lay -->

## 常见错误与限制

类别顺序不会自动重排。每条数据数量必须与类别数量一致，范围须有效且值在可见范围内。不要把极坐标连续 theta 查询当作类别索引。

## 逐项功能说明

<!-- walkthrough:end -->

## 详细行为与补充示例

### 参数

| 参数 | 用途 | 默认或要求 |
| --- | --- | --- |
| `categories` | 类别及顺序 | 显式指定 |
| `range` | 径向数据范围 | 线性、非反向 |

### 常见用法

至少三个唯一类别按输入顺序等角排列，默认上方开始、顺时针；`category_labels` 可提供等长的字符串/文字/公式。`radar_frame="polygon|circle"` 默认为 polygon，网格与类别射线对齐。支持 `line/scatter/area(values=...)`，线和面积默认闭合，类别间采用直线。

省略 `ranges` 时所有类别共用 `r.range`，未指定时由全部系列共同确定；不会分别归一化每个系列。显式 `ranges` 与共同 `r.range` 二选一，按每个指标的原始范围线性归一化；此时 r.ticks 使用 0–1 的共同位置，但文字显示各指标的原始值。独立范围下使用 offset 时，每条指标射线显示自己的倍率。原始负值在有效范围内正常显示，不翻转类别方向。超出显式范围时报错；缺失值断线，并取消该系列封闭面积。

`a.data(category=...,value=...)` 使用原始值；`a.axis(name=类别,anchor=...)` 引用对应射线。雷达图使用线性、非反向尺度，不使用 theta 轴、内孔或断轴。

### 限制与相关主题

[投影与径向轴](polar-projection.zh-CN.md) · [极坐标图层](polar-layers.zh-CN.md)
