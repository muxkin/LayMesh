# 视频、音频与 PowerPoint 媒体

这些素材由 FFmpeg 的 testsrc2 和 sine 生成，`demo.mp4` 为 3 秒、`controls.mp4` 为 12 秒 H.264/AAC，音频为合成测试音。用于验证播放和导出，不是正式配乐。

```sh
laymesh render examples/media/deck.laydoc -o deck.zip.html
laymesh animate examples/media/deck.laydoc -o deck.mp4
laymesh render examples/media/first.lay -o frame.png --time 0.5 --width 640
```

- `first.lay`：页面视频、视频原声及页内音轨。
- `second.lay`：验证跨页背景音乐继续。
- `deck.laydoc`：整份演示的背景音乐。
- `click.lay`：点击播放与独立的 MP4 导出时间。
- `step.lay`：步骤启动视频与音效，包括延迟和片段裁剪。
- `controls.lay`／`controls.laydoc`：四个视频分别采用自动、点击、步骤和无绑定启动，验证独立控件及跨页背景音乐。
- `manual.lay`：完全没有 `play` 绑定的页面，视频仍可通过控件手动播放。

`laymesh render examples/media/controls.laydoc -o video-controls.zip.html` 生成带资源的离线试用文件。单独点击视频不会启动背景音乐；鼠标移动时显示控制条，停止操作 2 秒后隐藏。

FFmpeg 复用 LayMesh 的共用管理器。首版保证 H.264 MP4、MP3、WAV；M4A 在原生宿主中准备兼容音轨。其他容器暂时明确诊断。

## Mainline PowerPoint acceptance fixture

`laymesh render examples/media/powerpoint.laydoc -o presentation.pptx`

This equal-size two-slide deck covers native automatic/click/manual/step video,
trim, delay, loop, volume/mute, crop/rotation and continuous background music.
`controls.laydoc` intentionally has different page sizes and exercises PPTX size
rejection. Run `python scripts/test-media-pptx.py --binary target/debug/laymesh`
and `dotnet run --project scripts/openxml-validator -- presentation.pptx` for
package checks. Windows Microsoft 365 playback and save/reopen are pending; use
the checklist in `docs/topics/media.en.md` or `docs/topics/media.zh-CN.md`.

Linux browser QA uses `scripts/test-media-web.cjs` and
`scripts/test-media-controls.cjs`. Set `PLAYWRIGHT_MODULE` to the installed
Playwright module and `PULSE_SERVER` to a test audio server when needed.
For WebKit use `MEDIA_BROWSERS=webkit MEDIA_HEADED=1` with a virtual X display:
the Linux headless WPE backend in Playwright 1.62.1 returned transparent video
pixels here, while GTK rendered actual frames. The playback test checks decoded
pixel alpha as well as playback time. This does not establish macOS Safari or
Windows PowerPoint compatibility.
