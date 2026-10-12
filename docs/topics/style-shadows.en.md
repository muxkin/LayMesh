# Shadows and effects

[Style manual](lcss.en.md) · Next: [Compatibility and troubleshooting](style-compatibility.en.md)

## Three CSS shadow types

| Effect | Shape |
| --- | --- |
| box-shadow | Layout border box, including transparent objects and elliptical content |
| text-shadow | Text or formula content alpha |
| filter:drop-shadow(...) | Rendered content alpha; currently one drop-shadow |

~~~lcss
.card { box-shadow:1mm 2mm 3mm 0mm #0004; }
.title { text-shadow:0.5mm 0.5mm 1mm #0006; }
.cutout { filter:drop-shadow(1mm 1mm 1mm #0005); }
~~~

Box shadow fields are x/y offsets, blur radius, spread and color. Offsets are required; blur and spread may be omitted; omitted color uses currentColor. Text shadows and drop-shadow do not accept spread or inset.

## Layers, inset and negative spread

~~~lcss
.card {
  box-shadow:1mm 2mm 3mm #192c4e40,
             3mm 4mm 6mm -1mm #2e2a6820;
}
.inset { box-shadow:inset 1mm 1mm 3mm #33415580; }
~~~

Commas create layers, first on top. Positive spread expands and negative spread contracts. Outer shadows remove the box interior; inset shadows sit above the background and below content/border. Effects follow group transforms and clipping; the canvas can crop overflow.

<!-- example:examples/gallery/styles/layers.lay -->

## partial-overrides

Change only an imported shadow's blur radius:

~~~lcss
@import "@laymesh/bootstrap.css";
.shadow.custom { box-shadow-blur:2mm !important; }
~~~

class="bg-primary rounded shadow custom" preserves offsets, spread, color and existing layers. Bootstrap's important shadows require an important and more specific override.

| Intention | Declaration |
| --- | --- |
| 2mm blur radius | box-shadow-blur:2mm |
| Expand by 2mm | box-shadow-spread:2mm |
| Offset right/down by 2mm | box-shadow-offset:2mm 2mm |
| Change color | box-shadow-color:#0d6efd66 |
| Make inset | box-shadow-position:inset |

From left to right: the library's original shadow, only blur changed to 2mm, only spread changed to 2mm, and only x/y offset changed to 2mm. Each variant changes one field and preserves color and the other fields. Edit the complete source to compare them.

<!-- example:examples/gallery/styles/shadow-parameters.lay -->

These longhands follow the [CSS Borders Level 4 working draft](https://www.w3.org/TR/css-borders-4/#box-shadow), which remains a work in progress. The following rules describe the current LayMesh implementation. One value applies to all existing layers; comma lists apply per layer and shorter lists repeat. Blur alone creates no shadow. A single offset preserves layer count; an explicit offset list can set it. One length sets both x/y, and none disables that layer.

A new box-shadow resets its components; box-shadow:none clears shadows. Important longhands still participate in the cascade and cannot be overridden by an ordinary shorthand.

<!-- example:examples/manual/css-shadow-override.lay -->

## Blur and the existing effects stack

CSS box/text blur radius is divided by two for Gaussian sigma: 2mm blur corresponds to sigma=1mm. The third drop-shadow length directly specifies sigma.

effects=[shadow(...),glow(...)] is a separate ordered LayMesh stack with its existing target, order and blur semantics. shadow(blur=2mm) uses sigma=2mm and requires nonnegative spread. Box-shadow longhands do not edit this stack. See [Artistic text and effects](art-effects.en.md).

## Export and budgets

SVG, PNG and PDF use the same effect definitions. PDF subjects remain vector while blurred effects rasterize locally. Export DPI affects raster detail without changing millimeter lengths. Existing pixel and memory limits still apply; reduce affected area or relevant raster density when a budget error occurs.
