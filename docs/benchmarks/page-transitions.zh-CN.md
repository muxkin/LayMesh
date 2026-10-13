# 字体与页间转场验证记录

测试日期：2026-10-13。实现固定 GL Transitions 1.74.0 / CanvasKit 0.42.0。

## 字体性能

当前方案已取消本机字体名称扫描，以内置同名字体字典支持中文搜索，仅在作品实际需要或用户主动加载时读取字体文件。

2026-10-13，Headless Chromium，真实字体管理器与选择器 DOM，模拟 FontData 目录，80 轮搜索与选择：

| 目录 | 连接及目录呈现 | 搜索 P95 | 选择 P95 | 无关字体读取 | 必需字体读取 |
| --- | --- | --- | --- | --- | --- |
| 1,005 | 7.3 ms | 0.4 ms | 0.1 ms | 0 | 1 |
| 5,000 | 16.2 ms | 0.5 ms | 0.1 ms | 0 | 1 |

必需字体读取、名称解析与界面更新合计 9.0／18.3 ms；重新连接未重复读取已加载字体。以上使用模拟枚举和内存 Blob，**没有测量 Windows 的系统枚举或真实 FontData.blob 延迟**。搜索／选择达到 P95 < 50ms；没有全量后台索引等待。

此前记录的 433／2,067 ms 是反复读取内存测试字体命名表的时间，未包含真实 FontData.blob 获取成本，不能代表用户电脑上的索引速度；历史结果保留供对照。新数据见 [font-dictionary-performance.json](page-transitions/font-dictionary-performance.json)。`test-web-fonts-viewport.cjs` 的 Chromium、Firefox、WebKit 回归已覆盖中文搜索、坏字体、替代应用、重连和导出；连接与目录操作现在额外断言零无关字体读取。三浏览器另用真实 DejaVu Sans 文件验证导入同款字体后自动编译，未配置替代关系；见 [font-regressions.json](page-transitions/font-regressions.json)。中英文微软雅黑匹配与 UI／地区区分通过模拟目录验证；尚未在两台实体 Windows 机器上验证授权和枚举。

## 效果与导航

125 / 125 效果完成 CanvasKit CPU 测试：128×72、72×128、透明 96×96、修改参数，以及需要额外纹理的两个效果；逐项检查起止帧、5 个中间采样、反向端点和快照释放。中间帧总览见 [midpoints.png](../../site/media/page-transitions-midpoints.png)，逐项记录见 [compatibility.json](page-transitions/compatibility.json)。原生 GLSL 与固定 SkSL 在同一 Chromium SwiftShader WebGL2 上比较 0.1／0.25／0.5／0.75／0.9 五个时刻，125 / 125 无编译错误；最大平均通道误差 5.193 / 255，来自随机噪声效果 TVStatic。非随机效果的误差明显更低；阈值为 8 / 255。见 [glsl-reference.json](page-transitions/glsl-reference.json)。没有发现需要从当前可用目录剔除的转换视觉失败项。

浏览器离线 HTML 测试覆盖首次显示、前后翻页、跳页、连续操作、反向额外纹理、反复导航内存、减少动态效果、窗口／全屏放映、尺寸变化、退出、离线零网络请求及播放零编译。记录见 [navigation.json](page-transitions/navigation.json)。原有 `test-web-export.cjs` 三引擎回归通过；网站 `test-preview-presentation.cjs`、`test-page-transitions-site.cjs`（额外纹理懒加载、反向播放、同页刷新）和真实 VS Code `test-vscode-preview.py --page-transitions` 通过。后者直接断言文档页切换产生 InvertedPageCurl 帧，且无控制台错误。网站转场记录见 [site-preview.json](page-transitions/site-preview.json)。自动播放在三个引擎中均完成连续跨页，验证转场期间时间线冻结、标题动画从零开始；见 [auto-playback.json](page-transitions/auto-playback.json)。

额外状态测试覆盖取消、同页／首次显示、减少动态效果、模拟着色器运行失败直切、上下文丢失和销毁。Rust 样式、文档、动画、显示列表与增量更新回归通过；内置样式库补全与 LCSS 参数测试通过。225 项文档媒体已重新生成并检查。

## 文件管理与编辑交互

资源树固定使用 Headless Tree core 1.7.0；菜单固定使用 jQuery-contextMenu 2.10.2 与私有 jQuery 3.7.1。上游文件、版本、SHA-256 和许可证随本地包携带。

`test-workspace-interactions.cjs` 在 Chromium、Firefox、WebKit 全部通过，覆盖嵌套／空目录、未保存文本、行内重命名、引用更新、多选拖动、同名冲突、复制／删除、失败回滚、文件操作撤销／重做、ZIP 空目录、导出后重新打开。测试还覆盖只读菜单、当前入口删除保护、剪贴板拒绝、键盘菜单、弹窗输入、窗口／全屏放映、播放版右键，以及颜色块 12×12px、键盘调色与撤销。记录见 [workspace-interactions.json](page-transitions/workspace-interactions.json)。原有 `test-web-workspace.cjs` 三浏览器回归通过，覆盖连续导入与导出等待；同一次导入中的文件操作保持原子性。

文件引用通过语言 AST 与 CSS 解析器定位；条件赋值、循环和动态表达式展示待修复引用，需选择取消或仅移动。`test-workspace-files.mjs` 与 Rust `file_references` 测试覆盖导入、页面、样式、图片、纹理与字体路径，以及资源选择器后缀。这里的事务只修改当前离线作品，不修改外部源文件。

## laydoc 配置

`test-laydoc-transitions.cjs` 通过真实原生编译池与网站 Worker 验证：同一个源文件分别配置文档默认 `cube`、页面覆盖 `fade`、`none` 与默认复用，结果为 `cube / fade / none / cube`，实际编译 3 次。页面 LCSS 中缺失的纹理不会在上层配置覆盖后被读取。见 [laydoc-overrides.json](page-transitions/laydoc-overrides.json)。Rust 测试覆盖默认时长与缓动、参数诊断、LCSS 回退及相对声明文件的纹理路径；实际 VS Code 预览／放映验证结果见 [vscode-laydoc.json](page-transitions/vscode-laydoc.json)。

## 性能边界

本机未验证真实硬件 GPU。Chromium 使用 SwiftShader；Firefox 使用 CanvasKit CPU；WebKit 报告 WebGL2，但硬件归属未确认。软件 CPU 路径仅降低中间帧分辨率至最长边 128 像素，首尾页使用原始完整快照；这减少了 Firefox 长时间阻塞，但仍不等同于流畅 GPU 播放。当前短转场测试中 Firefox 放映的 180 ms 请求实际约 1,047 ms / 2 帧，WebKit 约 196 ms / 14 帧，Chromium SwiftShader 约 216 ms / 3 帧。不能据此承诺真实设备帧率。

额外纹理使用位图资源；SVG 纹理明确报错。所有效果、许可证、资源都在本地包中，播放不依赖 CDN。无转场配置的旧文档继续直切；静态导出不播放转场。

## 复现

```sh
python scripts/build-transitions.py
python scripts/build-web-export.py
cargo build --release -p laymesh-cli --locked
node scripts/test-page-transition-shaders.mjs
node scripts/test-page-transition-lifecycle.mjs
node scripts/test-page-transition-reference.cjs
node scripts/test-page-transitions.cjs
node scripts/test-page-transition-auto.cjs
node scripts/test-font-catalog-performance.cjs
node scripts/test-web-fonts-viewport.cjs
node scripts/test-workspace-files.mjs
node scripts/test-workspace-interactions.cjs
node scripts/test-web-workspace.cjs
node scripts/test-laydoc-transitions.cjs /tmp/laydoc-check http://100.64.0.6:8786/
```

浏览器测试使用 `PLAYWRIGHT_MODULE` 指定已安装的 Playwright；测试服务默认绑定 Tailscale `100.64.0.6`，可通过 `LAYMESH_TEST_HOST` 指定本机 Tailscale 地址。改前性能对照可通过 `LAYMESH_FONT_BASELINE` 指向改动前的 `font-names.mjs`、`font-picker.mjs` 和 `web-fonts.mjs` 快照目录。
