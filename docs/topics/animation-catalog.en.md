# All 97 Animate.css effects

[Animation manual](animation.en.md) · [First animation](animation-start.en.md)

## Choose and play an effect

The catalog comes from the original distributed Animate.css 4.1.1 file. One shared preview changes editable Lay source and recompiles, samples and draws with LayMesh WASM. Browser CSS does not imitate the effects.

<!-- animation-catalog -->

## Using an effect

Import @laymesh/animate.css and combine its class with animate__animated in the action. Full source can be copied, edited or saved. Replay starts an exit effect from the visible object; Static restores the original layout.

animate__fast/faster/slow/slower change duration; animate__delay-1s through delay-5s set delays; animate__repeat-1/2/3 and animate__infinite set iteration counts. The library uses --animate-duration, --animate-delay and --animate-repeat. Explicit duration/iterations still win. This catalog explicitly uses 1s for comparison; remove that override to test speed helpers.

Descriptions identify the motion family. Exact movement, origin, easing and duration follow the pinned upstream definition. Two- and three-dimensional effects act on LayMesh instances within the [compatibility boundary](animation-export.en.md).

<!-- effects:start -->
## Emphasis

| Class | Effect |
| --- | --- |
| `animate__bounce` | Vertical bounce |
| `animate__flash` | Repeated flashing |
| `animate__pulse` | Gentle scale pulse |
| `animate__rubberBand` | Elastic stretch and recoil |
| `animate__shakeX` | Horizontal shake |
| `animate__shakeY` | Vertical shake |
| `animate__headShake` | Horizontal movement with a 3D head shake |
| `animate__swing` | Swing around the top edge |
| `animate__tada` | Scale and rotation emphasis |
| `animate__wobble` | Side-to-side wobble |
| `animate__jello` | Elastic skew |
| `animate__heartBeat` | Double heartbeat scaling |

## Back entrances

| Class | Effect |
| --- | --- |
| `animate__backInDown` | Back entrances; downward |
| `animate__backInLeft` | Back entrances; leftward |
| `animate__backInRight` | Back entrances; rightward |
| `animate__backInUp` | Back entrances; upward |

## Back exits

| Class | Effect |
| --- | --- |
| `animate__backOutDown` | Back exits; downward |
| `animate__backOutLeft` | Back exits; leftward |
| `animate__backOutRight` | Back exits; rightward |
| `animate__backOutUp` | Back exits; upward |

## Bouncing entrances

| Class | Effect |
| --- | --- |
| `animate__bounceIn` | Bouncing entrances |
| `animate__bounceInDown` | Bouncing entrances; downward |
| `animate__bounceInLeft` | Bouncing entrances; leftward |
| `animate__bounceInRight` | Bouncing entrances; rightward |
| `animate__bounceInUp` | Bouncing entrances; upward |

## Bouncing exits

| Class | Effect |
| --- | --- |
| `animate__bounceOut` | Bouncing exits |
| `animate__bounceOutDown` | Bouncing exits; downward |
| `animate__bounceOutLeft` | Bouncing exits; leftward |
| `animate__bounceOutRight` | Bouncing exits; rightward |
| `animate__bounceOutUp` | Bouncing exits; upward |

## Fading entrances

| Class | Effect |
| --- | --- |
| `animate__fadeIn` | Fading entrances |
| `animate__fadeInDown` | Fading entrances; downward |
| `animate__fadeInDownBig` | Fading entrances; downward, large displacement |
| `animate__fadeInLeft` | Fading entrances; leftward |
| `animate__fadeInLeftBig` | Fading entrances; leftward, large displacement |
| `animate__fadeInRight` | Fading entrances; rightward |
| `animate__fadeInRightBig` | Fading entrances; rightward, large displacement |
| `animate__fadeInUp` | Fading entrances; upward |
| `animate__fadeInUpBig` | Fading entrances; upward, large displacement |
| `animate__fadeInTopLeft` | Fading entrances; top left |
| `animate__fadeInTopRight` | Fading entrances; top right |
| `animate__fadeInBottomLeft` | Fading entrances; bottom left |
| `animate__fadeInBottomRight` | Fading entrances; bottom right |

## Fading exits

| Class | Effect |
| --- | --- |
| `animate__fadeOut` | Fading exits |
| `animate__fadeOutDown` | Fading exits; downward |
| `animate__fadeOutDownBig` | Fading exits; downward, large displacement |
| `animate__fadeOutLeft` | Fading exits; leftward |
| `animate__fadeOutLeftBig` | Fading exits; leftward, large displacement |
| `animate__fadeOutRight` | Fading exits; rightward |
| `animate__fadeOutRightBig` | Fading exits; rightward, large displacement |
| `animate__fadeOutUp` | Fading exits; upward |
| `animate__fadeOutUpBig` | Fading exits; upward, large displacement |
| `animate__fadeOutTopLeft` | Fading exits; top left |
| `animate__fadeOutTopRight` | Fading exits; top right |
| `animate__fadeOutBottomRight` | Fading exits; bottom right |
| `animate__fadeOutBottomLeft` | Fading exits; bottom left |

## Flips

| Class | Effect |
| --- | --- |
| `animate__flip` | Perspective 3D flip |
| `animate__flipInX` | Flips; around the x axis |
| `animate__flipInY` | Flips; around the y axis |
| `animate__flipOutX` | Flips; around the x axis |
| `animate__flipOutY` | Flips; around the y axis |

## Lightspeed

| Class | Effect |
| --- | --- |
| `animate__lightSpeedInRight` | Lightspeed; rightward |
| `animate__lightSpeedInLeft` | Lightspeed; leftward |
| `animate__lightSpeedOutRight` | Lightspeed; rightward |
| `animate__lightSpeedOutLeft` | Lightspeed; leftward |

## Rotating entrances

| Class | Effect |
| --- | --- |
| `animate__rotateIn` | Rotating entrances |
| `animate__rotateInDownLeft` | Rotating entrances |
| `animate__rotateInDownRight` | Rotating entrances |
| `animate__rotateInUpLeft` | Rotating entrances |
| `animate__rotateInUpRight` | Rotating entrances |

## Rotating exits

| Class | Effect |
| --- | --- |
| `animate__rotateOut` | Rotating exits |
| `animate__rotateOutDownLeft` | Rotating exits |
| `animate__rotateOutDownRight` | Rotating exits |
| `animate__rotateOutUpLeft` | Rotating exits |
| `animate__rotateOutUpRight` | Rotating exits |

## Specials

| Class | Effect |
| --- | --- |
| `animate__hinge` | Swing on a hinge, then drop |
| `animate__jackInTheBox` | Scale and rotate into view |
| `animate__rollIn` | Roll into view |
| `animate__rollOut` | Roll out of view |

## Zooming entrances

| Class | Effect |
| --- | --- |
| `animate__zoomIn` | Zooming entrances |
| `animate__zoomInDown` | Zooming entrances; downward |
| `animate__zoomInLeft` | Zooming entrances; leftward |
| `animate__zoomInRight` | Zooming entrances; rightward |
| `animate__zoomInUp` | Zooming entrances; upward |

## Zooming exits

| Class | Effect |
| --- | --- |
| `animate__zoomOut` | Zooming exits |
| `animate__zoomOutDown` | Zooming exits; downward |
| `animate__zoomOutLeft` | Zooming exits; leftward |
| `animate__zoomOutRight` | Zooming exits; rightward |
| `animate__zoomOutUp` | Zooming exits; upward |

## Sliding entrances

| Class | Effect |
| --- | --- |
| `animate__slideInDown` | Sliding entrances; downward |
| `animate__slideInLeft` | Sliding entrances; leftward |
| `animate__slideInRight` | Sliding entrances; rightward |
| `animate__slideInUp` | Sliding entrances; upward |

## Sliding exits

| Class | Effect |
| --- | --- |
| `animate__slideOutDown` | Sliding exits; downward |
| `animate__slideOutLeft` | Sliding exits; leftward |
| `animate__slideOutRight` | Sliding exits; rightward |
| `animate__slideOutUp` | Sliding exits; upward |

<!-- effects:end -->
