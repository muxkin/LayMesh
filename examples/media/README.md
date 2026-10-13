# 视频和音频试验

这些素材由 FFmpeg 的 testsrc2 和 sine 生成，视频为 3 秒 H.264/AAC，音频为合成测试音。用于验证播放和导出，不是正式配乐。

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

FFmpeg 复用 LayMesh 的共用管理器。首版保证 H.264 MP4、MP3、WAV；M4A 在原生宿主中准备兼容音轨。其他容器暂时明确诊断。
