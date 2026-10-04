# 投影与径向轴

<!-- walkthrough:start -->
## 用途与概念

polar 用角度 theta 和半径 r 表示数据。它的默认 0° 朝右、正方向逆时针，与页面线段 angle 的顺时针 y 向下约定不同；angle_unit 决定数据角度单位。

## 最小完整示例

此文件包含画布及必要定义，可直接运行 `laymesh validate` 或 `laymesh render`，不依赖前文未声明的变量。

<!-- example:examples/manual/plot.lay -->

## 参数与默认行为

裸几何长度使用画布单位，裸字号和描边宽度使用 pt。显式调用参数优先于继承和主题默认值；下面的接口参考逐项列出允许类型、可选值与默认来源。

## 组合用法

设置 theta_zero、theta_direction、inner_radius 可形成仪表或环形图。wrap=shortest 跨周期走最短方向，raw 保留角度差；interpolation=polar 与 chord 分别在数据与页面中连接。

<!-- example:examples/plot/polar-directions.lay -->

## 常见错误与限制

负半径按角度加半圈后取绝对值，并发出警告；对数径向轴要求正值。plot_* 仍指投影矩形，实际角度轴线应通过 axes 的 spine 查询。

## 逐项功能说明

<!-- walkthrough:end -->

## 详细行为与补充示例

### 参数

| 参数 | 用途 | 默认或要求 |
| --- | --- | --- |
| `plot_area` | 投影所在矩形 | 圆心在矩形中心 |
| `theta / r` | 角度与半径轴 | 原始数据单位 |

### 常见用法

[完整源码](../../examples/plot/polar-directions.lay)。右图采用北向零度、顺时针方向；左图圆圈使用负半径，与反向半周的正半径点重合，产生可隐藏的预期警告。

| 设置 | 默认及行为 |
| --- | --- |
| `projection` | `cartesian`；新增 `polar` 和 `radar` |
| `angle_unit` | `deg`；可选 `rad`，控制无单位角数据、范围、刻度、误差和柱宽 |
| `theta_zero` | `0 deg`，从页面向右方向逆时针旋转的角度；例如 `90 deg` 指向上方 |
| `theta_direction` | `ccw`；可选 `cw` |
| `theta=axis(...)` | 线性角轴，默认整圆；显式递增范围不超过一周，跨零度扇区写作 `(300,420)` |
| `r=axis(...)` | 线性、log 或 symlog，支持反向、字号、颜色、富文本、科学计数法及独立偏移；范围非负，log 范围为正 |
| `r_label_angle` | `22.5 deg`，径向轴及其刻度所在角方向，跟随零度位置和方向 |
| `inner_radius` | `0 mm`；可设物理内孔，须小于外半径；径向范围映射到内外半径之间 |
| `wrap` | 曲线默认 `shortest`；350→10 沿短角路径，恰好半周保留原差值方向；`raw` 保留数值角度差，适用于多圈 |
| `interpolation` | 默认 `polar`，在角度/半径空间插值后投影；`chord` 直接连接投影端点 |
| `closed` | 普通曲线默认 false；设 true 连接末端与起点 |

`theta/r` 的 range、ticks 和数据使用无单位数值，单位由 `angle_unit` 解释；`theta_zero/r_label_angle` 使用现有 `deg` 物理角度语法。角刻度默认显示度数，弧度显示数值；需要 π 分数可用 `tick_text` 配合公式。未指定径向范围时线性尺度从零开始，log 使用有效半径的正值范围。

`a.data(theta=...,r=...)` 使用原始值，超出扇区或径向范围时报错。`a.plot_*` 九个锚点仍引用绘图区矩形；`a.axis(name="theta|r",anchor="start|center|end")` 引用外圆弧或径向射线，center 为物理弧长/径向长度的中点。随后再执行图表旋转、组缩放和容器偏移。

负半径按 `(theta+半周, abs(r))` 投影，发出 `W_POLAR_NEGATIVE_RADIUS`；不修改输入数组。连续路径在原始半径过零处切分，误差端帽只画在真实且可见的端点。对数半径不接受实际零值；零基线和跨零误差区间先裁剪再取对数。圆弧和投影曲线的局部细分误差上限为 0.001 mm，锚点使用解析坐标。

### 限制与相关主题

[极坐标图层](polar-layers.zh-CN.md) · [雷达图](radar.zh-CN.md)
