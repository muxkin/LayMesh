# Rust + RaTeX 架构

LayMesh 使用同一套 Rust 代码完成 `.lay` 解析、LCSS 样式、资源解析、布局和绘图。原生 CLI、Python/Jupyter 及浏览器 WASM 共享这些实现；无需 Node.js、npm 或动态 JavaScript 引擎。

| 组件 | 职责 |
| --- | --- |
| [laymesh-core](../crates/laymesh-core/src/lib.rs) | 受限 DSL、物理单位、布局、绘图、字体和 RaTeX 公式 |
| [laymesh-render](../crates/laymesh-render/src/lib.rs) | SVG；原生构建启用 resvg PNG 与 krilla PDF |
| [laymesh-language](../crates/laymesh-language/src/lib.rs) | 不执行用户代码的补全、诊断、悬停、签名与导入解析 |
| [laymesh-cli](../crates/laymesh-cli/src/main.rs) | validate、render、inspect、LSP 标准输入输出服务 |
| [laymesh-wasm](../crates/laymesh-wasm/src/lib.rs) | 浏览器 Worker 的编译、SVG 和语言服务绑定 |

字体库只嵌入公式所需 KaTeX 字体，正文从系统或用户文件读取；浏览器必须由用户授权载入。无法提供字符时警告并绘制方框，继续完成输出。

Python 平台 wheel 只携带一个 Rust 原生程序，保留桥接 API、Notebook magic 和独立保存源码功能。网页使用预先锁版本的静态 ESM 编辑模块，Python 脚本生成文档、资源清单和站点，Cargo 与 wasm-bindgen 编译 WASM。

VS Code 扩展使用宿主提供的基础接口启动原生 `laymesh lsp --stdio`，不携带 npm 依赖树。其客户端仅负责协议与编辑器接口转换。
