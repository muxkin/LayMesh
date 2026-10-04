# 旋转与图层

<!-- example:examples/gallery/positioning/rotation.lay -->

## 参数

| 参数 | 用途 | 默认或要求 |
| --- | --- | --- |
| `rotation` | 旋转角 | 0 |
| `opacity` | 不透明度 | 1 |

## 常见用法

画布和布局使用物理尺寸。可用单位为 `mm`、`cm`、`in`、`pt` 和 `px`；`px` 按画布的 `layout_dpi` 换算，默认值为 96。`--dpi` 只决定 PNG 导出像素数，不改变 SVG、PDF 的物理尺寸或对象位置。例如 160 × 100 mm 的画布按 300 DPI 导出约为 1890 × 1181 px。

放置时，`anchor` 指当前实例的锚点，`target` 指画布、当前组或此前创建的实例的锚点；`offset` 在目标位置上增加偏移：

九个锚点是 `top_left`、`top_center`、`top_right`、`middle_left`、`center`、`middle_right`、`bottom_left`、`bottom_center`、`bottom_right`。相对定位只能引用同一坐标空间中已经创建的实例。`add` 顺序也是从底到顶的绘制顺序。实例可以设置 `rotation`、`opacity`；图片实例还可分别设置 `width`、`height`、`fit` 和 `crop`。同一张图片放置多次时，各实例的缩放和裁剪互不影响。

## 限制与相关主题

[单位与尺寸](units.zh-CN.md) · [放置与锚点](anchors.zh-CN.md) · [组与复用](groups.zh-CN.md)


## workflow

本流程的完整源码与可执行验证文件列在[功能覆盖清单](feature-map.zh-CN.md)。组合使用时请遵循本页的输入条件与限制。
