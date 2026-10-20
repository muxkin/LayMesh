# 外观属性与单位

[样式手册](lcss.zh-CN.md) · 下一章：[阴影与效果](style-shadows.zh-CN.md)

## 单位与物理尺寸

LCSS 长度支持 mm/cm/in/inch/pt/px，以及 CSS 的 pc/q。非零长度应带单位；零可省略。角度支持 deg/rad/grad/turn，时间支持 s/ms。不要把 .lay 参数的裸数默认单位照搬到 CSS 声明。

px 使用画布 layout_dpi：`1px=25.4/layout_dpi mm`，默认 96。样式和动画共用此上下文。导出 --dpi 改变栅格密度，不改变字体、阴影或画布的毫米尺寸。

| 单位 | 基准 |
| --- | --- |
| em | 元素计算字号；font-size 自身相对父级字号 |
| rem | 根字号；未声明时为 16 个布局 px |
| vw/vh | 固定画布宽/高的 1% |
| vmin/vmax | 固定画布较小/较大尺寸的 1% |

calc() 保持量纲；min()、max()、clamp() 用于同类可比较值。clamp(3mm,1.1rem,5mm) 把首选字号限制在 3–5mm；calc(1mm + 0.2em) 把固定留白与字号相加。百分比只在属性有明确基准时可用：圆角基于对象宽高，动画平移和原点基于未变换布局框；不能假设所有长度属性都支持百分比。

## 颜色与填充

颜色支持命名颜色、十六进制 RGB/RGBA、RGB 百分比与 alpha、HSL，以及 LayMesh 的 HSV/OKLCH。transparent 表示透明，currentColor 取得当前计算文字颜色。color 是文字颜色，fill 是封闭形状内部，background 是对象区域底色。

支持单层线性、径向渐变，以及 LayMesh 纹理 hatch(slash|cross|dots,...) 和 url("texture.png")。多层背景、圆锥渐变和未实现的插值颜色空间不能假设有效。DSL 对应 linear_gradient、radial_gradient、hatch、image_fill。

~~~lcss
.card {
  color:hsl(210 40% 25%);
  background:linear-gradient(120deg,#60a5fa,#075985);
  border:0.5pt solid currentColor;
  border-radius:2mm;
  opacity:0.9;
}
~~~

## 字体、边框与留白

字体列表按顺序查找已安装或提供的字体，文件路径相对于样式文件。正文缺字会警告并画方框；共享项目时保留使用的字体文件。

font 简写包含字体样式、字重、字号、可选行高和字体族。LCSS line-height:1.4 是字号倍数，line-height:5mm 是物理行高。

形状轮廓使用 border-*，line 素材使用 line-*；path 的开放与闭合路径都用 border-* 设置描边。border:0.5pt solid #94a3b8 展开为宽度、线型和颜色，顺序继续参与级联。圆角当前支持统一半径，单值百分比按布局框计算。

padding 支持一个值、两个值（上下/左右）、三个值（上/左右/下）或四个值（上/右/下/左），参与边界测量，不承担网页盒模型的自动排版。opacity、border-opacity、line-opacity 控制不同层次。

<!-- example:examples/manual/style-project.lay -->

## 逐项参考

下面列出每项静态属性的值类型、单位、默认行为、继承和声明示例。专用属性只作用于适用素材；未显式列出的默认值取素材默认值。动画属性见[关键帧与 CSS 动画](animation-keyframes.zh-CN.md)。

<!-- properties:start -->
### font-family

系统字体族名、字体文件路径或有序列表；不内置正文字体；缺字警告并显示方框。

合法值：string | string[].

单位：无.

默认：system sans-serif. 继承。

适用对象：文字与公式.

```lcss
.sample { font-family: "DejaVu Sans", sans-serif; }
```

### math-font

数学字体：默认 ratex-katex；也可指定带 OpenType MATH 表的已安装字体名称或字体文件路径。旧 mathjax-* 名称警告后映射到默认字体。

合法值：string.

单位：无.

默认：ratex-katex (inherit). 继承。

适用对象：文字与公式.

```lcss
.sample { math-font: ratex-katex; }
```

### math-text-fallback

公式文本使用 font_family 与文本字体回退，并按实际字形尺寸排版；false 仅检查数学字体覆盖。数学符号仍使用 math_font。

合法值：boolean.

单位：无.

默认：true (inherit). 继承。

适用对象：文字与公式.

```lcss
.sample { math-text-fallback: true; }
```

### font-size

字号；非零 CSS 长度需要单位，未指定时继承；em 和百分比相对父级字号。

合法值：length.

单位：项目绝对长度或 CSS 相对长度；px 按 layout_dpi 换算.

默认：继承 / inherit. 继承。

适用对象：文字与公式.

```lcss
.sample { font-size: 4mm; }
```

### font-weight

字重数值 1–1000；normal=400、bold/bolder=700、lighter=300。相对字重关键字目前使用这些固定映射。

合法值：number | keyword.

单位：无.

默认：400. 继承。

适用对象：文字与公式.

```lcss
.sample { font-weight: 700; }
```

### font-style

normal、italic 或 oblique；oblique 当前按 italic 处理。

合法值：enum: `normal`, `italic`, `oblique`.

单位：无.

默认：normal. 继承。

适用对象：文字与公式.

```lcss
.sample { font-style: italic; }
```

### color

文字颜色；currentColor 供边框和阴影引用当前计算颜色。

合法值：color.

单位：无.

默认：#555555. 继承。

适用对象：文字与公式.

```lcss
.sample { color: #243447; }
```

### line-height

显式长度，或无单位的字号倍数；默认 1.2 倍字号。

合法值：length | number.

单位：项目绝对长度或 CSS 相对长度；px 按 layout_dpi 换算.

默认：1.2 × font_size. 继承。

适用对象：开放线条.

```lcss
.sample { line-height: 1.4; }
```

### background

对象区域的底色或填充，不是文字或轴线颜色

合法值：paint.

单位：无.

默认：none. 不继承。

适用对象：对象区域；具体适用范围随素材而定.

```lcss
.sample { background: linear-gradient(120deg, #60a5fa, #075985); }
```

### padding

内部留白；接受 1–4 个 CSS 长度，依次采用全边、上下/左右、上/左右/下、上/右/下/左规则。

合法值：length | length[].

单位：项目绝对长度或 CSS 相对长度；px 按 layout_dpi 换算.

默认：0. 不继承。

适用对象：对象区域；具体适用范围随素材而定.

```lcss
.sample { padding: 1mm 2mm 1mm 2mm; }
```

### border-radius

统一底框圆角；非零长度需要单位，百分比基于布局框；暂不支持独立角或椭圆圆角。

合法值：length.

单位：项目绝对长度或 CSS 相对长度；px 按 layout_dpi 换算.

默认：0. 不继承。

适用对象：对象区域；具体适用范围随素材而定.

```lcss
.sample { border-radius: 2mm; }
```

### border-color

封闭轮廓颜色，none 表示无边框

合法值：color.

单位：无.

默认：none. 不继承。

适用对象：对象区域；具体适用范围随素材而定.

```lcss
.sample { border-color: #94a3b8; }
```

### border-width

封闭轮廓线宽；非零 CSS 长度需要单位；也接受 thin/medium/thick，分别为 1/3/5 个布局 px。

合法值：length.

单位：项目绝对长度或 CSS 相对长度；px 按 layout_dpi 换算.

默认：按素材默认值. 不继承。

适用对象：对象区域；具体适用范围随素材而定.

```lcss
.sample { border-width: 0.5pt; }
```

### border-style

边框线型；与填充独立

合法值：enum: `none`, `solid`, `dashed`, `dotted`, `dash_dot`, `double`, `triple`.

单位：无.

默认：solid. 不继承。

适用对象：对象区域；具体适用范围随素材而定.

```lcss
.sample { border-style: solid; }
```

### border-dash

交替的边框线段和间隔 CSS 长度列表；须成对，非零长度需要单位。

合法值：length[].

单位：项目绝对长度或 CSS 相对长度；px 按 layout_dpi 换算.

默认：按素材默认值. 不继承。

适用对象：对象区域；具体适用范围随素材而定.

```lcss
.sample { border-dash: 2mm 1mm; }
```

### border-dash-offset

边框虚线相位；接受可正可负的 CSS 长度，非零值需要单位。

合法值：length.

单位：项目绝对长度或 CSS 相对长度；px 按 layout_dpi 换算.

默认：0. 不继承。

适用对象：对象区域；具体适用范围随素材而定.

```lcss
.sample { border-dash-offset: 0.5mm; }
```

### border-cap

轮廓虚线端点形状

合法值：enum: `butt`, `round`, `square`.

单位：无.

默认：butt. 不继承。

适用对象：对象区域；具体适用范围随素材而定.

```lcss
.sample { border-cap: round; }
```

### border-join

轮廓交点形状

合法值：enum: `miter`, `round`, `bevel`.

单位：无.

默认：miter. 不继承。

适用对象：对象区域；具体适用范围随素材而定.

```lcss
.sample { border-join: round; }
```

### border-miter-limit

尖角的最大延伸与线宽之比

合法值：value.

单位：无.

默认：4. 不继承。

适用对象：对象区域；具体适用范围随素材而定.

```lcss
.sample { border-miter-limit: 4; }
```

### border-opacity

边框独立透明度 0–1

合法值：value.

单位：无.

默认：1. 不继承。

适用对象：对象区域；具体适用范围随素材而定.

```lcss
.sample { border-opacity: 0.7; }
```

### line-color

开放线条颜色；默认采用数据系列颜色或主题颜色

合法值：color.

单位：无.

默认：按素材默认值. 不继承。

适用对象：开放线条.

```lcss
.sample { line-color: #0d6efd; }
```

### line-width

开放线条宽度；非零 CSS 长度需要单位；也接受 thin/medium/thick，分别为 1/3/5 个布局 px。

合法值：length.

单位：项目绝对长度或 CSS 相对长度；px 按 layout_dpi 换算.

默认：按素材默认值. 不继承。

适用对象：开放线条.

```lcss
.sample { line-width: 0.7pt; }
```

### line-style

线型；double/triple 保留透明间隔

合法值：enum: `none`, `solid`, `dashed`, `dotted`, `dash_dot`, `double`, `triple`.

单位：无.

默认：solid. 不继承。

适用对象：开放线条.

```lcss
.sample { line-style: solid; }
```

### line-dash

交替的实线段和间隔 CSS 长度列表；非零长度需要单位。

合法值：length[].

单位：项目绝对长度或 CSS 相对长度；px 按 layout_dpi 换算.

默认：按素材默认值. 不继承。

适用对象：开放线条.

```lcss
.sample { line-dash: 2mm 1mm; }
```

### line-dash-offset

线条虚线相位；接受可正可负的 CSS 长度，非零值需要单位。

合法值：length.

单位：项目绝对长度或 CSS 相对长度；px 按 layout_dpi 换算.

默认：0. 不继承。

适用对象：开放线条.

```lcss
.sample { line-dash-offset: 0.5mm; }
```

### line-cap

开放端点形状

合法值：enum: `butt`, `round`, `square`.

单位：无.

默认：butt. 不继承。

适用对象：开放线条.

```lcss
.sample { line-cap: round; }
```

### line-join

线段交点形状

合法值：enum: `miter`, `round`, `bevel`.

单位：无.

默认：miter. 不继承。

适用对象：开放线条.

```lcss
.sample { line-join: round; }
```

### line-miter-limit

尖角的最大延伸与线宽之比

合法值：value.

单位：无.

默认：4. 不继承。

适用对象：开放线条.

```lcss
.sample { line-miter-limit: 4; }
```

### line-opacity

线条独立透明度 0–1

合法值：value.

单位：无.

默认：1. 不继承。

适用对象：开放线条.

```lcss
.sample { line-opacity: 0.8; }
```

### fill

形状内部的颜色、渐变、纹理或图片；none 表示无填充。

合法值：paint.

单位：无.

默认：none. 不继承。

适用对象：对象区域；具体适用范围随素材而定.

```lcss
.sample { fill: #dbeafe; }
```

### opacity

对象透明度，范围 0–1；与父级透明度相乘。

合法值：value.

单位：无.

默认：1. 不继承。

适用对象：对象区域；具体适用范围随素材而定.

```lcss
.sample { opacity: 0.8; }
```

### tick-font-size

刻度字号；使用带单位的 CSS 长度，默认继承字号。

合法值：length.

单位：项目绝对长度或 CSS 相对长度；px 按 layout_dpi 换算.

默认：继承字号 / inherit font_size. 不继承。

适用对象：图表坐标轴、标签及网格.

```lcss
.sample { tick-font-size: 8pt; }
```

### label-font-size

轴标题字号；使用带单位的 CSS 长度，默认继承字号。

合法值：length.

单位：项目绝对长度或 CSS 相对长度；px 按 layout_dpi 换算.

默认：继承字号 / inherit font_size. 不继承。

适用对象：图表坐标轴、标签及网格.

```lcss
.sample { label-font-size: 10pt; }
```

### tick-color

刻度线与文字的颜色

合法值：color.

单位：无.

默认：按素材默认值. 不继承。

适用对象：图表坐标轴、标签及网格.

```lcss
.sample { tick-color: #243447; }
```

### grid-color

网格线颜色

合法值：color.

单位：无.

默认：#dddddd. 不继承。

适用对象：图表坐标轴、标签及网格.

```lcss
.sample { grid-color: #cbd5e1; }
```

### grid-line-width

网格线宽；使用带单位的 CSS 长度，默认 0.3pt。

合法值：length.

单位：项目绝对长度或 CSS 相对长度；px 按 layout_dpi 换算.

默认：0.3pt. 不继承。

适用对象：图表坐标轴、标签及网格.

```lcss
.sample { grid-line-width: 0.3pt; }
```

### grid-line-dash

网格虚线长度列表；非零 CSS 长度需要单位。

合法值：value.

单位：无.

默认：[]. 不继承。

适用对象：图表坐标轴、标签及网格.

```lcss
.sample { grid-line-dash: 1mm 1mm; }
```

### start-head

起始端头部配置，默认无

合法值：head.

单位：无.

默认：按素材默认值. 不继承。

适用对象：开放线条.

```lcss
.sample { start-head: head(shape="open",size=(3mm,2mm)); }
```

### end-head

结束端头部配置，默认无

合法值：head.

单位：无.

默认：按素材默认值. 不继承。

适用对象：开放线条.

```lcss
.sample { end-head: head(shape="triangle",size=(3mm,2mm)); }
```

### start-cap

独立端帽，默认继承 line_cap

合法值：enum: `butt`, `round`, `square`.

单位：无.

默认：line_cap. 不继承。

适用对象：开放线条.

```lcss
.sample { start-cap: round; }
```

### end-cap

独立端帽，默认继承 line_cap

合法值：enum: `butt`, `round`, `square`.

单位：无.

默认：line_cap. 不继承。

适用对象：开放线条.

```lcss
.sample { end-cap: square; }
```

### text-fill

字形填充 / Glyph paint; defaults to color

合法值：paint.

单位：无.

默认：按素材默认值. 不继承。

适用对象：文字与公式.

```lcss
.sample { text-fill: #0d6efd; }
```

### text-stroke-color

字形描边颜色 / Glyph stroke color

合法值：color.

单位：无.

默认：none. 不继承。

适用对象：文字与公式.

```lcss
.sample { text-stroke-color: #ffffff; }
```

### text-stroke-width

字形描边宽度 / Glyph stroke width

合法值：length.

单位：项目绝对长度或 CSS 相对长度；px 按 layout_dpi 换算.

默认：0. 不继承。

适用对象：文字与公式.

```lcss
.sample { text-stroke-width: 0.3pt; }
```

### effects

阴影、发光、倒影与柔边效果

合法值：effect[].

单位：无.

默认：[]. 不继承。

适用对象：对象区域；具体适用范围随素材而定.

```lcss
.sample { effects: [shadow(blur=1mm,offset=(1mm,2mm))]; }
```

### path

单条开放路径素材，保留原始曲线段；原路径端点为颈部，箭尖向外延伸

合法值：material.

单位：无.

默认：按素材默认值. 不继承。

适用对象：箭头形状及端部.

```lcss
.sample { path: path(commands=[move_to(0,0),line_to(20mm,10mm)]); }
```

`path` 是 LayMesh 构造器扩展，不是网页 CSS 几何属性，也不能读取 `.lay` 变量。

### warp

字形变形 / Text warp configuration

合法值：text_warp.

单位：无.

默认：按素材默认值. 不继承。

适用对象：文字与公式.

```lcss
.sample { warp: text_warp(kind="wave",amplitude=1mm,wavelength=10mm); }
```

### extrude

二维矢量挤出 / Vector extrusion

合法值：text_extrude.

单位：无.

默认：按素材默认值. 不继承。

适用对象：文字与公式.

```lcss
.sample { extrude: text_extrude(depth=1mm,angle=45deg,color="#334155"); }
```

### shaft-width

箭身宽度：等宽、首尾宽度对或按箭身弧长 0–1 递增的控制点；分段线性插值

合法值：length | (length,length) | (number,length)[].

单位：项目绝对长度或 CSS 相对长度；px 按 layout_dpi 换算.

默认：3mm. 不继承。

适用对象：箭头形状及端部.

```lcss
.sample { shaft-width: 2mm; }
```

### heads

头部所在端；燕尾只支持单头

合法值：enum: `start`, `end`, `both`.

单位：无.

默认：end. 不继承。

适用对象：箭头形状及端部.

```lcss
.sample { heads: end; }
```

### head-size

头部的纵向长度与横向宽度；两端参数覆盖 head_size

合法值：(length,length).

单位：项目绝对长度或 CSS 相对长度；px 按 layout_dpi 换算.

默认：twice the corresponding neck width. 不继承。

适用对象：箭头形状及端部.

```lcss
.sample { head-size: (4mm,3mm); }
```

### start-head-size

头部的纵向长度与横向宽度；两端参数覆盖 head_size

合法值：(length,length).

单位：项目绝对长度或 CSS 相对长度；px 按 layout_dpi 换算.

默认：head_size. 不继承。

适用对象：箭头形状及端部.

```lcss
.sample { start-head-size: (4mm,3mm); }
```

### end-head-size

头部的纵向长度与横向宽度；两端参数覆盖 head_size

合法值：(length,length).

单位：项目绝对长度或 CSS 相对长度；px 按 layout_dpi 换算.

默认：head_size. 不继承。

适用对象：箭头形状及端部.

```lcss
.sample { end-head-size: (4mm,3mm); }
```

### line

线条/边框简写：宽度 样式 颜色

合法值：shorthand.

单位：无.

默认：按素材默认值. 不继承。

适用对象：开放线条.

```lcss
.sample { line: 0.6pt solid #243447; }
```

### border

线条/边框简写：宽度 样式 颜色

合法值：shorthand.

单位：无.

默认：按素材默认值. 不继承。

适用对象：对象区域；具体适用范围随素材而定.

```lcss
.sample { border: 0.5pt solid #94a3b8; }
```

### background-color

背景颜色

合法值：color.

单位：无.

默认：按素材默认值. 不继承。

适用对象：对象区域；具体适用范围随素材而定.

```lcss
.sample { background-color: #eff6ff; }
```

### background-image

按从上到下顺序排列的渐变、图片或纹理

合法值：paint.

单位：无.

默认：按素材默认值. 不继承。

适用对象：对象区域；具体适用范围随素材而定.

```lcss
.sample { background-image: linear-gradient(120deg,#60a5fa,#075985); }
```

### font

字体简写：可选样式与字重、必需字号和字体族、可选 /行高；声明同时重置这些分项。

合法值：string.

单位：无.

默认：按素材默认值. 不继承。

适用对象：文字与公式.

```lcss
.sample { font: italic 700 4mm "DejaVu Sans"; }
```

### box-shadow

逗号分层的盒阴影：x/y 偏移、可选 blur/spread/颜色及 inset；首层在上，声明重置全部阴影分项。

合法值：shadow-list: `none`.

单位：无.

默认：none. 不继承。

适用对象：对象区域；具体适用范围随素材而定.

```lcss
.sample { box-shadow: 1mm 2mm 3mm 0mm #0004; }
```

### text-shadow

文字阴影

合法值：shadow-list: `none`.

单位：无.

默认：none. 继承。

适用对象：文字与公式.

```lcss
.sample { text-shadow: 0.5mm 0.5mm 1mm #0006; }
```

### filter

单个 drop-shadow()

合法值：filter: `none`.

单位：无.

默认：none. 不继承。

适用对象：对象区域；具体适用范围随素材而定.

```lcss
.sample { filter: drop-shadow(1mm 1mm 1mm #0005); }
```

### box-shadow-blur

四级工作草案：模糊半径，保留已有层

合法值：length-list.

单位：CSS 长度；px 按 layout_dpi 换算.

默认：按素材默认值. 不继承。

适用对象：对象区域；具体适用范围随素材而定.

```lcss
.sample { box-shadow-blur: 2mm; }
```

[已有层、列表与简写重置规则](style-shadows.zh-CN.md#partial-overrides)

### box-shadow-spread

四级工作草案：可正可负的扩散

合法值：length-list.

单位：CSS 长度；px 按 layout_dpi 换算.

默认：按素材默认值. 不继承。

适用对象：对象区域；具体适用范围随素材而定.

```lcss
.sample { box-shadow-spread: -0.5mm; }
```

[已有层、列表与简写重置规则](style-shadows.zh-CN.md#partial-overrides)

### box-shadow-offset

四级工作草案：偏移控制阴影层数

合法值：offset-list.

单位：CSS 长度；px 按 layout_dpi 换算.

默认：按素材默认值. 不继承。

适用对象：对象区域；具体适用范围随素材而定.

```lcss
.sample { box-shadow-offset: 1mm 2mm; }
```

[已有层、列表与简写重置规则](style-shadows.zh-CN.md#partial-overrides)

### box-shadow-color

四级工作草案：阴影颜色

合法值：color-list.

单位：无.

默认：按素材默认值. 不继承。

适用对象：对象区域；具体适用范围随素材而定.

```lcss
.sample { box-shadow-color: #0d6efd66; }
```

[已有层、列表与简写重置规则](style-shadows.zh-CN.md#partial-overrides)

### box-shadow-position

四级工作草案：内外阴影

合法值：keyword-list: `inset`, `outset`.

单位：无.

默认：按素材默认值. 不继承。

适用对象：对象区域；具体适用范围随素材而定.

```lcss
.sample { box-shadow-position: inset; }
```

[已有层、列表与简写重置规则](style-shadows.zh-CN.md#partial-overrides)

### background-position

逐层背景位置

合法值：string.

单位：无.

默认：0% 0%. 不继承。

适用对象：对象区域；具体适用范围随素材而定.

```lcss
.sample { background-position: center, right 2mm bottom 3mm; }
```

### background-size

逐层背景尺寸，支持 auto/cover/contain

合法值：string.

单位：无.

默认：auto. 不继承。

适用对象：对象区域；具体适用范围随素材而定.

```lcss
.sample { background-size: cover, 20mm 15mm; }
```

### background-repeat

逐层平铺，支持 repeat/no-repeat/space/round

合法值：string.

单位：无.

默认：repeat. 不继承。

适用对象：对象区域；具体适用范围随素材而定.

```lcss
.sample { background-repeat: no-repeat, space round; }
```

<!-- properties:end -->
