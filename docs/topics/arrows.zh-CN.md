# 形状箭头

<!-- walkthrough:start -->
## 用途与概念

形状箭头是一体式闭合轮廓；构造素材后通过 add 放置。默认 arrow() 为直箭头，各模板使用 arrow.arc() 等独立入口，共享填充、边框与特效。

## 最小完整示例

此文件包含画布及必要定义，可直接运行 `laymesh validate` 或 `laymesh render`，不依赖前文未声明的变量。

<!-- example:examples/manual/arrow.lay -->

## 参数与默认行为

裸几何长度使用画布单位，裸字号和描边宽度使用 pt。显式调用参数优先于继承和主题默认值；下面的接口参考逐项列出允许类型、可选值与默认来源。

## 组合用法

弧形和自由路径在箭身末端沿切线接出居中的三角头。自由路径保留完整曲线，弧形从总扫角中扣除头部占角。在 add 中用 start/end 将最终箭尖或无头端连接锚点；shaft_width 按箭身弧长插值。

<!-- example:examples/gallery/shapes/arrows.lay -->

## 常见错误与限制

圆弧半径、总扫角、头部尺寸与端点距离必须一致；没有正箭身解、端点重合、整圈扫角、非法宽度或自交轮廓报错。centerline 包含头部轴线，path 是闭合轮廓。

## 逐项功能说明

### arrow

创建可复用的直箭头形状；可在 add 中通过 start/end 连接锚点，使用 fill、border_* 与 effects。

返回：可复用的形状箭头素材。

[最小完整源码](../../examples/manual/arrow.lay) · [组合源码](../../examples/gallery/shapes/arrows.lay) · [全部参数](interface-reference.zh-CN.md#arrow)

### arrow-arc

创建弧形箭身并沿末端切线接出直边三角头；总扫角包含头部，连接时将头部计入端点求解。

返回：可复用的形状箭头素材。

[最小完整源码](../../examples/manual/arrow-arc.lay) · [组合源码](../../examples/gallery/shapes/arrows.lay) · [全部参数](interface-reference.zh-CN.md#arrow-arc)

### arrow-bent

创建 L 形折弯箭头；可设置中心线跨度与圆角，并连接两个锚点。

返回：可复用的形状箭头素材。

[最小完整源码](../../examples/manual/arrow-bent.lay) · [组合源码](../../examples/gallery/shapes/arrows.lay) · [全部参数](interface-reference.zh-CN.md#arrow-bent)

### arrow-uturn

创建 U 形回转箭头；双端连接调整中心线，保留箭身与头部物理尺寸。

返回：可复用的形状箭头素材。

[最小完整源码](../../examples/manual/arrow-uturn.lay) · [组合源码](../../examples/gallery/shapes/arrows.lay) · [全部参数](interface-reference.zh-CN.md#arrow-uturn)

### arrow-chevron

创建单头燕尾箭头；尾部凹口与箭身组成同一个闭合轮廓。

返回：可复用的形状箭头素材。

[最小完整源码](../../examples/manual/arrow-chevron.lay) · [组合源码](../../examples/gallery/shapes/arrows.lay) · [全部参数](interface-reference.zh-CN.md#arrow-chevron)

### arrow-path

保留完整自由开放路径作为箭身，在两端沿切线向外接出直边三角头；支持变宽与独立头部尺寸。

返回：可复用的形状箭头素材。

必需输入：`path`.

[最小完整源码](../../examples/manual/arrow-path.lay) · [组合源码](../../examples/gallery/shapes/arrows.lay) · [全部参数](interface-reference.zh-CN.md#arrow-path)

<!-- walkthrough:end -->

## 详细行为与补充示例

## 几何与放置

`arrow()` 创建直箭头，其他模板通过 `arrow.arc()`、`arrow.bent()`、`arrow.uturn()`、`arrow.chevron()`、`arrow.path()` 构造。它们都返回可复用的闭合形状素材，使用 `fill`、`border_*` 和 `effects`。线条上的头部仍使用 `line(..., end_head=head(...))`。

```lay
page=canvas(size=(160mm,110mm))
a=page.add(rect(size=(20mm,14mm),fill="#dde5f2"),offset=(10mm,60mm))
b=page.add(rect(size=(20mm,14mm),fill="#dde5f2"),offset=(120mm,60mm))
page.add(arrow(shaft_width=(2mm,5mm),fill="#087f8c"),start=a.middle_right,end=b.middle_left)
page.add(arrow.arc(sweep_angle=90deg,fill="#ee784b"),start=a.top_center,end=b.top_center)
page.add(arrow.arc(radius=-80mm,fill="#5273c5"),start=a.bottom_center,end=b.bottom_center)
```

本地圆弧使用正 `radius`、`start_angle`（默认 0°）和有符号 `sweep_angle`。`radius` 是圆弧箭身中心线的半径；`start_angle` 是最终逻辑起点相对圆心的极角；`sweep_angle` 是包含两端头部的总极角扫角。页面角度 0° 朝右、90° 朝下，正扫角顺时针，大弧使用绝对值超过 180°、小于 360° 的扫角。

三角头从箭身端点沿切线向外接出，底边中心与颈部重合。头部长为 `L`、箭身半径为 `R` 时，头部占角为 `atan(L/R)`。系统从总扫角中扣除两端头部占角，计算圆弧箭身长度；未设置头部的一端按 `L=0` 计算。剩余箭身扫角必须为正。

连接两个端点时，只给扫角会自动求半径与圆心；只给有符号半径时，正值选择顺时针小弧，负值选择逆时针小弧。求解按追加头部后的真实端点计算：两端到圆心的距离分别为 `sqrt(R²+L_start²)` 与 `sqrt(R²+L_end²)`，小弧的半圆边界距离为二者之和，不再使用纯圆弧的半弦限制。若同时指定半径和扫角，半径必须为正，并校验包含头部后的实际端点距离；不会静默修改约束。扫角求半径存在多个正箭身解时，固定选择较大半径，再检查轮廓。连接放置重新决定起始朝向，同一素材可重复使用。

`bent` 的中心线依次经过 `(0,h)`、`(0,0)`、`(w,0)`；`uturn` 再延伸到 `(w,h)`。`span=(w,h)` 默认 `(30mm,20mm)`，`corner_radius` 默认较小跨度的四分之一，0 表示尖角。燕尾的 `notch_depth` 默认为尾端箭身宽度。自由路径接受单条开放的 `line/arc/path/polyline` 素材，只读取几何，不继承其填充、线色或头部。完整原路径用作箭身；其端点成为颈部，末端头沿终点切线向外追加，起始头沿起点切线反方向追加。头部不截短原曲线，因此需为追加长度留出布局空间。

直箭头的双端连接重算长度与方向，圆弧使用上述约束求解。其他模板和自由路径连接两端时，对箭身中心线做平移、旋转和等比调整，箭身宽度、头部尺寸保持物理长度。自由路径的连接方程计入两端追加头部长；多个正缩放解固定选择较大者。`start/end` 对应最终箭尖或无头端。普通 `add(size=...)` 则按现有形状缩放整个几何。直箭头、折弯、U 形及燕尾的原有尺寸语义不变。

双端连接沿用 `start_offset/end_offset`、每端的 `*_offset_space` 与整体 `offset`。它与显式 `anchor/target/rotation/size` 互斥。端点必须属于同一容器；组重放重新解析锚点。详见[锚点与定位](anchors.zh-CN.md#双端点连接)。

## 变宽与头部

`shaft_width=3mm` 表示等宽；`shaft_width=(2mm,5mm)` 表示首尾线性渐变；`shaft_width=[(0,2mm),(0.5,6mm),(1,3mm)]` 表示多处宽度控制。位置是扣除头部后箭身弧长的比例，必须从 0 到 1 严格递增，各区间线性插值。

`heads` 为 `end`（默认）、`start` 或 `both`，燕尾只支持单头。`head_size=(纵向长度,横向宽度)` 为两端共用尺寸，`start_head_size/end_head_size` 分别覆盖；未指定时，两项均为对应颈部宽度的两倍。头部宽度不得小于颈部宽度。箭头尖端保持在逻辑端点，方向沿端点切线；头部与箭身生成单一闭合轮廓。弧形和自由路径的头部底边以颈部为中心，与箭身末端切线垂直；两侧肩部对称，不再将弯曲箭身截到偏离曲线的头部底边。

过短路径、无法确定方向的尖点、非法宽度和自交轮廓会产生错误。不会通过缩小头部或改变用户宽度掩盖无效几何。

## centerline

实例 `.start/.end` 是逻辑起终点。`.centerline` 保留生成箭头的开放中心线，支持现有路径查询，如 `.at(fraction=0.5)`、`.segments`、`.length` 和切线查询；弧形和自由路径的 `.centerline` 包含箭身及追加的直线头部轴线，所以 `.length` 计入头部长，端点及切线与最终箭尖一致。自由路径保留原始曲线段；有起始头时这些段的索引顺延一位，末端头轴线位于最后一段。原始路径段不会变成渲染采样点。`.path` 是闭合形状轮廓，`.ink` 是填充和描边区域，`.bounds` 是布局框。

[锚点连接示例](../../examples/gallery/shapes/arrow-connections.lay)使用中心线中点定位扫角说明文字。

## LCSS arrow styles

所有模板在 LCSS 中共享 `arrow` 类型选择器。`arrow.demo` 表示带 `demo` 类的箭头，不表示 `arrow.demo()` 构造器。可设置 `shaft-width`、`heads`、`head-size`、`start-head-size`、`end-head-size`，以及现有填充、边框和特效。尺寸对可写为 `8mm 10mm`，宽度控制点沿用列表形式。

<!-- example:examples/manual/arrow-style.lay -->

## 导出与兼容

SVG/PDF 中箭头主体保持矢量；模糊阴影与发光沿用现有特效处理。普通 PPTX 箭头为可编辑的自定义矢量几何，特殊效果可能触发 `W_PPTX_RASTER` 局部栅格回退；不承诺 PowerPoint 预设箭头的黄色调节柄。

`arrow()` 现在表示形状箭头。含 `line_*`、线条端帽或 `start_head/end_head` 的旧调用可识别为线条箭头；仅当参数全部适用于 `line` 时提供安全迁移修复。只有 `length/angle` 或 `dx/dy` 的旧写法与新接口无法自动区分，按新形状语义解释。迁移工具不改写新的命名空间构造器或用户自定义同名函数。

[模板展示](../../examples/gallery/shapes/arrows.lay) · [样式与效果](../../examples/gallery/shapes/arrow-effects.lay)
