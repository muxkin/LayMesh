# Conditions and loops

<!-- walkthrough:start -->
## Purpose and concepts

for follows list order or dictionary insertion order. Pair and nested unpacking are supported; enumerate adds indices and zip stops at the shortest input. Iteration snapshots are fixed at loop entry.

## Minimal complete example

Run this file directly with `laymesh validate` or `laymesh render`; it contains its own canvas and required definitions.

<!-- example:examples/manual/range.lay -->

## Parameters and default behavior

Unitless geometry uses the canvas unit; unitless type and stroke sizes use pt. Explicit call parameters override inherited/theme defaults. The linked interface reference lists accepted types, choices and defaults per parameter.

## Composition

Store dimensions, colors or offsets in lists and place them using range(len(items)). while, break and continue control iteration, with explicit reproducible exit conditions.

<!-- example:examples/gallery/scripting/control-flow.lay -->

## Common errors and limits

Loops allow at most 10,000 iterations, function depth is at most 64 and statement execution is bounded. New local bindings do not automatically leak outward; reassigning existing outer bindings follows scope rules.

## Individual functions

### enumerate

Return index/value pairs for loop unpacking; start defaults to zero and input is limited to 10,000 items.

Returns: list

Required inputs: `seq`.

[Minimal complete source](../../examples/gallery/scripting/dictionaries.lay) · [Composition source](../../examples/gallery/scripting/dictionaries.lay) · [All parameters](interface-reference.en.md#enumerate)

### zip

Combine positional iterable inputs into tuples, stopping at the shortest input; no arguments return an empty list.

Returns: list

[Minimal complete source](../../examples/gallery/scripting/dictionaries.lay) · [Composition source](../../examples/gallery/scripting/dictionaries.lay) · [All parameters](interface-reference.en.md#zip)

<!-- walkthrough:end -->

## Detailed behavior and further examples

### Parameters

| Parameter | Purpose | Default or requirement |
| --- | --- | --- |
| `if / else` | Conditional branches | Restricted DSL |
| `for` | Iterate sequences | Bounded by evaluation limits |

### Common usage

Use conditions to select layouts and finite loops to repeat placements. Evaluation and iteration are limited; prefer native batch layers for large datasets.

[Complete parameters and rules](../language-reference.en.md)

### Limits and related topics

[Variables and expressions](values.en.md) · [Functions and modules](modules.en.md)

## workflow

Complete sources and executable verification fixtures for this workflow are listed in the [feature coverage map](feature-map.en.md). Follow this page’s input conditions and limits when composing features.

## Pair unpacking and snapshots

`for key in d` iterates keys; `for key,value in d.items()` iterates pairs. Nested patterns such as `for i,(name,ys) in enumerate(d.items())` bind in the loop scope; `_` discards a value. Shapes must match exactly and repeated variable names are rejected.

`enumerate(seq,start=0)` returns index/value pairs. `zip(a,b,...)` stops at the shortest input, and zero arguments return an empty list. Lists, dictionaries and geometry collections are iterable. Iteration snapshots are fixed at loop entry, so updating the original dictionary does not add work to the current loop. Existing break, continue, scope and evaluation limits remain in force. Dictionary membership uses `key in d` or `key not in d`.

[Complete dictionary plot](../../examples/plot/dictionary-series.lay) · [Colormap presets](cmaps.en.md)
