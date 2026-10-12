# Bootstrap：类、参数与完整目录

[样式与动画库](style-libraries.zh-CN.md)

## 是什么、从哪里来

Bootstrap 是[开源网页界面框架](https://getbootstrap.com/docs/5.3/)，包含组件、网页布局和样式工具类。这里固定使用 5.3.8 的原始 CSS，MIT 许可。LayMesh 使用其中受支持的外观声明；给矩形加类不会创建网页按钮或网格。

## 第一张卡片

<!-- source:examples/libraries/bootstrap.lay -->

`bg-primary` 设置背景，`rounded` 设置圆角，`shadow` 添加阴影。颜色类作用于背景或文字，要选择相应对象。

## 按功能逐步学习

| 功能 | 先尝试 | 进一步调整 |
| --- | --- | --- |
| 背景和文字 | `bg-primary`、`bg-success`、`text-primary` | 配色变体，背景/文字不透明度工具类 |
| 边框 | `border border-primary` | 颜色、宽度、移除边框 |
| 圆角 | `rounded` | 目录中的半径变体；使用显式 `border_radius` 作单次例外 |
| 阴影 | `shadow-sm`、`shadow`、`shadow-lg` | 内嵌 CSS 单项覆盖 blur/spread/offset |
| 文字 | `fw-bold`、`fst-italic` | 字号、行高、字体族；实际字体由项目提供 |
| 透明度与留白 | `opacity-50`、受支持的 padding 类 | 留白不替代锚点排版 |

[局部覆盖示例](style-shadows.zh-CN.md#partial-overrides)讲清 2mm 模糊、扩散与偏移的区别。库声明中的 `!important` 不会压过显式 `.lay` 参数。

## 完整可检索目录

每个类、变量和关键帧保留上游声明及支持状态。可用项都能切换到完整源码与实际预览；辅助类可能搭配基础类。部分支持表示只应用可实现的声明，不表示完整网页组件已实现。

目录案例用低具体程度的 `rect` 底色与边框，让圆角、透明度和动画可见；类声明与变量属性照常级联。

<!-- style-catalog:bootstrap -->
