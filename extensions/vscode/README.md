# LayMesh

Preview and edit scientific figures in `.lay` and `.lcss`. Get live SVG previews, unit-aware rulers, canvas and plot coordinate readouts, completion, parameter hover, signature help, definitions, workspace references, occurrence highlights, safe rename, diagnostics and color editing. English and Simplified Chinese interfaces are included.

支持 `.lay` 与 `.lcss`：实时 SVG 预览、单位标尺、鼠标画布和绘图坐标、代码补全、参数说明、诊断与选色器，界面支持中文和英文。

Marketplace packages are provided for **Windows x64** and **Linux x64**, including matching remote extension hosts. Preview uses a bundled native Rust program; no Python, Rust toolchain or npm installation is needed by extension users. macOS, Windows ARM64 and browser-only VS Code require their own compatible packages. Language analysis is static and never executes figure code; preview rendering is a separate opt-in process in a trusted workspace.

Build from the repository root with `python scripts/build-editors.py`. Create a stable Marketplace package with `python scripts/package-editor.py --target linux-x64 --output release/dist/laymesh-language-0.3.6-linux-x64.vsix`, then audit it with `python scripts/check-editor.py release/dist/laymesh-language-0.3.6-linux-x64.vsix --binary target/release/laymesh --target linux-x64 --marketplace`. No npm packages are needed. To install locally, use VS Code's **Extensions: Install from VSIX** command.

Build Windows x64 on Windows with `cargo build --release --locked -p laymesh-cli`, then stage `target/release/laymesh.exe`. A Linux build host with MinGW-w64 can instead run `rustup target add x86_64-pc-windows-gnu` and `cargo build --release --locked -p laymesh-cli --target x86_64-pc-windows-gnu`. Package it in a separate directory to preserve the Linux development binary:

```sh
python scripts/build-editors.py --binary target/x86_64-pc-windows-gnu/release/laymesh.exe --extension-dir release/staging/vscode-win32-x64
python scripts/package-editor.py --extension-dir release/staging/vscode-win32-x64 --target win32-x64 --output release/dist/laymesh-language-0.3.6-win32-x64.vsix
python scripts/check-editor.py release/dist/laymesh-language-0.3.6-win32-x64.vsix --binary target/x86_64-pc-windows-gnu/release/laymesh.exe --target win32-x64 --marketplace
```

Run `python scripts/smoke-native-editor.py --binary target/release/laymesh.exe --output release/dist/windows-native-evidence.json` on Windows to verify the preview protocol, resources, units, paths, error recovery and bilingual language service. On Linux, `--wine /path/to/wine` runs the cross-built EXE through Wine. This protocol check is separate from testing the actual Windows VS Code interface.

Upload each platform VSIX to the same Marketplace extension using the same publisher, extension name, version and stable channel. To add Windows to an existing Linux release, choose **Update** on that extension and upload the Windows package; VS Code selects the package for its extension host. See [platform-specific publishing](https://code.visualstudio.com/api/working-with-extensions/publishing-extension#platform-specific-extensions).

Windows x64 和 Linux x64 均提供独立安装包，内置原生语言服务与预览程序，用户无需安装 Python、Rust 或 npm。发布 Windows 包时，在已有的 `Hyacine.laymesh-language` 扩展中选择 **Update** 并上传 Windows VSIX，沿用相同版本和正式版通道。远程开发时，安装包平台应匹配远程扩展宿主。

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

## Help language and other LSP clients

`laymesh.language` accepts `auto` (default), `zh-CN`, or `en`. In VS Code, `auto` follows the editor display language. It controls preview toolbars, coordinate labels, status, accessibility labels, process messages, newly opened color editors, and language help. Changing it updates an open preview immediately while preserving the image and view, without compiling. Command titles and Settings descriptions follow VS Code's display language through bundled English and Simplified Chinese translations. Compiler diagnostic details and user-authored figure text retain their original language. The server never reads the host OS language to choose documentation.

For an English preview in a Chinese editor, add `"laymesh.language": "en"` to Settings. Use `"zh-CN"` for Chinese, or `"auto"` to follow VS Code.

Other IDEs can launch `laymesh lsp --stdio` from the repository root. Send `initialize.locale` for UI language or `initializationOptions: {"language":"zh-CN"}` for a startup preference. Clients supporting `workspace/configuration` return the `laymesh` section; others can push `workspace/didChangeConfiguration` with `{"settings":{"laymesh":{"language":"en"}}}`. A valid configuration setting wins over initialization options; `auto` uses the initial client locale. Missing or unsupported UI languages default to English. See the [LSP setup guide](https://muxkin.github.io/LayMesh/en/docs/topics/editors.html#lsp-integration-in-other-ides).

Completion, hover and signatures negotiate Markdown or plain text per client capability. Signature parameter labels use ranges only when the client advertises support.

Library comments may use `## @lang zh-CN` and `## @lang en` sections. Missing fields fall back to untagged text, English, then the first authored version. Types, names, default values and inserted code are not translated. Existing untagged comments remain valid. See [multilingual documentation](https://muxkin.github.io/LayMesh/en/docs/topics/library-docs.html#multilingual-documentation).

提示、预览器和新打开的选色器默认跟随界面语言；设置 `"laymesh.language": "en"` 可使用英文，`"zh-CN"` 使用中文，`"auto"` 跟随 VS Code。切换语言时已打开的预览立即更新，保留图像和视图，不触发编译。命令名称和设置说明跟随 VS Code 显示语言；编译器的具体错误和用户图形文字保留原文。其他 IDE 使用相同的标准 LSP 服务，通过初始化选项或配置通知设置语言。库注释支持 `@lang` 分段，缺少译文时回退原文；详见[中文编辑器指南](https://muxkin.github.io/LayMesh/docs/topics/editors.zh-CN.html)。


## Live figure preview / 实时图形预览

Open a saved `.lay` file and click the preview icon in the editor title, use **LayMesh: Open Preview** (**LayMesh：打开预览** in Chinese), or choose it from the editor context menu. Each entry has its own preview beside the editor; reopening reuses its panel. Preview runs in a separate native process and requires a trusted workspace. Local files in remote extension hosts work in the same way; untitled and virtual files must first be saved to the host filesystem.

Edits refresh after 100 ms, including unsaved imported modules, stylesheets and text data. Changes to loaded local images, fonts and data also refresh the figure. A compilation error retains the last successful figure, marks it stale, and offers a source-location button. Language help continues independently.

The top and left rulers and canvas X/Y values follow `canvas(unit=...)`: `mm`, `cm`, `in`/`inch`, `pt`, or `px`. Pixels use `layout_dpi`, independently of raster export DPI. The origin is the canvas top left; X increases right and Y increases down. Explicit units mixed in the source do not change the preview's display unit.

Move the mouse over the figure to see canvas coordinates. Inside a plot, the bottom bar also displays every named data axis, including log, symlog, reversed and broken axes; a break gap shows `—` for that axis. Rotated/nested plots and insets retain their own coordinates. Polar plots display θ/r in their configured angle units; the center has no unique angle. Radar charts display a dimension value only within six screen pixels of its spoke, and only canvas coordinates at the center.

Toolbar: **Refresh**, **Fit**, **100%**, zoom out/in, **Rulers**, and **Export**. Use Ctrl/Command + wheel to zoom about the pointer, Space + left drag or middle drag to pan, and ordinary wheel/trackpad scrolling to pan. With the preview focused, `+`/`-` zoom and `0` fits. 100% maps one layout pixel to one CSS pixel using `layout_dpi`; it does not calibrate physical monitor dimensions. View state survives tab hiding, and source updates preserve the view.

```json
{
  "laymesh.preview.debounceMs": 100,
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

Use **LayMesh: Export Figure** from the editor title/context menu or **Export** in the preview toolbar. Choose SVG/PDF or PNG/JPEG/TIFF/WebP/BMP/GIF/ICO/PNM/TGA, then resolution and the options relevant to that format. JPEG supports quality and matte color, TIFF supports none/LZW/Deflate/PackBits, PNG supports fast/default/best, and WebP supports lossy/lossless, quality, method, alpha quality and near-lossless fidelity. Defaults are under `laymesh.export.*`; options are remembered per workspace and format. The current unsaved entry and opened imported buffers are exported by the bundled engine, including in remote windows. See [all export options and alpha limits](https://muxkin.github.io/LayMesh/en/docs/topics/export.html).

The native stdio transport also accepts `type: "export"`, an absolute `output` path and an `options` object alongside the existing `file`, `source` and `overlays` fields. A success returns `exported`, `bytes`, `warnings` and `dependencies`; failures return the structured `error`. The export command accepts `(uri, {output, options})` for editor automation. This entry has the same trust checks and option validation as interactive export.

Native preview directly displays full-resolution JPEG 90 images; only nonopaque alpha uses lossy WebP with method 0. It reuses image/font resources across text refreshes and provides configurable parallel encoding and memory budgets. Export defaults to 1200 DPI with independent PDF precision/transparency preservation. [Preview settings](https://muxkin.github.io/LayMesh/en/docs/topics/editors.html) · [Export configuration](https://muxkin.github.io/LayMesh/en/docs/topics/export.html).

Use **LayMesh: Set Preview Delay** in the Command Palette to enter any integer from 0 to 5000 ms (default 100). Set 0 to disable debounce. The command saves to the current workspace, or user settings if no workspace is open; changes apply immediately. You can also edit `laymesh.preview.debounceMs` in Settings.
