# Animation export, compatibility and troubleshooting

[Animation manual](animation.en.md) · [Presentation](animation-presentation.en.md)

## Video and animated GIF

The shared manager locates existing FFmpeg and requests consent to install it when missing. MP4 defaults to width 1920 at 30fps; animated GIF to width 960 at 20fps. Aspect ratio is preserved, MP4 dimensions align to even pixels, and multipage output centers pages within the first selected page's ratio. No audio is included.

~~~sh
laymesh animate examples/animation/presentation.laydoc -o presentation.mp4
laymesh animate examples/animation/first.lay -o entrance.gif
laymesh animate examples/animation/first.lay -o reduced.mp4 --reduced-motion reduce
laymesh render examples/animation/first.lay -o frame.png --time 0.4 --width 1920
laymesh render examples/animation/first.lay -o step.png --step 1
~~~

Time/step snapshots currently support PNG only. Ordinary SVG/PDF/PNG exports show complete static layout and do not apply print animation rules. PPTX does not contain native animations.

## Settings and cancellation

CLI overrides include width, fps, crf, preset, background, reduced-motion, media and page. Project .laymesh.json can configure formats separately:

| CLI option | Accepted values and defaults | Purpose |
| --- | --- | --- |
| --width | Integer 2–8192; MP4 1920, GIF 960 | Pixel width; height follows page aspect ratio |
| --fps | Greater than 0, at most 120; MP4 30, GIF 20 | Frames per second, independent of layout DPI |
| --crf | 0–51; default 18 | MP4 H.264 quality; lower values give higher quality and usually larger files |
| --preset | Default medium | MP4 encoding speed: ultrafast/superfast/veryfast/faster/fast/medium/slow/slower/veryslow |
| --background | #RRGGBB; default #ffffff | Video/GIF background for transparency and multipage margins |
| --reduced-motion | no-preference (default) / reduce | Animation media preference |
| --media | screen (default) / print | Animation stylesheet media branch |
| --page | Page number from 1 | Videos default to all pages; selects one page. Snapshots default to page 1 |
| --config | JSON file path | Project configuration |
| --time | Seconds, such as 0.4 | render a PNG at that time; mutually exclusive with --step |
| --step | Step number from 1 | render at the end of the step's action window, excluding hold |

Explicit CLI values override format-specific configuration, which overrides common animation configuration and then built-in defaults. MP4 uses libx264/yuv420p; animated GIF uses a palette. crf and preset affect MP4 only. Static-image dpi, quality and compression options do not apply to animation commands. PNG time snapshots retain alpha and require no FFmpeg.

~~~json
{"export":{"animation":{"crf":18,"reduced_motion":false},"animation_formats":{"mp4":{"width":1920,"fps":30},"gif":{"width":960,"fps":20}}}}
~~~

VS Code asks only for format and destination: choose MP4 Animation or GIF Animation; ordinary GIF remains static. Other options belong in laymesh.export.animation.* settings. Cancel CLI with Ctrl+C or VS Code through progress. Failures/cancellation remove temporary files and preserve an existing destination.

Video samples the continuous timeline, including hold, at --fps. Frame counts round up, so file duration may exceed planned duration by less than one frame. Manual click waits do not enter the video. Inspect the planned windows and duration, then check an interior frame:

~~~sh
laymesh inspect examples/animation/timing.lay --json
laymesh render examples/animation/timing.lay -o timing-frame.png --time 1.2 --width 960
laymesh animate examples/animation/timing.lay -o timing.mp4 --width 960 --fps 30 --crf 18 --preset medium
~~~

Every page in a multipage video needs steps and a positive total duration. A static page can define only a hold step, without animate:

~~~lay
page = canvas(size=(100mm,60mm))
page.add(text("A static page",font_size=5mm),offset=(12mm,20mm))
s = page.step(name="Hold",duration=0s,hold=2s)
~~~

## Print and reduced motion

Animate.css print and prefers-reduced-motion branches use important declarations. Ordinary duration overrides cannot beat them. Preview follows system preference; video defaults to screen/no-preference. Select reduce explicitly for reduced-motion video.

The compatibility target is the pinned Animate.css 4.1.1 set of 97 effects, including six nonplanar effects. Support includes animation lists, negative delays, fractional/infinite iterations, direction, fill, easing, local 3D projection and class attribute selectors. Browser layout, automatic transitions, shape morphing, data interpolation and PPTX native animation are not provided.

## Troubleshooting

| Symptom | Cause and resolution |
| --- | --- |
| Presentation does not start moving | Default waits for click; advance or set the first step after-previous |
| Missing definition warning | Import Animate.css and check class/name |
| Effect class alone does not move | Add animate__animated or explicit nonzero duration |
| Animation ends immediately | Check 0s, reduced motion, negative delay and zero iterations |
| 2s override ineffective | Check specificity, importance, variables and explicit duration |
| Object returns after animation | fill=none or an endpoint that returns to its original position |
| Exit becomes visible again | Use forwards/both; Animate.css base already sets both |
| Static preview shows an exited object | Static preview is complete layout; animation applies during playback |
| Leaving presentation restores position | Expected: animation does not edit static geometry |
| Infinite action cannot be linked | Use finite duration on its own step; avoid same-step links to endless actions |
| Seeking does not resume | Seeking pauses; resume explicitly |
| Export fails | Check FFmpeg, settings, destination and diagnostics |

inspect --json reports names, starts, windows, holds, targets and reduced-motion duration. W_ANIMATION diagnoses actions; none, 0s and media disabling are valid. A retained image after a source error is the previous result.

## Shared FFmpeg manager

CLI, Python and VS Code share a per-user directory on the same host: `%LOCALAPPDATA%/laymesh/ffmpeg` on Windows, `${XDG_DATA_HOME:-~/.local/share}/laymesh/ffmpeg` on Linux, and `~/Library/Application Support/laymesh/ffmpeg` on macOS. An explicit executable overrides discovery; otherwise private binaries, recorded external programs, configured search paths, PATH and common installations are checked in order. External programs are verified and recorded without copying or modification.

~~~sh
laymesh ffmpeg status --json
laymesh ffmpeg install
laymesh ffmpeg install --yes  # Explicit authorization for this invocation.
~~~

Only missing programs trigger confirmation. Noninteractive animation requires --yes; Python provides ffmpeg_status, ensure_ffmpeg, animate_file and animate_source, raises FFmpegDownloadRequired without interactive consent, and accepts allow_download=True for one call. Configuration never permanently authorizes downloads.

~~~json
{"export":{"ffmpeg":{"download_source":"auto","search_paths":[],"search_system":true}}}
~~~

Optional executable and directory select the program and shared directory; mirror_base_url overrides the mirror root. Corresponding VS Code settings are laymesh.ffmpeg.*. Auto selection uses locale/timezone hints and bounded reachability/latency probes, falling back between upstream and China mirror. Forced upstream/mirror uses only that source. Installation pins the ffmpeg-static b6.1.1 distribution, verifies SHA-256, and retains LICENSE, README and provenance. The distribution identifier names the download/cache bundle; each executable reports its own FFmpeg version during discovery. Supported download platforms are Windows x64, Linux x64/ARM64 and macOS x64/ARM64; other platforms can configure an existing program.
