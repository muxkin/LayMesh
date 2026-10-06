# 形状与路径

<!-- walkthrough:start -->
## 用途与概念

闭合形状用 fill 填充内部、border_* 描边；开放线条用 line_* 描边。path(commands=...) 保留源码节点和曲线段，不以渲染细分点代替几何身份。线身与头部只影响 ink，完整中心线仍到达逻辑端点。

## 最小完整示例

此文件包含画布及必要定义，可直接运行 `laymesh validate` 或 `laymesh render`，不依赖前文未声明的变量。

<!-- example:examples/manual/line.lay -->

## 参数与默认行为

裸几何长度使用画布单位，裸字号和描边宽度使用 pt。显式调用参数优先于继承和主题默认值；下面的接口参考逐项列出允许类型、可选值与默认来源。

## 组合用法

line() 或只带样式的 line(...) 可作为未指定几何的素材，在 add 中用 start/end 连接两点；也可沿用 dx/dy 或 length/angle 定义，二者互斥。构造可省略几何，放置必须能确定几何。start_head 反向沿起点切线，end_head 沿终点切线；triangle/open/stealth 以尖定位，dot/diamond/bar 以中心定位。零长度显式 angle 可生成独立头部。

<!-- example:examples/gallery/shapes/path-commands.lay -->

## 常见错误与限制

闭合子路径不接受头部；多条开放子路径各自设置端部。头部不会因短线缩小。零长度布局框沿用端帽或头部尺寸规则，path 弧长仍为零。arrow(...) 已移除，迁移为 line(...,end_head=head(...))。 未定义几何且未提供 start/end 时，错误定位到 add；两端重合的连接报错，不会自动变成零长度头部。

## 逐项功能说明

### rect

创建矩形素材；size 指定物理尺寸，fill 设置内部填充，border_* 设置边框，border_radius 设置圆角。调用 add 后才放置到页面。

返回：可重复放置的矩形素材。

必需输入：`size`.

[最小完整源码](../../examples/manual/rect.lay) · [组合源码](../../examples/basic.lay) · [全部参数](interface-reference.zh-CN.md#rect)

### ellipse

由 size=(宽,高) 定义椭圆素材。fill 与 border_* 分别控制内部与描边；布局框角点不一定在椭圆上。

返回：material

必需输入：`size`.

[最小完整源码](../../examples/manual/ellipse.lay) · [组合源码](../../examples/basic.lay) · [全部参数](interface-reference.zh-CN.md#ellipse)

### line

创建可复用线条素材，可省略几何并在 add 中指定 start/end；也可用 dx/dy 或 length/angle 定义。放置时必须能确定几何，零长度需要显式 angle，支持独立端帽和头部。

返回：material

[最小完整源码](../../examples/manual/line.lay) · [组合源码](../../examples/basic.lay) · [全部参数](interface-reference.zh-CN.md#line)

### path

由原始 move_to/line_to/quad_to/cubic_to/arc_to/close 指令定义路径。可含多个子路径；开放子路径支持端帽与头部。

返回：material

必需输入：`commands`.

[最小完整源码](../../examples/manual/path.lay) · [组合源码](../../examples/gallery/shapes/path-commands.lay) · [全部参数](interface-reference.zh-CN.md#path)

### polygon

用至少三个 points 定义自动闭合的多边形。节点顺序决定边界遍历方向，fill_rule 控制自交填充。

返回：material

必需输入：`points`.

[最小完整源码](../../examples/manual/polygon.lay) · [组合源码](../../examples/gallery/shapes/polygon-polyline.lay) · [全部参数](interface-reference.zh-CN.md#polygon)

### polyline

用至少两个 points 定义开放折线。line_* 控制描边，start_head/end_head 在逻辑端点定位，转角由 line_join 决定。

返回：material

必需输入：`points`.

[最小完整源码](../../examples/manual/polyline.lay) · [组合源码](../../examples/gallery/shapes/polygon-polyline.lay) · [全部参数](interface-reference.zh-CN.md#polyline)

### arc

定义 radius、start、end 指定的开放圆弧。页面角度 0 朝右、90 朝下；可设置端帽和头部，不自动封闭成扇形。

返回：material

必需输入：`radius`, `start`, `end`.

[最小完整源码](../../examples/manual/arc.lay) · [组合源码](../../examples/gallery/shapes/arc-sector.lay) · [全部参数](interface-reference.zh-CN.md#arc)

### sector

由半径和起止角定义闭合扇形。填充包括圆心与圆弧之间的区域，描边使用 border_*。

返回：material

必需输入：`radius`, `start`, `end`.

[最小完整源码](../../examples/manual/sector.lay) · [组合源码](../../examples/gallery/shapes/arc-sector.lay) · [全部参数](interface-reference.zh-CN.md#sector)

### star

由尖数、外半径和内半径定义闭合星形。inner_radius 小于 outer_radius，rotation 决定初始方向。

返回：material

必需输入：`points`, `outer_radius`, `inner_radius`.

[最小完整源码](../../examples/manual/star.lay) · [组合源码](../../examples/gallery/shapes/star-ring.lay) · [全部参数](interface-reference.zh-CN.md#star)

### ring

定义带透明内孔的圆环。inner_radius 小于 outer_radius，孔不以背景色遮盖，可透出底下的绘制。

返回：material

必需输入：`outer_radius`, `inner_radius`.

[最小完整源码](../../examples/manual/ring.lay) · [组合源码](../../examples/gallery/shapes/star-ring.lay) · [全部参数](interface-reference.zh-CN.md#ring)

### move_to

创建路径指令，在 x/y 开始一条新子路径，不绘制连接到上一子路径的线。只能放入 path(commands=...)。

返回：path-command

[最小完整源码](../../examples/manual/move_to.lay) · [组合源码](../../examples/gallery/shapes/path-commands.lay) · [全部参数](interface-reference.zh-CN.md#move_to)

### line_to

创建从当前节点到 x/y 的直线段指令。路径必须先有 move_to，节点与原始段身份保留。

返回：path-command

[最小完整源码](../../examples/manual/line_to.lay) · [组合源码](../../examples/gallery/shapes/path-commands.lay) · [全部参数](interface-reference.zh-CN.md#line_to)

### quad_to

创建二次贝塞尔段，cx/cy 是控制点，x/y 是终点。控制点通常不在实际曲线上，t 与弧长比例不同。

返回：path-command

[最小完整源码](../../examples/manual/quad_to.lay) · [组合源码](../../examples/gallery/shapes/path-commands.lay) · [全部参数](interface-reference.zh-CN.md#quad_to)

### cubic_to

创建三次贝塞尔段，两个控制点与终点按位置参数给出。查询 controls 时保留原始控制点，不按渲染细分改变索引。

返回：path-command

[最小完整源码](../../examples/manual/cubic_to.lay) · [组合源码](../../examples/gallery/shapes/path-commands.lay) · [全部参数](interface-reference.zh-CN.md#cubic_to)

### arc_to

创建椭圆弧路径段，参数包含半径、旋转、large_arc/sweep 布尔标志和终点。一个指令保持一条原始逻辑曲线段。

返回：path-command

[最小完整源码](../../examples/manual/arc_to.lay) · [组合源码](../../examples/gallery/shapes/path-commands.lay) · [全部参数](interface-reference.zh-CN.md#arc_to)

### close

创建闭合当前子路径的指令，连接末节点到起点并保留遍历方向。闭合路径首尾可重合，跨接缝区间须显式 wrap=true。

返回：path-command

[最小完整源码](../../examples/manual/close.lay) · [组合源码](../../examples/gallery/shapes/evenodd-hole.lay) · [全部参数](interface-reference.zh-CN.md#close)

### head

创建可复用的端点头部配置，不能单独放置。size 是沿朝向与横向的物理尺寸。

返回：head

[最小完整源码](../../examples/manual/head.lay) · [组合源码](../../examples/basic.lay) · [全部参数](interface-reference.zh-CN.md#head)

<!-- walkthrough:end -->

## 详细行为与补充示例

### 参数

| 参数 | 用途 | 默认或要求 |
| --- | --- | --- |
| `path` | 路径命令与坐标 | 坐标使用长度单位 |
| `fill_rule` | 填充规则 | nonzero / evenodd |

### 常见用法

使用 rect、ellipse、polygon、polyline、path、line 创建矢量素材。路径坐标以物理单位定义；填充规则控制重叠区域和孔洞。融合只支持同一容器中已经放置的矢量实例。

[完整参数与规则](../language-reference.zh-CN.md)

### 限制与相关主题

[图像导入与裁剪](images.zh-CN.md) · [文字与字体](fonts.zh-CN.md) · [资源路径](resources.zh-CN.md) · [数学公式](formulas.zh-CN.md) · [填充渐变与边框](fills.zh-CN.md)


### 线段方向与端点样式

`arrow(...)` 已移除。使用 `line(...,end_head=head(...))` 创建带头部的线条。旧调用给出迁移诊断，编辑器可修复可安全识别的内置调用。

```lay
page.add(line(length=40mm,angle=30deg,line_width=1mm,
              start_cap="round",
              end_head=head(shape="triangle",size=(5mm,4mm))),offset=(10mm,20mm))
page.add(line(length=0mm,angle=30deg,
              end_head=head(shape="triangle",size=(5mm,4mm))),anchor=self.path.end,offset=(60mm,20mm))
```

`dx/dy` 与 `length/angle` 互斥。长度非负；非零长度默认角度为零，零长度必须显式指定角度。`0deg` 朝右，`90deg` 朝下，支持 `deg/rad`。省略角度单位按度解释。

`line`、`polyline`、开放 `path` 和 `arc` 共用 `start_head/end_head` 和 `start_cap/end_cap`。端帽为 `butt/round/square`；line/polyline/arc 继承 `line_cap`，path 继承 `border_cap`，虚线内部保留对应的基础端帽。头部形状为 `triangle/open/stealth/dot/diamond/bar`，可独立设置 `size`、`fill`、`border_color`、`border_width` 和 `opacity`。三角、V 形、内凹头部以尖端定位；圆点、菱形、横杠以中心定位。size 是沿头部朝向和横向的物理尺寸，dot 使用椭圆轮廓，bar 使用矩形轮廓。

头部默认纵向尺寸为 `max(4×线宽,1.5mm)`，横向尺寸为 `max(0.9×纵向尺寸,线宽)`。闭合头部默认继承线色且无描边；open 默认无填充，描边继承线色和线宽。头部尺寸不随线长缩小，短线可只有头部，`ink.bounds` 反映实际占用。零长度没有线身；无头部时 round/square 端帽可画点或方块。

零长度线的布局框按头部或端帽的几何占用计算，与透明度无关；无头部的 butt 端帽保留线宽大小的布局框，但不产生绘制区域。起点与终点仍然重合，弧长为零。用 `anchor=self.path.end` 将这个点对齐到目标，调整头部尺寸时该点保持不变。

逻辑 `path` 保留完整原始中心线，尖头的尖与 `path.start/end` 重合，线条的描边可以进入头部内部。完整描边生成后，端部裁剪包络约束尖端附近外露的描边，避免按中心线交点提前截短。圆点、菱形和横杠的中心与端点重合。重叠区域使用头部属性，同样式区域合并后绘制，防止半透明颜色叠加；闭合空心头部内部留空，开放 V 形内部保留线条。曲线头部按端点一侧切线定向，方向无法确定时报错。多子路径分别使用端点样式，带头部的素材不能包含闭合子路径。

```lcss
.pointer {
  start-cap: round;
  end-head: head(shape="triangle",size=(5mm,4mm),fill="#0072b2");
  line-color: hsv(200 0.7 0.8 / 0.6);
}
```

## workflow

本流程的完整源码与可执行验证文件列在[功能覆盖清单](feature-map.zh-CN.md)。组合使用时请遵循本页的输入条件与限制。
