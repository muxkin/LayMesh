# 图像导入与裁剪

<!-- walkthrough:start -->
## 用途与概念

image 定义外部图片素材，add 决定实例尺寸、裁剪和适配。crop 的 box 使用 0–1 归一化图片坐标，先裁剪，再按目标 size 适配。

## 最小完整示例

此文件包含画布及必要定义，可直接运行 `laymesh validate` 或 `laymesh render`，不依赖前文未声明的变量。

<!-- example:examples/manual/image.lay -->

## 参数与默认行为

裸几何长度使用画布单位，裸字号和描边宽度使用 pt。显式调用参数优先于继承和主题默认值；下面的接口参考逐项列出允许类型、可选值与默认来源。

## 组合用法

同一图片分别放置完整版本和局部细节。contain 保留全部内容，cover 覆盖目标框并裁掉溢出，stretch 直接拉伸；单边 size 配 auto 可保持裁剪后的比例。

<!-- example:examples/gallery/images/crop.lay -->

## 常见错误与限制

SVG 内容先经过安全白名单；不支持 SVG crop。TIFF 仅支持单页 8 位灰度或 RGB。图片只有布局框，不根据像素提取 path 或 ink。

## 逐项功能说明

### image

从本地文件定义可重复放置的图片素材。尺寸、crop 与 contain/cover/stretch 适配在 add 中确定，路径相对于素材定义文件。

返回：material

必需输入：`src`.

[最小完整源码](../../examples/manual/image.lay) · [组合源码](../../examples/gallery/images/crop.lay) · [全部参数](interface-reference.zh-CN.md#image)

<!-- walkthrough:end -->

## 详细行为与补充示例

### 参数

| 参数 | 用途 | 默认或要求 |
| --- | --- | --- |
| `src` | 图片路径 | 必填 |
| `fit` | 图片框适配 | contain / cover / stretch |
| `crop` | 源图裁剪 | 可选 |

### 常见用法

支持 PNG、JPEG、安全 SVG，以及单页 8 位灰度或 RGB TIFF。crop 在源图中裁剪；contain 保持完整、cover 填满并裁切、stretch 分别缩放宽高。同一素材的实例互不影响。

[完整参数与规则](../user-guide.zh-CN.md)

### 限制与相关主题

[文字与字体](fonts.zh-CN.md) · [资源路径](resources.zh-CN.md) · [数学公式](formulas.zh-CN.md) · [形状与路径](shapes.zh-CN.md) · [填充渐变与边框](fills.zh-CN.md)
