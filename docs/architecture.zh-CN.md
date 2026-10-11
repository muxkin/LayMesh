# Rust + RaTeX 架构

LayMesh 使用同一套 Rust 代码完成 `.lay` 解析、LCSS 样式、资源解析、布局和绘图。原生 CLI、Python/Jupyter 及浏览器 WASM 共享这些实现；无需 Node.js、npm 或动态 JavaScript 引擎。

| 组件 | 职责 |
| --- | --- |
| [laymesh-core](../crates/laymesh-core/src/lib.rs) | 受限 DSL、物理单位、布局、绘图、字体和 RaTeX 公式 |
| [laymesh-render](../crates/laymesh-render/src/lib.rs) | 带版本的 DisplayList；Skia CPU 位图与 SkPDF 矢量 PDF；直接 DrawingML PPTX；SVG 文件适配 |
| [laymesh-language](../crates/laymesh-language/src/lib.rs) | 不执行用户代码的补全、诊断、悬停、签名与导入解析 |
| [laymesh-cli](../crates/laymesh-cli/src/main.rs) | validate、render、inspect、LSP 标准输入输出服务 |
| [laymesh-wasm](../crates/laymesh-wasm/src/lib.rs) | 浏览器 Worker 的编译、场景、动画采样与语言服务绑定 |

字体库只嵌入公式所需 KaTeX 字体，正文从系统或用户文件读取；浏览器必须由用户授权载入。无法提供字符时警告并绘制方框，继续完成输出。

Python 平台 wheel 只携带一个 Rust 原生程序，保留桥接 API、Notebook magic 和独立保存源码功能。网页使用预先锁版本的静态 ESM 编辑模块，Python 脚本生成文档、资源清单和站点，Cargo 与 wasm-bindgen 编译 WASM。

VS Code 扩展使用宿主提供的基础接口启动原生 `laymesh lsp --stdio`，不携带 npm 依赖树。其客户端负责协议与编辑器接口转换，扩展离线内置 CanvasKit 0.42.0 标准包，优先使用 Worker OffscreenCanvas 与 WebGL2，无法使用 Worker 绘制时退回主线程 CanvasKit，无 GPU 时使用 Skia 软件表面；原生编译器通过协议 3 发送毫米坐标的绘制指令、独立字体与图片资源、inspection 和 generation，预览使用 Canvas。正文字体由原生服务选择并作为资源传递，扩展无需联网。


渲染从 `Scene → DisplayList v1 → 输出后端` 进行。字形 ID、整形后位置、原始字符区间与公式源文本由 Rust 共用；图片保留编码资源。SVG 只在导入适配器和导出序列化器中使用；位图、PDF、PPTX 和新预览均不生成或解析内部 SVG。显式协议 1/2 兼容调用及 WASM `svg()` 保留，新预览不会自动退回 SVG。

原生绘制固定使用 `skia-safe` 0.153.3，不依赖显卡。Skia CPU 表面保留 100,000,000 总像素上限，支持边长超过 65535 像素的窄图。PDF 页面由 SkPDF 生成；阴影、发光和内阴影使用同一套 Skia 滤镜分块绘制，主体保留矢量。`pdf.rs` 的版本锁定对象兼容补丁将内部标记图替换为原始 16 位颜色/透明度数据，恢复精确物理页面尺寸，并为公式、艺术字写入 ActualText。多页合并会重排结构父级编号。原有图片策略仍在绘制前处理 JPEG 保留、压缩、DPI 与 16 位保真开关。SVG 与可编辑 DrawingML 保留专用序列化器。

播放使用 `RetainedScene`：节点路径在同一场景版本内稳定，资源具有独立 ID。WASM `frame_state()` 只返回矩阵、透明度、可见性，以及描线/揭示所需的局部替换几何。普通变换动画不复制完整场景，也不生成 RGBA。网页与 VS Code 共用 `site/live/skia-renderer.mjs`，按内容保留路径、画笔、图片和 SkPicture，编辑后只失效变化的绘制记录。GPU 缓存遵循预览预算，默认 256 MiB；可选绘制记录最多使用预算的四分之一。动画对象之间连续的静态区域使用原生像素精度 GPU 分块缓存，最多占预算一半，不读回 CPU、不缩小采样精度。未变化区域跨编辑复用，视口变化时重建，LRU 淘汰限制驻留量；内容不变的运动群组使用 GPU 分块图层，逐帧只应用变换与透明度；投影像素密度上界覆盖整个对象及透视变化，需要更高密度时重建，向上分档并保留播放期间所需的最高精度。描线、揭示及含动态子对象的群组仍直接绘制。静态与运动纹理总计最多占缓存预算四分之三，保留前一帧仍在使用的资源，避免逐帧淘汰再重建。容量不足时停止接纳新的缓存，不降低精度或省略效果。纹理与绘制记录字节从 Skia 缓存限额中预留。必需场景数据和可见表面独立于可淘汰缓存。放映暂停保留可复用缓存，预算淘汰、资源替换与销毁会释放旧对象。透视由 Skia 投影绘制和局部 saveLayer 处理，不再使用 CPU 逐像素投影。子对象静态时用对象墨迹边界限制图层范围，含动态子对象时保留保守范围。GPU 最多同时处理两次动画绘制；忙碌时推迟绘制并重新采样当前时间，避免积压旧帧。静态恢复和显式定位仍保证绘制。表面分配失败会明确报错，不自动降低分辨率。

两个 WASM 与 CanvasKit 标准 JS 模块均使用同源内容哈希文件名。`skia_assets.py` 校验固定依赖并记录原始/gzip/Brotli 体积；预览服务器实际发送 Content-Encoding，对哈希资源启用不可变缓存。VSIX 不包含重复的 `.gz`/`.br` 副本，完整离线工作。旧 CPU/PDF 输出保留在 Git 历史中作为回归基线；运行时已不依赖 vello_cpu、krilla。

性能测量边界见[动画验证](topics/animation.zh-CN.md)。[4K 测试页](../tests/skia/benchmark.html)在文档构建时复制为 `/skia-benchmark.html`：固定 3840×2160 缓冲区，排除静止停留，十档递增压力场景（也可选择原有三个动画场景）可选择快速或五轮、每轮 30 秒的完整测量。提交耗时与浏览器呈现估计分开记录；这些数据不能证明显示器实际扫描输出。硬件达标仍需两台目标电脑的真实 GPU/合成器证据与活动的 60 Hz 显示器。

基准页提供 60 fps 提交上限与跟随屏幕两档，两者保持同一 4K 缓冲区；快速模式为每场景一轮 10 秒。结果包含 WebGL 设备名称、刷新间隔统计、GPU 缓存用量及相对所选频率的帧数缺口。在 160 Hz 屏幕限到 60 fps 可比较负载，但不能代替 60 Hz 显示环境下的呈现测试。[外部独显测量摘要](../tests/skia/user-hardware-summary.json) 留作证据，核显验收仍待实测。
