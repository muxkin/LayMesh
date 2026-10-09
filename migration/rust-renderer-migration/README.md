# 纯 Rust 渲染器迁移实验结果

基于干净的 `main`（`9c6f26f`）建立 `codex/rust-renderer-migration`。新默认链路为 `Scene → DisplayList v1 → Vello CPU 0.3.0 / krilla 0.8.2 / DrawingML`，本轮未发布版本。CLI、Python 和现有导出函数签名保持兼容。

## 实现与兼容边界

- 绘制指令使用毫米坐标，携带路径、画笔、定位字形及字符区间、编码图片、变换、裁剪、遮罩和效果。普通文字共用 Rust 字体选择和整形；公式、SVG 文字和艺术字保留源文本。
- 所有位图编码器消费同一份 Vello CPU RGBA；现有 DPI、TIFF、WebP 等设置继续生效。输出按 1024 像素分块，保持 100,000,000 像素上限；宽图图片资源也分块并处理采样重叠。
- Rust Alpha 层实现阴影、发光、内阴影及扩散；效果块带模糊重叠，单块最多 16,000,000 RGBA 像素，累计效果预算最多 100,000,000 像素。
- PDF 直接写路径、定位字形、字体子集、渐变和图片，主体保持矢量，效果局部栅格化；正文搜索、公式和艺术字源文本复制、16 位 PNG/Alpha、PDF 图片压缩策略已通过现有合同。PPTX 直接生成 DrawingML，保留编辑对象、原始 JPEG 与局部回退诊断。
- VS Code 与网页通过 WASM CPU 绘制 Canvas，按缩放、可见区域和 DPR 重绘，inspection 继续提供坐标、拾取和标尺。协议 3 是默认，新客户端不自动退回 SVG；协议 1/2 及 WASM `svg()` 仅作为显式兼容接口。
- `resvg`、`krilla-svg` 和内部 SVG 解析路径已移除。SVG 文件导入仍先进行既有安全校验，再由 usvg 适配为绘制指令；文件导出保留专门序列化代码。原安全子集限制保持不变。
- 扩展内置 WASM 和公式字体，正文由原生服务选择并传输字体资源，预览可离线工作。CI 的原生打包任务已补齐 WASM target 和 wasm-bindgen 0.2.129 构建前提。

## 验证

本机完成 516 个 Rust 测试、51 个 Python 测试，891 处历史断言的源码映射和实际执行门禁全部通过。覆盖文字、连字、双向文字、字体集合、缺字、公式、艺术字、渐变、纹理、透明图片、遮罩、嵌套变换及现有几何容差。

原生与浏览器 WASM 使用同一份绘制指令和编码资源，8 组实际 RGBA 对照通过：7 组完全相同，vector 样例最大单通道差异为 1/255。70,000×2 像素窄图及 1024、2048、65535 像素附近边界采样没有发现接缝。Rust 回归额外覆盖超宽透明图片的分块采样、输出像素上限和未知绘制版本。

网页完成全部 186 个示例的编译与 960 像素宽实际绘制，桌面 1440×1000 和窄屏 390×844、字体载入和语言服务检查通过；无相关控制台错误。浏览器插件运行工具在本会话不可用，使用仓库现有 Chromium/CDP 验证工具完成检查。

实际 VS Code 扩展宿主与 webview 检查通过，包含 DPR 2、缩放/平移/标尺、坐标拾取、未保存导入、资源更新、过期 generation、错误恢复和渲染进程超时重启。空 PATH 下的语言功能、14 种直接导出、15 个位图编码格式以及三个箭头模板导出也通过。

10 个 PPTX 已进行 LibreOffice 打开、渲染及回读，文字、形状、组和局部图片仍保留其对象结构。PDF 用实际 `pdffonts`、`pdftotext`、`pdfimages` 和栅格对照验证，字体均嵌入并子集化，正文和 LaTeX 源文本可提取，图片与效果不会把整页变成位图。

## 原生导出性能与新旧像素

以下为完整 CLI 导出时间，包含新进程启动、字体/编译、绘制和编码，单位 ms。固定输入和 144 DPI；SVG 不接受 DPI 参数。每组先预热一次，再交替执行旧、新后端各五次，取中位数。源文件/资源指纹、每次耗时、输出体积及 Linux wait4 峰值 RSS 保存在 [benchmarks.json](benchmarks.json)。测量发生在本机验收环境，只用于这七个案例。

| 案例 | PNG 旧 → 新 | PDF 旧 → 新 | PPTX 旧 → 新 |
| --- | --- | --- | --- |
| typography | 72.4 → 87.6 | 69.0 → 83.9 | 63.3 → 79.1 |
| vector | 378.9 → 383.8 | 369.8 → 367.0 | 377.9 → 373.4 |
| art-text | 90.4 → 82.0 | 139.1 → 107.0 | 121.4 → 99.1 |
| shadow-glow | 67.0 → 52.2 | 104.9 → 46.7 | 72.7 → 42.5 |
| images-groups | 96.9 → 56.4 | 116.4 → 62.4 | 103.4 → 50.8 |
| svg | 66.4 → 80.4 | 43.5 → 59.7 | 63.3 → 76.7 |
| scientific | 48.2 → 49.3 | 36.7 → 37.7 | 47.3 → 41.4 |

效果案例有改善，文字和 SVG 素材案例存在开销增加，不能据此宣称所有场景提速。初次测量发现原生字体加载失去 memmap 的回退，已显式恢复 `fontdb/memmap`；重复字体资源也按字体键缓存。

同一输入的新旧 PNG 白底 RGB 平均绝对差（0–255）如下。所有案例 Alpha 通道差异为 0；透明图片及效果 Alpha 另外通过原生/WASM 与合同测试检查。差异图和指标用于检查抗锯齿、效果边缘和采样变化，不要求不同栅格器逐像素相等。

| 案例 | RGB MAE | 144 DPI 对照 |
| --- | --- | --- |
| typography | 0.302 | [旧](comparisons/baseline-typography.png) / [新](comparisons/current-typography.png) |
| vector | 0.348 | [旧](comparisons/baseline-vector.png) / [新](comparisons/current-vector.png) |
| art-text | 0.305 | [旧](comparisons/baseline-art-text.png) / [新](comparisons/current-art-text.png) |
| shadow-glow | 0.196 | [旧](comparisons/baseline-shadow-glow.png) / [新](comparisons/current-shadow-glow.png) |
| images-groups | 1.509 | [旧](comparisons/baseline-images-groups.png) / [新](comparisons/current-images-groups.png) |
| svg | 0.153 | [旧](comparisons/baseline-svg.png) / [新](comparisons/current-svg.png) |
| scientific | 0.232 | [旧](comparisons/baseline-scientific.png) / [新](comparisons/current-scientific.png) |

## 预览与内存

VS Code 同一 images-groups 案例，1200×600 Canvas、DPR 2，排除编辑防抖，详见 [preview-performance.json](preview-performance.json)。新帧安装完成后直接绘制，避免后台 webview 延迟动画回调；鼠标与缩放更新仍按动画帧合并。旧客户端在无头环境下有明显调度波动，端到端中位数不应当作通用用户性能承诺。

- 旧客户端：冷启动 155 ms，五次热更新中位数 229 ms。
- 新客户端：冷启动 365 ms，五次热更新中位数 91 ms；热更新原生处理约 7.0 ms、资源等待 0.0 ms、指令安装 0.5 ms、RGBA 绘制/复制约 81.0 ms。
- 新预览进程峰值 RSS 51.3 MiB；该案例 WASM 线性内存 20.4 MiB。线性内存是容量高水位，不等于浏览器整体进程 RSS。
- 186 案例顺序运行时 WASM 线性内存最高 211.9 MiB。typography 浏览器案例记录了编译、指令生成、资源等待与绘制阶段；具体值见 [verification.json](verification.json)。
- 28 组原生导出中，新链路最大峰值 RSS 122.1 MiB，旧链路 121.6 MiB。
- Linux x64 实验 VSIX：旧 7.62 MB，新约 11.39 MB；新增 WASM 原文件 10.76 MB。离线包审计通过，未上传或发布。

## 圆环锯齿的根因及修复

`examples/effects/alpha-ring.png` 原来只有 128×128 像素、两个 Alpha 值（0、255），边缘没有覆盖率过渡。SVG 版本也是嵌入此 PNG，因此放大后同样产生阶梯；换渲染器不能还原原始素材已经丢失的曲线细节。

补充 [可编辑矢量源](../../examples/effects/alpha-ring.svg)，用 [build-example-assets.py](../../scripts/build-example-assets.py) 经本项目原生渲染器生成 1024×1024 透明 PNG，现有 225 个 Alpha 层级。颜色和透明图片示例用途保持一致。此素材修复独立于渲染器性能比较，新旧性能使用同一更新后的素材。

| 原始 128 像素素材 | 修复后的 1024 像素素材 |
| --- | --- |
| ![修复前](comparisons/ring-before.png) | ![修复后](comparisons/ring-after.png) |

已同步两处引用示例的响应式文档媒体，并重新生成整个引擎指纹对应的 `site/media/manifest.json`；186 个媒体源和 615 个文档页面检查通过。

## 复现与交付范围

```sh
cargo build --release --locked -p laymesh-cli
python scripts/build-example-assets.py
python scripts/test-contracts.py --offline
python scripts/build-editors.py --binary target/release/laymesh
python scripts/build-docs-media.py
python scripts/build-docs.py --check
python scripts/benchmark-renderers.py --baseline /path/to/main-laymesh --output /tmp/benchmark.json
```

网页验证需要将静态服务绑定到本机 Tailscale 地址，再运行 `test-renderer-migration.py`、`smoke-browser.py --all-examples` 和 `test-browser-contours.py`。本次服务为 `http://100.64.0.6:8767/`。预览检查使用 `test-vscode-preview.py` 和 `test-vscode-preview-performance.py`。

精简指标和代表性对照图片随实验分支保存。完整原始验证日志、PDF/PPTX/PNG/SVG 输出、旧引擎和实验安装包保存在本地 `release/verification/rust-renderer-migration/`，不提交安装二进制和原始日志。

本次本机验证平台为 Linux x64、Chromium、VS Code 与 LibreOffice；Windows/macOS 的实际宿主及 Microsoft PowerPoint 尚未在本轮执行。CI 构建前提已更新，不能将本地通过等同于远端 CI 通过。未增加 GPU、公开混合模式或新滤镜。


## 完整公式网页复核（2026-10-09）

公式画廊已改用共享绘制指令与 Vello CPU WASM Canvas，SVG 仅提供文件下载。
[Tailscale 画廊](http://100.64.0.6:8768/)保留此前优化使用的全部 4,100 条记录、1,961 个不同公式，
四种字体、行内/独立模式与两种文字策略，共 65,600 个条目结果。
原始文件及字体 SHA-256、15,368 个固定审计组合的预期错误和缺字数量均通过核验。
其余结果保留解析/策略错误、缺字占位与 4 个既有尺寸异常，不将占位符记为正确字形。

实际 Chromium 全量绘制 30,412 个可绘制组合，无失败、无相关控制台错误。
在 16 px、DPR=1 的相同视口下，30,358 个组合的 RGBA 哈希相同；其余 54 个逐字节核对后，
每个只差一个通道的一个数值单位（1/255），保留为抗锯齿舍入差异。
检查了分类、行内模式、字号、基线与手机 DPR=2 重绘；截图见
[桌面](comparisons/math-gallery-desktop.png)、[手机](comparisons/math-gallery-mobile.png)，
[逐项验证记录](math-gallery-verification.json)包含差异路径及运行指纹。
全量检查使用仓库 Chromium CDP 工具；本会话未提供 Browser 插件所需的 Node REPL。

此次补充测试发现并修复了 RaTeX `\colorbox{red}{…}` 等 CSS 命名颜色的绘制兼容问题。
渲染库 7 项测试通过，包括颜色、透明 PNG、字体与直接 PDF 检查。
复现入口是 `experiments/opentype-math/verify-gallery.py`；生成方式见公式实验说明。

文档媒体已重新生成并通过 186 项指纹检查，615 个文档页面检查通过；
图片与组的普通网页预览重新验证了 Canvas、编辑刷新、字体文件载入及桌面/手机布局，无相关控制台错误。
