# Video and audio

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

Time PNG snapshots use video frames. PDF, SVG and still images use a supplied poster or the first trimmed frame. PPTX embeds playable media and posters. Animated GIF includes frames without audio.

Manual playback, seeking, mute and volume changes belong to the current playback session. Exports retain source schedules and settings and never include the controls. Unbound videos remain posters in MP4.

Offline HTML/ZIP carries the referenced media and compatible audio. Blocked browser playback offers an enable-media button. The first implementation supports H.264 MP4, MP3 and WAV; native hosts also prepare compatible M4A audio. Other containers are diagnosed. Video preparation uses the shared FFmpeg manager: `laymesh ffmpeg install`.

See `examples/media` for runnable examples with synthetic test video and tones.

## PowerPoint media

`laymesh render examples/media/powerpoint.laydoc -o presentation.pptx` produces one complete deck. All pages must have the same canvas dimensions; errors identify the offending page and preserve the existing destination. `--page N` selects one slide; `--all-pages` retains numbered single-slide files. Python `render_document(..., output="presentation.pptx")` defaults to the complete document. VS Code still asks only for format and destination; its current-page entry stays single-page. Editable HTML and media-bearing playback HTML also export complete PPTX decks.

Videos, soundtracks and posters are embedded and deduplicated across objects and pages. Moving the PPTX requires no original assets. Text and ordinary shapes remain editable. Page media uses native automatic/click playback, and step bindings create a media start sequence. Delay, trimming, loop, volume and mute are retained. Unbound videos remain manually playable. `export_start` only schedules deterministic animation exports; it does not change PPTX click behavior. Document music starts on the first slide and continues across slides; page media stops on leaving its slide. Audio icons sit outside the slide and remain accessible in the Selection Pane.

Video placement, size, rectangular crop, ordinary rotation and mirroring are mapped. Unsupported filters, opacity, shear and group effects emit `W_PPTX_MEDIA` while keeping the video playable. Ordinary object animations and page transitions retain the existing static PPTX behavior.

The acceptance target is **Windows Microsoft 365 PowerPoint**. Linux checks cover structure, assets and scheduling, including Microsoft Open XML SDK validation. Actual Windows playback, opening without a repair prompt and save/reopen behavior remain pending. No media compatibility commitment is made for macOS PowerPoint, WPS or LibreOffice.

Windows checklist: copy only the exported `powerpoint.pptx` to a machine without the source assets. Open without repair, then check the first slide's automatic video (0.5 s delay, 1–4 s loop and volume), the upper-right click video (mute, crop and rotation), lower-left manual video and lower-right next-step video. Advance to slide two: previous videos must stop and background music continue. Save to a new file, reopen and repeat. Record Windows and PowerPoint versions and each result.

References: [Open XML audio](https://learn.microsoft.com/en-us/office/open-xml/presentation/how-to-add-an-audio-to-a-slide-in-a-presentation), [music across slides](https://support.microsoft.com/en-us/powerpoint/play-music-across-multiple-slides-in-your-slide-show-in-powerpoint), [media trim times](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-pptx/c835eea3-fd6f-4055-8a57-92b193b7b2e3).
