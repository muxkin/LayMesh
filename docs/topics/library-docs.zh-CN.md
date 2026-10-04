# 库作者文档注释

在函数或变量声明前写连续的 `##` 注释，即可为网页编辑器和 VS Code 提供补全详情、悬停说明和调用提示。注释只参与静态分析，不改变绘图结果，也不会执行示例代码。

## 函数与参数

下面是一个可放入组件模块的完整函数。第一段是用途摘要，后面的段落可以使用 Markdown；调用时还会显示当前参数的说明。

```lay
## 创建带标题的矩形面板，返回可重复放置的组合。
##
## 使用 `page.add` 放置面板，再通过实例锚点排列其他内容。
## @param {string} title - 面板标题，支持行内公式。
## @param {size} size - 面板尺寸；裸数字使用画布默认单位。
## @returns {group} 包含矩形和标题的组合。
## @example
## panel = titled_panel("结果", size=(40, 25))
export function titled_panel(title, size=(40, 25)) {
  panel = group()
  panel.add(rect(size=size))
  panel.add(text(title), offset=(2, 2))
  return panel
}
```

`title`、`size` 的顺序、必需性及默认值直接读取声明，不需要在注释中重复。`{类型}` 可以省略；它是给读者和编辑器看的说明，不新增运行时类型约束。单位、允许值和限制写在参数说明中。

## 标签与变量说明

| 写法 | 用途 |
| --- | --- |
| `@param {类型} 名称 - 说明` | 具名参数和位置参数的说明；类型可省略 |
| `@returns {类型} 说明` | 返回值说明；类型可省略 |
| `@type {类型}` | 变量的显示类型 |
| `@example` | 后续注释行作为 LayMesh 示例代码，直到下一个标签或文档块结束 |
| `@lang zh-CN` / `@lang en` | 开始一个语言分段 |
| `@see` | 相关主题或 Markdown 链接；允许多个 |

```lay
## 面板默认强调色。
## @type {color}
export accent = "#245447"
```

标签的说明可以续写到下一行。文档块必须紧接声明；文档内部的空行写为 `##`，真正的空白行会断开关联。普通 `#` 注释仍是普通注释。同样的写法适用于没有 `export` 的本地函数和变量。

输入 `## ` 后按 `Ctrl+Space` 插入与下一条声明匹配的文档模板；输入 `## @` 可选择标签，输入 `## @param ` 可补全尚未说明的参数名。

参数名写错、重复的参数或单值标签、不支持的标签会产生 `W_DOC` 编辑器警告。它们不阻止渲染；没有注释的函数仍显示从源码提取的签名。

## 多语言说明

使用 `@lang` 为同一声明提供不同语言的说明。每个分段持续到下一个 `@lang` 或文档块结束；摘要、参数、返回值、示例和相关链接都可以分别编写。

```lay
## @lang zh-CN
## 创建带标题的面板。
## @param {string} title - 面板标题。
## @returns {group} 可重复放置的组合。
## @lang en
## Create a titled panel.
## @param {string} title - Panel heading.
## @returns {group} A reusable group.
export function titled_panel(title) {
  panel = group()
  panel.add(rect(size=(40, 25)))
  panel.add(text(title), offset=(2, 2))
  return panel
}
```

网页和 IDE 一次显示一种语言，选择方式见[提示语言](editors.zh-CN.md#提示语言)。每项缺失说明按“当前语言 → 未标语言的默认原文 → 英文 → 源码中第一个可用版本”回退；示例和相关链接分别作为列表选择。不会自动翻译。没有 `@lang` 的现有注释仍然有效，默认原文可以写在第一个语言标签之前。

`zh` 及中文地区标签归入 `zh-CN`，英文地区标签归入 `en`；其他有效语言标签可保留为原文回退版本。一个文档块内每种规范化语言只能出现一次。同一参数在不同语言中重复说明合法，同一语言中的重复标签会产生警告。名称、默认值和类型保持一致；冲突类型产生 `W_DOC`，编辑器保留源码中首先声明的类型，运行行为不变。

输入 `## ` 后按 `Ctrl+Space`，可选“双语文档模板”；`## @lang ` 提供 `zh-CN` 和 `en`。`@param` 参数名补全只排除当前语言分段中已经说明的参数。导出变量也可为每种语言分别提供文字和相同的 `@type`。

## 模块和显示方式

库的文档跟随实际导入关系解析，导入别名保留原函数说明。修改模块中的说明或默认值后，两端编辑器会刷新提示，不需要重新渲染图形。同名局部函数使用所在作用域的说明。编辑未完成时，仍提供已识别声明的帮助。

作者的注释按上述语言和回退规则展示；内置接口说明随站点或编辑器语言切换。支持段落、列表、强调、代码块和链接。网页不会执行注释中的 HTML，也不加载远程图片。

下面的示例包含有文档注释的 `card-component.lay`。切换文件即可查看；在入口文件调用 `card(...)` 时可查看参数帮助。

<!-- example:examples/gallery/containers/module-import.lay -->

[函数与模块](modules.zh-CN.md) · [编辑器与按键](editors.zh-CN.md) · [公开参数参考](interface-reference.zh-CN.md)

## workflow

本流程的完整源码与可执行验证文件列在[功能覆盖清单](feature-map.zh-CN.md)。组合使用时请遵循本页的输入条件与限制。
