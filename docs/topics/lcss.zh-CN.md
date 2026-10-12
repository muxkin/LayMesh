# 样式手册

从[样式与动画库](style-libraries.zh-CN.md)开始：先了解内置库、来源和导入方法，再逐项尝试、调参和组合；单文件 `.lay` 就能起步。

尺寸、锚点、数据和定位写在 .lay；重复外观放入主题。按下列章节学习，再用逐项参考查询参数。

本手册描述当前源码；新增离线样式库和诊断功能应使用包含这些改动的引擎构建。发行包功能以对应发布说明为准。

## 学习路线

1. [入门与样式库](style-start.zh-CN.md)
2. [选择器、变量与级联](style-cascade.zh-CN.md)
3. [外观属性与单位](style-properties.zh-CN.md)
4. [阴影与效果](style-shadows.zh-CN.md)
5. [兼容与排错](style-compatibility.zh-CN.md)

## 样式与动画一起使用

从[三组综合案例](style-animation-examples.zh-CN.md)学习完整组合：[三库卡片](style-animation-examples.zh-CN.md#styled-cards)、[艺术字海报](style-animation-examples.zh-CN.md#art-direction)、找回的[复杂仪表板](style-animation-examples.zh-CN.md#complex-dashboard)。每组包含可编辑布局、主题和实际播放。

## 完整可运行示例

<!-- example:examples/manual/style-project.lay -->

第一张卡片相对 `page.top_left` 设置页边距；后续卡片连接前一张的 `top_right`，标签连接各自卡片的 `bottom_left`。`card_size` 和 `gap` 统一控制尺寸与间距：例如把宽度从 30mm 改成 26mm，后两张卡片和标签随锚点移动。主题保存外观，循环消除重复，锚点保存排版关系，见[定位手册](anchors.zh-CN.md)与[循环和函数](control-flow.zh-CN.md)。

## 常用问题

显式 .lay 参数优先于 LCSS，包括重要声明。主题控制圆角时应省略对象的同名参数；[覆盖规则](style-cascade.zh-CN.md)说明原因。只把阴影模糊改成 2mm，见[局部覆盖](style-shadows.zh-CN.md#partial-overrides)。

<details>
<summary>原章节链接</summary>

## 加载与覆盖顺序

[查看完整章节](style-start.zh-CN.md)

## 填充与边框

[查看完整章节](style-start.zh-CN.md)

## 资源和限制

[查看完整章节](style-start.zh-CN.md)

## style-properties

[查看完整章节](style-properties.zh-CN.md)

## css-libraries

[查看完整章节](style-start.zh-CN.md)

## shadow-overrides

[查看完整章节](style-shadows.zh-CN.md)

## css-units

[查看完整章节](style-properties.zh-CN.md)

## css-effects

[查看完整章节](style-shadows.zh-CN.md)

## css-compatibility

[查看完整章节](style-compatibility.zh-CN.md)

## workflow

[查看完整章节](style-start.zh-CN.md)

</details>
