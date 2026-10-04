# 插图与图像

输入格式、复用、裁剪与图片框适配。所有素材由仓库内的合成图生成，不依赖外部下载。

[返回画廊索引](README.zh-CN.md) · [执行结果](../examples-and-results.zh-CN.md)

## 输入格式

<a id="png"></a>

### PNG 位图

导入 PNG 素材并指定物理宽度；输出页内保留完整图片。

```lay
picture = image(src="../assets/sample.png")
placed = page.add(picture,size=(76 mm, auto), target=page.top_left, offset=(7 mm, 24 mm))
frame = rect(size=(76 mm, 47.5 mm), fill="none", border_color="#435b72", border_width=0.35 mm)
page.add(frame, target=placed.top_left)
```

![PNG 位图的实际渲染结果](../../site/media/gallery-images-png-1920.webp)

源码：[png.lay](../../examples/gallery/images/png.lay)。

复现命令：`laymesh validate examples/gallery/images/png.lay`；`laymesh render examples/gallery/images/png.lay -o png.png --dpi 150`。

实测：`有效：examples/gallery/images/png.lay（120 × 80 mm，3 个顶层实例）`；PNG **709 × 472 px**，150 DPI；警告：无。

<a id="jpeg"></a>

### JPEG 位图

用相同布局导入 JPEG，观察有损编码素材进入页面后的实际效果。

```lay
picture = image(src="../assets/sample.jpg")
placed = page.add(picture,size=(76 mm, auto), target=page.top_left, offset=(7 mm, 24 mm))
frame = rect(size=(76 mm, 47.5 mm), fill="none", border_color="#435b72", border_width=0.35 mm)
page.add(frame, target=placed.top_left)
```

![JPEG 位图的实际渲染结果](../../site/media/gallery-images-jpeg-1920.webp)

源码：[jpeg.lay](../../examples/gallery/images/jpeg.lay)。

复现命令：`laymesh validate examples/gallery/images/jpeg.lay`；`laymesh render examples/gallery/images/jpeg.lay -o jpeg.png --dpi 150`。

实测：`有效：examples/gallery/images/jpeg.lay（120 × 80 mm，3 个顶层实例）`；PNG **709 × 472 px**，150 DPI；警告：无。

<a id="svg"></a>

### SVG 矢量素材

导入安全 SVG 子集；图形在 SVG 和 PDF 导出时可继续保持矢量。

```lay
picture = image(src="../assets/sample.svg")
placed = page.add(picture,size=(76 mm, auto), target=page.top_left, offset=(7 mm, 24 mm))
frame = rect(size=(76 mm, 47.5 mm), fill="none", border_color="#435b72", border_width=0.35 mm)
page.add(frame, target=placed.top_left)
```

![SVG 矢量素材的实际渲染结果](../../site/media/gallery-images-svg-1920.webp)

源码：[svg.lay](../../examples/gallery/images/svg.lay)。

复现命令：`laymesh validate examples/gallery/images/svg.lay`；`laymesh render examples/gallery/images/svg.lay -o svg.png --dpi 150`。

实测：`有效：examples/gallery/images/svg.lay（120 × 80 mm，3 个顶层实例）`；PNG **709 × 472 px**，150 DPI；警告：无。

<a id="tiff-gray"></a>

### 8 位灰度 TIFF

演示单页、单通道 8 位 TIFF 输入；源图片由生成脚本确定。

```lay
picture = image(src="../assets/gray8.tiff")
placed = page.add(picture,size=(76 mm, auto), target=page.top_left, offset=(7 mm, 24 mm))
frame = rect(size=(76 mm, 47.5 mm), fill="none", border_color="#435b72", border_width=0.35 mm)
page.add(frame, target=placed.top_left)
```

![8 位灰度 TIFF的实际渲染结果](../../site/media/gallery-images-tiff-gray-1920.webp)

源码：[tiff-gray.lay](../../examples/gallery/images/tiff-gray.lay)。

复现命令：`laymesh validate examples/gallery/images/tiff-gray.lay`；`laymesh render examples/gallery/images/tiff-gray.lay -o tiff-gray.png --dpi 150`。

实测：`有效：examples/gallery/images/tiff-gray.lay（120 × 80 mm，3 个顶层实例）`；PNG **709 × 472 px**，150 DPI；警告：无。

<a id="tiff-rgb"></a>

### 8 位 RGB TIFF

演示单页、三通道 8 位 TIFF 输入。

```lay
picture = image(src="../assets/rgb8.tiff")
placed = page.add(picture,size=(76 mm, auto), target=page.top_left, offset=(7 mm, 24 mm))
frame = rect(size=(76 mm, 47.5 mm), fill="none", border_color="#435b72", border_width=0.35 mm)
page.add(frame, target=placed.top_left)
```

![8 位 RGB TIFF的实际渲染结果](../../site/media/gallery-images-tiff-rgb-1920.webp)

源码：[tiff-rgb.lay](../../examples/gallery/images/tiff-rgb.lay)。

复现命令：`laymesh validate examples/gallery/images/tiff-rgb.lay`；`laymesh render examples/gallery/images/tiff-rgb.lay -o tiff-rgb.png --dpi 150`。

实测：`有效：examples/gallery/images/tiff-rgb.lay（120 × 80 mm，3 个顶层实例）`；PNG **709 × 472 px**，150 DPI；警告：无。

## 实例与裁剪

<a id="reuse"></a>

### 同一素材重复放置

同一 image 定义放在两个位置并使用不同显示宽度，实例互不影响。

```lay
picture = image(src="../assets/sample.png")
first = page.add(picture,size=(62 mm, auto), target=page.top_left, offset=(7 mm, 26 mm))
page.add(picture,size=(34 mm, auto), target=first.top_right, offset=(9 mm, 0 mm))
```

![同一素材重复放置的实际渲染结果](../../site/media/gallery-images-reuse-1920.webp)

源码：[reuse.lay](../../examples/gallery/images/reuse.lay)。

复现命令：`laymesh validate examples/gallery/images/reuse.lay`；`laymesh render examples/gallery/images/reuse.lay -o reuse.png --dpi 150`。

实测：`有效：examples/gallery/images/reuse.lay（120 × 80 mm，3 个顶层实例）`；PNG **709 × 472 px**，150 DPI；警告：无。

<a id="crop"></a>

### 独立裁剪

右侧实例从源图裁取部分区域，左侧仍显示完整源图。

```lay
picture = image(src="../assets/sample.png")
first = page.add(picture,size=(53 mm, auto), target=page.top_left, offset=(7 mm, 26 mm))
page.add(picture,size=(44 mm, auto), crop=box(offset=(0.4, 0), size=(0.6, 1)),
         target=first.top_right, offset=(9 mm, 0 mm))
```

![独立裁剪的实际渲染结果](../../site/media/gallery-images-crop-1920.webp)

源码：[crop.lay](../../examples/gallery/images/crop.lay)。

复现命令：`laymesh validate examples/gallery/images/crop.lay`；`laymesh render examples/gallery/images/crop.lay -o crop.png --dpi 150`。

实测：`有效：examples/gallery/images/crop.lay（120 × 80 mm，3 个顶层实例）`；PNG **709 × 472 px**，150 DPI；警告：无。

## 适配模式

<a id="fit-contain"></a>

### contain 完整显示

图像等比缩放并完整落在框内，框中可能留空。

```lay
picture = image(src="../assets/sample.png")
frame = rect(size=(49 mm, 44 mm), fill="#e1e8ef", border_color="#39546b", border_width=0.4 mm)
placed = page.add(frame, target=page.top_left, offset=(13 mm, 26 mm))
page.add(picture,size=(49 mm, 44 mm), fit=contain, target=placed.top_left)
```

![contain 完整显示的实际渲染结果](../../site/media/gallery-images-fit-contain-1920.webp)

源码：[fit-contain.lay](../../examples/gallery/images/fit-contain.lay)。

复现命令：`laymesh validate examples/gallery/images/fit-contain.lay`；`laymesh render examples/gallery/images/fit-contain.lay -o fit-contain.png --dpi 150`。

实测：`有效：examples/gallery/images/fit-contain.lay（120 × 80 mm，3 个顶层实例）`；PNG **709 × 472 px**，150 DPI；警告：无。

<a id="fit-cover"></a>

### cover 填满并裁切

图像等比填满目标框，超出的部分由框裁掉。

```lay
picture = image(src="../assets/sample.png")
frame = rect(size=(49 mm, 44 mm), fill="#e1e8ef", border_color="#39546b", border_width=0.4 mm)
placed = page.add(frame, target=page.top_left, offset=(13 mm, 26 mm))
page.add(picture,size=(49 mm, 44 mm), fit=cover, target=placed.top_left)
```

![cover 填满并裁切的实际渲染结果](../../site/media/gallery-images-fit-cover-1920.webp)

源码：[fit-cover.lay](../../examples/gallery/images/fit-cover.lay)。

复现命令：`laymesh validate examples/gallery/images/fit-cover.lay`；`laymesh render examples/gallery/images/fit-cover.lay -o fit-cover.png --dpi 150`。

实测：`有效：examples/gallery/images/fit-cover.lay（120 × 80 mm，3 个顶层实例）`；PNG **709 × 472 px**，150 DPI；警告：无。

<a id="fit-stretch"></a>

### stretch 拉伸

图像按框的宽高分别缩放，便于观察非等比变形。

```lay
picture = image(src="../assets/sample.png")
frame = rect(size=(49 mm, 44 mm), fill="#e1e8ef", border_color="#39546b", border_width=0.4 mm)
placed = page.add(frame, target=page.top_left, offset=(13 mm, 26 mm))
page.add(picture,size=(49 mm, 44 mm), fit=stretch, target=placed.top_left)
```

![stretch 拉伸的实际渲染结果](../../site/media/gallery-images-fit-stretch-1920.webp)

源码：[fit-stretch.lay](../../examples/gallery/images/fit-stretch.lay)。

复现命令：`laymesh validate examples/gallery/images/fit-stretch.lay`；`laymesh render examples/gallery/images/fit-stretch.lay -o fit-stretch.png --dpi 150`。

实测：`有效：examples/gallery/images/fit-stretch.lay（120 × 80 mm，3 个顶层实例）`；PNG **709 × 472 px**，150 DPI；警告：无。
