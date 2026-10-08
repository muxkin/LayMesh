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

`text(spans=[span("比例："), formula(r"\frac{a}{b}")], font_size=12, color="#245447")` 中未指定字号和颜色的公式继承周围文字。局部 `span` 的字体列表覆盖父列表，未指定时继承。数学字体默认使用 RaTeX 的 KaTeX 公式字体；`font_family` 控制正文，不能替代数学字体包。

SVG/PDF 中公式为矢量图形，SVG 元数据保留 LaTeX 源码。公式错误会带文件和位置；自动公式受 RaTeX 支持范围限制，自定义宏定义及外部资源命令不可用。Python `{{变量}}` 仅在 DSL 字符串和注释之外绑定，不会替换本节的 f 字符串内容。

[文字与字体](fonts.zh-CN.md) · [参数参考](interface-reference.zh-CN.md) · [编辑器](editors.zh-CN.md)

## workflow

本流程的完整源码与可执行验证文件列在[功能覆盖清单](feature-map.zh-CN.md)。组合使用时请遵循本页的输入条件与限制。

## OpenType 数学字体（实验）

`math_font` 现在可指定带 OpenType MATH 表的已安装字体名称或字体文件路径，例如：

```lay
page=canvas(size=(100mm,50mm))
page.add(formula(r"\frac{a}{b}+\bm{\alpha}",math_font="XITS Math",font_size=16pt))
```

也可写 `math_font="./fonts/XITSMath-Regular.otf"`；路径相对于定义公式的模块，LCSS 中的路径相对于样式表。该属性可从画布、文字、span 和图表样式继承，适用于显式公式和 `$...$` 自动公式。浏览器需先加载对应字体文件。字体度量、数学间距、斜体修正和伸缩字形均来自所选字体；导出继续使用矢量路径。

当前实现已覆盖化学 `ce/pu`、证明树 `prooftree`、带标签箭头、`middle`、对齐与编号、数组横竖线、重叠排版、括注、取消线等构造。固定 RaTeX 0.1.14 测试集和额外领域用例共执行 15,448 次排版检查；化学 91 条、物理 22 条、证明树 38 条在四套字体、两种模式下全部渲染成功。解析器不支持的输入和原有禁止命令单独记录；字体缺字统一使用矢量方框占位，并以 `W_FONT` 标出缺失码点，公式其余部分继续排版和导出；字体不可用或缺少 MATH 表返回 `E_MATH_FONT`。`mathcal` 和 `mathscr` 暂时使用同一 Unicode 花体字母表。XITS 独立粗体文件的伸缩字形覆盖不完整；常规 XITS Math 可使用 `mathbf`、`boldsymbol` 和 `bm`。箭头缺少有效拼接构造时，延长所选字体原生轮廓的箭杆区域；其他无拼接构造的符号使用最大的原生变体。LaTeX 源码建议使用原始字符串，尤其是化学表达式内部包含 `$...$` 时。

数学字体命令由内层命令覆盖：`\mathbf{\mathcal A}` 选择花体 A，`\mathcal{\mathbf A}` 选择粗体正体 A。使用 `\mathbf{A1}` 写粗体字母和数字，使用 `\bm{\alpha x}` 写粗斜体符号。不同字体的花体造型与笔画粗细会不同。

OpenType 化学键 `~`、`~-`、`~--`、`~=`、`-~-` 按整体构造排版，以所选字体的减号轮廓确定宽度和线厚。实线与虚线共用左右端点，并继承当前字号、上下标层级和颜色。几何测试直接检查轮廓端点，也覆盖上标、嵌套下标和分数中的化学键。

[可编辑对照图和验证说明](../../experiments/opentype-math/README.md)。默认值仍为 `ratex-katex`，旧 `mathjax-*` 名称保留兼容映射。

公式中的正文文本使用 `font_family`，数学符号使用 `math_font`。例如：

```lay
page=canvas(size=(140mm,50mm),math_font="XITS Math",font_family=["Noto Serif","Noto Serif CJK SC"])
page.add(formula(r"\text{你好，世界！}\quad E=mc^2",font_size=16pt))
```

默认启用 `math_text_fallback=true`。`\text`、中文上下标、化学箭头说明和证明树中的文本在数学排版前完成字体选择、Unicode 塑形和真实尺寸测量，保留连字、结合音标和双向文字；导出文本字形为矢量轮廓。`\textbf`、`\textit` 可选用对应正文字体字重和斜体，缺少对应字形使用方框占位，缺少对应样式发出警告。数学粗体仍用 `\mathbf` / `\bm`。

正文列表按顺序匹配，显式列表不会偷偷加入系统字体。可使用字体路径；原生主机也可匹配系统字体，浏览器主机须先提供字体文件。`math_text_fallback=false` 禁用正文回退，用于检查数学字体自身的覆盖；所有公式字体的缺字策略一致：使用矢量方框替代，并发出 `W_FONT` 警告；显式正文字体列表耗尽时也保留公式。方框参与排版并随上下标缩放，已有字形与公式结构继续显示。没有可导出轮廓的彩色位图 emoji 同样使用方框。
