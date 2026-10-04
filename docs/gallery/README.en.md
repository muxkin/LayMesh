# LayMesh feature gallery

Each example shows a short explanation, key code, its actual rendered image immediately below, the full source, reproduction commands, and observed results. Images and source files live in this repository.

## Browse by capability

### [Images and illustrations](images.en.md) · 10 examples

Input formats, reuse, independent crops, and fitting images into frames. All sample assets are generated in this repository.

![Images and illustrations representative render](../../site/media/gallery-images-png-1920.webp)

[PNG bitmap](images.en.md#png) · [JPEG bitmap](images.en.md#jpeg) · [SVG vector asset](images.en.md#svg) · [8-bit grayscale TIFF](images.en.md#tiff-gray) · [8-bit RGB TIFF](images.en.md#tiff-rgb) · [Reuse one asset](images.en.md#reuse) · [Independent crops](images.en.md#crop) · [Contain](images.en.md#fit-contain) · [Cover](images.en.md#fit-cover) · [Stretch](images.en.md#fit-stretch)

### [Positioning and layers](positioning.en.md) · 6 examples

Build relationships with placement anchors; rotation, opacity, and add order then determine the final drawing.

![Positioning and layers representative render](../../site/media/gallery-positioning-anchors-1920.webp)

[Nine-point anchors](positioning.en.md#anchors) · [Relative placement chain](positioning.en.md#relative) · [Size-dependent placement](positioning.en.md#dependent-size) · [Rotation](positioning.en.md#rotation) · [Opacity](positioning.en.md#opacity) · [Drawing order](positioning.en.md#drawing-order)

### [Groups and components](containers.en.md) · 4 examples

Organize objects in local coordinates and reuse layouts through groups and local modules.

![Groups and components representative render](../../site/media/gallery-containers-group-local-1920.webp)

[Local group coordinates](containers.en.md#group-local) · [Nested groups and scaling](containers.en.md#group-nested) · [Place one group twice](containers.en.md#group-reuse) · [Local module component](containers.en.md#module-import)

### [Shapes and drawing](shapes.en.md) · 17 examples

Basic shapes, paths, fills, dashes, compound outlines, and visible-outline fusion.

![Shapes and drawing representative render](../../site/media/gallery-shapes-rect-1920.webp)

[Rounded rectangle](shapes.en.md#rect) · [Ellipse](shapes.en.md#ellipse) · [Line and arrow](shapes.en.md#line-arrow) · [Polygon and polyline](shapes.en.md#polygon-polyline) · [Arc and sector](shapes.en.md#arc-sector) · [Star and ring](shapes.en.md#star-ring) · [Path commands](shapes.en.md#path-commands) · [Even-odd hole](shapes.en.md#evenodd-hole) · [Linear gradient](shapes.en.md#linear-gradient) · [Radial gradient](shapes.en.md#radial-gradient) · [Custom dashes](shapes.en.md#dashes) · [Reusable outlines](shapes.en.md#outlines) · [Visible-outline fusion](shapes.en.md#fuse) · [Angled line on a straight edge](shapes.en.md#fuse-angled) · [Angled line on a curved edge](shapes.en.md#fuse-curved) · [Line joined to a hollow edge](shapes.en.md#fuse-outline-edge) · [Line joined to a hollow corner](shapes.en.md#fuse-outline-corner)

### [Text and formulas](typography.en.md) · 5 examples

Fonts, wrapping, colored spans, inline formulas, and standalone formulas.

![Text and formulas representative render](../../site/media/gallery-typography-font-1920.webp)

[Font names and fallback](typography.en.md#font) · [Width, wrapping, and alignment](typography.en.md#wrap-align) · [Colored text spans](typography.en.md#spans) · [Inline formula](typography.en.md#inline-formula) · [Display formula](typography.en.md#display-formula)

### [Scripting and calculations](scripting.en.md) · 2 examples

Variables, units, functions, and control flow within the restricted .lay language.

![Scripting and calculations representative render](../../site/media/gallery-scripting-units-builtins-1920.webp)

[Units and built-ins](scripting.en.md#units-builtins) · [Functions and loops](scripting.en.md#control-flow)

### [Scientific data plots](plots.en.md) · 10 examples

Cartesian, polar and radar plots with fixed physical data areas and manually placed panels and decorations.

![Scientific data plots representative render](../../site/media/plot-multi-axes-breaks-1920.webp)

[Multiple axes and independent gaps](plots.en.md#multi-axes-breaks) · [Layers and descriptive statistics](plots.en.md#statistics) · [Shared colors and independent colorbar](plots.en.md#shared-colors) · [Manual inset](plots.en.md#inset) · [Directional curves and signed radii](plots.en.md#polar-directions) · [Polar uncertainty and caps](plots.en.md#polar-errors) · [Angular histograms and annular bars](plots.en.md#polar-rose) · [Polar cells and periodic contours](plots.en.md#polar-field) · [Radar charts in original units](plots.en.md#radar) · [Polar and radar data from Python](plots.en.md#polar-data)

## More examples

[Native scientific plots and output](../plotting.en.md) · [Matplotlib and Notebook output](notebook.en.md) · [Eight complete examples](comprehensive.en.md) · [Execution results and tests](../examples-and-results.en.md)
