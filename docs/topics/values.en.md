# Variables and expressions

<!-- walkthrough:start -->
## Purpose and concepts

Variables hold numbers, strings, lists, material and instances. Fixed options have predefined string variables: round and "round" are identical values without a declaration. These are not macros and have no special expansion or assignment rules.

## Minimal complete example

Run this file directly with `laymesh validate` or `laymesh render`; it contains its own canvas and required definitions.

<!-- example:examples/manual/str.lay -->

## Parameters and default behavior

Unitless geometry uses the canvas unit; unitless type and stroke sizes use pt. Explicit call parameters override inherited/theme defaults. The linked interface reference lists accepted types, choices and defaults per parameter.

## Composition

Put [triangle,open,dot] in a list and loop over heads, or use round as a function default. User scope takes priority, so round="square" changes that scope’s value. ratex_katex represents "ratex-katex".

<!-- example:examples/gallery/scripting/units-builtins.lay -->

## Common errors and limits

String spellings remain legal without deprecation warnings. Unknown names still report E_NAME. Hovering an option variable shows only type and value; parameter documentation explains accepted values and their meaning.

## Individual functions

### range

Return a finite integer list excluding stop: one argument is stop, two are start/stop and three add step. step must be nonzero; at most 10,000 items.

Returns: integer[]

[Minimal complete source](../../examples/manual/range.lay) · [Composition source](../../examples/functions.lay) · [All parameters](interface-reference.en.md#range)

### len

Return list, dictionary or geometry collection size, or string length.

Returns: integer

[Minimal complete source](../../examples/manual/len.lay) · [Composition source](../../examples/gallery/scripting/units-builtins.lay) · [All parameters](interface-reference.en.md#len)

### append

Return a copy of a list with one item appended, without mutating the input. Reassign the result when accumulating values.

Returns: list

[Minimal complete source](../../examples/manual/append.lay) · [Composition source](../../examples/gallery/scripting/units-builtins.lay) · [All parameters](interface-reference.en.md#append)

### str

Convert supported numeric/boolean/string values to text, retaining length units. Use it in labels without evaluating arbitrary formatting expressions.

Returns: string

[Minimal complete source](../../examples/manual/str.lay) · [Composition source](../../examples/gallery/scripting/units-builtins.lay) · [All parameters](interface-reference.en.md#str)

### abs

Return absolute value while retaining numeric unit type. Accepts scalars or lengths, not material or lists.

Returns: number | length

[Minimal complete source](../../examples/manual/abs.lay) · [Composition source](../../examples/gallery/scripting/units-builtins.lay) · [All parameters](interface-reference.en.md#abs)

### min

Return the minimum of one or more values with compatible units. Input types must be compatible; the result retains units.

Returns: number | length

[Minimal complete source](../../examples/manual/min.lay) · [Composition source](../../examples/gallery/scripting/units-builtins.lay) · [All parameters](interface-reference.en.md#min)

### max

Return the maximum of one or more values with compatible units. Input types must be compatible; the result retains units.

Returns: number | length

[Minimal complete source](../../examples/manual/max.lay) · [Composition source](../../examples/gallery/scripting/units-builtins.lay) · [All parameters](interface-reference.en.md#max)

### dict

Create an empty dictionary or load an ordered nested JSON object; table validation remains strict.

Returns: dict

[Minimal complete source](../../examples/manual/dictionaries.lay) · [Composition source](../../examples/gallery/scripting/dictionaries.lay) · [All parameters](interface-reference.en.md#dict)

### dict-keys

Return dictionary keys as a list in insertion order.

Returns: list

[Minimal complete source](../../examples/manual/dictionaries.lay) · [Composition source](../../examples/gallery/scripting/dictionaries.lay) · [All parameters](interface-reference.en.md#dict-keys)

### dict-values

Return values in insertion order, preserving units and object types.

Returns: list

[Minimal complete source](../../examples/manual/dictionaries.lay) · [Composition source](../../examples/gallery/scripting/dictionaries.lay) · [All parameters](interface-reference.en.md#dict-values)

### dict-items

Return key/value pairs in insertion order for loop unpacking.

Returns: list

[Minimal complete source](../../examples/manual/dictionaries.lay) · [Composition source](../../examples/gallery/scripting/dictionaries.lay) · [All parameters](interface-reference.en.md#dict-items)

### dict-get

Look up a string key; return default when missing, or null by default.

Returns: value

Required inputs: `key`.

[Minimal complete source](../../examples/manual/dictionaries.lay) · [Composition source](../../examples/gallery/scripting/dictionaries.lay) · [All parameters](interface-reference.en.md#dict-get)

### dict-update

Return a merged dictionary; existing keys keep their position and new keys append. The original remains unchanged.

Returns: dict

Required inputs: `other`.

[Minimal complete source](../../examples/manual/dictionaries.lay) · [Composition source](../../examples/gallery/scripting/dictionaries.lay) · [All parameters](interface-reference.en.md#dict-update)

<!-- walkthrough:end -->

## Detailed behavior and further examples

### Parameters

| Parameter | Purpose | Default or requirement |
| --- | --- | --- |
| `mm / cm / in / pt / px` | Length units | Lengths require units |
| `range(...)` | Generate loop sequences | Bounded evaluation |

### Common usage

Variables store values or material definitions. Lengths require units. Lists, tuples, indexing and built-in operations run inside the restricted DSL, not arbitrary Python or JavaScript.

[Complete parameters and rules](../language-reference.en.md)

### Limits and related topics

[Conditions and loops](control-flow.en.md) · [Functions and modules](modules.en.md)

## workflow

Complete sources and executable verification fixtures for this workflow are listed in the [feature coverage map](feature-map.en.md). Follow this page’s input conditions and limits when composing features.

## Equivalent strings and local override

[Runnable comparison](../../examples/manual/variables-strings.lay) draws identical heads from triangle and "triangle", then uses a user round="square" binding. Lists, equality and scope all use ordinary string rules.

## Ordered dictionaries and iteration

Use string keys in `{ "B": value, "A": other }`; expressions may produce keys and values may retain lengths, colors, materials or geometry selectors. Dictionaries preserve insertion order. Duplicate keys replace their value without moving the key. `d["key"]` raises E_INDEX for a missing key; `d.get("key", default=null)` returns a fallback. `len(d)`, `.keys()`, `.values()` and `.items()` work with user dictionaries and table columns.

Assignment copies dictionaries: changing `b` after `b=a` does not change `a`. Nested dictionary writes such as `d["panel"]["color"]="#0072b2"` rebuild and rebind the root value; intermediate keys must exist, and imported bindings stay read-only. `.update(other)` returns a new merged dictionary: use `d=d.update(other)`. Objects held in dictionaries retain their existing handle behavior. Attribute access is reserved for methods; use indexing for keys such as "items".

`dict()` creates an empty dictionary; `dict(src="config.json")` loads nested JSON objects, retaining object order and validating finite, safely representable numbers. This does not turn table or array into permissive loaders.
