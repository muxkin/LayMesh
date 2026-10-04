# `.lay` 语言与功能参考

本页描述 当前 Rust 引擎的**可执行语法**。按“画布 → 素材定义 → 放置实例 → 导出”阅读；对应的完整代码和实际输出见[示例与执行结果](examples-and-results.zh-CN.md)。`.lay` 由 LayMesh 解析器解释，不能用 Python 或 JavaScript 运行。

## 1. 文件、值和单位

入口 `.lay` 文件必须定义且只能定义一个 `canvas(...)`。构造器返回可复用的**素材定义**，`page.add(...)` 返回一个**放置实例**。只有实例才有页面锚点。字符串支持单、双、三引号，以及 r/f/rf 前缀；普通文字中的 `$…$` 自动渲染公式。`#` 开始行注释；调用使用括号和逗号，缩进不影响语义。

| 写法 | 含义 |
| --- | --- |
| `12`、`true`、`false`、`"标题"` | 标量、布尔和字符串 |
| `12 mm`、`1.2 cm`、`0.5 in`、`10 pt` | 长度；内部统一用 mm |
| `16 px` | 长度；用画布 `layout_dpi` 换算，默认 96 |
| `45 deg` 或放置时 `45` | 角度，单位为度 |
| `(x, y)`、`[a, b]`、`items[0]` | 元组、列表、从零开始的索引 |
| `+ - * /`、比较、`and or not` | 带单位运算和布尔表达式；`and/or` 短路 |

同类长度可相加减；长度乘除标量仍是长度；长度除长度得到标量。不同类型相加、除零和超出范围的值会报错。`px` 须在画布建立 `layout_dpi` 后使用。**PNG 的输出 `--dpi` 与布局 `layout_dpi` 各自独立**。

### 画布 `canvas`

```text
page = canvas(name="Figure 1", size=(180 mm, 120 mm),
              layout_dpi=96, background="#ffffff")
```

| 参数 | 含义及默认值 |
| --- | --- |
| `size` | 必填，两个正长度 `(width, height)`，决定 SVG/PDF 物理页面 |
| `name` | 可选，Scene 名称；默认 `"Untitled"` |
| `layout_dpi` | 可选，正数；默认 96，仅影响布局 `px` 和没有显式尺寸的图像 |
| `background` | 可选，颜色值、十六进制字符串或 `"none"`；默认透明 |

画布名可以是 `page` 或其他变量名。导出 PNG 的像素宽高分别按 `round(mm × dpi / 25.4)` 计算。[基础示例](../examples/basic.lay)的 180 × 120 mm 页面在 300 DPI 实测为 2126 × 1417 px。

## 2. 素材构造函数

![SVG 素材输入的实际渲染结果](../site/media/gallery-images-svg-1920.webp)

下表中的路径参数相对于**定义该素材的 `.lay` 文件**解析；从模块导入后仍按原模块定位。除特殊注明，素材创建后可以多次 `add`。

| 函数 | 必填参数 | 可选参数及行为 | 示例 |
| --- | --- | --- | --- |
| `image` | `src="..."` | PNG、JPEG、BMP、WebP、GIF、ICO、PNM、TGA、安全 SVG、单页无符号 8/16 位灰度/RGB TIFF（可带 Alpha）；素材先读取并规范化 | [basic](../examples/basic.lay) |
| `text` | `content="..."` **或** `spans=[...]`，二选一；`font_size=10 pt` | `font_family`、`font_weight=400`、`font_style="normal"`、`color="#000000"`、`line_height`、`size=(80, auto)`、`align=left` | [typography](../examples/typography.lay) |
| `span` | 第一个位置参数为字符串 | 仅在 `text(spans=[...])` 中使用；可覆盖 `font_family`、`font_size`、`font_weight`、`font_style`、`color` | [typography](../examples/typography.lay) |
| `formula` | `source=r"..."`、`font_size=10pt` | `style=inline|display`、`math_font="ratex-katex"`；可独立放置或作为 text 的一个 span | [typography](../examples/typography.lay) |
| `rect` | `size=(宽, 高)` | `fill="none"`、`border_radius=0`，以及下文的描边参数 | [outlines](../examples/outlines.lay) |
| `ellipse` | `size=(宽, 高)` | `fill="none"` 与描边参数；不接受非零圆角 | [basic](../examples/basic.lay) |
| `line` | `dx/dy` 或 `length/angle` | 完整逻辑中心线；零长度须显式角度；独立端帽和 `head(...)`；默认线宽 `0.3pt` | [端部规则](topics/shapes.zh-CN.md) |
| `group` | 无 | `g.add(...)` 构建子元素；可嵌套与重复放置 | [scripted](../examples/scripted.lay) |

`text` 的 `font_weight` 须为 100–900 的整数；`align` 为 `left/center/right/justify`。设置 `size=(80, auto)` 后按 Unicode 换行机会排版，超长词再按字素边界换行；`\n` 强制换行。未设置宽度时保留单行或显式换行。放置时的 `size=(80, auto)` 可以覆盖定义时的文字排版宽度，重新计算行位置。[排版示例](../examples/typography.lay)同时演示混合字体、颜色、行内公式和独立公式。

`font_family` 可以是系统字体族名、名称与路径组成的有序列表、TTF/OTF 文件，或 `"fonts/collection.ttc#PostScriptFace"` 形式的 TTC/OTC 字体。默认字体由系统解析；发行包不携带正文字体。按字符簇依次回退；全部缺字时显示方框并汇总 `W_FONT`，继续导出。可变字体目前使用静态替代。公式由 RaTeX 排版，默认 `ratex-katex`；旧 `mathjax-*` 参数警告后映射到新默认字体；自定义宏、`\require` 和外部资源不接受。

### 矢量构造函数

![路径命令的实际渲染结果](../site/media/gallery-shapes-path-commands-1920.webp)

它们都接受 `fill`、`fill_rule=nonzero|evenodd` 和描边参数；`polyline`、`arc` 默认无填充并带黑色描边。所有坐标、半径和点坐标是长度，角度可用 `deg`。

| 函数 | 几何参数 | 说明与示例 |
| --- | --- | --- |
| `path` | `commands=[move_to(...), ...]` | 从 `move_to` 开始；可含多个子路径与洞；最多 10,000 条命令。见[矢量示例](../examples/vector.lay) |
| `polygon` | `points=[(x,y), ...]` | 至少 3 点，自动闭合 |
| `polyline` | `points=[(x,y), ...]` | 至少 2 点，不自动闭合 |
| `arc` | `radius`、`start`、`end` | 开放圆弧；角度跨度大于 0 且小于 360° |
| `sector` | `radius`、`start`、`end` | 扇形，自动封闭 |
| `star` | `points`、`outer_radius`、`inner_radius`，可选 `rotation` | 顶点数 3–1000，内半径小于外半径 |
| `ring` | `outer_radius`、`inner_radius` | 使用 `evenodd` 形成透明内孔 |

`path(commands=...)` 的命令函数如下；仅在路径命令列表中有意义：

| 命令 | 位置参数顺序 | 含义 |
| --- | --- | --- |
| `move_to` | `x, y` | 开始子路径 |
| `line_to` | `x, y` | 直线终点 |
| `quad_to` | `cx, cy, x, y` | 二次贝塞尔控制点与终点 |
| `cubic_to` | `c1x, c1y, c2x, c2y, x, y` | 三次贝塞尔控制点与终点 |
| `arc_to` | `rx, ry, rotation, large_arc, sweep, x, y` | 椭圆弧；两个标记是布尔值 |
| `close` | 无 | 关闭当前子路径 |

### 颜色、渐变和边框

![线性渐变的实际渲染结果](../site/media/gallery-shapes-linear-gradient-1920.webp)

`fill` 接受 `#RGB/#RGBA/#RRGGBB/#RRGGBBAA`、`rgb(...)`、`hsv(...)`、`oklch(...)`、`"none"`，以及 `linear_gradient(...)` 或 `radial_gradient(...)` 的返回值。渐变色标为 `(位置, 颜色)` 或 `(位置, 颜色, 透明度)`；位置和透明度均在 0–1。线性渐变可指定 `start=(x,y)`、`end=(x,y)`，默认 `(0,0)` 至 `(1,0)`；径向渐变可指定 `center` 与正数 `radius`，默认 `(0.5,0.5)` 和 `0.5`。渐变坐标按实例边界的比例解释。见[矢量图](../examples/vector.lay)。

```text

card = rect(size=(42 mm, 22 mm), fill="#e7f6f6", border_color="#087f8c", border_width=1.8 mm, border_style=double, border_dash=[4 * (1.8 mm), 2 * (1.8 mm), 0.01 * (1.8 mm), 2 * (1.8 mm)], border_cap="round", border_opacity=0.8)
```

封闭轮廓统一使用 `border_color/width/style/dash/cap/join/opacity`；开放线条使用对应的 `line_*`。样式为 `solid/dashed/dotted/dash_dot/double/triple/none`；双线和三线间隔透明。复用样式通过 LCSS 类，`outline()` 已移除。[主题与填充](topics/lcss.zh-CN.md)。

## 3. 放置、锚点、组与融合

![组复用的实际渲染结果](../site/media/gallery-containers-group-reuse-1920.webp)

```text
first = page.add(photo,size=(80 mm, auto), target=page.top_left,
                 offset=(8 mm, 12 mm))
second = page.add(photo,size=(40 mm, auto), anchor=top_left,
                  target=first.top_right, offset=(4 mm, 0 mm))
```

`canvas.add(material, ...)` 与 `group.add(material, ...)` 的第一个参数是素材定义，返回实例。无需引用的实例可以直接写 `page.add(...)`。通用选项为 `anchor`（默认 `top_left`）、`target`（默认当前容器的左上角）、`offset=(0 mm,0 mm)`、`rotation=0` 和 `opacity=1`。`add` 顺序即从后到前的绘制顺序。九个锚点是 `top_left/top_center/top_right`、`middle_left/center/middle_right`、`bottom_left/bottom_center/bottom_right`，旋转后按轴对齐外接框求值。目标必须位于同一画布或组中且此前已经放置。组自身只提供 `group.top_left` 作为组内定位原点。

新增显式几何视图 `instance.bounds/path/ink`，候选查询必须索引；`self` 自身选择器、弧长与参数取点、切线法线偏移和图表内部部件遵循统一定位规则。详见[几何锚点与路径定位](topics/anchors.zh-CN.md)。

线还可用 `instance.start/end` 读取端点，用 `anchor="start"/"end"` 放置端点。原生图表支持九个带 `plot_` 前缀的绘图区锚点，以及 `instance.data(x=...,y=...)` 数据锚点；例如 `anchor="plot_top_left"`、`target=chart.plot_center`。这些锚点随实例旋转，详见[固定绘图区与数据标注](plotting.zh-CN.md#固定绘图区与数据标注)。

按素材类型允许的实例选项：

| 素材 | 额外 `add` 参数 | 规则 |
| --- | --- | --- |
| 图片 | `size=(宽, 高)`、`fit=contain|cover|stretch`、`crop=box(offset=(x,y), size=(w,h))` | 只给一个尺寸时按裁剪后宽高比推导另一尺寸；裁剪取原图的 0–1 比例，SVG 素材不支持 `crop` |
| 文字 | `size=(80, auto)` | 为该实例重新换行；不改变原素材 |
| 原生图表 | `size=(宽, 高)` | 覆盖外框并重新排版，保持字体、线宽和标记尺寸；指定 `plot_area` 时绘图区也保持固定 |
| 矩形、椭圆、路径、组 | `size=(宽, 高)` | 缩放当前放置实例；组缩放全部子元素 |
| 公式、线 | 无尺寸覆盖 | 可使用所有通用选项 |

组使用自己的局部坐标。已放置的组会封存，不能再添加子元素；可再次放置同一组，嵌套最多 128 层，组不能包含自身。图片的裁剪先于适配：`contain` 保持比例完整放入框内，`cover` 保持比例并裁到框内，`stretch` 直接填满框。说明和实例见[使用指南](user-guide.zh-CN.md)。

`page.fuse(a, b, ...)` 或 `group.fuse(a, b, ...)` 把同一容器内**已经放置**的两个矢量实例合成新路径。可选 `points=(a.anchor,b.anchor)`；省略时按可见轮廓最近点连接。有间隙时必须给正的 `bridge_width`；`junction=miter|bevel|round`，`round` 还必须给 `radius`。结果使用 `fill` 与 `border_*`，也可指定描边选项、`fill_rule`、`opacity`。输入实例从当前绘制顺序中移除，之后不能再引用；同一素材定义的其他放置实例不受影响。图片、文字和组不能融合。见[矢量图的 `joined`](../examples/vector.lay)。

对椭圆或自由曲线使用显式 `points` 时，锚点须让连接段实际接触两侧的可见轮廓。锚点来自外接框，内凹曲线的 `middle_right` 等锚点可能落在曲线外；`junction=round` 遇到这种脱离轮廓的接点会返回带源码位置的 `E_FUSE`。省略 `points` 可由引擎选择最近的可见轮廓点。斜线接入[平面边界](gallery/shapes.zh-CN.md#fuse-angled)与[自由曲线边界](gallery/shapes.zh-CN.md#fuse-curved)的源码和渲染图见画廊。

融合后的路径默认使用 `evenodd` 填充规则，以保留空心轮廓的内孔；显式 `fill_rule` 仍按所给值使用。[右边中点](gallery/shapes.zh-CN.md#fuse-outline-edge)和[右上角](gallery/shapes.zh-CN.md#fuse-outline-corner)接斜线的空心矩形示例分别展示这两种接点。

## 4. 脚本语句与内置函数

![内置函数的实际渲染结果](../site/media/gallery-scripting-units-builtins-1920.webp)

```text
function twice(value, factor=2) { return value * factor }
count = 0
for index in range(3) { count = count + 1 }
if count == 3 { gap = twice(2 mm) } else { gap = 1 mm }
```

支持变量赋值/重赋值、`if / else if / else`、`for ... in ...`、`while`、`break`、`continue`、`function`、`return`。函数采用词法作用域，可有默认参数；分支或循环内新建的变量留在局部，赋值已有外层变量会更新外层值。条件必须是布尔值。画布和实例的 `.width/.height` 是只读长度。脚本限制：单个循环最多 10,000 次、函数调用深度 64、总执行语句数 1,000,000。

| 内置函数 | 调用与结果 |
| --- | --- |
| `range(stop)`、`range(start,stop[,step])` | 产生整数列表；终点不包含，步长不可为 0，最多 10,000 项 |
| `len(list_or_string)` | 列表或字符串的长度 |
| `append(list,value)` | 返回**新列表**，不原地修改 |
| `str(value)` | 数值、布尔或字符串转文本；长度附带 `mm` |
| `abs(number)` | 保留单位类型的绝对值 |
| `min(a,...)`、`max(a,...)` | 至少一个参数，所有数值单位类型一致 |

## 5. 本地组件模块

![本地模块的实际渲染结果](../site/media/gallery-containers-module-import-1920.webp)

```text
import { card, gap as spacing } from "./components/card.lay"
page = canvas(size=(180 mm, 120 mm))
tile = page.add(card("Example"), target=page.top_left,
                offset=(spacing, spacing))
```

模块路径只允许本地相对 `.lay` 文件。模块用 `export` 导出常量、素材、组或函数；可继续导入其他模块，导入名只读。入口画布须在使用导入值前建立，组件模块不能创建或直接绘制入口画布。缺失导出、循环导入均报错。完整例子见[scripted.lay](../examples/scripted.lay)与[card.lay](../examples/components/card.lay)。

## 6. 导出与诊断

```text
laymesh validate <file.lay>
laymesh inspect <file.lay> --json
laymesh render <file.lay> -o <output.svg|pdf|png> [--dpi <positive-number>]
```

仓库中用 `cargo run --release --locked -p laymesh-cli --` 作命令前缀。`validate` 完成语法、名称、单位、导入、图片/字体、字形覆盖和放置约束检查；`render` 先做相同检查再写文件。`--dpi` **只用于 PNG**，默认 96。错误以 `文件:行:列: E_代码: 原因` 报告并返回非零状态；`W_FONT`、`W_MATH_FONT`、`W_PLOT_LAYOUT` 等警告写到 stderr，但验证仍可成功。CLI 参数错误退出码为 2，布局或导出错误为 1。

输出区别：SVG 保留可编辑路径与普通 `<text>`；PDF 保留矢量图形并嵌入普通文字，公式额外存放原 LaTeX 源码；PNG 从同一 Scene 栅格化。SVG 输入经过白名单检查，脚本、外链、DTD、滤镜及未支持的特效会以 `E_SVG` 拒绝。PNG/JPEG/TIFF 输入规范化为 sRGB PNG 后进入 Scene；TIFF 限单页 8 位灰度或 RGB。[实测输出和错误示例](examples-and-results.zh-CN.md)给出了完整复现命令。

逐项源码、命令和结果见[分类画廊](gallery/README.zh-CN.md)。

`inspect --json` 提供绘图区几何、映射、变换和全部诊断。每个命令都支持 `--warnings show|hide`，优先于环境变量 `LAYMESH_WARNINGS`（默认 `show`）。隐藏警告不影响错误和检查 JSON 中的诊断。

## 原生绘图与数据

[`plot`、`axis`、`plot_style`、`table`、`array` 及图层方法的完整语法见科研绘图指南](plotting.zh-CN.md)。这些新增名称可被局部变量或函数遮蔽；原有 `plot = image(...)` 继续有效。

二维科研绘图新增多轴、独立断轴、描述统计及共享颜色，详见[完整参考](cartesian-plots.zh-CN.md)和[Python 数据示例](../examples/plot/complete-data.py)。

极坐标、雷达图、数据锚点以及 CLI/Python/Notebook 的可预设警告开关见[极坐标绘图参考](polar-plots.zh-CN.md)。


## 统一参数、主题与编辑器

几何默认 mm，字号与线宽类默认 pt。二维尺寸使用 `size`；开放线条使用 `line_*`，封闭轮廓使用 `border_*`。[全部公开参数](topics/interface-reference.zh-CN.md)逐项列出类型、单位和默认值；另见[字符串与公式](topics/formulas.zh-CN.md)、[LCSS](topics/lcss.zh-CN.md)、[编辑器](topics/editors.zh-CN.md)和[迁移](topics/migration.zh-CN.md)。

## 预定义变量与颜色

`round` 是等于 `"round"` 的普通预定义字符串变量；`top_left`、`triangle` 等固定选项可以用于赋值、列表、比较和函数参数。用户作用域优先，含连字符的值使用 `ratex_katex="ratex-katex"` 等别名。字符串形式继续合法，没有弃用警告。[完整变量登记表](topics/interface-reference.zh-CN.md#预定义字符串变量)。

HEX 最后一字节为透明度。RGB 通道允许 0–255 小数；HSV S/V 与 OKLCH L 为 0–1，C 非负，alpha 为 0–1。色相支持 deg/rad 或裸度数。颜色 alpha 与绘制不透明度相乘。当前颜色空间为 HEX、RGB、HSV、OKLCH；详见[颜色](topics/fills.zh-CN.md)和[功能覆盖清单](topics/feature-map.zh-CN.md)。

## 有序字典与遍历

使用字符串键，例如 `{ "B": value, "A": other }`；键可由表达式生成，值可保留长度单位、颜色、素材或几何查询。字典按插入顺序排列，重复键覆盖值且保留原位置。`d["key"]` 缺失时报 E_INDEX，`d.get("key", default=null)` 提供默认值。`len(d)`、`.keys()`、`.values()`、`.items()` 同样适用于表格列。

字典采用值语义：执行 `b=a` 后修改 b，不改变 a。`d["panel"]["color"]="#0072b2"` 等嵌套赋值重建并重新绑定根字典，中间键须存在，导入绑定仍不可修改。`.update(other)` 返回新字典，使用 `d=d.update(other)`。字典内的素材对象保留原有句柄行为。点号用于调用方法，名为 items 等的键通过索引访问。

`dict()` 创建空字典；`dict(src="config.json")` 加载嵌套 JSON 对象并保持对象顺序，数字须有限且能精确表示。table 和 array 继续执行原有的严格数据校验。

## 键值解包与循环快照

`for key in d` 遍历键，`for key,value in d.items()` 遍历键值对。支持 `for i,(name,ys) in enumerate(d.items())` 等嵌套解包，变量属于循环作用域，`_` 丢弃对应值；解包数量须匹配，不允许重复变量名。

`enumerate(seq,start=0)` 返回编号和值；`zip(a,b,...)` 在最短输入结束时停止，零参数返回空列表。列表、字典和几何集合均可迭代。循环开始时固定遍历快照，修改原字典不会给本轮循环追加工作。break、continue、作用域和执行预算保留原有规则；字典键判断写作 `key in d` 或 `key not in d`。

[完整字典绘图](../examples/plot/dictionary-series.lay) · [预设配色](topics/cmaps.zh-CN.md)
