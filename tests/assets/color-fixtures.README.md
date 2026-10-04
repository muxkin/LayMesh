# Color regression fixtures

`cmyk-samples.jpg` and `ycck-samples.jpg` are generated test-only images containing
four CMYK samples: `(0,255,255,0)`, `(255,0,255,0)`, `(255,255,0,0)`, and
`(0,0,0,255)`. They contain no third-party image or embedded profile.

The first was encoded with Pillow at quality 100 and no subsampling. The second
was generated with ImageMagick from the first, using `-sampling-factor 1x1
-quality 100`. Their Adobe APP14 transform values are 0 (CMYK) and 2 (YCCK).
Tests construct their own small ICC CMYK-to-XYZ lookup profile in memory and
attach it to each JPEG, so no system profile installation is required.

These fixtures and this description may be used under the repository license.
