use super::*;
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
    let frame = render_rgba(&to_display_list(scene)?, dpi)?;
    Ok((frame.width, frame.height, frame.rgba))
}

pub fn render_pdf(scene: &Scene) -> Result<Vec<u8>> {
    render_pdf_with_options(scene, &super::export::ExportOptions::default())
}
pub fn render_pdf_with_options(
    scene: &Scene,
    options: &super::export::ExportOptions,
) -> Result<Vec<u8>> {
    render_pdf_pages(std::iter::once(scene), options)
}
/// Append independently compiled pages to one vector PDF. Each page keeps its
/// physical size, text semantics and local raster/image policy.
pub fn render_pdf_pages<'a>(
    scenes: impl IntoIterator<Item = &'a Scene>,
    options: &super::export::ExportOptions,
) -> Result<Vec<u8>> {
    options.validate("pdf")?;
    let mut doc = PdfDocument::new();
    for scene in scenes {
        doc.append(scene, options)?;
    }
    doc.finish()
}
/// Allows document exporters to release each Scene after writing its page.
pub struct PdfDocument {
    pages: Vec<Vec<u8>>,
    count: usize,
}
impl Default for PdfDocument {
    fn default() -> Self {
        Self::new()
    }
}
impl PdfDocument {
    pub fn new() -> Self {
        Self {
            pages: Vec::new(),
            count: 0,
        }
    }
    pub fn append(&mut self, scene: &Scene, options: &super::export::ExportOptions) -> Result<()> {
        options.validate("pdf")?;
        let optimized = super::raster_policy::pdf_scene(scene, options)?;
        self.pages.push(super::pdf::render(
            &to_display_list(&optimized)?,
            options.dpi.unwrap_or(scene.export_dpi),
        )?);
        self.count += 1;
        Ok(())
    }
    pub fn finish(self) -> Result<Vec<u8>> {
        if self.count == 0 {
            return Err(error("PDF 文档至少需要一页"));
        }
        super::pdf::merge(self.pages)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn scene() -> Scene {
        Scene {
            media: Default::default(),
            page_transition: None,
            animation: None,
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
            font_requests: Default::default(),
        }
    }

    #[test]
    fn pdf_effects_keep_requested_resolution_above_5000_pixels() {
        let mut e = laymesh_core::engine::Engine::new(laymesh_core::model::Host::default());
        let scene=e.compile(r##"p=canvas(size=(170mm,20mm))
        p.add(rect(size=(150mm,4mm),fill="#ff0000",effects=[shadow(blur=0.2mm,offset=(0mm,1mm))]),offset=(8mm,5mm))"##,"tile.lay").unwrap();
        let list = to_display_list(&scene).unwrap();
        let tiles = raster::effect_tiles(&list, &list.nodes[0], 1200. / 25.4).unwrap();
        assert!(tiles.len() >= 7);
        assert!(tiles.iter().all(|t| t.pixels.width() <= 1024));
        assert!(tiles.iter().map(|t| t.rect.width()).sum::<f64>() * 1200. / 25.4 > 5000.);
        assert!(
            raster::effect_tiles(&list, &list.nodes[0], 25400. / 25.4)
                .err()
                .unwrap()
                .message
                .contains("budget")
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
            media: Default::default(),
            page_transition: None,
            animation: None,
            schema_version: 8,
            width: 60.,
            height: 30.,
            background: json!("none"),
            layout_dpi: 96.,
            canvas_unit: "mm".into(),
            export_dpi: 96.,
            nodes: vec![text, formula],
            fonts: fs.assets,
            font_requests: Default::default(),
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
