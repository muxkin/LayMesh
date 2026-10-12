# 动画手册

从[样式与动画库](style-libraries.zh-CN.md)开始：先了解内置库、来源和导入方法，再逐项尝试、调参和组合；单文件 `.lay` 就能起步。

动画附着在完成的静态排版上。先运行第一段动画，再学习时序、关键帧、效果与放映。

本手册描述当前源码；新增离线样式库和诊断功能应使用包含这些改动的引擎构建。发行包功能以对应发布说明为准。

## 学习路线

1. [第一段动画](animation-start.zh-CN.md)
2. [步骤与动作参数](animation-timing.zh-CN.md)
3. [自定义关键帧](animation-keyframes.zh-CN.md)
4. [97 个效果目录](animation-catalog.zh-CN.md)
5. [描线与揭示](animation-extensions.zh-CN.md)
6. [放映与多页演示](animation-presentation.zh-CN.md)
7. [导出、兼容与排错](animation-export.zh-CN.md)

## 样式与动画一起使用

从[三组综合案例](style-animation-examples.zh-CN.md)学习完整组合：[三库卡片](style-animation-examples.zh-CN.md#styled-cards)、[艺术字海报](style-animation-examples.zh-CN.md#art-direction)、找回的[复杂仪表板](style-animation-examples.zh-CN.md#complex-dashboard)。每组包含可编辑布局、主题和实际播放。

## 完整可运行示例

<!-- example:examples/animation/first.lay -->

## 常用问题

Animate.css 必须显式导入，同时使用基础类和效果类。放映默认等待点击第一步；进入即播放可设置 start="after-previous"。详见[第一段动画](animation-start.zh-CN.md)。

<details>
<summary>原章节链接</summary>

## step

[查看完整章节](animation-timing.zh-CN.md)

## 正式放映

[查看完整章节](animation-presentation.zh-CN.md)

## 退出与移动终态

[查看完整章节](animation-start.zh-CN.md)

## animate

[查看完整章节](animation-timing.zh-CN.md)

## “点击”“跟随前一动画”与自动放映

[查看完整章节](animation-start.zh-CN.md)

## Compatibility

[查看完整章节](animation-export.zh-CN.md)

## Extensions

[查看完整章节](animation-start.zh-CN.md)

## Export

[查看完整章节](animation-export.zh-CN.md)

## Verification

[查看完整章节](animation-export.zh-CN.md)

</details>
