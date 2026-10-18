# 视频与音频

视频像图片一样用 `add` 放置，音频没有版面几何。`play` 绑定播放规则；只定义或放置素材不会自动开始播放。

## video

```lay
p = canvas(size=(100mm,60mm))
v = p.add(video(src="demo.mp4"),size=(80mm,45mm),offset=(10mm,5mm))
p.play(v)
```

`p.play(v)` 在页面播放开始时启动；`s.play(v)` 在步骤 `s` 开始时启动；`p.play(v,trigger="click")` 点击视频启动，再点击暂停或继续。每个视频都可以独立操作，包括尚未开始的步骤视频和未绑定 `play` 的视频。

VS Code 预览、窗口／全屏放映及离线 HTML 共用视频控件：播放／暂停、片段进度、当前时间／总时长、静音和音量。鼠标移入、移动或触摸时显示，停止操作 2 秒后隐藏；拖动或键盘焦点在控件内时保持显示。暂停或结束时显示中央播放按钮，结束后从片段起点重播。小尺寸视频的音量滑块从声音按钮处展开。

独立操作只影响当前视频和原声，不启动其他音轨，不推进步骤，也不改变页面时钟。拖动结束后恢复拖动前的播放状态。手动操作后，本次页面访问的自动启动规则不再覆盖该视频的播放状态；翻页后重置。全局暂停、菜单及后台暂停仍统一管理媒体。控件随缩放、平移和动画变换定位，隐藏或离屏的视频不显示控件。

## audio

```lay
s = p.step(duration=3s,hold=0s)
s.play(audio(src="narration.mp3"),delay=200ms,volume=0.8)
```

## play

`.laydoc` 的背景音乐跨页连续播放；页内视频、配音或音效离开页面就停止。

```lay
d = document(name="演示")
d.play(audio(src="music.wav"),loop=true,volume=0.2)
d.add(page(src="first.lay"))
d.add(page(src="second.lay"))
```

`video` 接受 `src` 和可选的 `poster`，沿用图片的尺寸、fit、crop 和外观参数。`audio` 接受 `src`。所有路径相对于素材定义文件；每次 `add` 得到独立视频实例。

`play` 接受 `delay`、`trim_start`、`trim_end`、`loop`、`volume`、`muted`。时间使用 `s`／`ms`，音量为 0–1；默认不延迟、不循环、不静音，使用原音量和整个源文件。裁剪时间使用源文件的绝对时间。每个视频实例只能绑定一种启动规则。

等待下一步时，已启动的媒体继续播放。暂停、菜单及后台状态统一暂停；继续播放或恢复后再运行。普通预览首次打开保持完整静态排版。播放结束保留视频末帧；离开页面后重新进入，页内媒体重置。背景音乐不会随翻页重置。

## 导出

MP4 包含视频画面、视频原声、页面音轨和背景音乐。时间定位 PNG 使用相应视频帧。PDF、SVG 和静态图片使用封面或片段首帧。PPTX 内嵌可播放视频、音轨与封面，详见下文。动画 GIF 可以包含视频画面，没有声音。

手动播放、定位、音量和静音仅作用于当前播放会话；导出继续使用源码中的播放排程与参数，视频控件不进入导出画面。未绑定 `play` 的视频在 MP4 中保留封面。

点击视频需要单独的导出开始时间：

```lay
p.play(v,trigger="click",export_start=2s,trim_start=500ms,trim_end=3s)
```

`export_start` 为视频实际开始播放的页面时间，不再叠加 `delay`。未指定时，MP4 保留封面并提示 `W_MEDIA_EXPORT`。

有限媒体可以自动确定页面时长，或延长已有步骤的导出窗口。循环媒体需要已有步骤或其他有限媒体提供正的页面时长；背景音乐不延长文档。导出仍只选择格式和路径，其余编码参数沿用设置。

离线 HTML／ZIP 随作品携带媒体、封面和兼容音轨。浏览器阻止播放时显示“点击开启声音”。首版支持 H.264 MP4、MP3 和 WAV；原生宿主还可为 M4A 准备兼容音轨，其他容器明确诊断。视频准备使用共用 FFmpeg 管理器，可先运行 `laymesh ffmpeg install`。

可运行示例位于 `examples/media`。这些文件使用合成测试画面及测试音。

## PowerPoint 媒体

```sh
laymesh render examples/media/powerpoint.laydoc -o presentation.pptx
laymesh render examples/media/powerpoint.laydoc -o current.pptx --page 1
laymesh render examples/media/powerpoint.laydoc -o page.pptx --all-pages
```

`.laydoc` 默认打包为一份完整 PPTX，所有页面必须使用相同画布尺寸；错误会指出具体页码并保留原目标文件。`--page` 选择单页，`--all-pages` 继续生成编号的单页文件。Python `render_document(...,output="presentation.pptx")` 默认导出完整文档。VS Code 仍只选择格式和路径；“当前页”入口继续只导出当前页。离线 HTML 可编辑版与含媒体的放映版也支持多页 PPTX。

视频、音轨与封面全部内嵌；同一资源跨对象和页面去重，搬到另一台电脑无需原素材。文字和普通图形保持可编辑。页内媒体映射为原生自动／点击播放，步骤绑定映射为播放启动序列。支持延迟、片段裁剪、循环、音量及静音；未绑定的视频保留原生手动播放。`export_start` 仅控制确定时间的动画导出，不改变 PPTX 点击行为。文档音乐只在首张幻灯片创建，跨页播放；页内媒体离页停止。

视频的位置、尺寸、矩形裁剪、常规旋转和镜像保留。无法映射的滤镜、透明度、剪切或包含视频的分组特效会报告 `W_PPTX_MEDIA`，保留可播放视频。普通对象动画和页面转场仍按现有静态 PPTX 方式导出。声音图标放在页面外，可在 PowerPoint 的选择窗格中选中。

验收基准为 **Windows Microsoft 365 PowerPoint**。当前 Linux 检查已覆盖结构、资源及排程，并用 Microsoft Open XML SDK 校验；Windows 实际播放、无修复提示和保存后重开仍待验证。未承诺 macOS PowerPoint、WPS 或 LibreOffice 媒体兼容性。

Windows 验收：将 `powerpoint.pptx` 单独复制到没有原素材的电脑；打开应无修复提示；放映第一页，确认自动视频延迟 0.5 秒、1–4 秒片段循环和音量，右上视频点击播放且静音、裁剪与旋转正确，左下视频手动播放，按下一步启动右下视频。翻到第二页确认前页视频停止、背景音乐不中断；保存到新文件并重开重复播放检查。完成后记录 Windows、PowerPoint 版本和各项结果。

实现依据：[Open XML 音频结构](https://learn.microsoft.com/en-us/office/open-xml/presentation/how-to-add-an-audio-to-a-slide-in-a-presentation)、[PowerPoint 跨页音乐](https://support.microsoft.com/en-us/powerpoint/play-music-across-multiple-slides-in-your-slide-show-in-powerpoint)、[媒体裁剪时间](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-pptx/c835eea3-fd6f-4055-8a57-92b193b7b2e3)。
