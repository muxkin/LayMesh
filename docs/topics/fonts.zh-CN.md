# 正文字体与 Page 预览

<!-- walkthrough:start -->
## 用途与概念

text 的字号是物理尺寸。宽度约束触发换行，spans 可在同一段中设置不同颜色、字体、字重或公式。文字的度量决定布局框，而不会提供字形轮廓锚点。

## 最小完整示例

此文件包含画布及必要定义，可直接运行 `laymesh validate` 或 `laymesh render`，不依赖前文未声明的变量。

<!-- example:examples/manual/text.lay -->

## 参数与默认行为

裸几何长度使用画布单位，裸字号和描边宽度使用 pt。显式调用参数优先于继承和主题默认值；下面的接口参考逐项列出允许类型、可选值与默认来源。

## 组合用法

复用 text 素材时可给不同实例设置宽度，让每个实例独立换行。font_family 使用名称、字体文件路径或有序回退列表；跨机器复现时随作品保存字体。

<!-- example:examples/gallery/typography/font.lay -->

## 常见错误与限制

font_weight 是 100–900 的整数。缺字产生 W_FONT 并显示度量后的方框；普通系统字体不随文档打包。font_style 的固定选项仍可写 normal/italic 或对应字符串。

## 逐项功能说明

### text

创建文字素材，自动识别普通字符串中的行内公式；可设置字体回退、字号、颜色与底框。size 可限定换行宽度，原始字符串关闭自动公式解析。

返回：可重复放置的文字素材。

必需输入：`content / spans`.

[最小完整源码](../../examples/manual/text.lay) · [组合源码](../../examples/gallery/typography/font.lay) · [全部参数](interface-reference.zh-CN.md#text)

### span

定义 text(spans=[...]) 内的一段文字及局部字体或颜色覆盖。span 本身不是可以独立放置的素材。

返回：value

必需输入：`content`.

[最小完整源码](../../examples/manual/span.lay) · [组合源码](../../examples/gallery/typography/inline-formula.lay) · [全部参数](interface-reference.zh-CN.md#span)

<!-- walkthrough:end -->

## 详细行为与补充示例

`font_family` 接受系统字体族名、用户字体文件路径或按优先顺序排列的列表。文件支持 TTF、OTF、TTC 和 OTC；集合字体使用 `文件路径#字面名称`。相对路径以定义资源的 `.lay` 模块为准。省略该参数时由系统解析默认 sans-serif 字体。

Windows 路径可写成 `"C:/Fonts/Example.ttf"` 或 `r"C:\Fonts\Example.ttf"`；普通字符串中的反斜杠需要转义。Python 中的动态路径建议作为字符串通过 `{{font}}` 绑定传入，避免直接拼接 DSL 源码。

```lay
page=canvas(size=(100,40))
page.add(text("My data", font_family="Arial", font_size=12))
page.add(text("中文内容", font_family=["用户已安装的中文字体", "Arial"], font_size=12), offset=(0,15))
```

缺少字体、缺少对应字重或缺少字符时输出带源码位置的 `W_FONT`；无法找到的字符用直接绘制的矢量方框表示。即使没有任何系统字体也会继续导出 SVG、PNG、PDF。`--warnings hide` 或 `LAYMESH_WARNINGS=hide` 仅控制警告显示。

浏览器无法擅自读取本机字体。在预览的“字体”菜单中授权使用本机字体，或选择字体文件；用户载入的字体只留在当前网页内存中。未授权或未加载的其他字体仍可能显示方框。当前 HTTP 预览地址对本机字体访问有限制，GitHub Pages 的 HTTPS 环境需另行验证。

Times New Roman、Arial、Helvetica、宋体、黑体、楷体等常见名称仍可用于已安装或用户提供的字体；Page 不内置这些字体，也不会把其他字体伪装成这些名称。现有示例使用 DejaVu Sans 绘制英文和科学图表，Noto Sans CJK SC 用于中文混排。

导出文件可按实际使用情况嵌入用户字体，PDF 会子集化；这不表示发行包携带正文字体。跨设备精确复现时应自行提供同一字体文件并确认其许可允许相应使用。
