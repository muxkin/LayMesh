# 放置、几何锚点与路径定位

<!-- walkthrough:start -->
## 用途与概念

定位关系需要明确自身选哪个点、目标选哪个点、额外移动多少。默认九点属于布局框，圆角矩形和椭圆的框角通常不在轮廓上。需要轮廓点时使用 path 或 ink.boundary 查询并显式索引候选。

## 最小完整示例

此文件包含画布及必要定义，可直接运行 `laymesh validate` 或 `laymesh render`，不依赖前文未声明的变量。

<!-- example:examples/manual/path-at.lay -->

## 参数与默认行为

裸几何长度使用画布单位，裸字号和描边宽度使用 pt。显式调用参数优先于继承和主题默认值；下面的接口参考逐项列出允许类型、可选值与默认来源。

## 组合用法

将曲线的弧长中点用作文字 target，并将 rotation 设置为该点的 tangent_angle。需要沿线与法线偏移时使用 offset_space=target；尖角先选择 incoming 或 outgoing。

<!-- example:examples/gallery/positioning/anchors.lay -->

## 常见错误与限制

候选集合不能直接传给 target。空集合索引报错；连续重合和无限个最近点不会被随意抽样。self 只在 anchor 中绑定，未放置的素材不能充当目标实例。

## 逐项功能说明

### instance-data

在已放置图表中选择原始数据坐标点。轴变换、断轴、实例变换及组重放都重新解析此引用；点必须处于有效可见数据域。

返回：anchor

[最小完整源码](../../examples/manual/instance-data.lay) · [组合源码](../../examples/gallery/positioning/chart-parts.lay) · [全部参数](interface-reference.zh-CN.md#instance-data)

### instance-axis

按名称选择图表轴的数值起端、物理中点或数值终端。保留旧变换语义；新几何查询可通过 axes[name].spine.path 选轴线。

返回：anchor

[最小完整源码](../../examples/manual/instance-axis.lay) · [组合源码](../../examples/manual/geometry-composition.lay) · [全部参数](interface-reference.zh-CN.md#instance-axis)

### add

将素材放置到画布或组合中，使用 anchor 对齐素材锚点、target 指定目标、offset 设置偏移。同一素材可以重复放置，各实例可独立设置尺寸与样式。

返回：已放置实例；可读取测量尺寸并引用其锚点继续定位。

必需输入：`material`.

[最小完整源码](../../examples/manual/add.lay) · [组合源码](../../examples/gallery/positioning/anchors.lay) · [全部参数](interface-reference.zh-CN.md#add)

### ray

创建有起点和非零方向的查询射线。origin 与目标路径必须在同一容器；direction 在路径所选测量空间中解释。

返回：ray

必需输入：`origin`, `direction`.

[最小完整源码](../../examples/manual/ray.lay) · [组合源码](../../examples/manual/geometry-composition.lay) · [全部参数](interface-reference.zh-CN.md#ray)

### path-at

按弧长比例或距离选点；先选 segments[i] 才能使用原始参数 t。默认按放置后的路径测量。

返回：path_anchor

[最小完整源码](../../examples/manual/path-at.lay) · [组合源码](../../examples/manual/geometry-composition.lay) · [全部参数](interface-reference.zh-CN.md#path-at)

### path-nearest

返回所有全局最近路径位置，按子路径、源段和参数排序。必须索引；无限多个最近点报错。

返回：anchor_collection

必需输入：`to`.

[最小完整源码](../../examples/manual/path-nearest.lay) · [组合源码](../../examples/manual/geometry-composition.lay) · [全部参数](interface-reference.zh-CN.md#path-nearest)

### path-extrema

返回所选空间中的局部坐标极值，不含普通端点或常量区间。

返回：anchor_collection

[最小完整源码](../../examples/manual/path-extrema.lay) · [组合源码](../../examples/manual/geometry-composition.lay) · [全部参数](interface-reference.zh-CN.md#path-extrema)

### path-inflections

返回曲率改变符号且方向平滑的拐点。

返回：anchor_collection

[最小完整源码](../../examples/manual/path-inflections.lay) · [组合源码](../../examples/manual/geometry-composition.lay) · [全部参数](interface-reference.zh-CN.md#path-inflections)

### path-corners

返回方向不平滑的连接节点；方向需 with_side 明确选择。

返回：anchor_collection

[最小完整源码](../../examples/manual/path-corners.lay) · [组合源码](../../examples/manual/geometry-composition.lay) · [全部参数](interface-reference.zh-CN.md#path-corners)

### path-intersections

返回与射线相交的全部路径位置；自交处保留不同路径位置，连续重合报错。

返回：anchor_collection

必需输入：`ray`.

[最小完整源码](../../examples/manual/path-intersections.lay) · [组合源码](../../examples/manual/geometry-composition.lay) · [全部参数](interface-reference.zh-CN.md#path-intersections)

### path-between

沿原路径顺序选择两个路径锚点间的连续区间；端点必须来自同一实例和子路径。

返回：geometry_path

[最小完整源码](../../examples/manual/path-between.lay) · [组合源码](../../examples/manual/geometry-composition.lay) · [全部参数](interface-reference.zh-CN.md#path-between)

### path-in_space

切换弧长、最近点和特征查询的测量空间；结果仍是当前容器中的锚点。

返回：geometry_path

[最小完整源码](../../examples/manual/path-in_space.lay) · [组合源码](../../examples/manual/geometry-composition.lay) · [全部参数](interface-reference.zh-CN.md#path-in_space)

### anchor-with_side

选择尖角的 incoming 或 outgoing 方向；正法线为沿遍历方向的左侧。

返回：path_anchor

[最小完整源码](../../examples/manual/anchor-with_side.lay) · [组合源码](../../examples/manual/geometry-composition.lay) · [全部参数](interface-reference.zh-CN.md#anchor-with_side)

<!-- walkthrough:end -->

## 详细行为与补充示例

### 三种几何视图

| 视图 | 含义 | 用途 |
| --- | --- | --- |
| `bounds` | 原有布局框的轴对齐外接矩形，不计描边 | 排版、对齐与间距 |
| `path` | 闭合图形的边界或线条的逻辑中心线 | 连接、曲线标注与节点定位 |
| `ink` | 填充和描边形成的区域，包含端帽、虚线及箭头头部 | 贴住绘制边缘与计算占用范围 |

`placed.top_left` 等九点简写保留原语义，等同于 `placed.bounds.top_left`。`path.bounds` 和 `ink.bounds` 是对应几何的紧致外接框；**框角不保证位于图形上**。圆角矩形、椭圆的框角和边界点应分别选取：

```lay
page.add(dot, anchor=center, target=placed.bounds.top_left)
page.add(dot, anchor=center,
         target=placed.path.nearest(to=placed.bounds.top_left)[0])
page.add(dot, anchor=center,
         target=placed.ink.boundary.nearest(to=placed.bounds.top_left)[0])
```

箭头的 `path` 包含完整逻辑中心线，末端不会被箭头头部截短。线条的 `path.bounds` 可以零高度，`ink.bounds` 计入线宽、端帽及箭头头部。本轮仅支持原生矢量几何；图片与文字继续使用 `bounds`，不提取像素或字形轮廓。

### 节点、曲线段与沿途选点

```lay
route = curve.path.subpaths[0]
route.start
route.end
route.nodes[2]
route.segments[1].controls[0]  # 控制点可能不在曲线上，没有路径方向
route.at(fraction=0.5)         # 整条子路径的弧长一半
route.at(distance=10 mm)      # 从起点量出的弧长
route.segments[1].at(t=0.5)   # 原始曲线段参数，不等同于弧长一半
route.between(route.nodes[1], route.nodes[3]).at(fraction=0.5)
```

节点与段索引遵循原始几何定义，不随渲染细分数量改变；一个 `arc_to` 仍是一个逻辑段。多个子路径上的连续遍历操作须先选择 `subpaths[i]`。闭合路径保留起点与遍历方向，`start` 和 `end` 的坐标可以重合；跨闭合接缝必须显式写 `between(..., wrap=true)`。`between` 两端必须来自同一实例、同一几何路径和同一子路径，不能混用重合实例的点。

默认在当前容器中测量：不等比缩放会改变弧长、最近点及极值位置。`route.in_space("local")` 按原始局部几何测量，`in_space("parent")` 恢复容器测量；所有结果仍转换为当前容器中的锚点。直接访问外接框时，九点遵循所选测量空间的矩形。

### 搜索始终返回集合

```lay
route.nearest(to=another.bounds.center)[0]
route.extrema(axis="y")[0]
route.inflections()[0]
route.corners()[0]
route.intersections(ray(origin=another.bounds.center, direction=(1, 0)))[0]
```

即使只有一个候选，也必须显式索引后才能传给 `target`。无结果返回空集合，索引越界报 `E_INDEX`；可用 `len(candidates)` 检查数量，或 `for point in candidates` 逐一放置。候选按子路径、原始段、段内参数排序；相邻段共享节点去重，自交处不同路径位置保留各自身份。连续重合、无限多个全局最近点报告 `E_GEOMETRY`，不会擅自采样成有限候选。

`extrema` 返回所选空间中的局部坐标极值，普通端点和常量区间不算孤立极值；`inflections` 返回曲率改变符号的平滑拐点；`corners` 返回方向不平滑的连接节点。射线只取正向部分，方向 `(dx,dy)` 在路径所选测量空间中解释，方向不能为零。

### 自身锚点、方向与偏移

```lay
point = placed.path.at(fraction=0.5)
page.add(curve, anchor=self.path.start, target=point,
         offset=(2 mm, 3 mm), offset_space="target",
         rotation=point.tangent_angle)
```

`self` 是本次素材的符号根，仅在 `anchor` 中绑定，尺寸与变换完成后才解析；自身选择器不能依赖另一个实例。旧九点与 `anchor="start"/"end"` 保留原有变换语义。

路径锚点提供 `tangent`、`normal`（两个无单位分量）及 `tangent_angle`。正法线是沿遍历方向的左侧；在页面坐标中向右的切线对应向上的正法线。尖角或尖点方向不唯一时，用 `point.with_side("incoming")` 或 `with_side("outgoing")` 选择，未指定时方向查询报 `E_ANCHOR_DIRECTION`。`start/end` 分别采用出射/入射方向；单纯定位无需方向。

| 参数 | 含义 | 默认 |
| --- | --- | --- |
| `anchor` | 布局锚点名称或 `self` 几何选择器 | `top_left` |
| `target` | 同一容器中的已放置实例锚点 | 当前容器左上角 |
| `offset` | 两个物理长度分量 | `(0 mm, 0 mm)` |
| `offset_space` | `container` 为容器横纵方向；`target` 为目标切线、左法线方向 | `container` |
| `rotation` | 围绕实例中心旋转，也接受 `point.tangent_angle` | `0deg` |

### 图表内部部件

```lay
chart.plot_area.bounds.top_left
chart.axes["x"].bounds.bottom_center   # 完整轴组件：轴线、刻度与标题
chart.axes["x"].spine.path.at(fraction=0.5)
chart.axes["x"].label.bounds.center
chart.axes["x"].min                    # 数值范围最小端
chart.axes["x"].max                    # 数值范围最大端
chart.data(x=2, y=1.8)
```

轴组件的九点简写对应完整组件布局框；轴线通过 `spine` 单独选择。横向、纵向和 top/bottom/left/right 侧的轴都使用同一模型。`min/max` 根据数值域而非页面方向命名，反向轴不会交换数值含义。断轴的 `spine.path` 有多条子路径，连续遍历需先选一条。旧 `plot_*` 与 `axis(name="x",anchor="center")` 写法保留原变换语义。

实例身份、部件与选点条件保存在引用中；组重放和图表重新排版后，会重新计算路径、部件、数据锚点及直接读取的 `tangent_angle` 等测量值。

[完整接口清单](interface-reference.zh-CN.md) · [单位与尺寸](units.zh-CN.md) · [旋转与图层](transforms.zh-CN.md) · [组与复用](groups.zh-CN.md)

## geometry-model

类型参考列出全部视图成员和方法，源码身份与测量空间规则见上文；复合路径连续遍历前先选 `.subpaths[i]`。

下例分别读取实例、布局框、真实路径及绘制区域；同一个九点接口也能用于图表区域、完整坐标轴、标签、刻度和指数标签。红点标记布局框，蓝点标记绘制边界，黄色点标记控制点和轴标签。控制点须通过 `.segments[i].controls[j]` 选取；它提供位置，但通常不在曲线上。

<!-- example:examples/manual/geometry-members.lay -->

`point.x` 与 `point.y` 返回当前容器中的物理坐标；`point.tangent` 和 `point.normal` 返回两个无量纲分量，分别可用 `[0]` 和 `[1]` 读取。乘以物理长度可计算偏移，正法线始终在遍历方向左侧。`target=point` 保留锚点引用；数值二元组可用于 `offset`。重新放置后的依赖关系见[组合源码](../../examples/manual/geometry-composition.lay)。

[几何视图类型](interface-reference.zh-CN.md#几何视图类型)

## workflow

本流程的完整源码与可执行验证文件列在[功能覆盖清单](feature-map.zh-CN.md)。组合使用时请遵循本页的输入条件与限制。
