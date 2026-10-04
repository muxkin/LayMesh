# Complete applications

These examples combine several capabilities on one page. Start with the [feature gallery](README.en.md) to study features individually. Code blocks are excerpts; run the linked full source.

## First figure

Build a simple page with rectangles, text, and anchors.

```lay
page = canvas(size=(160 mm, 100 mm), background="#ffffff")

card = rect(size=(60 mm, 30 mm),
            fill="#e7f4fa", border_color="#28618a", border_width=0.4 mm)
left = page.add(card, target=page.top_left, offset=(12 mm, 15 mm))
page.add(card, target=left.top_right, offset=(8 mm, 0 mm))

title = text(content="Hello LayMesh", font_size=14 pt, color="#203864")
page.add(title, anchor=center, target=left.center)
```

![Actual render: First figure](../../site/media/hello-1920.webp)

Source: [hello.lay](../../examples/hello.lay).

Reproduce: `laymesh validate examples/hello.lay`; `laymesh render examples/hello.lay -o hello.png --dpi 150`.

Observed: 160 × 100 mm, 3 top-level placements; preview 945 × 591 px at 150 DPI.

## Image composition

Reuse one asset definition at two positions with different widths and crops.

```lay
page = canvas(name="LayMesh basic", size=(180 mm, 120 mm), layout_dpi=96, background="#ffffff")

photo = image(src="assets/photo.png")
left = page.add(photo,size=(76 mm, auto), target=page.top_left, offset=(10 mm, 25 mm))
right = page.add(photo,size=(55 mm, auto), crop=box(offset=(0.5, 0), size=(0.5, 1)), target=left.top_right, offset=(9 mm, 0 mm))

label = text(content="One image, two placements", font_family="DejaVu Sans", font_size=12 pt)
page.add(label, target=page.top_left, offset=(10 mm, 8 mm))
```

![Actual render: Image composition](../../site/media/basic-1920.webp)

Source: [basic.lay](../../examples/basic.lay).

Reproduce: `laymesh validate examples/basic.lay`; `laymesh render examples/basic.lay -o basic.png --dpi 150`.

Observed: 180 × 120 mm, 7 top-level placements; preview 1063 × 709 px at 150 DPI.

## Functions and control flow

Use lists, units, functions, and loops to compute visible text.

```lay
function twice(value) { return value * 2 }
values = [2, 4]
values = append(values, 6)
count = 0
for value in values { count = count + value }
for index in range(3) { count = count + index }
while count < 18 { count = count + 1 }
status = "pending"
if count == 18 { status = "ready" } else { status = "error" }
```

![Actual render: Functions and control flow](../../site/media/functions-1920.webp)

Source: [functions.lay](../../examples/functions.lay).

Reproduce: `laymesh validate examples/functions.lay`; `laymesh render examples/functions.lay -o functions.png --dpi 150`.

Observed: 120 × 55 mm, 3 top-level placements; preview 709 × 325 px at 150 DPI.

## Component cards

Import a local component and place three cards in a loop.

```lay
import { card } from "./components/card.lay"

page = canvas(name="LayMesh components", size=(180 mm, 90 mm), background="#ffffff")
spacing = 58 mm
for index in range(3) {
  label = "Card " + str(index + 1)
  tile = card(label)
  placed = page.add(tile,size=(42 mm, auto), target=page.top_left,
                    offset=(8 mm + index * spacing, 12 mm), opacity=0.95)
}
```

![Actual render: Component cards](../../site/media/scripted-1920.webp)

Source: [scripted.lay](../../examples/scripted.lay).

Reproduce: `laymesh validate examples/scripted.lay`; `laymesh render examples/scripted.lay -o scripted.png --dpi 150`.

Observed: 180 × 90 mm, 3 top-level placements; preview 1063 × 531 px at 150 DPI.

## Vector drawing

Combine paths, gradients, holes, dashes, and visible-outline fusion.

```lay
warm = linear_gradient(start=(0, 0), end=(1, 1),
                       stops=[(0, "#fff0ce"), (0.55, "#ff8a60"), (1, "#ad3462")])
cool = radial_gradient(center=(0.35, 0.3), radius=0.8,
                       stops=[(0, "#cbfff5"), (1, "#087f8c")])

wave = path(commands=[move_to(0 mm, 23 mm), cubic_to(14 mm, -5 mm, 31 mm, -5 mm, 45 mm, 23 mm),
                      cubic_to(31 mm, 40 mm, 14 mm, 40 mm, 0 mm, 23 mm), close()],
            fill=warm, border_color="#743152", border_width=0.5 mm)
page.add(wave, target=page.top_left, offset=(10 mm, 31 mm))
```

![Actual render: Vector drawing](../../site/media/vector-1920.webp)

Source: [vector.lay](../../examples/vector.lay).

Reproduce: `laymesh validate examples/vector.lay`; `laymesh render examples/vector.lay -o vector.png --dpi 150`.

Observed: 180 × 120 mm, 9 top-level placements; preview 1063 × 709 px at 150 DPI.

## Compound outlines

Reuse single, double, triple, and dashed outlines.

```lay




card = rect(size=(42 mm, 22 mm), border_radius=3 mm, fill="#e7f6f6", border_color="#087f8c", border_width=1.8 mm, border_style="solid")
page.add(card, target=page.top_left, offset=(10 mm, 30 mm))
page.add(rect(size=(42 mm, 22 mm), border_radius=3 mm, fill="#e7f6f6", border_color="#087f8c", border_width=1.8 mm, border_style=double),
         target=page.top_left, offset=(69 mm, 30 mm))
page.add(rect(size=(42 mm, 22 mm), border_radius=3 mm, fill="#e7f6f6", border_color="#087f8c", border_width=1.8 mm, border_style=triple),
         target=page.top_left, offset=(128 mm, 30 mm))
```

![Actual render: Compound outlines](../../site/media/outlines-1920.webp)

Source: [outlines.lay](../../examples/outlines.lay).

Reproduce: `laymesh validate examples/outlines.lay`; `laymesh render examples/outlines.lay -o outlines.png --dpi 150`.

Observed: 180 × 110 mm, 13 top-level placements; preview 1063 × 650 px at 150 DPI.

## Text and formulas

Mix fonts, colored spans, wrapping, and formula layout.

```lay
header = text(size=(148 mm, auto),
  spans=[
    span("文字排版", font_family="Noto Sans CJK SC", color="#203864", font_size=18 pt),
    span("  /  Typography", font_family="DejaVu Sans", color="#526f96", font_size=12 pt)
  ],
  font_size=12 pt)
page.add(header, target=page.top_left, offset=(7 mm, 6 mm))
```

![Actual render: Text and formulas](../../site/media/typography-1920.webp)

Source: [typography.lay](../../examples/typography.lay).

Reproduce: `laymesh validate examples/typography.lay`; `laymesh render examples/typography.lay -o typography.png --dpi 150`.

Observed: 160 × 100 mm, 8 top-level placements; preview 945 × 591 px at 150 DPI.

## Complete layout atlas

Combine images, vectors, text, groups, and components in a 16 cm wide page.

```lay
import { photo, icon } from "./components/materials.lay"
import { card } from "./components/card.lay"

scale = 160 / 297
page = canvas(name="LayMesh layout atlas", size=(16 cm, 420 mm * scale), background="#ffffff")
artboard = group()
artboard.add(rect(size=(297 mm, 420 mm), fill="#ffffff"), target=artboard.top_left)
font_name = "DejaVu Sans"
```

![Actual render: Complete layout atlas](../../site/media/showcase-1920.webp)

Source: [showcase.lay](../../examples/showcase.lay).

Reproduce: `laymesh validate examples/showcase.lay`; `laymesh render examples/showcase.lay -o showcase.png --dpi 150`.

Observed: 160 × 226.262626… mm, 1 top-level group; preview 945 × 1336 px at 150 DPI.
