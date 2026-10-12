# Draw and reveal extensions

[Animation manual](animation.en.md) · [Steps and actions](animation-timing.en.md)

## Built-in LayMesh keyframes

laymesh-draw/draw and laymesh-reveal/reveal belong to LayMesh rather than Animate.css. They do not require importing that library. Supply duration and an appropriate fill mode.

| Name | Behavior |
| --- | --- |
| laymesh-draw / draw | Progressively show a stroked path |
| laymesh-reveal / reveal | Expand a reveal region |

~~~lcss
.draw-motion { animation:laymesh-draw 1200ms ease both; }
.reveal-motion { animation:laymesh-reveal 1s ease both; }
~~~

Alternatively call s.animate(line_instance,name="laymesh-draw",duration=1s,fill="both"). The target must be placed; a filled shape does not become a stroked path.

## Charts and ordinary objects

Chart reveal clips the data area while retaining axes and legends. Ordinary objects reveal their layout box. This is not data interpolation and does not alter data, axis limits or static geometry.

<!-- example:examples/animation/timing.lay -->

## Custom progress

Nonstandard -laymesh-draw and -laymesh-reveal properties accept 0–1:

~~~lcss
@keyframes reveal-part {
  from { -laymesh-reveal:0; }
  to { -laymesh-reveal:1; }
}
~~~

Reference this definition with animation or name. Progress, duration, delay and fill participate in ordinary animation sampling and scheduling.
