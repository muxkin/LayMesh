# Video and audio (experimental)

## video

Place `video(src="demo.mp4",poster="optional.png")` with `add`, using the image size, fit, crop and appearance rules. Define nonvisual tracks with `audio(src="music.wav")`. Paths are relative to the material definition.

`canvas.play(video_instance)` starts on page playback; `step.play(video_instance)` starts with that step. `canvas.play(video_instance,trigger="click")` toggles that video without advancing the slide. A step-bound click video becomes available when that step starts. Each placed video has one binding; reused materials produce independent instances.

## audio

`canvas.play(audio(...))` and `step.play(audio(...))` provide page tracks. In a `.laydoc`, `document.play(audio(...),loop=true,volume=0.2)` supplies continuous background music.

## play

Playback options are `delay`, `trim_start`, `trim_end`, `loop`, `volume` and `muted`. Times require s/ms units; volume is 0–1. Defaults are no delay, no looping, full volume, unmuted and the full source interval. Trimming uses absolute source times.

Started media continues while waiting for the next step. Explicit pause, menus and background visibility pause media. Leaving a page stops its tracks; returning resets them. Document music continues across pages. Static preview waits for explicit playback.

MP4 combines video, original audio, page tracks and background music. Click videos require an explicit page-local `export_start`; it is the actual start, without adding `delay`. Missing schedules produce a poster and `W_MEDIA_EXPORT`. Finite clips can establish or extend page duration; looping page clips require a finite duration from steps or other media. Music does not extend the document.

Time PNG snapshots use video frames. PDF, SVG, still images and PPTX use a supplied poster or the first trimmed frame. PPTX does not embed playable media. Animated GIF includes frames without audio.

Offline HTML/ZIP carries the referenced media and compatible audio. Blocked browser playback offers an enable-media button. The experiment guarantees H.264 MP4, MP3 and WAV; native hosts also prepare compatible M4A audio. Other containers are diagnosed. Video preparation uses the shared FFmpeg manager: `laymesh ffmpeg install`.

See `examples/media` for runnable examples with synthetic test video and tones.
