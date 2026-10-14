# Video and audio (experimental)

## video

Place `video(src="demo.mp4",poster="optional.png")` with `add`, using the image size, fit, crop and appearance rules. Define nonvisual tracks with `audio(src="music.wav")`. Paths are relative to the material definition.

`canvas.play(video_instance)` starts on page playback; `step.play(video_instance)` starts with that step. `canvas.play(video_instance,trigger="click")` toggles that video without advancing the slide. All placed videos support independent controls, including videos before their step starts and videos without a `play` binding. Each placed video has at most one binding; reused materials produce independent instances.

VS Code preview, windowed/fullscreen presentation and offline HTML share play/pause, clip-relative seeking, elapsed/total time, mute and volume controls. Pointer movement or touch reveals the bar; it hides after two idle seconds. Dragging or keyboard focus keeps it visible. Paused and ended videos retain a central play button; replay starts at the clip beginning. Small videos expand the volume slider beside the sound button.

Independent actions affect only that video and its original audio. They do not start other tracks, advance steps or change the page clock. Seeking restores the previous playback state on release. Manual transport overrides automatic starts for that page visit; page navigation resets page videos. Global pause, menus and background pause still manage all media. Controls follow zoom, pan and sampled scene transforms and disappear for hidden or offscreen videos.

## audio

`canvas.play(audio(...))` and `step.play(audio(...))` provide page tracks. In a `.laydoc`, `document.play(audio(...),loop=true,volume=0.2)` supplies continuous background music.

## play

Playback options are `delay`, `trim_start`, `trim_end`, `loop`, `volume` and `muted`. Times require s/ms units; volume is 0–1. Defaults are no delay, no looping, full volume, unmuted and the full source interval. Trimming uses absolute source times.

Started media continues while waiting for the next step. Explicit pause, menus and background visibility pause media. Leaving a page stops its tracks; returning resets them. Document music continues across pages. Static preview waits for explicit playback.

MP4 combines video, original audio, page tracks and background music. Click videos require an explicit page-local `export_start`; it is the actual start, without adding `delay`. Missing schedules produce a poster and `W_MEDIA_EXPORT`. Finite clips can establish or extend page duration; looping page clips require a finite duration from steps or other media. Music does not extend the document.

Time PNG snapshots use video frames. PDF, SVG, still images and PPTX use a supplied poster or the first trimmed frame. PPTX does not embed playable media. Animated GIF includes frames without audio.

Manual playback, seeking, mute and volume changes belong to the current playback session. Exports retain source schedules and settings and never include the controls. Unbound videos remain posters in MP4.

Offline HTML/ZIP carries the referenced media and compatible audio. Blocked browser playback offers an enable-media button. The experiment guarantees H.264 MP4, MP3 and WAV; native hosts also prepare compatible M4A audio. Other containers are diagnosed. Video preparation uses the shared FFmpeg manager: `laymesh ffmpeg install`.

See `examples/media` for runnable examples with synthetic test video and tones.
