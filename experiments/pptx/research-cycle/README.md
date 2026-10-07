# 科研探索周期：LayMesh → CLI PPTX 复刻试验

复刻用户提供的 ABACUS 流程信息图。沿用 `codex/pptx-export` 分支的 Rust CLI；本次没有修改导出器、默认安装的 `laymesh`、字体处理或编辑器入口。画布为 **355.2 × 200 mm**，对应参考图 **1998 × 1125**，一张幻灯片。

时间范围、工时和“额外等待约6小时”来自参考图，仅为情景估算；保留了“情景估算，非实测”和适用条件。本试验不构成 ABACUS/Codex 性能基准。

## 可编辑内容

- **69 个文本框**，所有 `.lay` 文字内容按绘制顺序与 PPTX 对照，逐条相等。
- **239 个矢量形状**，包括循环箭头、节点、圆角卡片、图标、书脊及背景；形状可改颜色、位置和相应描边属性。曲线箭头是可编辑的自定义几何，不是 PowerPoint 智能连接线。
- **6 个源码显式分组**，对应页眉、工时比较、传统流程、Agent 流程、接续流程、页脚。
- **3 个原生圆角图片对象 / 3 份 PNG 资源**。办公室人物与睡眠场景用内置 `image_gen.imagegen` 生成，独立资源在 `assets/`。图片可移动、更换和调整裁剪；人物内部不是矢量对象。
- **0 个局部栅格回退**。唯一 PPTX 警告是原有的未嵌入字体提示。

生成提示全文、参考角色和执行方式见 `imagegen-prompts.json`。插画是按参考重新生成的近似复刻；人物细节、办公室背景、顶部水纹及图标造型与原图有差别。没有声称逐像素一致。参考图内的笔记本标记未复刻。

## 复现

在仓库根目录执行，始终使用分支构建的二进制：

```sh
python experiments/pptx/research-cycle/build_source.py
./target/release/laymesh validate experiments/pptx/research-cycle/research-cycle.lay
./target/release/laymesh render experiments/pptx/research-cycle/research-cycle.lay -o examples/output/research-cycle/research-cycle.pptx
./target/release/laymesh render experiments/pptx/research-cycle/research-cycle.lay -o examples/output/research-cycle/research-cycle.pdf
python experiments/pptx/research-cycle/verify.py
```

`build_source.py` 只是生成易修改的 `.lay` 排版源码；PDF/PNG/PPTX 均由 Rust CLI 导出，没有加入 Python 导出 API。修改源文件时可直接编辑 `.lay`，无需运行生成器。

交付 ZIP 包内包含 Linux 实验 CLI、`.lay` 和资源。解压后运行 `./render.sh` 可以直接重建导出文件，不替换系统 CLI。该 Linux CLI 仍依赖系统字体和运行库。

PPTX 保持默认 **1200 DPI** 的局部回退配置；本图没有发生回退。`research-cycle.png` 是 CLI 以 **142.875 DPI** 导出的 1998 × 1125 PNG；`native.png` 是原生 PDF 的同尺寸预览，`pptx-preview.png` 是 LibreOffice 打开 PPTX 后转 PDF 的预览。

## 本次验证

- `laymesh validate` 通过，画布及源码分组正确。
- ZIP/XML 可解析，所有关系目标可解析，所有绘图 ID 唯一；检查画布尺寸、69 条文字、原生几何、线性渐变、三张原生圆角图片和媒体嵌入。
- 用隔离的临时 LibreOffice 用户目录打开并转 PDF，再保存为 PPTX 后再次转 PDF。69 个文本框、全部绘图对象总数 **311** 和 6 个分组均保留。LibreOffice 把三个图片对象保存为含图片填充的形状，因此往返后的单独 `p:pic` 数为 0；嵌入图片仍保留。
- 检查原图、原生 PDF、PPTX PDF 和往返 PDF 的预览。流程、节点、说明、圆角、图层和图片均可见，未发现文字溢出、对象遗漏或意外裁剪。
- 全图原生/PPTX 平均 RGB 差异 **7.03/255**，往返前后 **1.10/255**。LibreOffice 会在中英文/数字混排处增加字间距；字体处理保持现状，因此不把全图像素差异当作几何等价判据。
- 文字内容及对象数量独立精确检查。额外依据 DrawingML 文本框位置屏蔽文字区域及邻近一个字号的水平余量，非文字区域原生/PPTX RGB 差异 **1.84/255**，往返前后 **0.22/255**。屏蔽范围在 `nontext-mask.png`，完整指标及检查项在 `evidence.json`。此指标不代替文字视觉检查。
- **PowerPoint 实机验证：未执行**。LibreOffice 结果不能保证所有 PowerPoint 版本表现完全相同。未嵌入 `Noto Sans CJK SC` 和 `DejaVu Sans`，跨机器字体替换可能改变字距和排版。

本次只增加实验源码与交付物，不改渲染器或文档媒体输入；因此没有重建文档媒体，也没有发布版本。
