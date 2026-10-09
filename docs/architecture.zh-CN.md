# Rust + RaTeX 架构

LayMesh 使用同一套 Rust 代码完成 `.lay` 解析、LCSS 样式、资源解析、布局和绘图。原生 CLI、Python/Jupyter 及浏览器 WASM 共享这些实现；无需 Node.js、npm 或动态 JavaScript 引擎。

| 组件 | 职责 |
| --- | --- |
| [laymesh-core](../crates/laymesh-core/src/lib.rs) | 受限 DSL、物理单位、布局、绘图、字体和 RaTeX 公式 |
| [laymesh-render](../crates/laymesh-render/src/lib.rs) | 带版本的 DisplayList；Vello CPU 位图与 krilla 矢量 PDF；直接 DrawingML PPTX；SVG 文件适配 |
| [laymesh-language](../crates/laymesh-language/src/lib.rs) | 不执行用户代码的补全、诊断、悬停、签名与导入解析 |
| [laymesh-cli](../crates/laymesh-cli/src/main.rs) | validate、render、inspect、LSP 标准输入输出服务 |
| [laymesh-wasm](../crates/laymesh-wasm/src/lib.rs) | 浏览器 Worker 的编译、绘制指令、RGBA Canvas 与语言服务绑定 |

字体库只嵌入公式所需 KaTeX 字体，正文从系统或用户文件读取；浏览器必须由用户授权载入。无法提供字符时警告并绘制方框，继续完成输出。

Python 平台 wheel 只携带一个 Rust 原生程序，保留桥接 API、Notebook magic 和独立保存源码功能。网页使用预先锁版本的静态 ESM 编辑模块，Python 脚本生成文档、资源清单和站点，Cargo 与 wasm-bindgen 编译 WASM。

VS Code 扩展使用宿主提供的基础接口启动原生 `laymesh lsp --stdio`，不携带 npm 依赖树。其客户端负责协议与编辑器接口转换，扩展内置同一套 WASM CPU 渲染器；原生编译器通过协议 3 发送毫米坐标的绘制指令、独立字体与图片资源、inspection 和 generation，预览使用 Canvas。正文字体由原生服务选择并作为资源传递，扩展无需联网。


渲染从 `Scene → DisplayList v1 → 输出后端` 进行。字形 ID、整形后位置、原始字符区间与公式源文本由 Rust 共用；图片保留编码资源。SVG 只在导入适配器和导出序列化器中使用；位图、PDF、PPTX 和新预览均不生成或解析内部 SVG。显式协议 1/2 兼容调用及 WASM `svg()` 保留，新预览不会自动退回 SVG。

Vello CPU 0.3.0 使用共享 f32 CPU 管线；普通输出以 1024 像素分块，支持边长超过 65535 像素的窄图，总像素上限保持 100,000,000。阴影、发光和内阴影由独立 Rust Alpha 层处理，效果块带模糊重叠；单块最多 16,000,000 RGBA 像素。krilla 0.8.2 直接生成矢量路径、字体子集、定位文字、渐变和图片；效果局部栅格化，公式与艺术字通过 ActualText 保留源文本。PPTX 保留可编辑对象，对无法准确表达的对象给出原有局部回退诊断。
