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
| SVG、PDF | 不接受位图编码参数 | 保留矢量图形、物理页面尺寸及透明度 |
| PNG | `compression=fast/default/best`，默认 `default` | RGBA；写入 DPI 元数据 |
| JPEG | `quality=1–100`，默认 90；`background="#ffffff"` | 不支持 Alpha；先与底色合成；写入 DPI 元数据 |
| TIFF | `compression=none/lzw/deflate/packbits`，默认 `lzw`，均无损 | 单页 RGBA，非预乘 Alpha；写入 DPI 元数据 |
| WebP | `webp_lossless=true`；`quality=0–100`；`webp_method=0–6`，默认 4 | RGBA；无损默认质量/压缩力度 100，有损默认质量 90；保留 Alpha |
| BMP | 无额外编码参数 | RGBA；写入 DPI 元数据 |
| GIF | `background="#ffffff"` | 单帧、最多 256 色；保留完全透明像素，半透明与底色合成 |
| ICO、TGA | 无额外编码参数 | RGBA；ICO 宽高最多 256 px；TGA 使用无压缩编码 |
| PAM、PNM | 无额外编码参数 | RGBA PAM，保留 Alpha |
| PPM、PGM、PBM | `background="#ffffff"` | 先与底色合成；RGB、灰度、黑白二值；PBM 灰度阈值 128 |

所有位图支持 `--dpi`：正数，最多 25400；CLI/Python 省略时默认 96。每轴像素数为 `round(毫米尺寸 × DPI / 25.4)`，最少 1；总像素上限为 100000000。DPI 不改变布局尺寸、字号或 `layout_dpi`。仅 PNG/JPEG/TIFF/BMP 写入密度元数据；其他格式的 DPI 控制像素尺寸。JPEG/GIF/TGA 每轴最多 65535 px，WebP 为 16383 px。

WebP 还支持 `--webp-alpha-quality 0–100`（默认 100；降低需使用有损模式）、`--webp-near-lossless 0–100`（仅无损模式，默认 100，100 完全无损）。`method` 越高编码越慢，通常文件越小；有损 `quality` 越高视觉质量越高。无损模式的 `quality` 控制压缩力度。近无损值小于 100 允许 RGB 样本近似，Alpha 仍保留。参数说明依据 [WebP 编码器文档](https://developers.google.com/speed/webp/docs/api)。

原生栅格导出使用 **8 位 sRGB RGB/RGBA**。16 位输入在解码、ICC 转换、裁剪及中间 PNG 中保留精度和原始强度范围；最终页面栅格化到 8 位。TIFF 输出不会声称提供原生 16 位精度。SVG/PDF 中的图像素材仍走现有颜色与透明度链路，详见[图片格式](images.zh-CN.md)。

## Python 和 Jupyter

```python
from laymesh import render_file
render_file("figure.lay", output="figure.jpg", dpi=300, quality=95, background="#ffffff")
render_file("figure.lay", output="figure.tif", dpi=600, compression="lzw")
render_file("figure.lay", output="figure.webp", dpi=300, webp_lossless=True,
            quality=100, webp_method=6, webp_near_lossless=100)
```

`render_source` 接受相同关键字参数；`output=None` 时不能指定编码参数。`%laymesh` 和 `%%laymesh` 接受上面示例中的同名 CLI 参数。SVG 预览保持可用，导出文件使用所选编码。

## VS Code

在 `.lay` 编辑器执行 **LayMesh：导出图形**，或点击实时预览的 **导出**。依次选择格式、DPI、适用于该格式的参数和目标文件。参数按工作区及格式记忆；默认值可在 `laymesh.export.*` 设置中调整。导出使用当前未保存的入口及已打开导入文件；无需安装 Python、Rust 或 npm。远程窗口的目标文件位于扩展宿主所在机器。

## 诊断

不合法、格式不适用或未知参数会被拒绝。CLI 参数错误退出 2；布局、资源或编码错误退出 1。所有命令支持 `--warnings show|hide`，优先于 `LAYMESH_WARNINGS`。先完成渲染再通过临时文件替换目标，正常失败保留已有文件；资源诊断保留源码位置。`inspect --json` 保留全部警告。

[CLI 参数](cli-reference.zh-CN.md) · [Python API](python-reference.zh-CN.md) · [编辑器](editors.zh-CN.md)
