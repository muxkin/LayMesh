# 字符串与数学公式

<!-- walkthrough:start -->
## 用途与概念

formula 接受不带美元定界符的 LaTeX 源码。使用 raw 字符串保留反斜杠；inline 适合文字行内，display 使用独立公式布局。math_font=ratex_katex 是内置字符串别名。

## 最小完整示例

此文件包含画布及必要定义，可直接运行 `laymesh validate` 或 `laymesh render`，不依赖前文未声明的变量。

<!-- example:examples/manual/formula.lay -->

## 参数与默认行为

裸几何长度使用画布单位，裸字号和描边宽度使用 pt。显式调用参数优先于继承和主题默认值；下面的接口参考逐项列出允许类型、可选值与默认来源。

## 组合用法

text(spans=[...]) 可组合 span 和 formula。普通文本中的 $...$ 与显式 formula 的前提不同：对控制反斜杠或格式化表达式有要求时使用 r/f 字符串，避免源码被二次解释。

<!-- example:examples/gallery/typography/display-formula.lay -->

## 常见错误与限制

公式引擎不接受外部资源、require 或自定义宏。旧 mathjax 字体名产生兼容警告，映射到当前默认字体。颜色支持所有当前颜色空间，与普通文字一致。

## 逐项功能说明

### formula

用不带美元定界符的 LaTeX 创建公式素材，或作为 text 的行内片段。raw 字符串保留反斜杠，style 控制 inline/display 排版。

返回：material

必需输入：`source`.

[最小完整源码](../../examples/manual/formula.lay) · [组合源码](../../examples/gallery/typography/display-formula.lay) · [全部参数](interface-reference.zh-CN.md#formula)

<!-- walkthrough:end -->

## 详细行为与补充示例

### 普通文字与原始字符串

```lay
page = canvas(size=(150, 100))
page.add(text("能量 $E=mc^2$", font_size=10), offset=(5, 5))
page.add(text(r"这里原样显示 $E=mc^2$"), offset=(5, 20))
page.add(formula(r"\frac{a}{b}", font_size=12), offset=(5, 35))
```

数学区域内直接书写 LaTeX 反斜杠，例如 `axis(label="$\frac{\Delta E}{k_B T}$")`。普通字符串支持单、双引号以及三引号多行字符串。`r` 保留原始文字并关闭自动公式识别；显式 `formula(r"…")` 始终渲染公式。资源路径不进行公式识别。

### 插值

```lay
score = 0.975
label = f"拟合结果：$R^2={score:.3f}$"
literal = rf"结果 {score:.2%}；原样显示 $R^2$"
```

花括号中可使用 DSL 表达式。数值格式支持常用的 `f`、`e`、`g`、`%`、`d`、精度、符号及对齐；`{{` 和 `}}` 表示字面花括号。`rf` 插值但不自动渲染公式。插值不会执行 Python 或 JavaScript。

### 混排与继承

`text(spans=[span("比例："), formula(r"\frac{a}{b}")], font_size=12, color="#245447")` 中未指定字号和颜色的公式继承周围文字。局部 `span` 的字体列表覆盖父列表，未指定时继承。数学字体使用 RaTeX 的 KaTeX 公式字体；`font_family` 控制正文，不能替代数学字体包。

SVG/PDF 中公式为矢量图形，SVG 元数据保留 LaTeX 源码。公式错误会带文件和位置；自动公式受 RaTeX 支持范围限制，自定义宏定义及外部资源命令不可用。Python `{{变量}}` 仅在 DSL 字符串和注释之外绑定，不会替换本节的 f 字符串内容。

[文字与字体](fonts.zh-CN.md) · [参数参考](interface-reference.zh-CN.md) · [编辑器](editors.zh-CN.md)

## workflow

本流程的完整源码与可执行验证文件列在[功能覆盖清单](feature-map.zh-CN.md)。组合使用时请遵循本页的输入条件与限制。
