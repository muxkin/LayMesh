# Presentation and multipage playback

[Animation manual](animation.en.md) · [Export and troubleshooting](animation-export.en.md)

## Preview and presentation

Default preview shows the full static layout. Preview controls play, pause, replay, seek and restore Static. The toolbar shows one Present button, which starts fullscreen directly. Its dropdown offers Windowed presentation. Escape or leaving browser fullscreen ends the presentation and returns directly to the regular preview. VS Code offers embedded and browser presentation.

Click the regular preview to start animation; it shares presentation context menus, arrow keys, Space and wheel navigation. On initial preview display, even a first step with start="after-previous" retains the complete static layout until a click starts entry. Subsequent after-previous links, delays, holds and automatic page transitions run normally. Dragging pans without advancing.

Presentation defaults to manual advancement, while a first after-previous step runs automatically on formal entry. For manual steps, first advance executes the first step; another advance during playback completes it and holds; the next advance executes the next step. Pages advance after all steps. after-previous links run automatically; cross-page Auto is a menu setting. Pages without animation advance directly, with a default 1s automatic hold.

## Controls and shortcuts

| Input | Behavior |
| --- | --- |
| Click, Space, Enter, Right/Down, PageDown, N | Advance or complete current step |
| Left/Up, PageUp, Backspace, P | Previous step boundary; previous page from page start |
| Wheel down/up | Advance/back; continuous gesture is combined |
| Digits then Enter | Jump to page |
| Home / End | First initial state / last completed state |
| S | Pause/resume |
| F | Fullscreen / exit fullscreen |
| B / W | Black/white screen; repeat to restore |
| Tab | Show controls and focus buttons |
| Right click | Presentation menu |
| Esc | Close menu; end formal presentation and return to preview; restore Static in preview |

The menu exposes Auto, speed, time, pages and control visibility. Controls can always show, show when the pointer reaches their area, or stay hidden; Tab remains available. Menus pause the clock and restore its previous state on closing. Backgrounding pauses; resume manually.

Wheel navigation applies only inside the presentation canvas; menus retain scrolling. Horizontal/modified scrolling and keyboard auto-repeat do not repeatedly advance.

## Multiple pages

.laydoc uses the Lay DSL to sequence independent .lay pages, each with its own size, resources, units, styles and steps.

<!-- example:examples/animation/presentation.laydoc -->

See [Documents](documents.en.md) for page references, stable IDs and export selection. Auto advances after windows and holds; the last completed page stays visible.

## Preparation, failure and exit

The first ready page can present while later pages prepare. Jumping to an unready page retains the current image and pauses until its first frame is ready. Failure reports page and source location with retry/cancel.

Presentation fixes the current document version; edits apply after exit. Exit restores original static geometry, selected page, zoom, scroll and focus. Screens fit at the original aspect ratio without stretching or cropping.

VS Code browser presentation uses a temporary session service bound to Tailscale; closing the session releases it. Choose embedded presentation for local-only use. Font, GPU, cache and measurement details belong in [Implementation and verification](animation-internals.en.md).

[Page transitions](page-transitions.en.md) provides 125 configurable effects, including page curl.
