# 文档构建与 GitHub Pages

文档站由双语 Markdown、导航清单和示例清单生成。正文使用完整可用宽度，本页目录是独立浮层；源码可在页面中查看，预览使用真实渲染的 WebP。

## 本地构建

需要 Rust 1.93.1、Python 3.11+、wasm32-unknown-unknown 目标及与 Cargo.lock 对应的 wasm-bindgen CLI。在仓库根目录执行：

```sh
python -m pip install -r release/docs-requirements.txt
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129 --locked
python scripts/build-docs.py
python scripts/build-docs.py --check
```

输出在 `site/dist/`。页面及资源使用相对链接，支持 `/LayMesh/` 项目子路径。部署构建由 Python 生成文档，读取随仓库维护的高清图片。

## 更新内容与图片

[导航清单](navigation.json)定义一级领域、二级主题组和三级主题页。[示例清单](../scripts/site_support.py)复用双语画廊清单，关联完整源码与依赖。主题正文位于 `docs/topics/`，概览位于 `docs/sections/`。

```sh
cargo build --release --locked -p laymesh-cli
python -m pip install -e './python' pillow
python scripts/build-gallery.py --write
python scripts/build-docs-media.py
python scripts/build-gallery.py --check
python scripts/build-docs.py
python scripts/build-docs.py --check
```

`python scripts/build-docs-media.py` 从 .lay 重新渲染 3840 像素宽图片，并生成 960、1920、3840 宽的无损 WebP。Notebook 示例使用已保存的 `.lay` 和素材；如需更新原始 Python 配方生成的素材，先运行 `python scripts/build-notebook-gallery.py --write`。原始位图的细节受其源分辨率限制。

图片位于 `site/media/`，需要随源码维护；`site/dist/` 无须提交。媒体清单记录源码、依赖和渲染配置指纹；过期或缺失预览会阻止构建。图标保留 SVG。完整静态和动画导出格式见[导出手册](../docs/topics/export.zh-CN.md)及[动画导出](../docs/topics/animation-export.zh-CN.md)。

源码区直接显示完整文件，不裁取摘要。多文件项目通过文件标签切换，图片与完整源码始终对应。

## 手册生成与验收

样式与动画的入口页及分章分别维护。API 参考由 api.json 生成；逐项外观参考和 97 个效果目录由固定库与同一元数据生成。章节正文保留在生成标记之外，重新生成不会追加重复说明。

```sh
python scripts/build-api-reference.py --check
python scripts/build-topic-walkthroughs.py --check
python scripts/build-style-animation-docs.py --check
python scripts/check-feature-docs.py
python scripts/check-gallery-docs.py
python scripts/build-docs-media.py --check
python scripts/build-docs.py --check
node scripts/test-docs-manual.cjs SITE_URL OUTPUT_DIRECTORY
```

修改元数据后先运行对应生成脚本（去掉 --check），再重建原生引擎、媒体和网站。浏览器脚本使用现有 Playwright/Chromium，检查单一共享预览、97 次选择编译、真实播放、静态恢复、手机宽度和主题编辑。可通过 PLAYWRIGHT_MODULE、CHROMIUM_PATH 指定安装位置；预览绑定 Tailscale，验收产物留在仓库外。

复杂案例的布局与主题是手工维护的 Lay 程序，使用组件、循环和锚点。`node scripts/build-dashboard-data.mjs --check` 只检查独立合成数据；运行时不依赖 Node。`node scripts/test-docs-manual.cjs SITE_URL OUTPUT_DIRECTORY --cases-only` 检查单处宽度修改后的锚点重排，以及三组项目的主题修改、真实放映、推进、回退和静态恢复。

## 在线编辑与实时预览

示例在同一个工作区显示代码和图形：工作区宽度至少 900px 时左侧代码、右侧图形，窄屏改为代码在上、图形在下。拖动分隔条或使用方向键调整比例，双击或按 Enter 恢复等宽。本页目录始终悬浮在正文上方。

所有教程默认显示完整源码，可直接编辑。多文件项目通过文件标签切换，切换会保留草稿和当前预览。运行和字体操作在可编辑文件中显示，Python 源码保持只读。

预览区域默认近黑色，可切换白色或棋盘格，不改变源码指定的画布底色及导出结果。WebP 与 SVG 共用适应区域、缩放、平移和放大工作区操作。背景、分隔比例及缩放只保留在页面内存。预览工具栏显示“放映”及下拉菜单：主按钮直接全屏，下拉菜单可选窗口放映。全屏 Esc 直接结束放映。常规预览支持点击、右键和方向键；首次入场等待点击，后续自动衔接正常执行。底部显示本次合计耗时，“详情”分别显示编译、绘制指令、绘制、初始化和资源加载；错误会展开诊断，并将保留的图形和耗时标为上次成功结果。

原生示例的完整源码、模块及 CSV/JSON 可直接编辑。停止输入 250ms 后在浏览器 Worker 中重新编译；“立即运行”跳过等待，“恢复原始示例”重置全部文件与撤销记录。源码与图形共同反映当前草稿。Python/Notebook 源码仅供查看。

编辑记录和用户载入字体只存在页面内存，不上传、不写浏览器存储。刷新、重新打开及从前进后退缓存恢复时重置。错误显示源码位置并保留标明的上次成功结果；超过 10 秒的计算可停止并重新运行。

耗时栏使用本次 `performance.now()` 实测，分别列出编译、绘制指令、绘制、初始化和资源准备。防抖及资源等待不计入绘图计算耗时；浏览器数据不能替代 CLI 基准。

`python scripts/build-docs.py` 通过 Cargo 构建 Rust/WASM，复制锁版本的静态编辑模块、Worker 和示例资源。Page 单独携带 DejaVu Sans 与 Noto Sans CJK SC 正文字体及许可文件；构建使用固定版本 FontTools 4.62.1，从全部示例及依赖提取字符，生成保留全部 10 个字体及其顺序、名称和排版特性的 Noto 字体子集；检查字符覆盖和 SHA-256。字体以内容哈希命名，从同源加载，WASM、资源清单和启动字体并行准备。首次修改源码后按需加载完整字体；若修改先于字体就绪，保留当前画面，完成后只编译最新内容。字体不进入 wheel 或 CLI。预览不依赖服务端运行时或 Python。TIFF 等素材由共用 Rust 解码器处理，原源码路径由资源清单映射。编译只访问注册资源。

源码始终完整显示，编辑器在工作区接近视口时加载；完整字体在首次修改后按需加载。动画可直接播放、暂停和定位，切换文件标签不会重置播放。默认打开完整静态排版；“完整排版”按钮恢复静态。准备期间点击播放会记住请求，首帧完成后从第一步开始；加载失败可重试。“详情”中的准备和首帧时间从本次导航开始计时，反映当前设备的实际等待。

播放使用浏览器刷新时钟，绘制队列仅保留正在处理的一帧和最新待绘帧。播放始终保持屏幕原生像素密度；步骤停留期间复用边界帧，减少重复栅格化。这不改变时间轴、Rust/WASM 插值或导出。详情按浏览器刷新节奏统计不同动画帧，显示动态段最近一秒的帧率。

启动回归与五次冷/热启动测量：`node scripts/test-browser-animation-startup.cjs URL OUTPUT_DIR --corpus`（使用已安装的 Playwright，可通过 `PLAYWRIGHT_MODULE`、`CHROMIUM_PATH` 指定）。`--corpus` 对比全部示例使用子集与完整字体时的 RGBA 校验值。

## 首次启用 Pages

在仓库 **Settings → Pages → Build and deployment → Source** 中选择 **GitHub Actions**。

- 推送 `main`：构建、检查，成功后发布。
- Pull request：仅构建检查。
- 手动运行：在 Actions 中选择 `docs-pages`，选择 `main` 后运行。

工作流使用 Rust 1.93.1 与 Python 3.13，依次执行 `python scripts/build-docs.py`、`python scripts/build-docs.py --check`，然后上传 `site/dist/`。发布使用官方 Pages Actions 和 `github-pages` 环境；正在执行的部署不会被新部署取消。

在 **Actions → docs-pages** 查看构建与部署步骤，在 **Settings → Pages** 查看最终站点地址。项目站点以仓库名作为子路径，例如 `/LayMesh/`。本地检查通过不等于线上部署已经完成，首次启用后仍需核对运行结果。

## 隐私与预览

公开文档、源码、图片元数据和诊断记录不包含个人目录、设备地址、凭据或私人数据。发布包排除依赖缓存和临时输出，检查会拒绝用户绝对路径。保留项目开源版权署名。

预览服务绑定本机的 Tailscale 接口；设备地址和浏览器验收产物不进入 Git 提交或发布包。

## 固定版本库目录

`python scripts/build-library-docs.py` 用编译器解析器提取固定上游 CSS 的全部类、变量与关键帧，探测可用属性，并逐项编译、渲染或采样可用案例。`--check` 对比完整目录、源码位置、固定资源哈希、引擎指纹及案例验证。先运行它，再重建媒体与网站。Bootstrap、Open Props、Animate 页面各只使用一个 WASM 工作区；不支持的条目也保留并说明原因。
