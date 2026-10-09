# 导出格式与分辨率

## workflow

CLI、Python、Jupyter 和 VS Code 使用同一原生导出器。扩展名选择格式，`.jpg/.jpeg`、`.tif/.tiff` 等价；`.pnm` 导出 RGBA PAM。

```sh
laymesh render figure.lay -o figure.png --dpi 300 --compression best
laymesh render figure.lay -o figure.jpg --dpi 300 --quality 95 --background "#ffffff"
laymesh render figure.lay -o figure.tif --dpi 600 --compression deflate
laymesh render figure.lay -o figure.webp --dpi 300 --webp-lossless false --quality 90 --webp-method 6 --webp-alpha-quality 100
```

| 格式 | 编码参数及默认值 | 透明度与分辨率 |
| --- | --- | --- |
| SVG | 不接受位图编码参数 | 保留矢量图形、物理页面尺寸及透明度 |
| PPTX | `dpi` 仅控制局部 PNG 回退，默认 1200 | 一张幻灯片；文字为文本框，公式及几何为可编辑形状；不嵌入字体 |
| PDF | `pdf_image_compression=auto`、`pdf_jpeg_quality=90`、`pdf_downsample=true` | 文字和公式保持矢量；位图采用 JPEG 或无损 Flate，不采用 WebP |
| PNG | `compression=fast/default/best`，默认 `default` | RGBA；写入 DPI 元数据 |
| JPEG | `quality=1–100`，默认 90；`background="#ffffff"` | 不支持 Alpha；先与底色合成；写入 DPI 元数据 |
| TIFF | `compression=none/lzw/deflate/packbits`，默认 `lzw`，均无损 | 单页 RGBA，非预乘 Alpha；写入 DPI 元数据 |
| WebP | `webp_lossless=true`；`quality=0–100`；`webp_method=0–6`，默认 4 | RGBA；无损默认质量/压缩力度 100，有损默认质量 90；保留 Alpha |
| BMP | 无额外编码参数 | RGBA；写入 DPI 元数据 |
| GIF | `background="#ffffff"` | 单帧、最多 256 色；保留完全透明像素，半透明与底色合成 |
| ICO、TGA | 无额外编码参数 | RGBA；ICO 宽高最多 256 px；TGA 使用无压缩编码 |
| PAM、PNM | 无额外编码参数 | RGBA PAM，保留 Alpha |
| PPM、PGM、PBM | `background="#ffffff"` | 先与底色合成；RGB、灰度、黑白二值；PBM 灰度阈值 128 |

所有位图支持 `--dpi`：正数，最多 25400；CLI、Python/Jupyter 与 VS Code 省略时默认 1200 DPI。PDF 也接受 DPI，作为内嵌图片降采样上限。每轴像素数为 `round(毫米尺寸 × DPI / 25.4)`，最少 1；总像素上限为 100000000。DPI 不改变布局尺寸、字号或 `layout_dpi`。仅 PNG/JPEG/TIFF/BMP 写入密度元数据；其他格式的 DPI 控制像素尺寸。JPEG/GIF/TGA 每轴最多 65535 px，WebP 为 16383 px。

WebP 还支持 `--webp-alpha-quality 0–100`（默认 100；降低需使用有损模式）、`--webp-near-lossless 0–100`（仅无损模式，默认 100，100 完全无损）。`method` 越高编码越慢，通常文件越小；有损 `quality` 越高视觉质量越高。无损模式的 `quality` 控制压缩力度。近无损值小于 100 允许 RGB 样本近似，Alpha 仍保留。参数说明依据 [WebP 编码器文档](https://developers.google.com/speed/webp/docs/api)。

原生栅格导出使用 **8 位 sRGB RGB/RGBA**。16 位输入在解码、ICC 转换、裁剪及中间 PNG 中保留精度和原始强度范围；最终页面栅格化到 8 位。TIFF 输出不会声称提供原生 16 位精度。SVG/PDF 中的图像素材仍走现有颜色与透明度链路，详见[图片格式](images.zh-CN.md)。

## PPTX 可编辑导出

需要 LayMesh 0.5.0 或更新版本。

```sh
laymesh render examples/export/pptx-editable.lay -o figure.pptx
laymesh render figure.lay -o figure.pptx --dpi 300
```

每份源码生成一张幻灯片，保留画布物理尺寸、背景和绘制顺序。普通文字按已布局的文字段生成可修改文本框，关闭自动缩放；公式转为可解组的矢量轮廓，不能直接修改为另一条公式。矩形、圆角矩形、椭圆和直线使用 DrawingML 原生预设；可表达的自定义路径合并填充与描边，保留可编辑的线宽、颜色、虚线、连接和对称端帽。非均匀缩放的描边、偏移虚线及装饰端点保留精确矢量轮廓。源码显式分组与公式分组保留，布局及 SVG 包装分组去除。图表保留组成它的形状与文字，不生成原生数据图表。

PNG/JPEG 图片作为独立资源嵌入并复用。矩形裁剪、圆角形状及箭头的图片填充支持 `contain/cover/stretch`、旋转、缩放、镜像和透明度，无需新增栅格回退。带留白的 `contain` 填充导出为带图片填充的可编辑形状，在 LibreOffice 中也保留留白。原生 SVG/PDF/位图的图片填充同样按形状实际宽高比适配。色标透明度一致的线性渐变保留为可编辑渐变填充；径向渐变、色标透明度变化、纹理、复杂裁剪或蒙版、滤镜、重叠对象的组透明度、多轮廓奇偶填充以及无法精确表示的文字或图片变换按最小完整子树转为透明 PNG；其他对象继续保持可编辑。`W_PPTX_RASTER` 列出回退对象和原因。效果流程允许只回退阴影层，保留主体为矢量。

`--dpi` 只控制局部回退图片的像素尺寸，沿用默认 1200，支持项目配置 `export.pptx.dpi`；不改变幻灯片大小。每个回退图片仍受 100000000 像素上限约束。`quality`、`compression`、`background`、`pdf_*`、`webp_*` 不适用于 PPTX。`--warnings hide` 可隐藏导出警告，失败仍保留原目标文件。

不嵌入字体。导出机缺少字形时仍保留原始可编辑文字；目标软件的字体替换决定其显示。`W_PPTX_FONT` 列出目标机器所需字体，字体替换可能改变文字排版。幻灯片宽高须在 25.4–1422.4 mm（1–56 英寸）之间；透明画布在演示软件中使用默认幻灯片底色。CLI、Python/Jupyter 和 VS Code 均可导出 PPTX；兼容性验证须区分 LibreOffice 打开、往返保存与 PowerPoint 实机检查。

[完整示例源码](../../examples/export/pptx-editable.lay) · [原生几何和图片适配验收例](../../examples/export/pptx-native.lay) · [复合描边验收例](../../examples/export/pptx-strokes.lay)

## Python 和 Jupyter

```python
from laymesh import render_file
render_file("figure.lay", output="figure.pptx")
render_file("figure.lay", output="figure.jpg", dpi=300, quality=95, background="#ffffff")
render_file("figure.lay", output="figure.tif", dpi=600, compression="lzw")
render_file("figure.lay", output="figure.webp", dpi=300, webp_lossless=True,
            quality=100, webp_method=6, webp_near_lossless=100)
```

`render_source` 接受相同关键字参数；`output=None` 时不能指定编码参数。`%laymesh` 和 `%%laymesh` 接受上面示例中的同名 CLI 参数。SVG 预览保持可用，导出文件使用所选编码。

PPTX 保留 SVG Notebook 预览，`%laymesh figure.lay -o figure.pptx` 或 `%%laymesh -o figure.pptx` 可直接导出；`dpi` 仅调节局部回退，默认 1200。

## VS Code

在 `.lay` 编辑器执行 **LayMesh：导出图形**，或点击实时预览的 **导出**。选择格式和目标文件后直接导出，不再询问编码参数。图片默认 1200 DPI，TIFF 默认 LZW 无损压缩。参数在 `laymesh.export.*` 设置中调整，也支持项目/全局配置；不再使用旧对话框记忆值。导出使用当前未保存的入口及已打开导入文件；无需安装 Python、Rust 或 npm。远程窗口的目标文件位于扩展宿主所在机器。

## 诊断

不合法、格式不适用或未知参数会被拒绝。CLI 参数错误退出 2；布局、资源或编码错误退出 1。所有命令支持 `--warnings show|hide`，优先于 `LAYMESH_WARNINGS`。先完成渲染再通过临时文件替换目标，正常失败保留已有文件；资源诊断保留源码位置。`inspect --json` 保留全部警告。

[CLI 参数](cli-reference.zh-CN.md) · [Python API](python-reference.zh-CN.md) · [编辑器](editors.zh-CN.md)

## PDF 位图压缩

自动模式最多采样 128 × 128 像素。颜色数不超过 32，或相邻像素近似相同的比例达到 90%（RGB 各通道差不超过 2），采用无损 Flate；其余不透明 8 位图片采用 JPEG 90。不再分别编码 JPEG/PNG 比较大小。透明和 16 位图片默认保持无损。满足颜色、方向、裁剪及分辨率条件的原始 JPEG 直接嵌入；`pdf_recompress_jpeg=true` 强制重编码。

| PDF 参数 | 默认值 | 含义 |
| --- | --- | --- |
| `pdf_image_compression` | `auto` | `auto`、`lossless` 或 `jpeg` |
| `pdf_jpeg_quality` | `90` | 新编码 JPEG 的质量，1–100 |
| `pdf_downsample` | `true` | 只缩小；考虑物理尺寸、适配方式、裁剪和组缩放 |
| `pdf_recompress_jpeg` | `false` | 允许 JPEG 原始字节直接嵌入 |
| `pdf_preserve_16bit` | `true` | 关闭后允许转成 8 位 |
| `pdf_preserve_alpha` | `true` | 关闭后先将透明区域合成到背景 |
| `pdf_alpha_background` | `#ffffff` | 关闭透明度保留时的背景色 |
| `pdf_auto_palette_limit` | `32` | 采样颜色数阈值，0–16384 |
| `pdf_auto_flatness_threshold` | `0.9` | 相邻像素平坦比例，0–1 |

强制 JPEG 仍遵守开启的精度和透明度保留选项，并返回 `W_PDF_LOSSLESS` 原因。要允许透明 16 位图使用 JPEG，需同时关闭两项保留设置。源文件不修改。全部参数支持 Python 关键字、CLI 选项（如 `--pdf-preserve-16bit false`）与 VS Code 设置。文字、公式和标尺保持矢量，PDF 不嵌入 WebP；导出独立使用规范化源像素。

## JSON 配置

项目采用源文件向上查找的最近 `.laymesh.json`；CLI 的 `--config PATH`（Python 的 `config=PATH`）可指定项目配置。全局配置路径：Linux 为 `$XDG_CONFIG_HOME/laymesh/config.json`，省略或为空时为 `~/.config/laymesh/config.json`；Windows 为 `%APPDATA%/laymesh/config.json`；macOS 为 `~/Library/Application Support/laymesh/config.json`。

优先级从高到低为：单次 API/CLI 参数、VS Code 文件夹或工作区显式设置、项目配置、VS Code 用户显式设置、全局配置、内置默认。未显式设置的 VS Code 默认值不会覆盖项目配置。配置变化应用于预览和下一次导出。

```json
{
  "export": {
    "dpi": 1200,
    "pdf": {"pdf_image_compression": "auto", "pdf_jpeg_quality": 90,
            "pdf_preserve_16bit": true, "pdf_preserve_alpha": true},
    "jpeg": {"quality": 90, "background": "#ffffff"},
    "png": {"compression": "fast"}
  },
  "preview": {"jpeg_quality": 90, "webp_quality": 90, "webp_method": 0,
              "image_threads": 0, "cache_mb": 256, "processing_memory_mb": 128}
}
```

SVG 不接受 DPI。各格式配置使用标准名称（`jpeg`、`tiff`、`webp` 等），参数名与 Python 导出关键字一致。位图导出保留 100000000 像素上限；过高 DPI 会在页面缓冲区分配前失败。
