# 入门与样式库

从[样式与动画库](style-libraries.zh-CN.md)开始：先了解内置库、来源和导入方法，再逐项尝试、调参和组合；单文件 `.lay` 就能起步。

[样式手册](lcss.zh-CN.md) · 下一章：[选择器、变量与级联](style-cascade.zh-CN.md)

## 外观与排版各放在哪里

`.lay` 决定页面尺寸、对象尺寸、数据、锚点和位置；LCSS 决定可复用的字体、颜色、边框、圆角与效果。给对象添加多个 class，可以组合外观。只用一次的调整可以继续写显式参数，显式参数优先。

~~~lay
page = canvas(size=(80mm,50mm))
style {
  .card { background:#eff6ff; border:0.5pt solid #0d6efd; border-radius:2mm; }
}
page.add(rect(size=(30mm,20mm),class="card"),anchor=center,target=page.center)
~~~

style 块内部采用样式语法；外部仍是 Lay 语法。类名之间用空格，源码中不写 CSS 选择器的点号。

## 独立主题与相对路径

把重复声明放入 theme.lcss，页面使用 `canvas(stylesheet="theme.lcss")`。主题里的图片、字体和 @import 相对于主题文件，页面中的资源相对于页面文件。复制项目时同时保留主题和依赖。

~~~lcss
/* theme.lcss */
@import "@laymesh/bootstrap.css";
@import "@laymesh/open-props.css";
.card {
  background: var(--gradient-7);
  border-radius: var(--radius-3);
  box-shadow: var(--shadow-3);
}
~~~

也可以使用有序列表：

~~~lay
page = canvas(size=(80mm,50mm),
              stylesheet=["@laymesh/bootstrap.css","theme.lcss"])
page.add(rect(size=(30mm,20mm),class="card"),anchor=center,target=page.center)
~~~

列表依次加载；内嵌声明排在入口的外部文件之后。同重要性、同具体程度时，后声明覆盖前声明。更具体的选择器和 !important 仍参与比较。

## 三套离线库

| 库与固定版本 | 导入路径 | 用途 |
| --- | --- | --- |
| Bootstrap 5.3.8 | @laymesh/bootstrap.css | 外观工具类 |
| Open Props 1.7.23 | @laymesh/open-props.css | 配色、圆角、渐变、阴影变量 |
| Animate.css 4.1.1 | @laymesh/animate.css | 动画类和关键帧 |

资源随引擎分发，原生、Python、WASM 和 VS Code 使用相同文件，无需联网下载。三套库都按需显式导入；animate__fadeInUp 类名不会自动加载库。版本、原始来源、许可及 SHA-256 记录在[资源清单](../../crates/laymesh-core/assets/styles/manifest.json)。

## Bootstrap 常用外观类

| 类 | 用法 |
| --- | --- |
| bg-primary、bg-success | 区域背景色 |
| text-primary | 文字颜色 |
| border、border-primary | 边框及边框颜色 |
| rounded | 圆角 |
| shadow-sm、shadow、shadow-lg | 阴影 |
| fw-bold、fst-italic | 字重、斜体 |

`class="bg-primary rounded shadow"` 把背景、圆角、阴影组合到矩形。Bootstrap 的宽高、margin、flex、grid 等网页布局类不会安排对象位置；尺寸和锚点排版关系继续写在 .lay。

## Open Props 与局部主题

Open Props 提供变量，需要把变量用于实际属性。--blue-7 可用于颜色，--radius-3 可用于圆角，--gradient-7 可用于背景，--shadow-3 可用于盒阴影。变量存在不等于所有上游表达式都可渲染；不支持的值使使用它的声明失效。

下面项目的三个卡片共用主题。第二个只把 Bootstrap 阴影的模糊改为 2mm，其余参数保留。打开完整源码可编辑页面及主题。

<!-- example:examples/manual/style-project.lay -->

第一张卡片相对 `page.top_left` 设置页边距；后续卡片连接前一张的 `top_right`，标签连接各自卡片的 `bottom_left`。`card_size` 和 `gap` 统一控制尺寸与间距：例如把宽度从 30mm 改成 26mm，后两张卡片和标签随锚点移动。主题保存外观，循环消除重复，锚点保存排版关系，见[定位手册](anchors.zh-CN.md)与[循环和函数](control-flow.zh-CN.md)。

锚点对齐的是布局框，阴影不会增加卡片间距。固定画布不会自动变大；内容增大后仍需检查边界。局部路径、图表数据和首个对象的页边距可以使用坐标，重复对象之间应保存对齐与间距关系。

## 为何加载库后出现警告

完整上游文件包含尚未实现的网页属性。每个文件集中给出一条 W_CSS，有效的外观声明仍然生效。文件缺失、导入循环会明确报错，详见[兼容与排错](style-compatibility.zh-CN.md)。

## 样式与动画综合案例

[三库卡片与艺术字海报](style-animation-examples.zh-CN.md)展示主题、布局、关键帧和步骤如何一起使用，并提供已有复杂仪表板场景的完整源码。
