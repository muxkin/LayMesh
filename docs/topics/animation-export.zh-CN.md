# 动画导出、兼容与排错

[动画手册](animation.zh-CN.md) · [放映与多页演示](animation-presentation.zh-CN.md)

## 视频与动态 GIF

共用管理器优先查找已有 FFmpeg，缺少时确认后下载。MP4 默认宽 1920、30fps；动态 GIF 默认宽 960、20fps。保持画面比例，MP4 尺寸向偶数对齐；多页按首个选中页比例居中留边。不包含音频。

~~~sh
laymesh animate examples/animation/presentation.laydoc -o presentation.mp4
laymesh animate examples/animation/first.lay -o entrance.gif
laymesh animate examples/animation/first.lay -o reduced.mp4 --reduced-motion reduce
laymesh render examples/animation/first.lay -o frame.png --time 0.4 --width 1920
laymesh render examples/animation/first.lay -o step.png --step 1
~~~

--time 与 --step 快照目前只支持 PNG。普通 SVG/PDF/PNG 导出显示完整静态排版，不应用动画 print 规则；PPTX 不包含原生动画。

## 编码设置与取消

CLI 可覆盖 width、fps、crf、preset、background、reduced-motion、media、page。项目 .laymesh.json 可以分别配置 MP4/GIF：

| CLI 参数 | 合法值与默认 | 作用 |
| --- | --- | --- |
| --width | 2–8192 的整数；MP4 1920、GIF 960 | 输出像素宽度，按比例计算高度 |
| --fps | 大于 0、至多 120；MP4 30、GIF 20 | 每秒帧数；不是布局 DPI |
| --crf | 0–51，默认 18 | MP4 的 H.264 质量参数；越小质量越高、文件通常越大 |
| --preset | 默认 medium | MP4 编码速度；支持 ultrafast/superfast/veryfast/faster/fast/medium/slow/slower/veryslow |
| --background | #RRGGBB，默认 #ffffff | 视频和动态 GIF 的透明区域及多页留边合成底色 |
| --reduced-motion | no-preference（默认）/ reduce | 选择动画媒体条件 |
| --media | screen（默认）/ print | 选择动画样式的媒体分支 |
| --page | 从 1 开始的页码 | 多页视频默认全本；指定后只导出该页；快照默认第 1 页 |
| --config | JSON 文件路径 | 选择项目配置 |
| --time | 秒数，例如 0.4 | render 导出指定时间的 PNG；不能与 --step 同用 |
| --step | 从 1 开始的步骤号 | render 导出该步骤动作窗口的终点，不含 hold |

显式 CLI 参数覆盖格式专用配置，格式配置覆盖通用动画配置，最后才使用内置默认值。MP4 使用 libx264/yuv420p，动态 GIF 使用调色板；crf 和 preset 只影响 MP4。普通图片的 dpi、quality、compression 等参数不适用于动画命令。PNG 时间快照保留透明度；不需要 FFmpeg。

~~~json
{"export":{"animation":{"crf":18,"reduced_motion":false},"animation_formats":{"mp4":{"width":1920,"fps":30},"gif":{"width":960,"fps":20}}}}
~~~

VS Code 只选择格式和路径：选“MP4 动画”或“GIF 动画”；普通 GIF 仍指静态图片。其余参数在 laymesh.export.animation.* 设置中修改。CLI 用 Ctrl+C 取消，VS Code 用进度通知取消；失败或取消清理临时文件并保留原目标文件。

视频采用含 hold 的连续时间线，按 --fps 采样；帧数向上取整，文件时长因此可能比计划时长多不到一帧。手动点击等待不进入视频。先用 inspect --json 查看窗口和时长，再用 --time 检查中间帧：

~~~sh
laymesh inspect examples/animation/timing.lay --json
laymesh render examples/animation/timing.lay -o timing-frame.png --time 1.2 --width 960
laymesh animate examples/animation/timing.lay -o timing.mp4 --width 960 --fps 30 --crf 18 --preset medium
~~~

多页视频中的每页都必须定义步骤，并具有正的总时长。静态页可以只创建停留步骤，不调用 animate：

~~~lay
page = canvas(size=(100mm,60mm))
page.add(text("A static page",font_size=5mm),offset=(12mm,20mm))
s = page.step(name="停留",duration=0s,hold=2s)
~~~

## 打印与减少动态效果

Animate.css 的 print 和 prefers-reduced-motion 分支带重要声明，普通时长覆盖不能压过它。预览遵循系统减少动态效果偏好，视频默认 screen/no-preference；需要减少动态效果的视频时显式选择 reduce。

兼容目标为固定 Animate.css 4.1.1 的 97 个效果，包含六个非平面效果。支持动画列表、负延迟、小数/无限次数、方向、填充、缓动、三维局部投影及 class 属性选择器。不支持任意网页布局、自动 transition、形状变形、数据插值或 PPTX 原生动画。

## 按现象排查

| 现象 | 原因与处理 |
| --- | --- |
| 点击放映没有动作 | 默认等待步骤点击；点击画面，或第一步设置 after-previous |
| 提示没有动画定义 | 显式导入 Animate.css，检查类名与 name |
| 只有效果类仍不动 | 添加 animate__animated 或显式非零 duration |
| 动画立即结束 | 检查 0s、减少动态效果、负 delay 或零次数 |
| 修改 2s 未生效 | 检查具体程度、重要声明、变量及显式 duration |
| 动画结束回到原位 | fill=none，或最后关键帧本就回到原处 |
| 退出效果结束又显示 | 采用 forwards/both；Animate.css 基础类默认 both |
| 静态预览仍显示退出对象 | 静态版面是完整排版；放映状态才应用动画 |
| 退出放映后恢复原位 | 预期行为，动画不修改静态几何 |
| 无限动作无法自动衔接 | 给独立 step 有限 duration，不在同一步引用无结束动作 |
| 时间定位后不继续播放 | 定位保持暂停，显式继续 |
| 导出失败 | 检查 FFmpeg、格式参数、输出位置和诊断 |

inspect --json 给出步骤名、起点、窗口、hold、目标及减少动态效果时的时长。W_ANIMATION 是动作诊断；none、0s 和媒体禁用合法。源代码错误时保留上次画面，不代表当前修改生效。

## 共用 FFmpeg 管理器

CLI、Python 与 VS Code 在同一宿主共享用户私有目录：Windows 为 `%LOCALAPPDATA%/laymesh/ffmpeg`，Linux 为 `${XDG_DATA_HOME:-~/.local/share}/laymesh/ffmpeg`，macOS 为 `~/Library/Application Support/laymesh/ffmpeg`。显式 executable 配置优先；否则依次检查私有程序、此前记录的外部程序、search_paths、PATH 和常见安装位置。外部程序只校验并记录位置，不复制、不修改。

~~~sh
laymesh ffmpeg status --json
laymesh ffmpeg install
# 非交互任务明确授权本次安装：
laymesh ffmpeg install --yes
~~~

只在没有可用程序时申请下载确认。终端按 y/yes 确认；非交互导出可以给 animate 传 --yes。Python 提供 ffmpeg_status、ensure_ffmpeg、animate_file、animate_source；非交互缺少程序会抛 FFmpegDownloadRequired，allow_download=True 仅授权本次调用。配置文件不能永久允许下载。

~~~json
{"export":{"ffmpeg":{"download_source":"auto","search_paths":[],"search_system":true}}}
~~~

可选 executable 与 directory 配置程序和共享目录；mirror_base_url 设置镜像根地址。VS Code 对应 laymesh.ffmpeg.* 设置。auto 模式参考语言/时区并测量上游与中国镜像可达性、响应速度，失败换源；固定 upstream/mirror 时只使用所选源。安装固定 ffmpeg-static b6.1.1 发行包，下载校验 SHA-256，保留许可证与安装来源。发行包标识用于下载和缓存目录；查找结果中的实际 FFmpeg 版本由各平台程序自行报告。支持 Windows x64、Linux x64/ARM64、macOS x64/ARM64；其他平台可配置已有程序。
