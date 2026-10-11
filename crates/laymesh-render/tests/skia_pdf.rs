#![cfg(feature = "native")]
use laymesh_core::{asset_cache, engine::compile_source, model::Host};
use laymesh_render::{ExportOptions, PdfDocument, render_pdf};
use lopdf::{Document, Object};
use serde_json::json;

#[test]
fn formula_source_has_a_to_unicode_fallback_without_actual_text_support() {
    let formulas = [
        r"\frac{a}{\sqrt{b^2}}",
        r"\begin{matrix}a&b\\c&d\end{matrix}",
    ];
    let mut source = "p=canvas(size=(100,70))\n".to_owned();
    for (i, formula) in formulas.iter().enumerate() {
        source += &format!(
            "p.add(formula({}),offset=(5,{}))\n",
            serde_json::to_string(formula).unwrap(),
            5 + i * 30
        );
    }
    let scene = compile_source(&source, "semantic-fallback.lay", Host::default()).unwrap();
    let bytes = render_pdf(&scene).unwrap();
    // lopdf decodes the font ToUnicode map without reading ActualText tags.
    let text = Document::load_mem(&bytes)
        .unwrap()
        .extract_text(&[1])
        .unwrap();
    let text: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    for formula in formulas {
        assert_eq!(text.matches(formula).count(), 1, "{text}");
    }
}

#[test]
fn pdf_preserves_exact_16bit_color_and_alpha_samples() {
    for gray in [false, true] {
        let rgba = image::ImageBuffer::from_fn(7, 5, |x, y| {
            image::Rgba([
                1234 + x as u16 * 137,
                32123 + y as u16 * 229,
                65531 - x as u16 * 97,
                17 + (x + y * 7) as u16 * 1789,
            ])
        });
        let image = if gray {
            image::DynamicImage::ImageLumaA16(image::ImageBuffer::from_fn(7, 5, |x, y| {
                image::LumaA([rgba.get_pixel(x, y)[0], rgba.get_pixel(x, y)[3]])
            }))
        } else {
            image::DynamicImage::ImageRgba16(rgba.clone())
        };
        let expected_color: Vec<u8> = rgba
            .pixels()
            .flat_map(|p| {
                p.0[..if gray { 1 } else { 3 }]
                    .iter()
                    .flat_map(|v| v.to_be_bytes())
                    .collect::<Vec<_>>()
            })
            .collect();
        let expected_alpha: Vec<u8> = rgba.pixels().flat_map(|p| p[3].to_be_bytes()).collect();
        let mut scene = compile_source(
            "p=canvas(size=(70mm,50mm))",
            "precision.lay",
            Host::default(),
        )
        .unwrap();
        scene.nodes = vec![
            json!({"kind":"image","mime":"image/png","rasterKey":asset_cache::insert(image),"width":70.,"height":50.,"intrinsicWidth":7,"intrinsicHeight":5,"fit":"contain","x":0,"y":0}),
        ];
        let bytes = render_pdf(&scene).unwrap();
        let doc = Document::load_mem(&bytes).unwrap();
        let images: Vec<_> = doc
            .objects
            .values()
            .filter_map(|o| o.as_stream().ok())
            .filter(|s| s.dict.get(b"SMask").is_ok())
            .collect();
        assert_eq!(images.len(), 1);
        let image = images[0];
        assert_eq!(
            image
                .dict
                .get(b"BitsPerComponent")
                .unwrap()
                .as_i64()
                .unwrap(),
            16
        );
        assert_eq!(
            image
                .decompressed_content()
                .unwrap_or_else(|_| image.content.clone()),
            expected_color
        );
        let alpha = doc
            .get_object(image.dict.get(b"SMask").unwrap().as_reference().unwrap())
            .unwrap()
            .as_stream()
            .unwrap();
        assert_eq!(
            alpha
                .dict
                .get(b"BitsPerComponent")
                .unwrap()
                .as_i64()
                .unwrap(),
            16
        );
        assert_eq!(
            alpha
                .decompressed_content()
                .unwrap_or_else(|_| alpha.content.clone()),
            expected_alpha
        );
    }
}

#[test]
fn multipage_pdf_retains_structure_parent_trees_and_exact_sizes() {
    let mut pdf = PdfDocument::new();
    for (w, h) in [(160., 90.), (90., 160.)] {
        let source = format!(
            "p=canvas(size=({w}mm,{h}mm))\np.add(text(\"中文测试\"),offset=(5mm,5mm))\np.add(formula(\"x^2+1\"),offset=(5mm,15mm))"
        );
        let scene = compile_source(&source, "pages.lay", Host::default()).unwrap();
        pdf.append(&scene, &ExportOptions::default()).unwrap();
    }
    let bytes = pdf.finish().unwrap();
    let doc = Document::load_mem(&bytes).unwrap();
    let root = doc.catalog().unwrap().get(b"StructTreeRoot").unwrap();
    let root_id = root.as_reference().unwrap();
    let root = doc.dereference(root).unwrap().1.as_dict().unwrap();
    let children = root.get(b"K").unwrap().as_array().unwrap();
    assert_eq!(children.len(), 2);
    for child in children {
        let d = doc.dereference(child).unwrap().1.as_dict().unwrap();
        assert_eq!(d.get(b"P").unwrap().as_reference().unwrap(), root_id);
    }
    let mut parents = vec![];
    for ((_, id), (w, h)) in doc.get_pages().into_iter().zip([(160., 90.), (90., 160.)]) {
        let d = doc.get_object(id).unwrap().as_dict().unwrap();
        let b = d.get(b"MediaBox").unwrap().as_array().unwrap();
        assert!((b[2].as_float().unwrap() as f64 - w * 72. / 25.4).abs() < 0.0001);
        assert!((b[3].as_float().unwrap() as f64 - h * 72. / 25.4).abs() < 0.0001);
        parents.push(d.get(b"StructParents").unwrap().as_i64().unwrap());
    }
    assert_ne!(parents[0], parents[1]);
    let tree = doc
        .dereference(root.get(b"ParentTree").unwrap())
        .unwrap()
        .1
        .as_dict()
        .unwrap();
    let keys: Vec<_> = tree
        .get(b"Nums")
        .unwrap()
        .as_array()
        .unwrap()
        .chunks_exact(2)
        .map(|p| p[0].as_i64().unwrap())
        .collect();
    assert!(parents.iter().all(|p| keys.contains(p)));
    assert!(
        doc.objects
            .values()
            .any(|o| matches!(o,Object::Dictionary(d) if d.get(b"ActualText").is_ok()))
    );
}

#[test]
fn opacity_groups_keep_distinct_formula_actual_text() {
    let scene=compile_source("p=canvas(size=(80,40))\np.add(formula(\"x+1\"),offset=(5,5),opacity=0.5)\np.add(formula(\"y+2\"),offset=(5,20),opacity=0.6)","semantic-groups.lay",Host::default()).unwrap();
    let bytes = render_pdf(&scene).unwrap();
    let doc = Document::load_mem(&bytes).unwrap();
    let mut texts = vec![];
    for object in doc.objects.values() {
        if let Ok(stream) = object.as_stream() {
            let data = stream
                .decompressed_content()
                .unwrap_or_else(|_| stream.content.clone());
            if let Ok(content) = lopdf::content::Content::decode(&data) {
                for op in content.operations {
                    if op.operator == "BDC" {
                        if let Some(bytes) = op
                            .operands
                            .get(1)
                            .and_then(|o| o.as_dict().ok())
                            .and_then(|d| d.get(b"ActualText").ok())
                            .and_then(|o| o.as_str().ok())
                        {
                            let u: Vec<_> = bytes[2..]
                                .chunks_exact(2)
                                .map(|p| u16::from_be_bytes([p[0], p[1]]))
                                .collect();
                            texts.push(String::from_utf16(&u).unwrap());
                        }
                    }
                }
            }
        }
    }
    texts.sort();
    assert_eq!(texts, vec!["x+1", "y+2"]);
}
