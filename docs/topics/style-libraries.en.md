# Style and animation libraries: import, explore, compose

## Start here

Run an example first, then explore entries and adjust parameters. Libraries ship with the engine; none is enabled automatically. Import the ones you use explicitly. A separate `.lcss` file is optional: one `.lay` file can combine appearance and motion.

## What the three libraries provide

| Library | Pinned version / MIT | Upstream and official docs | Bundled import | Contents |
| --- | --- | --- | --- | --- |
| Bootstrap | 5.3.8 | [Project](https://github.com/twbs/bootstrap) · [Docs](https://getbootstrap.com/docs/5.3/) | `@laymesh/bootstrap.css` | Appearance utilities and component styles; LayMesh applies supported appearance |
| Open Props | 1.7.23 | [Project](https://github.com/argyleink/open-props) · [Docs](https://open-props.style/) | `@laymesh/open-props.css` | Color, gradient, radius, shadow, font, easing and animation variables |
| Animate.css | 4.1.1 | [Project](https://github.com/animate-css/animate.css) · [Docs](https://animate.style/) | `@laymesh/animate.css` | Base class, 97 effects, speed/delay/repeat utilities and keyframes |

The [resource manifest](../../crates/laymesh-core/assets/styles/manifest.json) pins original CSS, versions, MIT licenses and SHA-256. [License notices](../../release/THIRD_PARTY_NOTICES.md) are retained. `@laymesh/` names an engine resource; it is not a directory you need to create.

## Understand five essential names

| Name | Location | Purpose |
| --- | --- | --- |
| `stylesheet` | `canvas()` argument | Import one file or an ordered file list |
| `class` | Material, instance or `animate()` argument | Reference space-separated class names, without selector dots |
| `style { ... }` | `.lay` source | Inline CSS declarations; no separate theme required |
| `var(--name)` | Style property value | Consume an imported or authored variable |
| `@keyframes` | Inline style or stylesheet | Define custom motion, scheduled by `animate()` |

Open Props variables need property consumers. Bootstrap appearance utilities can be applied directly. Animate.css effect classes work with the `animate__animated` base class.

## Run each library first

[Bootstrap guide and catalog](style-bootstrap.en.md) · [Open Props guide and catalog](style-open-props.en.md) · [Animate.css guide and catalog](style-animate-library.en.md)

Each complete source below imports only one library.

<!-- example:examples/libraries/bootstrap.lay -->

<!-- example:examples/libraries/open-props.lay -->

<!-- example:examples/libraries/animate.lay -->

## Bootstrap with motion: no authored CSS

<!-- example:examples/libraries/bootstrap-animate.lay -->

`bg-primary rounded shadow` supplies appearance; `animate__animated animate__fadeInUp` supplies motion. `after-previous` starts on presentation entry. Omit it to wait for a click.

## Open Props with motion: a small inline style block

<!-- example:examples/libraries/open-props-animate.lay -->

`--gradient-7`, `--radius-3` and `--shadow-3` paint the object; Animate.css controls its entrance. The project still uses a single `.lay` file.

## Combine all three and override one field

<!-- example:examples/libraries/all-three.lay -->

Change only Bootstrap shadow blur to `2mm`, preserving other fields. Importance and selector specificity match the library declaration. Loops retain instances and compute delay; anchors position cards and labels.

## Next: parameters, reuse and third-party CSS

1. [Selectors and cascade](style-cascade.en.md): explicit Lay parameters win; importance, specificity and order compare CSS declarations.
2. [Properties and units](style-properties.en.md), [shadows](style-shadows.en.md): change colors, corners and blur before relative units and expressions.
3. [Action parameters](animation-timing.en.md): duration, delay, iterations and scheduling.
4. [Shared themes and third-party CSS](style-third-party.en.md): extract a theme when files share appearance.
5. [Complete projects](style-animation-examples.en.md): progress to artistic text and the complex dashboard.

## Versions and compatibility

This guide describes current source builds; published packages follow their release notes. Unmodified upstream CSS loads, with appearance limited to LayMesh support. Dimensions, placement and data use Lay and anchors. Unsupported content is summarized; see [compatibility](style-compatibility.en.md).
