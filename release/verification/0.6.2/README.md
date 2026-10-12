# 0.6.2 本地交付验证

本次 Linux 本地安装包要求 glibc ≥ 2.39、GLIBCXX ≥ 3.4.26；Windows 包只依赖系统 DLL。

工具栏移除内部“预览”文字和播放控制显示开关，加入刷新、缩放菜单、页面箭头与合并放映按钮。动画页底部只有上一步、开始/暂停、下一步；高级设置在“更多”中。

旧 0.6.1 Linux VSIX 内的引擎对截图源码报 E_ASSET；同一源码在新 0.6.2 中成功渲染。Windows 新 EXE 通过 Wine 验证了反斜线、中文/空格路径、内置 Animate.css 和预览/LSP。Windows 原生 VS Code 界面尚未验证。

FFmpeg 在同一用户、同一宿主共享私有目录。当前 Linux 实际目录与已复用的系统程序见 ffmpeg-default-status.json；其 directory 为 /home/keqing/.local/share/laymesh/ffmpeg，found.path 为 /usr/bin/ffmpeg。正常安装路径不使用 /tmp。Windows 为 %LOCALAPPDATA%/laymesh/ffmpeg，macOS 为 ~/Library/Application Support/laymesh/ffmpeg。

下载确认截图 ffmpeg-confirmation.png 使用显式覆盖的测试隔离目录 /tmp/laymesh-preview-*/ffmpeg-private，防止测试污染正式缓存；它不代表默认安装位置。GUI 回归在一次性 VS Code 配置中启用 custom 对话框，以测试真实确认和取消，无替换提示接口。

- FFmpeg 原生单元测试 7 项：定位优先级、确认边界、完整性、镜像失败后切换、并发只安装一次、取消清理。单元测试使用本地服务器与可验证的测试程序。发布准备另行以 zh-CN 语言提示从 npmmirror 真实下载，校验固定 SHA-256、LICENSE、README 后实际导出 MP4；见 real-ffmpeg-download.json。下载发行包为 ffmpeg-static b6.1.1，Linux 程序实际报告 FFmpeg 7.0.2，界面与手册明确区分发行包和程序版本。
- Python：实际导出 MP4、GIF，ffprobe 检查尺寸/时间；确认缺失和跨入口路径一致性；CLI 协议及静态导出回归。
- 网站：Art Direction 在 3840×1840、1440×1000、390×844 下的实际截图；首次/刷新/延迟 CSS、菜单边界、隐藏焦点、Tab/Escape、手动/自动播放、窗口/全屏退出。
- 完整放映：默认隐藏、三种显示方式、快捷键、全屏、重复打开/退出、主线程回退与取消初始化。
- 文档：745 页面和 220 媒体来源的构建指纹与图片校验。
- 两个 VSIX 均为 0.6.2 stable，平台及许可证检查通过；本地交付包，未发布到 Marketplace。公开发行包以 CI 审核结果为准；发布状态见 publication-verification.json。

FFmpeg 下载源与 SHA-256 元数据来自 [ffmpeg-static b6.1.1](https://github.com/eugeneware/ffmpeg-static/releases/tag/b6.1.1)，镜像根地址沿用 [上游 README](https://github.com/eugeneware/ffmpeg-static#custom-binaries-url)。
