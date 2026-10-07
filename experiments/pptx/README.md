# CLI PPTX 导出质量修复验收

分支：`codex/pptx-export`。实验 CLI：Linux x86_64，`laymesh 0.4.0`；未替换系统默认命令，未发布版本。字体处理保持原状。

## 信息图结果

| 版本 | 可编辑文本框 | 其他矢量形状 | 分组 | 图片对象 |
|---|---:|---:|---:|---:|
| 修复前 LayMesh PPTX | 84 | 1123 | 1556 | 1 |
| 修复后 LayMesh PPTX | 84 | 815 | 5 | 1 |
| 直接制作版（既有对照） | 84 | 814 | 0 | 1 |

本图全部 84 个文本框内容逐框保留，人物插图改为原生圆角矩形图片填充；没有栅格回退。PNG 资源独立嵌入，可替换图片、调整裁剪与圆角。其余线宽、虚线、颜色等可准确表达的属性为原生 DrawingML；不能准确映射的描边仍保留精确矢量轮廓。

## 验证

- 14 项 PPTX Rust 测试通过：包结构、唯一 ID、关系引用、画布尺寸、原生几何、合并描边、显式分组、旋转/缩放/镜像/透明度、PNG/JPEG 资源复用、裁剪/圆角、尖角描边选区、复合描边透明间隙、渐变硬色标及复杂效果回退。
- 完整 render Rust 测试通过；图像格式/颜色、PDF/PNG 像素一致性及 CLI Rust 回归通过；共享无 native 特性构建检查通过。
- 10 个 CLI 例子通过 LibreOffice 打开、转 PDF 和往返保存，逐文本框内容、绘图数量及页面尺寸检查通过。15 种位图格式、参数非法值、警告开关、失败保护已有文件和原有导出 transport 检查通过。
- 信息图往返保存后仍为 84 个文本框、5 个分组、900 个形状；LibreOffice 将原图片对象保存为带独立 PNG 资源的图片填充形状。总绘图对象数未减少，逐框文字内容未变。
- 原生 PDF 与 LibreOffice PDF 固定为 1974×1118 像素对照：全图 RGB 平均绝对差 3.5157/255，人物区域 3.3899/255；往返保存前后 0.7967/255。逐区目视检查未发现遗漏、溢出、错误裁剪或图层颠倒；剩余差别是字体提示、图片重采样与边缘抗锯齿。
- 导出文档已更新；原生/WASM 文档构建、176 份媒体来源及 591 个文档页面检查通过，媒体清单已重建。
- LibreOffice：LibreOffice 25.2.3.2 520(Build:2)。**PowerPoint 实机验证未执行**，不能将 LibreOffice 通过等同于 PowerPoint 已验证。

## 剩余限制

本图无回退项；通用导出中的径向渐变、色标透明度变化、纹理、复杂蒙版/裁剪、滤镜、重叠对象的组透明度及不可准确表达的文字/图片变换仍局部回退，并给出 `W_PPTX_RASTER`。公式可解组编辑轮廓，不是可修改源码的 Office 公式；图表由文字和形状组成。字体不嵌入，跨机器替换可能改变排版。默认回退 DPI 仍为 1200。

## 文件与复现

- `research-agent.lay` 与 `assets/researcher.png`：修复后源码和已有 AI 插图；图中曲线及模型是示意图。
- `research-agent.pdf`：原生 PDF；`research-agent.png`：CLI 直接 PNG（142 DPI）；`native.png`：原生 PDF 渲染的等尺寸对照 PNG。
- `research-agent.pptx`、`pptx-preview.png`：修复后 PPTX 及实际 LibreOffice 预览。
- `comparison.png`：修复前/后原生图、修复前/后 PPTX、直接制作版、往返保存版六图对照。
- `before/`：既有对照；`roundtrip/`：LibreOffice 往返保存结果；`regression/evidence.json` 与 `evidence.json`：机器检查证据。
- `bin/laymesh`：本分支构建的实验 CLI，SHA-256 `60b0fbc4ac15b019c1cdfa22845af0bf62c5a9f66f2a2abc01b5fc83217061e2`。

```sh
cargo build --release -p laymesh-cli
target/release/laymesh render experiments/pptx/research-agent.lay -o figure.pptx
python scripts/test-pptx-export.py --libreoffice --output examples/output/pptx-quality-fix/regression
```

源码、实现与验收例只保留在实验分支；交付目录位于 `examples/output/pptx-quality-fix/`，生成文件不提交到 Git。
