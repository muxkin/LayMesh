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

播放使用 `RetainedScene`：节点路径在同一场景版本内稳定，资源具有独立 ID。WASM `presentation_state()` 返回矩阵、透明度、可见性，以及普通揭示动画的裁剪范围；未变化的路径保持在渲染器中。`frame_state()` 保留完整几何采样接口，描线和图表数据区域揭示沿用它的几何语义。普通变换动画不复制完整场景，也不生成 RGBA。网页与 VS Code 共用 `site/live/skia-renderer.mjs`，按内容保留路径、画笔、图片和 SkPicture，编辑后只失效变化的绘制记录。页面、绘图对象、文字排版和图片依附当前文档及有效版本保留，不设置缓存容量、页数或分类比例配额。未变化的绘图记录与资源跨编辑复用，删除或替换后按引用关系释放，关闭文档时整体释放。连续静态区域和运动群组使用实际显示像素精度的 GPU 分块缓存，逐帧应用变换与透明度；投影密度覆盖完整对象及透视变化，需要更高精度时重建。普通揭示动画只更新裁剪并复用内容图层；描线、图表数据区域揭示及含动态子对象的群组仍直接绘制。放映分别维护页面安装、入口画面和动画预热；已有入口画面可直接显示。支持 OffscreenCanvas 的窗口放映使用独立的播放与预加载 Worker；每页保留在构建它的渲染器中，切页时交换前后台角色，只为跨渲染器转场交接上一页的 ImageBitmap；显示缓冲区保留已呈现像素，交接不重新采样或绘制上一页。后台使用完整显示精度的 CPU 表面准备画面与图层，不向当前页的 GPU 队列提交整页预热；显示时复用这些缓存；CPU 与 GPU 光栅化的边缘抗锯齿可有差异，单独保留像素对照，不以逐像素一致描述跨后端缓存。压缩页面直接交给 Worker 解包，避免 UI 线程重复构造大对象。第一页面不等待后台运行时启动；两份运行时在关闭放映时一起释放。不支持 Worker 的回退及演讲者镜像模式仍串行绘制，只在动画停止时准备其他页。CPU 转场也使用实际显示尺寸。补丁替换当前场景中的属性，恢复状态保留当前场景，不累积历史传输量。透视由 Skia 投影绘制和局部 saveLayer 处理，不再使用 CPU 逐像素投影。子对象静态时用对象墨迹边界限制图层范围，含动态子对象时保留保守范围。GPU 最多同时处理两次动画绘制；忙碌时推迟绘制并重新采样当前时间，避免积压旧帧。静态恢复和显式定位仍保证绘制。表面分配失败会明确报错，不自动降低分辨率。

两个 WASM 与 CanvasKit 标准 JS 模块均使用同源内容哈希文件名。`skia_assets.py` 校验固定依赖并记录原始/gzip/Brotli 体积；预览服务器实际发送 Content-Encoding，对哈希资源启用不可变缓存。VSIX 不包含重复的 `.gz`/`.br` 副本，完整离线工作。旧 CPU/PDF 输出保留在 Git 历史中作为回归基线；运行时已不依赖 vello_cpu、krilla。

性能测量边界见[动画验证](topics/animation.zh-CN.md)。[4K 测试页](../tests/skia/benchmark.html)在文档构建时复制为 `/skia-benchmark.html`：固定 3840×2160 缓冲区，排除静止停留，十档递增压力场景（也可选择原有三个动画场景）可选择快速或五轮、每轮 30 秒的完整测量。提交耗时与浏览器呈现估计分开记录；这些数据不能证明显示器实际扫描输出。硬件达标仍需两台目标电脑的真实 GPU/合成器证据与活动的 60 Hz 显示器。

基准页提供 60 fps 提交上限与跟随屏幕两档，两者保持同一 4K 缓冲区；快速模式为每场景一轮 10 秒。结果包含 WebGL 设备名称、刷新间隔统计、GPU 缓存用量及相对所选频率的帧数缺口。在 160 Hz 屏幕限到 60 fps 可比较负载，但不能代替 60 Hz 显示环境下的呈现测试。[外部独显测量摘要](../tests/skia/user-hardware-summary.json) 留作证据，核显验收仍待实测。
