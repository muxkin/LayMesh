//! Synchronous SVG compatibility API. Interactive consumers use CanvasKit;
//! this adapter rasterizes only 3D objects and shares the same mesh rasterizer.
use crate::{display_list::*, error, portable_svg, surface3d};
use kurbo::{Affine, Rect};
use laymesh_core::Result;
use std::io::Cursor;
fn png(w: u32, h: u32, rgba: Vec<u8>) -> Result<Vec<u8>> {
    let im =
        image::RgbaImage::from_raw(w, h, rgba).ok_or_else(|| error("Invalid object raster"))?;
    let mut bytes = Cursor::new(Vec::new());
    im.write_to(&mut bytes, image::ImageFormat::Png)
        .map_err(|e| error(e.to_string()))?;
    Ok(bytes.into_inner())
}
fn texture(list: &DisplayList, node: &DrawNode, b: Rect, w: u32, h: u32) -> Result<Vec<u8>> {
    if w as u64 * h as u64 > 16_777_216 {
        return Err(error("3D texture tile exceeds 64 MiB"));
    }
    let mut list = list.clone();
    let mut node = node.clone();
    node.transform = Affine::translate((-b.x0, -b.y0));
    node.opacity = 1.;
    node.effects.clear();
    if let Some(hint) = node.hint.as_object_mut() {
        hint.remove("surface3d");
    }
    list.nodes = vec![node];
    list.width = b.width();
    list.height = b.height();
    let svg = portable_svg::render_with_mode(
        &list,
        w as f64 / b.width() * 25.4,
        Some(&mut |node, r, w, h, _| crate::gradient_raster::conic(node, r, w, h)),
        false,
    )?;
    let tree =
        usvg::Tree::from_str(&svg, &usvg::Options::default()).map_err(|e| error(e.to_string()))?;
    let mut pixels =
        resvg::tiny_skia::Pixmap::new(w, h).ok_or_else(|| error("3D texture allocation failed"))?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(
            w as f32 / tree.size().width(),
            h as f32 / tree.size().height(),
        ),
        &mut pixels.as_mut(),
    );
    let mut rgba = pixels.take();
    for p in rgba.chunks_exact_mut(4) {
        if p[3] > 0 {
            for i in 0..3 {
                p[i] = ((p[i] as u32 * 255 + p[3] as u32 / 2) / p[3] as u32).min(255) as u8;
            }
        }
    }
    Ok(rgba)
}
fn bake(n: &mut DrawNode, list: &mut DisplayList, density: f64) -> Result<()> {
    let density = density * portable_svg::max_scale(n.transform);
    if let DrawKind::Group { children, .. } = &mut n.kind {
        for c in children {
            bake(c, list, density)?;
        }
    }
    let Some(mesh) = surface3d::mesh(n) else {
        return Ok(());
    };
    if density <= 0. || !density.is_finite() {
        return Err(error("Invalid 3D export density"));
    }
    let mesh = surface3d::refine(&mesh, density)?;
    let source_density = density * mesh.texture_scale;
    let b = mesh.bounds;
    let x0 = (b[0] * density).floor();
    let y0 = (b[1] * density).floor();
    let width = (b[2] * density).ceil() - x0;
    let height = (b[3] * density).ceil() - y0;
    if width * height > 100_000_000. {
        return Err(error("3D object exceeds the 100000000-pixel budget"));
    }
    let mut children = vec![];
    let mut pending = vec![];
    for y in (0..height as u32).step_by(512) {
        for x in (0..width as u32).step_by(512) {
            pending.push((
                x,
                y,
                (width as u32 - x).min(512),
                (height as u32 - y).min(512),
            ));
        }
    }
    while let Some((x, y, w, h)) = pending.pop() {
        let region = [
            (x0 + x as f64) / density,
            (y0 + y as f64) / density,
            (x0 + x as f64 + w as f64) / density,
            (y0 + y as f64 + h as f64) / density,
        ];
        let Some(source) = surface3d::texture_region(&mesh, region, source_density) else {
            continue;
        };
        let tw = ((source[2] - source[0]) * source_density).round() as u32;
        let th = ((source[3] - source[1]) * source_density).round() as u32;
        if tw as u64 * th as u64 * 16 + w as u64 * h as u64 * 132 > 64 * 1024 * 1024 {
            if w > 1 || h > 1 {
                if w >= h {
                    let a = w / 2;
                    pending.extend([(x, y, a, h), (x + a, y, w - a, h)]);
                } else {
                    let a = h / 2;
                    pending.extend([(x, y, w, a), (x, y + a, w, h - a)]);
                }
                continue;
            }
            return Err(error("three_d texture exceeds the 64 MiB working budget"));
        }
        let mut input = texture(list, n, surface3d::rect(source), tw, th)?;
        if let Some(mask) = &mesh.coverage {
            let coverage = texture(list, mask, surface3d::rect(source), tw, th)?;
            input = surface3d::material_texture(&input, &coverage, tw, th, mesh.material_alpha)?;
        }
        let mut tile = mesh.clone();
        tile.source = source;
        let rgba = surface3d::raster(&tile, &input, tw, th, region, w, h)?;
        let id = format!("surface3d-{}", list.resources.len());
        list.resources.insert(
            id.clone(),
            Resource {
                mime: "image/png".into(),
                data: png(w, h, rgba)?.into(),
                font_index: None,
                family: None,
            },
        );
        children.push(DrawNode::new(DrawKind::Image {
            resource: id,
            rect: surface3d::rect(region),
            nearest: false,
        }));
    }
    n.kind = DrawKind::Group {
        children,
        clip: None,
        mask: None,
    };
    n.hint.as_object_mut().unwrap().remove("surface3d");
    Ok(())
}
pub(crate) fn render(mut list: DisplayList, dpi: f64) -> Result<String> {
    let mut nodes = std::mem::take(&mut list.nodes);
    for n in &mut nodes {
        bake(n, &mut list, dpi / 25.4)?;
    }
    list.nodes = nodes;
    portable_svg::render_with_mode(
        &list,
        dpi,
        Some(&mut |node, r, w, h, _| crate::gradient_raster::conic(node, r, w, h)),
        false,
    )
}
