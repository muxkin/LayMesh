# Assertion and rendering verification

The Node baseline `78db22d` has 183 tests and 891 statically identified assertion call sites. This is not a count of dynamic assertions. The maps in `assertion-maps/` associate every call site with a source-qualified Rust/Python check, retaining parameter loops and helper assertions. Compilation replay and successful export are reported separately.

| Mapping | Original assertion sites | Main checks |
| --- | ---: | --- |
| `core.json` | 151 | Language, units, styles, instances, anchors, groups, arrows and fusion |
| `scientific.json` | 75 | Markers, layer order, scientific labels, colorbars, bounds and physical offsets |
| `plot_basic.json` | 129 | Axes, ticks, data, layouts, legends, inspect and CLI exports |
| `plot.json` | 201 | Statistics, histograms, ECDF, named/broken axes, labels, polar/radar fields |
| `render.json` | 195 | Fonts, shaping, formulae, images, opacity, SVG/PNG/PDF and LCSS |
| `integration.json` | 140 | Language service, localization and native LSP |
| Total | **891** | All sites mapped; final execution evidence is in `delivery.json` |

`python scripts/test-contracts.py --offline` first runs the acceptance-gate fault tests, then the workspace and all Python tests. Cargo JSON binds each test executable to its actual source file; Python records each loaded module file. Source, map, corpus, example and fixture hashes must match the completed logs. Missing fields, missing tests, truncated results, skipped Python tests or stale source cannot pass. Notebook dependencies must be installed for the acceptance run. The default-ignored nearest-boundary microbenchmark is separately executed; it is not a skipped functional contract.

## Rendering contracts

- Fixed fonts live only under test fixtures. Body font paths, ordered fallback, inherited spans, grapheme misses, TTC/OTC face selection, fsType embedding restrictions, final shaped overflow and source-located warnings have explicit checks. Empty-font scenes export vector boxes in all three formats. These checks are not a Unicode conformance-suite claim.
- RaTeX uses native KaTeX metrics. Formula glyph checks compare every path command, endpoint and control point to the library DisplayList; line rectangles use the center coordinate minus half thickness. Original LaTeX and selectable body text are extracted from actual PDFs. Required Poppler/MuPDF tools fail the acceptance run when unavailable.
- PNG physical size, alpha and pHYs/DPI are checked. ICC RGB/gray/CMYK/YCCK conversion and alpha preservation are checked with reference profiles and independent PNG/PDF samples. SVG/PDF formula output is vector; `audit-render-assets.py` checks actual embedded fonts and PDF image objects.
- Legend samples directly verify line and marker combination, hollow marker paint, error direction, hatch strokes, clipping, opacity, sample size and default/explicit physical spacing. Axis text color inheritance and explicit override are separate contracts.
- The eight shared cases in `tests/fixtures/contour-contracts.json` cover saddle topology, holes, equal thresholds, missing cells, nonuniform mapping, polar periodicity and translucent bands. Native and browser WASM consume the same fixture and inspect geometry plus rendered pixels.
- Retained curve recipes are re-evaluated after nested scaling and rotation. Explicit curves, compound caps, fused holes, polar ink/grid/clip and hatch endpoints have targeted final-physical-error checks at 0.001 mm. These numerical checks are not a formal proof over all floating-point inputs.

## Fault injection and evidence boundaries

`test-core-mutants.py` changes seven actual implementation branches in an isolated source copy. The unmodified test must pass; the modified implementation must compile and fail the named test. A compile error does not count as detecting a regression.

Additional checks mutate actual Scene/export outputs to recreate 17 plotting regressions and five rendering regressions, then apply the same validator used for the real output. Browser language tests reject 11 output mutations; the shared contour tests reject missing geometry and changed opacity. Physical-tolerance tests bypass retained recipes and require the former large error to be detected. These are explicitly output/recipe mutations, not claims of production-code mutation testing. Invalid input and error-recovery tests are counted separately.

The strengthened 217-plot comparator fails missing contractual fields and mismatched geometry metadata, ordinary text or paint. Dedicated tests inspect draw order, clip geometry, legend combinations, holes and export pixels; the comparator alone is not full visual equivalence. The 461-case compiler replay verifies success/errors and locations, not every assertion in the original tests.

Actual browser editing, font loading, undo/redo, resources, completion/hover/signature/LCSS diagnostics and VS Code host flows have separate artifact-bound evidence. The final verified counts, hashes, timings and platform limits are recorded in `README.md` and `delivery.json`.
