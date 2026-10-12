# 本地第三方 CSS 与共享主题

[样式与动画库](style-libraries.zh-CN.md)

## 原样保存外部 CSS

第三方库通常是标准 `.css`。直接导入它，不用改名为 `.lcss`。下载到项目目录，并一起保留许可证及相对依赖；当前编译器不从 URL/CDN 自动下载。下面使用原样保存的 [Milligram](https://milligram.io/) 1.4.1 CSS（MIT），[清单](../../examples/libraries/third-party/vendor/manifest.json)记录固定源和 SHA-256。它是此示例的项目依赖，不是新增的内置库。

## 完整项目结构

```text
third-party/
  direct.lay
  main.lay
  theme.css
  palette.css
  assets/pattern.svg
  vendor/milligram.css
  vendor/LICENSE.txt
  vendor/manifest.json
```

[源码和资源目录](../../examples/libraries/third-party/main.lay)在完整源码标签中逐个提供；图片、主题和依赖跟随项目保存。

## 方法一：直接 stylesheet

<!-- example:examples/libraries/third-party/direct.lay -->

单文件用 `stylesheet="vendor/milligram.css"`；多个文件按列表顺序加载。示例后加载项目 palette，统一标签外观。来源于库的 button 类只提供可实现的外观。

## 方法二：字符串和 url 导入

<!-- example:examples/libraries/third-party/main.lay -->

`theme.css` 使用 `@import "vendor/milligram.css";` 和 `@import url("palette.css");`。同样写法也可以放进 `.lay` 的 `style` 块。导入路径相对声明导入的文件，背景 SVG 相对声明 `url()` 的样式文件；不是相对终端当前目录。

## 再把主题共享到多页

<!-- example:examples/libraries/shared.laydoc -->

两个 `.lay` 页面共同导入 `shared-theme.lcss`。自己写的共享主题也可以命名为 `.css`；`.lcss` 便于编辑器识别 LayMesh 样式及扩展。只有需要跨文件复用时才引入独立主题。

## 兼容、覆盖与编辑支持

导入完整 CSS 不代表全部网页功能可用。有效颜色、字体、边框、圆角、阴影等继续生效，不支持的选择器、属性和规则集中为 W_CSS；参见[兼容手册](style-compatibility.zh-CN.md)。文件缺失和导入循环仍报错。

同重要性及具体程度时后声明覆盖前声明；显式 `.lay` 参数始终优先。库的阴影分项覆盖见[阴影手册](style-shadows.zh-CN.md)。

VS Code 中外部 `.css` 保持原生 CSS 编辑服务。LayMesh 只将它作为依赖索引，在 `.lay` 和 `.lcss` 中提供已导入类、变量、关键帧的补全、悬停和跳转；未保存修改也会更新索引。网页编辑器和 Python 查询使用同一语言服务。
