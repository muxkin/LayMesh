# 单位与尺寸

<!-- walkthrough:start -->
## 用途与概念

几何裸值采用 canvas.unit，初始为 mm；字号和线宽裸值采用 pt。显式单位最便于阅读和跨主题复现。布局 px 使用 layout_dpi，位图输出 DPI 则决定最终像素数。

## 最小完整示例

此文件包含画布及必要定义，可直接运行 `laymesh validate` 或 `laymesh render`，不依赖前文未声明的变量。

<!-- example:examples/manual/line.lay -->

## 组合用法

长度乘无单位数仍是长度；两个同类长度相除得到比例。size=(40mm,auto) 常用于图片保留比例与文本自然高度，是否支持 auto 由素材决定。

<!-- example:examples/gallery/positioning/dependent-size.lay -->

## 常见错误与限制

页面尺寸必须为正且不接受 auto。不能把角度与长度相加。line(angle=...) 支持 deg/rad，裸角度按度；极坐标数据角度由 angle_unit 指定。

<!-- walkthrough:end -->

## 详细行为与补充示例

### 物理长度与标量

```lay
page = canvas(size=(150, 100), unit="mm")
panel = rect(size=(40, 25), fill="#eef5f2", border_color="#245447", border_width=0.6)
page.add(panel, offset=(2cm, 35))
```

尺寸、位置、间距、几何半径和 padding 跟随画布单位。字号、行高、边框宽度、线宽与虚线长度的裸值按 pt 解释。显式单位优先：`2cm`、`2 cm`、`2inch`、`2in` 都可使用。`layout_dpi=96` 只控制 px 到物理长度的换算，PNG 导出 `--dpi` 独立控制像素数。

数据、透明度、缩放比例、计数、坐标范围仍是无单位标量。裸数字仅在物理参数入口换算；`2mm + 3` 不合法，`2mm + 3cm` 合法。

单位只修饰紧邻的数字，乘除按通常优先级和从左到右的顺序计算。按比例计算长度可以写 `160mm*100/90` 或 `(160*100/90)*1mm`；`160*100/90mm` 表示 `(160*100)/(90mm)`，得到倒数长度，不能用作尺寸。两个长度相除得到无单位比例，例如 `100mm/90mm`。

量纲错误保持 `E_UNIT` 和运算符的源码位置。对于 `160*100/90mm`，提示说明 `mm` 属于 `90`，展示 `(160*100)/(90mm)`，并给出已验证合法的 `160mm*100/90` 和 `(160*100/90)*1mm`。其他数字或单位会生成相应建议；变量、函数等复杂表达式只提示原因与修改方向，不自动改写源码。`1+2mm` 仍不合法，`10mm/2mm` 得到无量纲数 5；将这个 5 放进采用 cm 的 size 参数时才得到 5cm。

### 自动尺寸

图片 `size=(80, auto)` 固定宽度并保持原图比例；文字使用它限定换行宽度，高度随内容计算。`page.add(plot, size=(120, 90))` 会重新排版图表；固定绘图区不随外框缩放。

```lay
photo = image(src="sample.png")
page.add(photo, size=(2inch, auto), offset=(2cm, 35))
```

`.width` 和 `.height` 保留为实例的只读测量值。不能再向构造器或 `add` 传入 `width=`、`height=`。

### 具名区域

固定绘图区使用 `plot_area=box(offset=(20, 10), size=(80, 55))`。图片裁剪使用 `crop=box(offset=(0.1, 0.1), size=(0.8, 0.8))`；裁剪数值是原图归一化坐标，仍是 0–1 的标量。

[完整参数](interface-reference.zh-CN.md) · [公式](formulas.zh-CN.md) · [LCSS](lcss.zh-CN.md)

## workflow

本流程的完整源码与可执行验证文件列在[功能覆盖清单](feature-map.zh-CN.md)。组合使用时请遵循本页的输入条件与限制。

LCSS 与动画共用这些项目单位与 `layout_dpi`，包括 `inch`、`deg/rad`、`s/ms`。CSS 还接受相对长度和带量纲的 `calc/min/max/clamp` 表达式，见[CSS 单位](lcss.zh-CN.md#css-units)。
