# 离线 HTML / ZIP 放映与编辑

导出 Universal ZIP-HTML，将预编译场景、动画轨道、图片和播放器保存在同一个 `.zip.html` 文件中。桌面浏览器可直接打开；整个文件也能用 ZIP 工具提取。普通 `.html` 后缀使用相同容器。播放使用 CanvasKit，文字与公式为矢量轮廓，显示缓冲区随视口和设备像素比更新。

```sh
laymesh render examples/animation/presentation.laydoc -o presentation.zip.html
laymesh render examples/animation/presentation.laydoc -o editable.zip.html --html-editable
laymesh render examples/animation/presentation.laydoc -o presentation.zip
laymesh render examples/animation/presentation.laydoc -o with-fonts.zip.html --html-editable --html-embed-fonts
laymesh render examples/animation/first.lay -o original.html --html-image-compression original
```

`.laydoc` 默认打包全本，`--page N` 选择页面。标准 ZIP 将资源保留为独立文件，解压后通过静态 HTTP 服务打开 `index.html`。直接离线打开请使用 Universal 文件；不需要浏览器扩展或 CDN。

播放版只包含绘制、动画采样和播放控件。可编辑版还包含实际源码依赖、CodeMirror、编译器、语言服务、字体需求和内置样式库。编辑器支持补全、悬停说明、参数提示、诊断和定义跳转；内置库只读。默认不额外收集系统字体，项目自带及手动导入的字体始终保留。`--html-embed-fonts` 显式包含实际使用的完整系统字体。

源码编辑会暂停播放，准备当前文档全部页面；错误时显示诊断并保留上次成功画面。播放和翻页使用已准备场景。可下载更新后的可编辑文件、播放版或标准 ZIP；浏览器通过下载保存副本。

可编辑文件打开即显示浅色文件工作区：左侧文件树、中间可关闭的源码／资源标签页、右侧预览。关闭源码标签保留编辑缓冲区及撤销历史；内置样式库只读。文件选择或拖放将资源导入选中的项目目录，同名文件可替换或保留两份。导入图片立即打开透明棋盘预览，显示原始尺寸、体积和逻辑路径；复制引用后可在源码中使用。仅添加未引用资源时，保存沿用预编译场景，无需重新编译。TIFF 等由项目解码器预览；不支持的资源可下载原文件。

预览默认显示完整排版；播放与放映进入动画。适应、缩放、平移、页面单位标尺及坐标读数使用真实渲染视口。Ctrl／⌘ + 滚轮缩放，空格 + 拖动或中键平移；页面放大后显示横向及纵向滚动条。视口立即移动已有像素，再由 Worker 合并补绘清晰画面；缩放、平移与滚动不安装页面、不准备动画、不编译源码。放映使用与 VS Code／网页预览相同的独立 `Presentation` 画布、浮动控制栏、菜单和快捷键，从文档首页开始；Esc 退出并恢复工作区页面、视口、焦点和播放状态。

打开可编辑文件后会显示非阻塞字体提示。点击“连接本机字体”才请求浏览器授权；支持 Local Font Access 的安全上下文（桌面 Chromium／Edge，包括支持的本地文件环境）只枚举字体元数据，通过内置常用别名支持中英文搜索；仅读取作品需要或用户选中的字体，不再全量扫描命名表。Firefox／Safari、不安全的 HTTP 页面、拒绝授权等情况可导入字体。字体面板明确区分“原字体”和“替换为”，两处选择器均可搜索中文名、英文名、样式及 PostScript 名。选项先保存为草稿，点击底部“确认并应用”统一加载并重新准备全部场景；关闭或“暂不处理”保留草稿，重新打开可继续。加载失败保留面板和上次成功画面，显示具体错误并允许重试；替代关系随项目保存，映射存在但目标字节缺失仍显示待补齐。授权得到的系统字体默认不随副本打包，可在 HTML 导出面板中开启“包含使用的系统字体”。初次播放和翻页均不需要字体授权。

ZIP Deflate 压缩源码、场景、字体及 WASM。图片默认尝试质量 85 的 WebP，保留像素尺寸和透明度，编码较大时保留原编码；`auto/original/lossless` 可配置。选择包含系统字体时，中文字体通常是主要体积来源。不设置用户文件体积上限；浏览器内存及 ZIP 实现仍有平台限制。

可编辑包保留项目图片的原文件与逻辑路径，播放场景中的图片单独优化，以保证重新编译和资源替换可用。无损导出保留透明像素下的 RGB 值。浏览器再次下载时，平台不支持无损 WebP 编码则保留原编码。

```json
{"export":{"html":{"html_image_compression":"auto","html_image_quality":85,"html_editable":false,"html_embed_fonts":false}}}
```

Python 的 `render_file`、`render_source`、`render_document` 接受 `.html/.zip` 输出及 `html_editable`、`html_embed_fonts`、`html_image_compression`、`html_image_quality`。HTML/ZIP 不接受输出 DPI。

```python
from laymesh import render_file, render_document
render_file("figure.lay", output="figure.zip.html", html_editable=True)
render_document("report.laydoc", output="report.zip")
```

VS Code 的导出格式中提供 HTML/ZIP 的播放版和可编辑版。只选择格式和路径，图片策略在 `laymesh.export.html.*` 设置中调整。`laymesh.export.html.embedFonts` 默认为 `false`，仅控制额外收集的系统字体。

开发者在构建原生发行包前运行 `python scripts/build-web-export.py`，需要 Rust WASM target 和匹配的 wasm-bindgen CLI。运行时被嵌入原生程序；`LAYMESH_WEB_RUNTIME` 可显式覆盖运行时 ZIP，便于开发验证。

自动验证使用 `node scripts/test-web-export.cjs` 、`node scripts/test-web-workspace.cjs` 和 `node scripts/test-web-fonts-viewport.cjs`（需 Playwright 的 Chromium、Firefox、WebKit），输出体积和耗时 JSON 及截图。`window.__LAYMESH_WEB.metrics` 包含恢复、解压、显示准备、编辑编译和绘制耗时，播放 WASM、CanvasKit 与编译器堆内存，以及驻留缓存估算；这些数值不等于浏览器进程或 GPU 的总内存。GPU 绘制耗时为提交耗时。`pageInstalls` 统计实际安装，视口交互应保持该数值与 `compiles` 不变；交互布局延迟、绘制提交和绘制确认分别记录，不把布局帧率当作最终显示帧率。

## 从浏览器导出图片与文档

点击“导出”，选择格式、当前页／全部页面和文件名，然后保存。支持 PDF、SVG、PPTX、PNG、JPEG、TIFF、WebP、BMP、静态 GIF、ICO、PAM／PNM、PPM、PGM、PBM 和 TGA。PDF 的“全部页面”生成多页文件，保留每页原尺寸；其他格式的“全部页面”生成包含编号文件的 ZIP。HTML 可编辑版、播放版与标准 ZIP 保留为同一面板中的快捷操作。

图片默认 **1200 DPI**，TIFF 默认 **LZW**；分辨率、PNG／TIFF 压缩、JPEG 质量和不透明背景在“设置”中修改，并随作品保存。导出使用完整页面，不受预览缩放、滚动或动画进度影响。ICO 宽高最大 256 像素，超限会提示调整 DPI，不会静默降低分辨率。超过 100,000,000 像素也会明确报错。

SVG 与 PDF 保留矢量内容，文字使用轮廓，作品未选择嵌入字体时不会额外保存系统字体文件；PDF 复用固定版本 svg2pdf 0.13.0，将滤镜作用的局部转换成图片。PPTX 复用原生 DrawingML 导出器，保留可编辑文字、形状和图片，仅对复杂裁剪、蒙版或特效生成局部图片，并在状态栏列出回退说明。浏览器 WebP 使用无损编码。PDF 的图像优化细项、WebP 有损参数及 MP4／动画 GIF 编码仍由原生 CLI／VS Code 提供；不会将静态 GIF 冒充动画导出。

导出在独立 Worker 中运行，可取消；浏览器支持保存文件选择器时直接选择保存位置，其他环境使用浏览器下载。原有画面已准备完成时无需字体授权或重新编译；修改源码后必须先编译成功，错误时不会导出旧画面。播放版也能导出静态格式。所有编解码器随作品携带，无 CDN 或服务器依赖。0.6.3 之前导出的旧 HTML 如缺少矢量快照，需用新版 LayMesh 重新导出作品。

## 文件工作区与字体

可编辑 HTML 使用 Headless Tree 管理真实目录。资源管理器的“＋”和右键菜单可新建文件／文件夹、导入文件或整个目录；支持多选、拖动移动、重命名、剪切／复制／粘贴、下载和可撤销删除。文件夹下载为 ZIP。操作影响当前作品，保存可编辑 HTML 后保留目录层级与空目录。

移动或重命名会更新可静态解析的页面、样式、图片和纹理引用，同时重算文件内部的相对路径。动态拼接路径或语法不完整的文件会列出待检查项。入口可以移动／重命名，不能直接删除；生成资源与内置库保持只读。文件操作的撤销与文本撤销分开；若随后继续编辑文本，先撤销文本编辑再撤销文件操作。

文件树、标签页、编辑器、预览、放映和弹窗输入框都有对应的右键菜单，可用 Shift+F10 或菜单键打开。剪贴板受到浏览器限制时，使用 Ctrl / ⌘ + C、V。

字体面板不再扫描全部字体文件的中文名称。常用字体通过内置中英文别名匹配，未知字体使用系统名称。连接授权后，作品所需的同款字体会自动加载；导入原字体后自动恢复编辑，无需设置替代。只有换用其他字体时才需要明确设置替代关系。随作品保存的字体直接可用，未获得本机字体授权不影响已有画面的播放。
