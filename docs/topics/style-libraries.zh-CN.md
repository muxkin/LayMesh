# 样式与动画库：从导入到组合

## 从这里开始

本章先运行，再查目录和调参。内置库随引擎提供，但默认没有启用任何一套；使用哪一套，就显式导入哪一套。`.lcss` 文件不是必需的，单个 `.lay` 文件也可以组合样式与动画。

## 三套库分别是什么

| 库 | 固定版本 / MIT | 上游项目与官方文档 | 内置导入路径 | 提供什么 |
| --- | --- | --- | --- | --- |
| Bootstrap | 5.3.8 | [项目](https://github.com/twbs/bootstrap) · [文档](https://getbootstrap.com/docs/5.3/) | `@laymesh/bootstrap.css` | 现成外观工具类与组件样式；LayMesh 使用受支持的外观部分 |
| Open Props | 1.7.23 | [项目](https://github.com/argyleink/open-props) · [文档](https://open-props.style/) | `@laymesh/open-props.css` | 配色、渐变、圆角、阴影、字体、缓动及动画变量 |
| Animate.css | 4.1.1 | [项目](https://github.com/animate-css/animate.css) · [文档](https://animate.style/) | `@laymesh/animate.css` | 基础类、97 个效果、速度/延迟/重复工具类和关键帧 |

三套原始 CSS、版本、许可证和 SHA-256 由[资源清单](../../crates/laymesh-core/assets/styles/manifest.json)固定。[许可证文件](../../release/THIRD_PARTY_NOTICES.md)随项目保留。`@laymesh/` 是引擎资源名，不是需要自己创建的磁盘目录。

## 读懂最少的五个名字

| 名称 | 写在哪里 | 用途 |
| --- | --- | --- |
| `stylesheet` | `canvas()` 参数 | 导入一个文件或有序文件列表 |
| `class` | 素材、实例或 `animate()` 参数 | 引用类名；多个类以空格分隔，不带选择器的点号 |
| `style { ... }` | `.lay` 文件 | 内嵌 CSS 声明，避免单独主题文件 |
| `var(--name)` | 样式属性值 | 使用已导入或自定义的变量 |
| `@keyframes` | `style` 或样式文件 | 定义自定义动画；由 `animate()` 执行动作 |

Open Props 提供变量，需要把变量用于属性；Bootstrap 常用工具类可以直接套用；Animate.css 的效果类需与 `animate__animated` 基础类一起使用。

## 先分别运行三个库

[Bootstrap 用法与目录](style-bootstrap.zh-CN.md) · [Open Props 用法与目录](style-open-props.zh-CN.md) · [Animate.css 用法与目录](style-animate-library.zh-CN.md)

下面分别只有一个库，源码可直接运行。

<!-- example:examples/libraries/bootstrap.lay -->

<!-- example:examples/libraries/open-props.lay -->

<!-- example:examples/libraries/animate.lay -->

## Bootstrap 加动画：无需自写 CSS

<!-- example:examples/libraries/bootstrap-animate.lay -->

外观由 `bg-primary rounded shadow` 提供，动作使用 `animate__animated animate__fadeInUp`。步骤设置 `after-previous`，进入放映后自动开始；省略时默认等待点击。

## Open Props 加动画：少量内嵌样式

<!-- example:examples/libraries/open-props-animate.lay -->

`--gradient-7`、`--radius-3`、`--shadow-3` 作用于外观，Animate.css 处理入场。这份项目仍只有一个 `.lay` 文件。

## 组合三个库并局部覆盖

<!-- example:examples/libraries/all-three.lay -->

只改 Bootstrap 阴影模糊为 `2mm`，保留其他字段。`!important` 与更具体的选择器对应库的阴影声明。循环保存实例并计算延迟，卡片和标签通过锚点排列。

## 下一步：调参、复用、第三方库

1. [选择器与覆盖](style-cascade.zh-CN.md)：显式 `.lay` 参数优先，CSS 之间再比较重要性、具体程度与顺序。
2. [属性与单位](style-properties.zh-CN.md)、[阴影](style-shadows.zh-CN.md)：先改颜色、圆角、模糊，再学相对单位和计算表达式。
3. [动作参数](animation-timing.zh-CN.md)：修改时长、延迟、重复和衔接。
4. [共享主题与第三方 CSS](style-third-party.zh-CN.md)：只有跨文件重复时才抽出主题。
5. [综合项目](style-animation-examples.zh-CN.md)：再阅读艺术字与复杂仪表板。

## 版本与兼容范围

本教程面向当前源码构建；已发布包以其发布说明为准。完整上游 CSS 可以加载，外观按 LayMesh 支持范围生效。库不会提供 HTML 组件行为，尺寸、定位和数据继续用 `.lay` 与锚点；未知内容集中提示，详见[兼容与排错](style-compatibility.zh-CN.md)。
