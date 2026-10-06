use super::*;
use krilla::{
    Document,
    geom::{Point, Size, Transform},
    num::NormalizedF32,
    page::PageSettings,
    paint::Fill,
    tagging::{ContentTag, SpanTag},
    text::{Font, GlyphId, KrillaGlyph},
};
use krilla_svg::{SurfaceExt, SvgSettings};
use resvg::{tiny_skia, usvg};
use std::sync::Arc;
fn tree(scene: &Scene) -> Result<usvg::Tree> {
    tree_svg(scene, &svg(scene, false)?)
}
fn tree_svg(scene: &Scene, xml: &str) -> Result<usvg::Tree> {
    let mut db = usvg::fontdb::Database::new();
    for (key, _) in used_fonts(scene) {
        let Some(asset) = scene.fonts.get(&key) else {
            continue;
        };
        let ids = db.load_font_source(usvg::fontdb::Source::Binary(Arc::new(asset.data.clone())));
        for id in ids {
            let Some(mut face) = db.face(id).cloned() else {
                continue;
            };
            if face.index != asset.index {
                db.remove_face(id);
                continue;
            }
            if let Some((_, lang)) = face.families.first() {
                face.families.insert(0, (key.clone(), *lang));
            }
            db.remove_face(id);
            db.push_face_info(face);
        }
    }
    let options = usvg::Options {
        fontdb: Arc::new(db),
        resources_dir: None,
        ..Default::default()
    };
    usvg::Tree::from_str(xml, &options).map_err(|e| error(format!("SVG 渲染失败：{e}")))
}
/// Render transparent RGBA PNG at the requested physical resolution and write a pHYs chunk.
pub fn render_png(scene: &Scene, dpi: f64) -> Result<Vec<u8>> {
    super::export::render_export(
        scene,
        "png",
        &super::export::ExportOptions {
            dpi: Some(dpi),
            ..Default::default()
        },
    )
}

pub(crate) fn rasterize(scene: &Scene, dpi: f64) -> Result<(u32, u32, Vec<u8>)> {
    if !dpi.is_finite() || dpi <= 0. || dpi > 25_400. {
        return Err(error("DPI 必须为 0–25400 之间的正数"));
    }
    let w = (scene.width * dpi / 25.4).round().max(1.);
    let h = (scene.height * dpi / 25.4).round().max(1.);
    if w * h > 100_000_000. || w > u32::MAX as f64 || h > u32::MAX as f64 {
        return Err(error("导出像素数超过 100000000 上限"));
    }
    let tree = tree(scene)?;
    let mut pixmap = tiny_skia::Pixmap::new(w as u32, h as u32)
        .ok_or_else(|| error("无法分配导出像素缓冲区"))?;
    let transform = tiny_skia::Transform::from_scale(
        w as f32 / tree.size().width(),
        h as f32 / tree.size().height(),
    );
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    let mut rgba = Vec::with_capacity(w as usize * h as usize * 4);
    for p in pixmap.pixels() {
        let p = p.demultiply();
        rgba.extend_from_slice(&[p.red(), p.green(), p.blue(), p.alpha()]);
    }
    Ok((w as u32, h as u32, rgba))
}
fn formula_semantics(
    node: &Json,
    surface: &mut krilla::surface::Surface,
    font: &Font,
    face: &ttf_parser::Face,
) {
    let kind = jstr(node, "kind", "");
    let (w, h) = (jnum(node, "width", 0.), jnum(node, "height", 0.));
    let angle = jnum(node, "rotation", 0.).to_radians();
    let (c, s) = (angle.cos(), angle.sin());
    let (x, y) = (jnum(node, "x", 0.) + w / 2., jnum(node, "y", 0.) + h / 2.);
    surface.push_transform(&Transform::from_row(
        c as f32,
        s as f32,
        -s as f32,
        c as f32,
        (x - c * w / 2. + s * h / 2.) as f32,
        (y - s * w / 2. - c * h / 2.) as f32,
    ));
    match kind {
        "group" => {
            let sx = if jnum(node, "contentWidth", 0.) > 0. {
                w / jnum(node, "contentWidth", w)
            } else {
                1.
            };
            let sy = if jnum(node, "contentHeight", 0.) > 0. {
                h / jnum(node, "contentHeight", h)
            } else {
                1.
            };
            surface.push_transform(&Transform::from_scale(sx as f32, sy as f32));
            for child in node["children"].as_array().into_iter().flatten() {
                formula_semantics(child, surface, font, face);
            }
            surface.pop();
        }
        "text" => {
            if let Some(source) = node["artText"].as_str() {
                let mut semantic = node.clone();
                semantic["kind"] = json!("formula");
                semantic["source"] = json!(source);
                semantic["x"] = json!(0.);
                semantic["y"] = json!(0.);
                semantic["rotation"] = json!(0.);
                formula_semantics(&semantic, surface, font, face);
            }
            for run in node["runs"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|r| r["kind"] == "formula")
            {
                formula_semantics(run, surface, font, face);
            }
        }
        "formula" => {
            let source = jstr(node, "source", "");
            if !source.is_empty() {
                let size = h.max(0.1) as f32;
                let units = face.units_per_em() as f32;
                let fallback = face.glyph_index('x').unwrap();
                let glyphs: Vec<_> = source
                    .char_indices()
                    .map(|(offset, ch)| {
                        let gid = face.glyph_index(ch).unwrap_or(fallback);
                        KrillaGlyph::new(
                            GlyphId::new(gid.0 as u32),
                            face.glyph_hor_advance(gid).unwrap_or(500) as f32 / units,
                            0.,
                            0.,
                            0.,
                            offset..offset + ch.len_utf8(),
                            None,
                        )
                    })
                    .collect();
                let advance = glyphs.iter().map(|g| g.x_advance).sum::<f32>() * size;
                surface.push_transform(&Transform::from_scale(
                    if advance > 0. { w as f32 / advance } else { 1. },
                    1.,
                ));
                surface.set_fill(Some(Fill {
                    opacity: NormalizedF32::ZERO,
                    ..Default::default()
                }));
                surface.set_stroke(None);
                surface.start_tagged(ContentTag::Span(
                    SpanTag::empty().with_actual_text(Some(source)),
                ));
                surface.draw_glyphs(
                    Point::from_xy(0., jnum(node, "ascent", h * 0.8) as f32),
                    &glyphs,
                    font.clone(),
                    source,
                    size,
                    false,
                );
                surface.end_tagged();
                surface.pop();
            }
        }
        _ => {}
    }
    surface.pop();
}

/// Rasterize only effect layers, using bounded tiles with overlap for blur kernels.
/// Unlike krilla-svg's filter fallback this never silently caps at 5000 pixels.
fn pdf_effect_layers(scene: &Scene, dpi: f64) -> Result<String> {
    let xml = svg(scene, false)?;
    if !xml.contains("data-laymesh-effect") {
        return Ok(xml);
    }
    let parsed = roxmltree::Document::parse(&xml).map_err(|e| error(e.to_string()))?;
    let defs: String = parsed
        .descendants()
        .filter(|n| n.has_tag_name("defs"))
        .map(|n| &xml[n.range()])
        .collect();
    let original = tree_svg(scene, &xml)?;
    let mut replacements = Vec::new();
    let mut total_pixels = 0u64;
    for layer in parsed
        .descendants()
        .filter(|n| n.attribute("data-laymesh-effect") == Some("true"))
    {
        if layer
            .ancestors()
            .skip(1)
            .any(|n| n.attribute("data-laymesh-effect") == Some("true"))
        {
            continue;
        }
        let id = layer.attribute("id").unwrap();
        let Some(usvg::Node::Group(group)) = original.node_by_id(id) else {
            continue;
        };
        let bbox = group.layer_bounding_box();
        let tr = group.abs_transform();
        let scale = ((tr.sx as f64).hypot(tr.ky as f64)).max((tr.kx as f64).hypot(tr.sy as f64));
        // usvg absolute transforms include the root mm-to-CSS-pixel scale.
        let root_density = original.size().width() as f64 / scene.width;
        let density = dpi / 25.4 * (scale / root_density).max(1e-9);
        let width = (bbox.width() as f64 * density).ceil().max(1.) as u32;
        let height = (bbox.height() as f64 * density).ceil().max(1.) as u32;
        total_pixels = total_pixels
            .checked_add(width as u64 * height as u64)
            .ok_or_else(|| error("PDF effect pixel budget overflow"))?;
        if total_pixels > 100_000_000 {
            return Err(error(
                "PDF effects exceed the 100000000-pixel budget; reduce DPI or effect size",
            ));
        }
        let filter_id = layer
            .attribute("filter")
            .unwrap_or("")
            .trim_start_matches("url(#")
            .trim_end_matches(')');
        let filter = parsed
            .descendants()
            .find(|n| n.attribute("id") == Some(filter_id));
        let blur = filter
            .into_iter()
            .flat_map(|n| n.descendants())
            .filter(|n| n.has_tag_name("feGaussianBlur"))
            .filter_map(|n| {
                n.attribute("stdDeviation")
                    .and_then(|v| v.parse::<f64>().ok())
            })
            .fold(0f64, f64::max);
        let spread = filter
            .into_iter()
            .flat_map(|n| n.descendants())
            .filter(|n| n.has_tag_name("feMorphology"))
            .filter_map(|n| n.attribute("radius").and_then(|v| v.parse::<f64>().ok()))
            .fold(0f64, f64::max);
        let offset = filter
            .into_iter()
            .flat_map(|n| n.descendants())
            .filter(|n| n.has_tag_name("feOffset"))
            .flat_map(|n| [n.attribute("dx"), n.attribute("dy")])
            .flatten()
            .filter_map(|v| v.parse::<f64>().ok())
            .map(f64::abs)
            .fold(0f64, f64::max);
        let overlap = ((4. * blur + spread + offset) * density).ceil() as u32 + 2;
        let tile = 1024u32;
        if (tile as u64 + 2 * overlap as u64).pow(2) > 16_000_000 {
            return Err(error(
                "PDF blur tile exceeds the 64MB buffer budget; reduce blur or DPI",
            ));
        }
        let mut images = String::new();
        for y in (0..height).step_by(tile as usize) {
            for x in (0..width).step_by(tile as usize) {
                let tw = tile.min(width - x);
                let th = tile.min(height - y);
                let pw = tw + 2 * overlap;
                let ph = th + 2 * overlap;
                let x0 = bbox.x() as f64 + (x as f64 - overlap as f64) / density;
                let y0 = bbox.y() as f64 + (y as f64 - overlap as f64) / density;
                // Bound the filter region itself, not just the output buffer. Otherwise
                // resvg primitives can allocate the complete (potentially huge) filter.
                let tile_defs = if let Some(filter) = filter {
                    let children: String = filter
                        .children()
                        .filter(|n| n.is_element())
                        .map(|n| &xml[n.range()])
                        .collect();
                    let bounded = format!(
                        "<filter id='{filter_id}' filterUnits='userSpaceOnUse' primitiveUnits='userSpaceOnUse' color-interpolation-filters='sRGB' x='{x0}' y='{y0}' width='{}' height='{}'>{children}</filter>",
                        pw as f64 / density,
                        ph as f64 / density
                    );
                    defs.replace(&xml[filter.range()], &bounded)
                } else {
                    defs.clone()
                };
                let isolated = format!(
                    "<svg xmlns='http://www.w3.org/2000/svg' xmlns:xlink='http://www.w3.org/1999/xlink' width='{pw}' height='{ph}' viewBox='{x0} {y0} {} {}'>{tile_defs}{}</svg>",
                    pw as f64 / density,
                    ph as f64 / density,
                    &xml[layer.range()]
                );
                let tree = tree_svg(scene, &isolated)?;
                let mut pix = tiny_skia::Pixmap::new(pw, ph)
                    .ok_or_else(|| error("Cannot allocate PDF effect tile"))?;
                resvg::render(&tree, tiny_skia::Transform::identity(), &mut pix.as_mut());
                let mut crop = tiny_skia::Pixmap::new(tw, th)
                    .ok_or_else(|| error("Cannot allocate PDF tile crop"))?;
                for row in 0..th as usize {
                    let from = ((row + overlap as usize) * pw as usize + overlap as usize) * 4;
                    let to = row * tw as usize * 4;
                    crop.data_mut()[to..to + tw as usize * 4]
                        .copy_from_slice(&pix.data()[from..from + tw as usize * 4]);
                }
                let bytes = crop.encode_png().map_err(|e| error(e.to_string()))?;
                images += &format!(
                    "<image x='{}' y='{}' width='{}' height='{}' preserveAspectRatio='none' href='data:image/png;base64,{}'/>",
                    bbox.x() as f64 + x as f64 / density,
                    bbox.y() as f64 + y as f64 / density,
                    tw as f64 / density,
                    th as f64 / density,
                    base64::engine::general_purpose::STANDARD.encode(bytes)
                );
            }
        }
        replacements.push((
            layer.range(),
            format!("<g data-laymesh-effect-raster='{dpi}'>{images}</g>"),
        ));
    }
    let mut output = xml.clone();
    for (range, value) in replacements.into_iter().rev() {
        output.replace_range(range, &value);
    }
    Ok(output)
}

/// Render vector PDF with subset embedded user fonts and searchable formula LaTeX source.
pub fn render_pdf(scene: &Scene) -> Result<Vec<u8>> {
    render_pdf_with_options(scene, &super::export::ExportOptions::default())
}
pub fn render_pdf_with_options(
    scene: &Scene,
    options: &super::export::ExportOptions,
) -> Result<Vec<u8>> {
    let optimized = super::raster_policy::pdf_scene(scene, options)?;
    let scene = &optimized;
    let xml = pdf_effect_layers(scene, options.dpi.unwrap_or(scene.export_dpi))?;
    let tree = tree_svg(scene, &xml)?;
    let size = Size::from_wh(
        (scene.width * 72. / 25.4) as f32,
        (scene.height * 72. / 25.4) as f32,
    )
    .ok_or_else(|| error("PDF 画布尺寸无效"))?;
    let mut doc = Document::new();
    let mut page = doc.start_page_with(PageSettings::new(size));
    let mut surface = page.surface();
    surface
        .draw_svg(&tree, size, SvgSettings::default())
        .ok_or_else(|| error("PDF SVG 绘制失败"))?;
    // A transparent text glyph gives formula outlines a selectable text range. Its ActualText
    // is the original LaTeX, not a visual transcription, and only a math font is used.
    let data = ratex_katex_fonts::ttf_bytes("KaTeX_Main-Regular.ttf")
        .ok_or_else(|| error("缺少公式字体"))?
        .into_owned();
    let face = ttf_parser::Face::parse(&data, 0).map_err(|_| error("无效公式字体"))?;
    let font = Font::new(data.clone().into(), 0).ok_or_else(|| error("PDF 公式字体读取失败"))?;
    surface.push_transform(&Transform::from_scale(
        (72. / 25.4) as f32,
        (72. / 25.4) as f32,
    ));
    for node in &scene.nodes {
        formula_semantics(node, &mut surface, &font, &face);
    }
    surface.pop();
    surface.finish();
    page.finish();
    doc.finish()
        .map_err(|e| error(format!("PDF 导出失败：{e:?}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn scene() -> Scene {
        Scene {
            schema_version: 8,
            width: 25.4,
            height: 12.7,
            background: json!("none"),
            layout_dpi: 96.,
            canvas_unit: "mm".into(),
            export_dpi: 300.,
            nodes: vec![],
            warnings: vec![],
            fonts: Default::default(),
        }
    }

    #[test]
    fn pdf_effects_keep_requested_resolution_above_5000_pixels() {
        let mut e = laymesh_core::engine::Engine::new(laymesh_core::model::Host::default());
        let scene=e.compile(r##"p=canvas(size=(170mm,20mm))
        p.add(rect(size=(150mm,4mm),fill="#ff0000",effects=[shadow(blur=0.2mm,offset=(0mm,1mm))]),offset=(8mm,5mm))"##,"tile.lay").unwrap();
        let xml = pdf_effect_layers(&scene, 1200.).unwrap();
        let doc = roxmltree::Document::parse(&xml).unwrap();
        let tiles: Vec<_> = doc
            .descendants()
            .filter(|n| n.has_tag_name("image"))
            .collect();
        assert!(tiles.len() >= 7);
        let mut width = 0.;
        for tile in tiles {
            width += tile.attribute("width").unwrap().parse::<f64>().unwrap();
            let data = tile
                .attribute("href")
                .unwrap()
                .strip_prefix("data:image/png;base64,")
                .unwrap();
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(data)
                .unwrap();
            let im = image::load_from_memory(&bytes).unwrap();
            assert!(im.width() <= 1024);
        }
        assert!(width * 1200. / 25.4 > 5000.);
        assert!(xml.contains("fill=\"#ff0000\""));
        assert!(!xml.contains("data-laymesh-effect=\"true\""));
        assert!(
            pdf_effect_layers(&scene, 25400.)
                .unwrap_err()
                .message
                .contains("pixel budget")
        );
    }
    #[test]
    fn png_resolution_and_transparency() {
        let png = render_png(&scene(), 300.).unwrap();
        let reader = png::Decoder::new(std::io::Cursor::new(png))
            .read_info()
            .unwrap();
        assert_eq!((reader.info().width, reader.info().height), (300, 150));
        assert_eq!(reader.info().pixel_dims.unwrap().xppu, 11811);
        assert_eq!(reader.info().color_type, png::ColorType::Rgba);
    }
    #[test]
    fn empty_font_scene_exports_all_backends() {
        let mut scene = scene();
        let mut fonts = laymesh_core::text::FontSystem::new(false);
        scene.nodes.push(
            laymesh_core::text::layout_text(
                &json!({"content":"A中文","font_size":3.}),
                None,
                &mut fonts,
                &mut scene.warnings,
                "x",
                Loc::default(),
            )
            .unwrap(),
        );
        scene.nodes.push(
            laymesh_core::text::formula(
                &json!({"content":r"\frac{a+b}{\sqrt{x}}","font_size":3.}),
                &mut fonts,
                &mut scene.warnings,
                "x",
                Loc::default(),
            )
            .unwrap(),
        );
        assert!(fonts.assets.is_empty());
        assert!(render_svg(&scene).unwrap().len() > 100);
        assert!(render_png(&scene, 96.).unwrap().len() > 100);
        let pdf = render_pdf(&scene).unwrap();
        assert!(pdf.starts_with(b"%PDF-"));
    }
}

#[cfg(test)]
mod extraction_tests {
    use super::*;
    #[test]
    fn pdf_text_and_formula_source_extract() {
        let mut fs = laymesh_core::text::FontSystem::new(false);
        fs.register_font(
            "test",
            include_bytes!("../../../tests/assets/GFSNeohellenic.otf").to_vec(),
        );
        let mut warnings = vec![];
        let mut text = laymesh_core::text::layout_text(
            &json!({"content":"Selectable body","font_size":4.}),
            None,
            &mut fs,
            &mut warnings,
            "test.lay",
            Loc::default(),
        )
        .unwrap();
        text["x"] = json!(2.);
        text["y"] = json!(2.);
        let mut formula = laymesh_core::text::formula(
            &json!({"source":r"\frac{a+b}{\sqrt{x}}","font_size":4.}),
            &mut fs,
            &mut warnings,
            "test.lay",
            Loc::default(),
        )
        .unwrap();
        formula["x"] = json!(2.);
        formula["y"] = json!(10.);
        let scene = Scene {
            schema_version: 8,
            width: 60.,
            height: 30.,
            background: json!("none"),
            layout_dpi: 96.,
            canvas_unit: "mm".into(),
            export_dpi: 96.,
            nodes: vec![text, formula],
            fonts: fs.assets,
            warnings,
        };
        let bytes = render_pdf(&scene).unwrap();
        let path = std::env::temp_dir().join(format!(
            "laymesh-extraction-{}-{}.pdf",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&path, bytes).unwrap();
        let result = std::process::Command::new("pdftotext")
            .arg("-raw")
            .arg(&path)
            .arg("-")
            .output();
        std::fs::remove_file(path).unwrap();
        let output =
            result.expect("pdftotext is required for native PDF extraction regression checks");
        assert!(output.status.success());
        let text = String::from_utf8_lossy(&output.stdout);
        assert!(text.contains("Selectable body"), "{text}");
        assert!(text.contains(r"\frac{a+b}{\sqrt{x}}"), "{text}");
    }
}
