> Historical experiment record. The mainline implementation and current acceptance boundaries are documented in `docs/topics/media.en.md` and `docs/topics/media.zh-CN.md`.

# 视频与音乐试验

分支：`codex/media-playback-experiment`。本地试验实现，尚未发布。

视频沿用图片的放置、裁剪及外观规则，音频作为独立音轨。页面、步骤和文档通过 `play` 指定启动规则：

```lay
p = canvas(size=(100mm,65mm))
v = p.add(video(src="demo.mp4"),size=(80mm,45mm),offset=(10mm,5mm))
p.play(v)
p.play(audio(src="music.wav"),loop=true,volume=0.2)
s = p.step(duration=3s,hold=0s)
```

`s.play(v)` 随步骤启动；`p.play(v,trigger="click")` 独立点击播放、暂停和继续。文档的 `d.play(audio(...),loop=true)` 在翻页时保持同一音轨。媒体使用独立时钟，等待下一步时继续播放；暂停、菜单及后台状态统一暂停。离开页面停止该页媒体，重新进入时重置。

所有放置的视频现在都有独立播放器，包括自动视频、尚未开始的步骤视频以及未绑定 `play` 的视频。控制条提供播放／暂停、片段进度、时间、静音和音量，操作停止 2 秒后隐藏，拖动或键盘焦点时保持显示。独立操作不启动背景音乐或其他视频、不推进步骤、不改变页面时钟；手动播放状态在本次页面访问中优先，缩放和全屏保留它，翻页重置。手动操作不改变源码导出排程，控件不进入导出。

`examples/media/controls.laydoc` 使用 12 秒合成视频演示四种启动方式；`manual.lay` 完全没有播放绑定。构建后可离线打开 `target/media-controls/controls.zip.html`。

完整参数与可运行示例见[中文手册](../../docs/topics/media.zh-CN.md)、[英文手册](../../docs/topics/media.en.md)及[示例](../../examples/media/README.md)。视频和音轨均为 FFmpeg 生成的合成素材。

## 已验证

验证环境为本机 Linux、原生 Rust/Skia CLI、真实 VS Code Webview，以及 Playwright 的 Chromium、Firefox、WebKit。VS Code 使用隔离配置和开发扩展，通过 CDP 用户手势操作按钮，没有改动用户的 VS Code 配置。

| 检查 | 结果 |
| --- | --- |
| 媒体绑定、片段裁剪、时长、参数诊断及多页共享 | 6 个核心媒体用例通过 |
| 既有动画与绘制回归 | 19 个动画用例；绘制列表、更新和 SVG 用例通过 |
| 完整动画导出 | 97 种效果的 MP4/GIF 解码、元数据、截图、取消及编码器失败清理通过 |
| 媒体 MP4 | 5 秒、150 帧，H.264 与 AAC；完整解码通过 |
| 实际音频内容 | 检测到视频原声；翻页后原声停止，文档音乐继续；点击导出的延迟、裁剪和结束时间通过 |
| VS Code | 视频解码、兼容 MP3 音轨、画布像素变化、暂停和恢复通过 |
| 三种浏览器 | 静态等待、播放、暂停、视频帧、独立媒体时钟、定位恢复、跨页音乐、点击视频及退出通过 |
| 独立视频控件 | Chromium、Firefox、WebKit 中四种启动方式、无绑定页面、片段定位／拖动、音量／静音、结束重播、2 秒隐藏和键盘焦点通过 |
| 控件位置和导航隔离 | 三种浏览器的缩放、滚动、采样变换、透明／离屏隐藏、键盘与滚轮隔离、菜单、翻页重置和窗口尺寸变化通过；Chromium 的触摸、窄窗口和全屏通过 |
| VS Code 独立控件 | Linux 真实宿主中独立播放、步骤前播放、无绑定页面、MP3 音轨定位／音量／静音、高 DPI 对齐、菜单暂停、窗口放映及退出恢复通过 |
| 离线可编辑 HTML | Chromium 中为第二页新增视频和音轨，WASM 编译后可播放 |
| Tailscale 放映服务 | 媒体 MIME、HEAD、字节范围及错误范围响应通过 |
| 静态导出 | PDF、SVG、PPTX、GIF 导出成功；PPTX 有字体未嵌入的常规提示 |
| 文档 | 新 API 与生成参考覆盖检查通过；233 份文档媒体、823 个文档页面通过校验 |

测试检查了音频解码和导出信号，未进行人工听感评审。控件截图与结果在 `/tmp/laymesh-media-controls`、`/tmp/laymesh-media-controls-vscode` 和 `/tmp/laymesh-media-controls-native-unbound`；可重新运行测试生成。原生检查通过 CDP 用户手势触发真实 webview 的控件，浏览器检查使用实际鼠标、键盘和触摸输入。浏览器插件的执行工具在此会话不可用，因此使用已有的 Playwright 浏览器安装。

## 复现

在本工作树根目录执行；构建目录独立于主项目：

```sh
python scripts/build-web-export.py
cargo build --locked -p laymesh-cli
cargo test --locked -p laymesh-core --test media
python scripts/test-media-export.py --binary target/debug/laymesh
target/debug/laymesh render target/media-trial/deck.laydoc -o target/media-trial/deck.zip.html
target/debug/laymesh render target/media-trial/click.lay -o target/media-trial/click.zip.html
target/debug/laymesh render target/media-trial/deck.laydoc -o target/media-trial/editable.zip.html --html-editable --html-embed-fonts true
node scripts/test-media-web.cjs
python scripts/build-editors.py --binary target/debug/laymesh --extension-dir target/media-vscode
python scripts/test-media-vscode.py
target/debug/laymesh render examples/media/controls.laydoc -o target/media-controls/controls.zip.html
target/debug/laymesh render examples/media/manual.lay -o target/media-controls/manual.zip.html
node scripts/test-media-controls.cjs
python scripts/test-media-vscode.py --controls --output /tmp/laymesh-media-controls-vscode
python scripts/test-media-vscode.py --controls --file examples/media/manual.lay --output /tmp/laymesh-media-controls-native-unbound
node scripts/test-media-host.cjs
python scripts/test-animation-export.py --binary target/debug/laymesh
python scripts/build-api-reference.py
python scripts/check-feature-docs.py
python scripts/build-docs-media.py --binary target/debug/laymesh --jobs 4
python scripts/build-docs-media.py --check
python scripts/build-docs.py
python scripts/build-docs.py --check
```

浏览器测试需安装 Playwright 及三种浏览器；可通过 `PLAYWRIGHT_MODULE` 指定现有模块路径。原生媒体测试需可用 FFmpeg；编码器查找复用项目共用管理器和配置。VS Code 检查使用 Linux 安装路径，可通过 `--code` 覆盖。

生成的 `target/media-trial/deck.zip.html` 可直接离线打开；`editable.zip.html` 包含源码编辑器；`deck.mp4` 是带混音的导出成品。

## 试验边界

首版以 H.264 MP4、MP3、WAV 为验证范围；原生宿主也能为 M4A 准备兼容音轨。VS Code 使用 MP3 音轨兼容 AAC 限制，普通浏览器使用视频原声。Windows 与 macOS 尚未实机验证；浏览器被阻止播放时需要点击开启声音。

点击视频导出 MP4 必须明确 `export_start`，否则保留封面并发出 `W_MEDIA_EXPORT`。PDF、SVG、静态图片和 PPTX 使用封面或片段首帧；PPTX 暂不嵌入可播放媒体。GIF 没有声音。循环页面媒体需要有限页面时长；背景音乐不延长文档。尚未测量长视频或大量并发视频的性能。
