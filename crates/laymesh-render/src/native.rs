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
    usvg::Tree::from_str(&svg(scene, false)?, &options)
        .map_err(|e| error(format!("SVG 渲染失败：{e}")))
}
/// Render transparent RGBA PNG at the requested physical resolution and write a pHYs chunk.
pub fn render_png(scene: &Scene, dpi: f64) -> Result<Vec<u8>> {
    if !dpi.is_finite() || dpi <= 0. || dpi > 25_400. {
        return Err(error("DPI 必须为 0–25400 之间的正数"));
    }
    let w = (scene.width * dpi / 25.4).round().max(1.);
    let h = (scene.height * dpi / 25.4).round().max(1.);
    if w * h > 100_000_000. || w > u32::MAX as f64 || h > u32::MAX as f64 {
        return Err(error("PNG 像素数超过上限"));
    }
    let tree = tree(scene)?;
    let mut pixmap = tiny_skia::Pixmap::new(w as u32, h as u32)
        .ok_or_else(|| error("无法分配 PNG 像素缓冲区"))?;
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
    let mut out = vec![];
    {
        let mut enc = png::Encoder::new(&mut out, w as u32, h as u32);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.set_pixel_dims(Some(png::PixelDimensions {
            xppu: (dpi / 0.0254).round() as u32,
            yppu: (dpi / 0.0254).round() as u32,
            unit: png::Unit::Meter,
        }));
        enc.set_source_srgb(png::SrgbRenderingIntent::Perceptual);
        let mut writer = enc.write_header().map_err(|e| error(e.to_string()))?;
        writer
            .write_image_data(&rgba)
            .map_err(|e| error(e.to_string()))?;
    }
    Ok(out)
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
/// Render vector PDF with subset embedded user fonts and searchable formula LaTeX source.
pub fn render_pdf(scene: &Scene) -> Result<Vec<u8>> {
    let tree = tree(scene)?;
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
            export_dpi: 300.,
            nodes: vec![],
            warnings: vec![],
            fonts: Default::default(),
        }
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
