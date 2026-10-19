//! CPU conic-fill fallback for the synchronous, preview-only WASM SVG API.
//! Interactive previews and async exports use Skia; this consumes the same
//! resolved sRGB stops and never parses CSS or repeats color-space conversion.
use crate::{
    display_list::{DrawKind, DrawNode, Paint},
    error,
};
use kurbo::{Point, Rect, Shape};
use laymesh_core::Result;
pub(crate) fn conic(node: &DrawNode, b: Rect, w: u32, h: u32) -> Result<Vec<u8>> {
    let DrawKind::Path {
        path,
        fill:
            Paint::Conic {
                center,
                angle,
                stops,
                transform,
            },
        even_odd,
        ..
    } = &node.kind
    else {
        return Err(error("Expected a resolved conic fill"));
    };
    if w as u64 * h as u64 > 16_777_216 {
        return Err(error("Conic fill exceeds the 64 MiB tile budget"));
    }
    let inverse = transform.inverse();
    let mut image = image::RgbaImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let p = Point::new(
                b.x0 + (x as f64 + 0.5) * b.width() / w as f64,
                b.y0 + (y as f64 + 0.5) * b.height() / h as f64,
            );
            let winding = path.winding(p);
            if if *even_odd {
                winding % 2 == 0
            } else {
                winding == 0
            } {
                continue;
            }
            let p = inverse * p;
            let t = ((p.x - center[0]).atan2(center[1] - p.y) - angle)
                .rem_euclid(std::f64::consts::TAU)
                / std::f64::consts::TAU;
            let j = stops.partition_point(|s| s.offset <= t);
            let c = if j == 0 {
                stops[0].rgba
            } else if j == stops.len() {
                stops[j - 1].rgba
            } else {
                let a = &stops[j - 1];
                let z = &stops[j];
                let u = (t - a.offset) / (z.offset - a.offset);
                std::array::from_fn(|i| a.rgba[i] + (z.rgba[i] - a.rgba[i]) * u)
            };
            image.put_pixel(
                x,
                y,
                image::Rgba(c.map(|v| (v.clamp(0., 1.) * 255.).round() as u8)),
            );
        }
    }
    let mut out = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image)
        .write_to(&mut out, image::ImageFormat::Png)
        .map_err(|e| error(e.to_string()))?;
    Ok(out.into_inner())
}
