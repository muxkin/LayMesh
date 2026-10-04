# 填充渐变与边框

<!-- walkthrough:start -->
## 用途与概念

颜色可以是十六进制字符串或 rgb/hsv/oklch 构造值。HEX 的最后两位是 alpha，#ffffff90 约为 56.47% 不透明度。颜色 alpha 与对象、描边及祖先组的不透明度相乘。

## 最小完整示例

此文件包含画布及必要定义，可直接运行 `laymesh validate` 或 `laymesh render`，不依赖前文未声明的变量。

<!-- example:examples/manual/rgb.lay -->

## 参数与默认行为

裸几何长度使用画布单位，裸字号和描边宽度使用 pt。显式调用参数优先于继承和主题默认值；下面的接口参考逐项列出允许类型、可选值与默认来源。

## 组合用法

将颜色用于填充、文字、描边、渐变 stop 和图表配色。linear_gradient 与 radial_gradient 使用布局框的归一化坐标；hatch 是物理间距图案，image_fill 将图片适配到形状内部。

<!-- example:examples/gallery/shapes/linear-gradient.lay -->

## 常见错误与限制

RGB 为 0–255，HSV 的 S/V 和 OKLCH 的 L 为 0–1，C 非负，alpha 为 0–1；非法或非有限值报错。OKLCH 超色域压缩色度并提示映射。当前不提供 HSL/CMYK。

## 逐项功能说明

### linear_gradient

创建线性渐变填充，stops 的位置在 0–1；start/end 为素材布局框中的归一化坐标。颜色 alpha 与 stop 透明度相乘。

返回：paint

必需输入：`stops`.

[最小完整源码](../../examples/manual/linear_gradient.lay) · [组合源码](../../examples/gallery/shapes/linear-gradient.lay) · [全部参数](interface-reference.zh-CN.md#linear_gradient)

### radial_gradient

创建径向渐变填充，center 和 radius 相对于布局框。可复用同一填充在不同大小的形状中，最终绘制由实例尺寸决定。

返回：paint

必需输入：`stops`.

[最小完整源码](../../examples/manual/radial_gradient.lay) · [组合源码](../../examples/gallery/shapes/radial-gradient.lay) · [全部参数](interface-reference.zh-CN.md#radial_gradient)

### hatch

创建 slash/cross/dots 重复图案填充。spacing 和 line_width 为物理尺寸，图案限制在素材填充区域内。

返回：paint

[最小完整源码](../../examples/manual/hatch.lay) · [组合源码](../../examples/unified.lay) · [全部参数](interface-reference.zh-CN.md#hatch)

### image_fill

将本地图片作为形状内部填充，fit 决定适配方式。与 image 素材不同，填充受形状轮廓限制，可用于圆角矩形。

返回：paint

必需输入：`src`.

[最小完整源码](../../examples/manual/image_fill.lay) · [组合源码](../../examples/manual/paint-composition.lay) · [全部参数](interface-reference.zh-CN.md#image_fill)

### rgb

创建 RGB 颜色；支持 alpha=0–1。

返回：color

必需输入：`r`, `g`, `b`.

[最小完整源码](../../examples/manual/rgb.lay) · [组合源码](../../examples/gallery/shapes/colors-alpha.lay) · [全部参数](interface-reference.zh-CN.md#rgb)

### hsv

创建 HSV 颜色；支持 alpha=0–1。

返回：color

必需输入：`h`, `s`, `v`.

[最小完整源码](../../examples/manual/hsv.lay) · [组合源码](../../examples/gallery/shapes/colors-alpha.lay) · [全部参数](interface-reference.zh-CN.md#hsv)

### oklch

创建 OKLCH 颜色；支持 alpha=0–1。

返回：color

必需输入：`l`, `c`, `h`.

[最小完整源码](../../examples/manual/oklch.lay) · [组合源码](../../examples/gallery/shapes/colors-alpha.lay) · [全部参数](interface-reference.zh-CN.md#oklch)

<!-- walkthrough:end -->

## 详细行为与补充示例

### 参数

| 参数 | 用途 | 默认或要求 |
| --- | --- | --- |
| `fill` | 颜色或渐变 | 也接受 none |
| `border_* / line_*` | 闭合与开放描边设置 | 默认值与选项见共享参考 |

### 常见用法

fill 支持颜色、none、渐变、hatch 与 image_fill。闭合轮廓使用 border_*，开放描边使用 line_*，LCSS 类可复用设置。outline(...) 已移除；渐变描边尚未实现。

[完整参数与规则](../language-reference.zh-CN.md)

### 限制与相关主题

[图像导入与裁剪](images.zh-CN.md) · [文字与字体](fonts.zh-CN.md) · [资源路径](resources.zh-CN.md) · [数学公式](formulas.zh-CN.md) · [形状与路径](shapes.zh-CN.md)


### RGB、HSV、OKLCH 与透明度

支持 `#RGB/#RGBA/#RRGGBB/#RRGGBBAA`，最后一组是透明度，例如 `#ffffff90` 的透明度为 144/255。也可用 `rgb(255,80,40,alpha=0.6)`、`hsv(200deg,0.7,0.8,alpha=0.6)`、`oklch(0.7,0.15,200deg,alpha=0.6)`。颜色字符串与 LCSS 使用 `rgb(255 80 40 / 0.6)`、`hsv(200 0.7 0.8 / 0.6)`、`oklch(0.7 0.15 200 / 0.6)`。HSV 是 LayMesh 扩展；CMYK 暂不支持。

RGB 通道为 0–255，HSV 饱和度和明度为 0–1，OKLCH 明度为 0–1、色度非负，透明度为 0–1。色相支持 deg/rad，默认度数。OKLCH 保留源通道，使用恒定明度和色相、二分降低色度的固定算法映射到 sRGB（48 次迭代）。面板显示映射提示，CLI、WASM 与三种导出使用相同结果。

颜色透明度与绘制 opacity 相乘。填充、描边、文字、渐变和色标共用规则；颜色列表继续按 RGB 通道线性插值，同时插值透明度。
