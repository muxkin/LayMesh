# Academic presentation fixture

`report.laydoc` contains 24 narrative slides on thermal diffusion and numerical
verification, followed by 8 animation comparison slides. It is a complete LAY
deck, not an imported PowerPoint file or a sequence of flattened screenshots.

The 32 pages contain 141 plots, 119 formulas, 1,213 explicitly placed objects and
all 97 Animate.css v4.1.1 effects. Plot internals add further paths and glyphs.
The main narrative mixes diagrams, meshes, numerical profiles, time histories,
convergence plots and dense small multiples. The appendices animate groups that
contain text, formulas, plots and local shadows; their effect names remain in
the source and manifest. Appendix actions use `fill="both"`: exit effects remain hidden at their
endpoint. Rewind or replay restores the corresponding earlier state. This is a compatibility workload, not advice to
use all effects in an academic talk.

All numeric values are analytical or computed teaching data, **not experimental
measurements**. The baseline is the unit-domain heat equation with zero boundary
values and a sine initial condition. Analytical profiles are evaluated directly;
the one-dimensional numerical profiles and errors use an explicit three-point
finite-difference update. The convergence experiment reduces `dt` with `h²` and
therefore measures coupled space/time refinement, not spatial order alone.
The irregular-domain drawing is explicitly schematic and is not the solver domain.

Reproduce sources and values with Python's standard library:

```sh
python tests/presentation/academic/generate.py
target/release/laymesh validate tests/presentation/academic/report.laydoc
target/release/laymesh render tests/presentation/academic/report.laydoc -o academic.pdf
```

`manifest.json` records source SHA-256 values, page inventory, effect coverage
and the computed convergence rows. Font selection is explicit: Noto Sans CJK SC
for prose and DejaVu Sans for numeric axes. Formula layout uses the existing math
fonts. No external experimental files or private research data are included.

The built preview is `/presentation-test.html?deck=academic`; `?deck=stress`
retains the older three-page stress workload. Open the academic test, wait for
whole-document preparation, then click Present. Preparation does not rehearse
animations. The page records preparation and actual page activation latency;
the five-round measurement button uses a fixed 3840×2160 buffer and separates
resident and nonresident transitions. Headless/software GPU measurements are
functional evidence only.

Manual mode advances on clicks, downward wheel gestures or navigation keys;
upward wheel gestures step back. Auto playback remains a player-menu setting.
The `.lay` files encode each step's action window and `hold`, not a document-level
automatic-playback flag. Actions in a step share a start; explicit `delay` can
schedule an action after another one, but is not a linked “after previous” trigger.

The `?deck=terminal` test demonstrates a persistent 320 px translation, a true
exit and a subsequent hold. It also distinguishes presentation state from the
unchanged static layout. Preparation uses at most four compiler workers (at
most two on devices reporting <=4 GiB memory); `?workers=1` provides a serial
comparison. Results include preparation phases, resource construction and
bounded first-play/replay histograms. Histogram bins are 1 ms, with a final
>200 ms bucket; maxima and the eight slowest requests retain exact values.
They describe submission completion, not monitor scan-out.


Progressive preparation exposes the first page before the full document is ready.
The report distinguishes `firstPageMs`, full `prepareMs`, and each transition's
`compiledAtInput`/`resident` state. Background compilation of future pages is
allowed while presenting; revisiting a compiled page must not compile it again.
The stream test deliberately blocks a future compile and verifies playback,
clock suspension and latest-target navigation. Native pool checks measure
five rounds and interrupt background compilation for current-page previews.
