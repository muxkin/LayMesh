# Functions and modules

<!-- walkthrough:start -->
## Purpose and concepts

function encapsulates computation or material composition. Component .lay files export variables/functions; the entry imports selected names and may rename them.

## Minimal complete example

Run this file directly with `laymesh validate` or `laymesh render`; it contains its own canvas and required definitions.

<!-- example:examples/manual/group.lay -->

## Parameters and default behavior

Unitless geometry uses the canvas unit; unitless type and stroke sizes use pt. Explicit call parameters override inherited/theme defaults. The linked interface reference lists accepted types, choices and defaults per parameter.

## Composition

Functions may accept sizes, titles or cap variables and return material or groups. Predefined variables also work in modules, with user/module bindings taking priority. Resource paths are relative to the module file.

<!-- example:examples/gallery/containers/module-import.lay -->

## Common errors and limits

Imports are read-only; cycles and missing exports error. Modules do not create or draw directly on the entry canvas. Create the entry canvas before using imported components.

## Individual functions

<!-- walkthrough:end -->

## Detailed behavior and further examples

### Parameters

| Parameter | Purpose | Default or requirement |
| --- | --- | --- |
| `import` | Import local components | Path relative to importing file |
| `function` | Define reusable functions | Components do not create an entry canvas |

### Common usage

Imports must be local relative `.lay` paths. Modules use `export` for constants, materials, groups, or functions and may import further modules; imported names are read-only. The entry canvas must exist before imported values are used. A component module cannot create or draw on the entry canvas. Missing exports and import cycles are errors. See [scripted.lay](../../examples/scripted.lay) and [card.lay](../../examples/components/card.lay).

### Limits and related topics

[Variables and expressions](values.en.md) · [Conditions and loops](control-flow.en.md)


[Library documentation comments](library-docs.en.md) explain how to attach function and parameter help to your reusable components.

## workflow

Complete sources and executable verification fixtures for this workflow are listed in the [feature coverage map](feature-map.en.md). Follow this page’s input conditions and limits when composing features.
