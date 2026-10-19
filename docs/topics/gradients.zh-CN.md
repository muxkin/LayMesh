# CSS 渐变与多层背景

[对照画廊：原始配方、跨端输出与差异说明](../benchmarks/gradients-2026-10-19/gallery.html)

[填充构造器](fills.zh-CN.md) · [Open Props](style-open-props.zh-CN.md) · [属性参考](style-properties.zh-CN.md)

## 几何与色标

`gradient(css="...")` 和 CSS 填充支持 `linear-gradient`、`radial-gradient`、`conic-gradient` 及三种 `repeating-*` 形式。几何遵循 [CSS Images](https://www.w3.org/TR/css-images-4/#gradients)：线性方向支持角度和 `to top right`；径向支持圆、椭圆、显式尺寸及最近／最远边或角。中心可位于对象框外。圆锥渐变的零度指向正上方，顺时针增加。

线性／径向色标支持物理长度和百分比；圆锥色标支持百分比及 `deg`、`rad`、`grad`、`turn`，也可使用混合 `calc()`。一个颜色可有两个位置，相同位置保留硬边。缺省位置在色标修正后分配，中点提示使用标准幂函数。负值和超过 100% 的位置保留到最终几何计算。重复渐变保留首末色标的周期与相位；零长度周期使用标准平均颜色规则。

长度根据 `layout_dpi`、当前／根字体和画布尺寸计算；导出 DPI 不改变布局。`linear_gradient`、`radial_gradient` 保持已有归一化坐标接口；`conic_gradient(stops=...,center=(0.5,0.5),angle=0deg)` 提供同类可复用 paint。完整色标语法使用 `gradient(css=...)`。

## 颜色插值

`in <space>` 支持 `srgb`、`srgb-linear`、`display-p3`、`display-p3-linear`、`a98-rgb`、`prophoto-rgb`、`rec2020`、`xyz`／`xyz-d65`、`xyz-d50`、`lab`、`lch`、`oklab`、`oklch`、`hsl`、`hwb`。极坐标空间支持 `shorter`、`longer`、`increasing`、`decreasing hue` 四条色相路径。缺失分量和透明度预乘遵循 [CSS Color 4](https://www.w3.org/TR/css-color-4/#interpolation)。

源通道保持浮点精度，插值完成后再映射到输出 sRGB。传统 RGB／命名色／HSL／HWB 默认使用 sRGB 插值，现代颜色默认使用 OKLab。Rust 统一完成颜色计算，对复杂插值按误差细分色标，并保留硬边的重复位置。原生 Skia 和 CanvasKit 消费相同结果。本接口不包含自定义颜色配置文件或渐变之间的形变动画。

## 多层背景

```lcss
.card {
  background-color: #f8fafc;
  background-image: linear-gradient(90deg,#f008,transparent), url("texture.png");
  background-size: auto, cover;
  background-position: center, right bottom;
  background-repeat: no-repeat;
}
```

第一张图在最上层，底色位于全部图层下方。图像、位置、尺寸、平铺列表按序匹配，较短列表重复，超出部分忽略；`none` 仍占一层。`background` 简写重置各分项，底色仅允许写在最后一层。非法声明不会抹掉前面的有效声明。

`background-size` 支持一或两个长度／百分比、`auto`、`cover`、`contain`。`background-position` 支持百分比、长度、方向关键字和边缘偏移。`background-repeat` 支持 `repeat`、`no-repeat`、`repeat-x`、`repeat-y`、`space`、`round` 和双轴配置。定位区域仍为 LayMesh 对象布局框，图层按对象路径和圆角裁切；对象透明度、边框和阴影只施加一次。不引入浏览器滚动、attachment 或 padding/content-box 定位和裁切模型。

<!-- example:examples/manual/gradient-composition.lay -->

## 预览与导出

原生预览、VS Code、网站与离线 HTML 使用 Skia 渐变着色器。显示列表版本 2 增加圆锥填充；新读取端仍接受版本 1，跨版本增量更新会替换完整基线。

SVG／PDF／PPTX 保留可表达的矢量，必要时按所选导出 DPI 仅将填充区域转为局部位图，其他文字、描边继续使用已有输出路径。正常兜底静默完成；栅格区域沿用分块内存预算，尺寸无法满足或输出失败仍报错。技术记录放在导出任务和 PPTX 对象名称中，不逐对象弹出警告。

目标格式无法表达重叠填充与描边的隔离透明度时，将该对象局部合成后再施加一次透明度，其他对象不受影响。离线放映版 PDF 会检查导出库采样函数的颜色误差，仅在无法保留已解析颜色时对填充使用局部兜底。

验证脚本为 `scripts/test-gradient-parity.cjs`，保留原始库 CSS，记录浏览器版本、sRGB 条件、引擎／库哈希、可编辑输入与实际输出。浏览器近似可能与标准公式不同，例如 Chromium 用九个插入色标近似中点提示；这类差异必须单独报告，不能计作像素对照通过。
