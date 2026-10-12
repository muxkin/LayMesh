# Custom keyframes and CSS animation

[Animation manual](animation.en.md) · [Effect catalog](animation-catalog.en.md)

## Definition, selection and scheduling

@keyframes defines changing properties, animation declarations select the definition and playback parameters, and animate() schedules it on an instance. Defining keyframes alone does not start playback.

<!-- example:examples/animation/keyframes.lay -->

Edit keyframes.lcss in Full source. from equals 0%, to equals 100%, with intermediate percentage stops. Opacity goes from 0 to 1 and the card moves 35mm right. A middle frame setting only opacity does not discard transform.

## Shorthand and longhands

~~~lcss
.motion { animation:move 2s ease-in-out 200ms 2 alternate both running; }
~~~

The shorthand includes name, duration, easing, delay, count, direction, fill and state. First time is duration, second is delay. Commas define multiple animations. Redeclaring animation resets corresponding longhands; longhand lists match names using CSS list rules.

| Property | Initial value and meaning |
| --- | --- |
| animation-name | none; keyframe names |
| animation-duration | 0s; nonnegative duration per iteration |
| animation-delay | 0s; negatives start partway through |
| animation-timing-function | ease; progress between frames |
| animation-iteration-count | 1; zero, fractions and infinite accepted |
| animation-direction | normal; iteration direction |
| animation-fill-mode | none; values outside active interval |
| animation-play-state | running; paused freezes the action |

Important declarations inside keyframes are ignored. Missing endpoints use base properties; duplicate stops merge per property. Later animations in a list override earlier ones on shared properties; different properties can animate together.

## Properties and transforms

Animate opacity, transform, transform-origin, visibility, backface-visibility and [LayMesh extensions](animation-extensions.en.md). Arbitrary color animation, shape morphing and data interpolation are not provided.

| Functions | Purpose |
| --- | --- |
| translate, translateX/Y/Z, translate3d | Movement |
| scale, scaleX/Y/Z, scale3d | Scaling |
| rotate, rotateX/Y/Z, rotate3d | Rotation |
| skew, skewX/Y | Skew |
| matrix, matrix3d | Direct matrices |
| perspective | Perspective |

Order matters. Translation/origin percentages use the untransformed box; px uses layout_dpi. transform-origin defaults to center. hidden back faces can disappear during flips. Local 3D projection does not introduce browser 3D scene layout.

Animated opacity replaces the object's opacity while ancestor opacity still multiplies. Animated transforms do not change static anchors or reposition other objects.

## Easing

linear is constant-speed; ease/ease-in/ease-out/ease-in-out adjust acceleration. cubic-bezier(x1,y1,x2,y2) defines progress with x controls in 0–1. steps(n,jump-start|jump-end|jump-none|jump-both) jumps in discrete intervals; start/end are common aliases. CSS steps() is not a Lay presentation step.

A keyframe's animation-timing-function applies from that stop to the next; the example uses ease-out after 50%.

## Iterations, direction and fill

normal plays forward, reverse backward, alternate switches each iteration, and alternate-reverse starts backward. Fractional counts stop partway through the last iteration; infinite actions require a finite step window.

| Fill | Outside the active interval |
| --- | --- |
| none | No animated value retained |
| backwards | Starting value during delay |
| forwards | Final value after ending |
| both | Both behaviors |

Animate.css base sets both, so exits normally stay invisible. none restores base values. Movement stays only if its endpoint and fill request it; pulse naturally returns because its endpoints match.

The first backwards/both action can establish an entrance state. Future inactive actions do not seize shared properties. Seeking resamples the state; leaving presentation restores the original full static layout.

## Complete styled animation projects

[Library cards and artistic poster](style-animation-examples.en.md) combine themes, layout, keyframes and steps, with complete source for the existing complex dashboard.
