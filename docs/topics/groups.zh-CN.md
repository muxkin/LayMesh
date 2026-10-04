# 组与复用

<!-- walkthrough:start -->
## 用途与概念

group 是带局部坐标的可复用容器。先向组添加实例，再把整个组放入页面。组内 target 引用同一组内已经存在的实例，组的放置会重放这些关系。

## 最小完整示例

此文件包含画布及必要定义，可直接运行 `laymesh validate` 或 `laymesh render`，不依赖前文未声明的变量。

<!-- example:examples/manual/group.lay -->

## 参数与默认行为

裸几何长度使用画布单位，裸字号和描边宽度使用 pt。显式调用参数优先于继承和主题默认值；下面的接口参考逐项列出允许类型、可选值与默认来源。

## 组合用法

用组组合底板、标题和连接线，可以多次放置、旋转或缩放。fuse 是另一项操作：把同容器中已放置的两个矢量实例合并成一个新轮廓，原实例退出绘制和引用。

<!-- example:examples/gallery/containers/group-reuse.lay -->

## 常见错误与限制

第一次放置封存组，之后不能添加子项。组不能包含自身；不同容器的锚点不能直接混用。fuse 不接受图片、文字或组，也不公开通用布尔运算。

## 逐项功能说明

### group

创建带局部坐标的复用容器。先添加子实例再放置整个组；第一次放置封存内容，重复放置重放内部定位关系。

返回：material

[最小完整源码](../../examples/manual/group.lay) · [组合源码](../../examples/gallery/containers/group-reuse.lay) · [全部参数](interface-reference.zh-CN.md#group)

### fuse

融合同容器中两个已放置矢量实例的可见轮廓，原实例退出绘制与引用。间隔连接需要正 bridge_width；images/text/groups 不支持。

返回：instance

[最小完整源码](../../examples/manual/fuse.lay) · [组合源码](../../examples/gallery/shapes/fuse-angled.lay) · [全部参数](interface-reference.zh-CN.md#fuse)

<!-- walkthrough:end -->

## 详细行为与补充示例

### 参数

| 参数 | 用途 | 默认或要求 |
| --- | --- | --- |
| `width / height` | 组实例缩放尺寸 | 可选；连同子元素缩放 |
| `target / offset` | 容器中的位置 | 组内使用局部坐标 |

### 常见用法

先完成组内放置，再把组加入画布。组可复用和嵌套；缩放组会同时缩放其子元素，包括文字与线宽。

[完整参数与规则](../user-guide.zh-CN.md)

### 限制与相关主题

[单位与尺寸](units.zh-CN.md) · [放置与锚点](anchors.zh-CN.md) · [旋转与图层](transforms.zh-CN.md)

## workflow

本流程的完整源码与可执行验证文件列在[功能覆盖清单](feature-map.zh-CN.md)。组合使用时请遵循本页的输入条件与限制。
