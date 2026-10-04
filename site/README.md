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

[导航清单](navigation.json)定义一级领域、二级主题组和三级主题页。[示例清单](../scripts/site_support.py)复用双语画廊清单，关联源码、摘要与依赖。主题正文位于 `docs/topics/`，概览位于 `docs/sections/`。

```sh
cargo build --release --locked -p laymesh-cli
python -m pip install -e './python[plot,data]' pillow
python scripts/build-gallery.py --write
python scripts/build-docs-media.py
python scripts/build-gallery.py --check
python scripts/build-docs.py
python scripts/build-docs.py --check
```

`python scripts/build-docs-media.py` 从 .lay 重新渲染 3840 像素宽图片，并生成 960、1920、3840 宽的无损 WebP。Notebook 示例使用已保存的 `.lay` 和素材；如需更新原始 Python 配方生成的素材，先运行 `python scripts/build-notebook-gallery.py --write`。原始位图的细节受其源分辨率限制。

图片位于 `site/media/`，需要随源码维护；`site/dist/` 无须提交。媒体清单记录源码、依赖和渲染配置指纹；过期或缺失预览会阻止构建。图标保留 SVG。LayMesh CLI 导出格式仍为 SVG、PDF、PNG。

源码区从同一文件读取摘要和全文。`# BEGIN DEMO` / `# END DEMO` 指定命名摘要区；其余示例使用清单声明的 main 区。图片与完整源码始终对应。

## 在线编辑与实时预览

示例在同一个工作区显示代码和图形：工作区宽度至少 900px 时左侧代码、右侧图形，窄屏改为代码在上、图形在下。拖动分隔条或使用方向键调整比例，双击或按 Enter 恢复等宽。本页目录始终悬浮在正文上方。

单击摘要中的代码行，或聚焦摘要后按 Enter，可进入完整源码并定位对应位置；拖选和复制不会切换模式。已有编辑会保留，摘要仍显示原始图形。运行和字体操作在可编辑源码模式显示，Python 源码保持只读。

预览区域默认近黑色，可切换白色或棋盘格，不改变源码指定的画布底色及导出结果。WebP 与 SVG 共用适应区域、缩放、平移和放大工作区操作。背景、分隔比例及缩放只保留在页面内存。底部显示本次合计耗时，“详情”分别显示编译、SVG、初始化和资源加载；错误会展开诊断，并将保留的图形和耗时标为上次成功结果。

原生示例的完整源码、模块及 CSV/JSON 可直接编辑。停止输入 250ms 后在浏览器 Worker 中重新编译；“立即运行”跳过等待，“恢复原始示例”重置全部文件与撤销记录。摘要始终显示原始示例。Python/Notebook 源码仅供查看。

编辑记录和用户载入字体只存在页面内存，不上传、不写浏览器存储。刷新、重新打开及从前进后退缓存恢复时重置。错误显示源码位置并保留标明的上次成功结果；超过 10 秒的计算可停止并重新运行。

耗时栏使用本次 `performance.now()` 实测，分别列出编译、SVG 生成、初始化和资源准备。防抖及资源等待不计入绘图计算耗时；浏览器数据不能替代 CLI 基准。

`python scripts/build-docs.py` 通过 Cargo 构建 Rust/WASM，复制锁版本的静态编辑模块、Worker 和示例资源。Page 单独携带 DejaVu Sans 与 Noto Sans CJK SC 正文字体及许可文件；Worker 优先从 CDN 获取并核对 SHA-256，失败时读取 Page 副本。字体不进入 wheel 或 CLI。预览不依赖服务端运行时或 Python。TIFF 等素材由共用 Rust 解码器处理，原源码路径由资源清单映射。编译只访问注册资源。

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
