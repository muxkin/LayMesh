# LayMesh

Preview and edit scientific figures in `.lay` and `.lcss`. Get live Canvas previews, unit-aware rulers, canvas and plot coordinate readouts, completion, parameter hover, signature help, definitions, workspace references, occurrence highlights, safe rename, diagnostics and color editing. English and Simplified Chinese interfaces are included.

支持 `.lay` 与 `.lcss`：实时 Canvas 预览、单位标尺、鼠标画布和绘图坐标、代码补全、参数说明、诊断与选色器，界面支持中文和英文。

Version **0.6.4** shares its version with the Python package and Rust engine. Platform VSIX builds cover **Windows x64**, **Linux x64 / ARM64** and **macOS Intel / Apple Silicon**, including matching remote extension hosts. Preview uses a bundled native Rust program; no Python, Rust toolchain or npm installation is needed by extension users. Windows ARM64 and browser-only VS Code require their own compatible packages. Language analysis is static and never executes figure code; preview rendering is a separate opt-in process in a trusted workspace.

Build with Rust 1.93.1, `rustup target add wasm32-unknown-unknown` and `cargo install wasm-bindgen-cli --version 0.2.129 --locked`, then run `python scripts/build-editors.py` from the repository root. The extension bundles CanvasKit 0.42.0 and Rust WASM sampling, receives drawing protocol 3 from the native engine, and draws offline with WebGL2 or Skia software fallback. Create a stable Marketplace package with `python scripts/package-editor.py --target linux-x64 --output release/dist/laymesh-language-0.6.4-linux-x64.vsix`, then audit it with `python scripts/check-editor.py release/dist/laymesh-language-0.6.4-linux-x64.vsix --binary target/release/laymesh --target linux-x64 --marketplace`. No npm packages are needed. To install locally, use VS Code's **Extensions: Install from VSIX** command.

Build Windows x64 with the MSVC target on Windows using `cargo build --release --locked -p laymesh-cli`, then stage `target/release/laymesh.exe`. Linux cross builds require the MSVC SDK, LLVM and a compatible static Skia build; MinGW is not supported by this Skia release. Package Windows in a separate directory to preserve the Linux development binary:

```sh
python scripts/build-editors.py --binary target/release/laymesh.exe --extension-dir release/staging/vscode-win32-x64
python scripts/package-editor.py --extension-dir release/staging/vscode-win32-x64 --target win32-x64 --output release/dist/laymesh-language-0.6.4-win32-x64.vsix
python scripts/check-editor.py release/dist/laymesh-language-0.6.4-win32-x64.vsix --binary target/release/laymesh.exe --target win32-x64 --marketplace
```

Run `python scripts/smoke-native-editor.py --binary target/release/laymesh.exe --output release/dist/windows-native-evidence.json` on Windows to verify the preview protocol, resources, units, paths, error recovery and bilingual language service. On Linux, `--wine /path/to/wine` runs the cross-built EXE through Wine. This protocol check is separate from testing the actual Windows VS Code interface.

Stable version tags build and audit five platform VSIX packages in GitHub Actions. After the existing PyPI approval and upload, GitHub Release provides these exact packages and `SHA256SUMS`. Marketplace uploads are manual through the [Hyacine publisher portal](https://marketplace.visualstudio.com/manage/publishers/Hyacine); no Actions Marketplace secret is needed. See the [release procedure](https://github.com/muxkin/LayMesh/blob/main/release/README.en.md) and [platform-specific publishing](https://code.visualstudio.com/api/working-with-extensions/publishing-extension#platform-specific-extensions).

0.6.4 与 Python 包、Rust 引擎使用相同版本，构建 Windows x64、Linux x64 / ARM64 和 macOS Intel / Apple Silicon 独立安装包，内置原生语言服务与预览程序。稳定标签触发五平台构建，PyPI 人工审批和上传成功后生成 GitHub Release；从 Release 下载审核包，通过发布者门户手动上传 Marketplace。远程开发时，安装包平台应匹配远程扩展宿主。

Artistic text supports glyph gradients/outlines, open paths, arc/wave/perspective warps and vector extrusion. Text, shapes, transparent images and groups support inner/outer shadow and glow. See the [effects guide](https://muxkin.github.io/LayMesh/en/docs/topics/art-effects.html).

Keyboard: Ctrl+Space requests completion, Ctrl+Shift+Space requests a signature, F12 goes to a definition, Shift+F12 finds workspace references, and F2 renames a binding. Full signatures open on request rather than on typed punctuation; hover over a function or parameter for documentation. On macOS use the corresponding editor key bindings.

Geometry defaults to mm. Bare typography, border/line widths and dash lengths use pt. See the bilingual documentation for LCSS, formulas and migration from older parameter names.

Document library functions with adjacent `##` comments and `@param`, `@returns`, `@example`, and `@see`; document exported variables with `@type`. Signatures and defaults come from declarations. Imports and aliases retain their documentation, and source or dependency changes refresh help without executing code. See [library documentation](https://muxkin.github.io/LayMesh/en/docs/topics/library-docs.html) and the [Chinese guide](https://muxkin.github.io/LayMesh/docs/topics/library-docs.zh-CN.html).

The website uses Tab to accept completion and Enter to insert a newline. This extension does not override VS Code key bindings. For the same Enter behavior, use language-specific settings:

```json
{
  "[laymesh]": { "editor.acceptSuggestionOnEnter": "off" },
  "[lcss]": { "editor.acceptSuggestionOnEnter": "off" }
}
```

## Presentation playback / 演示放映

Open a `.lay` or `.laydoc` preview and choose **Present**. The first ready page can play while later pages compile in the background. Up to four native compiler processes yield to editing previews. Click, wheel or arrow keys advance; right-click opens navigation and playback options. **Tab** reveals the hidden control bar; its menu offers always visible, bottom-position hover, or hidden. Escape ends the presentation. Inline presentation fills the preview; the browser entry provides fullscreen through a session-scoped Tailscale service. The current document version stays fixed until exit.

打开 `.lay` 或 `.laydoc` 预览后选择**放映**，首页就绪即可开始，其余页在后台准备。最多四个原生编译进程，编辑预览优先。支持点击、滚轮、键盘、右键菜单、手动/自动推进。控制栏默认隐藏，**Tab** 呼出；右键可选一直显示、鼠标到位置显示或不显示，Esc 退出。错误详情使用浮层，编译失败保留上次成功画面。浏览器全屏入口复用已编译产物；MP4/GIF 导出自动查找已有 FFmpeg；缺少时确认后下载。

See the [animation and step guide](https://muxkin.github.io/LayMesh/en/docs/topics/animation.html) for `start="click"`, `start="with-previous"`, `start="after-previous"`, durations and CSS compatibility.

## File icons and formatting / 文件图标与格式化

`.lay` and `.lcss` have distinct SVG file icons for light and dark themes. VS Code uses these language icons when the active file icon theme permits them; a theme can provide its own icons or disable language icons.

Use **Format Document** or **Format Selection** from the editor context menu. Both languages use the bundled Rust formatter, including `style { ... }` inside `.lay`. Short calls stay on one line; long calls and comma-separated collections expand at `laymesh.format.lineWidth` (default **100**). Indentation follows the editor's tab size and spaces/tabs setting. Literal strings, interpolations and comment text retain their spelling; indivisible content may exceed the target width. Incomplete syntax or a transformation that cannot preserve meaning produces no edits. Selection formatting may expand to a complete statement or style declaration.

`.lay` 和 `.lcss` 提供可区分的浅色、深色 SVG 文件图标，显示遵循当前文件图标主题的规则。在编辑器中执行**格式化文档**或**格式化选区**即可整理代码，也支持 `.lay` 内嵌的 `style { ... }`。默认目标行宽为 **100**，通过 `laymesh.format.lineWidth` 调整；短调用保留一行，长参数与列表自动展开，缩进遵循编辑器设置。字符串、插值和注释正文保持原样；代码不完整或无法保证语义一致时不修改。选区会按需要扩展至完整语句或样式声明。

Save-time formatting follows your existing `editor.formatOnSave` setting. To enable it only for these languages:

保存时是否格式化沿用 `editor.formatOnSave`，扩展不会自动开启。可仅为这两种语言设置：

```json
{
  "laymesh.format.lineWidth": 100,
  "[laymesh]": {
    "editor.defaultFormatter": "Hyacine.laymesh-language",
    "editor.formatOnSave": true
  },
  "[lcss]": {
    "editor.defaultFormatter": "Hyacine.laymesh-language",
    "editor.formatOnSave": true
  }
}
```

## Help language and other LSP clients

`laymesh.language` accepts `auto` (default), `zh-CN`, or `en`. In VS Code, `auto` follows the editor display language. It controls preview toolbars, coordinate labels, status, accessibility labels, process messages, newly opened color editors, and language help. Changing it updates an open preview immediately while preserving the image and view, without compiling. Command titles and Settings descriptions follow VS Code's display language through bundled English and Simplified Chinese translations. Compiler diagnostic details and user-authored figure text retain their original language. The server never reads the host OS language to choose documentation.

For an English preview in a Chinese editor, add `"laymesh.language": "en"` to Settings. Use `"zh-CN"` for Chinese, or `"auto"` to follow VS Code.

Other IDEs can launch `laymesh lsp --stdio` from the repository root. Send `initialize.locale` for UI language or `initializationOptions: {"language":"zh-CN"}` for a startup preference. Clients supporting `workspace/configuration` return the `laymesh` section; others can push `workspace/didChangeConfiguration` with `{"settings":{"laymesh":{"language":"en"}}}`. A valid configuration setting wins over initialization options; `auto` uses the initial client locale. Missing or unsupported UI languages default to English. See the [LSP setup guide](https://muxkin.github.io/LayMesh/en/docs/topics/editors.html#lsp-integration-in-other-ides).

Completion, hover and signatures negotiate Markdown or plain text per client capability. Signature parameter labels use ranges only when the client advertises support.

Library comments may use `## @lang zh-CN` and `## @lang en` sections. Missing fields fall back to untagged text, English, then the first authored version. Types, names, default values and inserted code are not translated. Existing untagged comments remain valid. See [multilingual documentation](https://muxkin.github.io/LayMesh/en/docs/topics/library-docs.html#multilingual-documentation).

提示、预览器和新打开的选色器默认跟随界面语言；设置 `"laymesh.language": "en"` 可使用英文，`"zh-CN"` 使用中文，`"auto"` 跟随 VS Code。切换语言时已打开的预览立即更新，保留图像和视图，不触发编译。命令名称和设置说明跟随 VS Code 显示语言；编译器的具体错误和用户图形文字保留原文。其他 IDE 使用相同的标准 LSP 服务，通过初始化选项或配置通知设置语言。库注释支持 `@lang` 分段，缺少译文时回退原文；详见[中文编辑器指南](https://muxkin.github.io/LayMesh/docs/topics/editors.zh-CN.html)。


## Live figure preview / 实时图形预览

Open a saved `.lay` file and click the preview icon in the editor title, use **LayMesh: Open Preview** (**LayMesh：打开预览** in Chinese), or choose it from the editor context menu. Each entry has its own preview beside the editor; reopening reuses its panel. Preview runs in a separate native process and requires a trusted workspace. Local files in remote extension hosts work in the same way; untitled and virtual files must first be saved to the host filesystem.

Preview updates are scheduled 30 ms after the last edit by default, including unsaved imported modules, stylesheets and text data. Explicit debounce settings continue to take precedence. Changes to loaded local images, fonts and data also refresh the figure. A compilation error retains the last successful figure, marks it stale, and offers a source-location button. Language help continues independently.

The top and left rulers and canvas X/Y values follow `canvas(unit=...)`: `mm`, `cm`, `in`/`inch`, `pt`, or `px`. Pixels use `layout_dpi`, independently of raster export DPI. The origin is the canvas top left; X increases right and Y increases down. Explicit units mixed in the source do not change the preview's display unit.

Move the mouse over the figure to see canvas coordinates. Inside a plot, the bottom bar also displays every named data axis, including log, symlog, reversed and broken axes; a break gap shows `—` for that axis. Rotated/nested plots and insets retain their own coordinates. Polar plots display θ/r in their configured angle units; the center has no unique angle. Radar charts display a dimension value only within six screen pixels of its spoke, and only canvas coordinates at the center.

Toolbar: **Refresh**, **Fit**, **100%**, zoom out/in, **Rulers**, and **Export**. Use Ctrl/Command + wheel to zoom about the pointer, Space + left drag or middle drag to pan, and ordinary wheel/trackpad scrolling to pan. With the preview focused, `+`/`-` zoom and `0` fits. 100% maps one layout pixel to one CSS pixel using `layout_dpi`; it does not calibrate physical monitor dimensions. View state survives tab hiding, and source updates preserve the view.

```json
{
  "laymesh.preview.debounceMs": 30,
  "laymesh.preview.renderTimeoutMs": 30000,
  "laymesh.preview.showRulers": true
}
```

打开已保存的 `.lay`，点击右上角预览按钮或执行“LayMesh: 打开预览”。顶部和左侧标尺跟随画布单位；底部同时显示画布与绘图区数据坐标。Ctrl/⌘＋滚轮缩放，空格＋拖动或中键平移。错误时保留上一张成功图并标记过期，点击错误可跳转源码。未保存的模块、样式和数据优先于磁盘内容；30 秒超时会终止并重启独立渲染进程。

### Native preview protocol

`laymesh preview --stdio` writes `{"type":"ready","protocol":1}` at startup, then accepts one JSON object per line:

```json
{"id":1,"file":"/absolute/main.lay","source":"page=canvas(size=(4,3),unit=\"cm\")","overlays":{"/absolute/values.lay":"export width=2"}}
```

Each response echoes `id` and contains `svg` plus `inspection`, or a structured `error` with source location. `dependencies` includes attempted resource paths on success and failure. Source and overlays stay in memory; resource paths resolve from the real entry path. Inspection geometry remains mm, while `page.unit` and `page.layout_dpi` describe the display conversion. Scene JSON adds `canvasUnit`; old scenes default to mm.

## Variables and workspace refactoring / 变量与工作区重构

Variable hover and completion show inferred types at the cursor. Reassignments are write references to the same binding; ordinary value aliases keep their own definition. Function and imported function aliases continue to navigate to the original function. References include formatted-string expressions and named arguments to user functions, excluding comments, literal string text and object member names.

The server indexes unopened `.lay` files in all workspace folders, plus local imports. Build/dependency directories and release staging are skipped, and directory symlinks are not followed. Unsaved buffers take precedence. Explicit import aliases rename locally; renaming an export updates imported source names and unaliased uses while preserving explicit aliases. Incomplete affected code, invalid names or binding capture prevent rename; versioned edits guard open buffers.

变量悬停和补全显示光标位置的推断类型；重复赋值记作同一绑定的写引用，普通值别名跳到自身定义。函数及导入函数别名继续跳到原函数。引用覆盖格式化字符串表达式和用户函数命名参数，排除注释、普通字符串及对象成员名。Shift+F12 查找全部工作区引用，F2 安全重命名。显式导入别名仅在当前绑定内改名；导出名改名同步更新导入源名及未起别名的使用，保留显式别名。未保存内容优先，受影响代码不完整、名称冲突或文档版本变化时不会生成不安全编辑。

Image imports include BMP, WebP, GIF (first frame), ICO, PNM and TGA alongside PNG/JPEG/SVG and single-page unsigned 8/16-bit TIFF with alpha. Animated WebP uses the first frame. 16-bit imports and crops retain their intensity range without automatic contrast stretching; transparent pixels composite normally in preview and exports.

## Export images

Use **LayMesh: Export Figure** from the editor title/context menu or **Export** in the preview toolbar. Choose the format and destination, then export directly—there are no DPI or encoding option dialogs. Raster images default to **1200 DPI**, and TIFF defaults to **LZW lossless compression**. To change DPI, quality, compression, transparency or PDF image options, open VS Code Settings and search **LayMesh Export** (`laymesh.export.*`). Explicit settings and project/global configuration apply directly; old per-export dialog choices are no longer used. Export uses the current unsaved entry and opened imported buffers, including in remote windows.

在编辑器或预览工具栏点击 **导出**，选择格式和保存位置后直接导出，不再逐项询问 DPI、压缩或 PDF 参数。图片默认 **1200 DPI**，TIFF 默认 **LZW 无损压缩**。需要修改时，在 VS Code 设置中搜索 **LayMesh Export**，调整 `laymesh.export.*`（如 `laymesh.export.dpi`、`laymesh.export.tiffCompression`）。设置及项目/全局配置直接生效，旧导出对话框记忆值不再使用。PDF 保持矢量输出，DPI 仅用于内嵌图片的降采样上限。

The native stdio transport also accepts `type: "export"`, an absolute `output` path and an `options` object alongside the existing `file`, `source` and `overlays` fields. A success returns `exported`, `bytes`, `warnings` and `dependencies`; failures return the structured `error`. The export command accepts `(uri, {output, options})` for editor automation. This entry has the same trust checks and option validation as interactive export.

Native preview directly displays full-resolution JPEG 90 images; only nonopaque alpha uses lossy WebP with method 0. It reuses image/font resources across text refreshes and retains the current document’s reusable resources. Image encoding uses the available parallelism without per-cache quotas. Export defaults to 1200 DPI with independent PDF precision/transparency preservation. [Preview settings](https://muxkin.github.io/LayMesh/en/docs/topics/editors.html) · [Export configuration](https://muxkin.github.io/LayMesh/en/docs/topics/export.html).

Use **LayMesh: Set Preview Delay** in the Command Palette to enter any integer from 0 to 5000 ms (default 100). Set 0 to disable debounce. The command saves to the current workspace, or user settings if no workspace is open; changes apply immediately. You can also edit `laymesh.preview.debounceMs` in Settings.

Version 0.5.0 includes PPTX export as **PowerPoint (PPTX)** in the format picker. Text and supported shapes remain editable; complex effects use reported local PNG fallback. DPI controls fallback only (default 1200), and fonts are not embedded.

0.5.0 的格式列表支持 **PowerPoint (PPTX)**。文字和支持的形状可编辑，复杂效果局部回退为 PNG 并给出警告；DPI 仅控制回退图片（默认 1200），不嵌入字体。

打开 `.laydoc` 可预览独立 `.lay` 页面，通过页面选择器切换、打开页面源码、导出当前页或整本 PDF。编排语法与 Lay 相同，页面顺序由 `doc.add(page(...))` 决定。

Open `.laydoc` to preview independent `.lay` pages, select pages, open their sources, and export the current page or the entire PDF. Orchestration uses the same Lay DSL; `doc.add(page(...))` determines page order.

## Preview toolbar and FFmpeg / 预览工具栏与 FFmpeg

The preview toolbar provides Refresh, Fit/zoom, page navigation, joined Present/options, Export and More. Animated pages use three small footer buttons for previous step, play/pause and next step. Playback settings remain in More. There is no playback visibility toggle. Presentation keeps its existing hidden controls and Tab/Escape shortcuts. Bundled `@laymesh/animate.css`, Bootstrap and Open Props resolve inside the engine without fetching CSS.

工具栏提供刷新、适应/缩放、页面切换、放映及选项、导出、更多。动画页底部只保留上一步、开始/暂停、下一步三个小按钮；播放设置放在“更多”中，没有播放控制显示开关。

VS Code, CLI and Python share one FFmpeg manager and per-user host directory. An explicit `laymesh.ffmpeg.executable` overrides discovery. Otherwise the private copy is checked first, then the previously recorded external program, configured search paths, PATH and common installation locations. External programs are verified and recorded without being copied. If none works, a confirmation dialog precedes downloading; cancellation cleans staging files. Auto mode uses locale/timezone hints and bounded source probes to select the reachable faster upstream or China mirror, with automatic fallback and pinned SHA-256 checks. Set `laymesh.ffmpeg.downloadSource` or `mirrorBaseUrl` to override source selection. Remote extension hosts store FFmpeg on the remote host.

默认目录：Windows `%LOCALAPPDATA%/laymesh/ffmpeg`；Linux `${XDG_DATA_HOME:-~/.local/share}/laymesh/ffmpeg`；macOS `~/Library/Application Support/laymesh/ffmpeg`。仅缺少可用程序时请求下载确认，配置文件不能免除确认。下载的版本同时保留 LICENSE、README 和安装来源记录。
