//! Export every distinct gallery formula through the real formula/SVG pipeline.
//! Input and metadata assembly live in experiments/opentype-math/build-gallery.py.
use kurbo::Shape;
use laymesh_core::{
    Loc,
    model::{Scene, jnum},
    text::{FontSystem, formula},
};
use serde_json::{Value, json};
use std::{collections::BTreeSet, error::Error, fs, path::PathBuf};
include!("../../../tests/fonts/text/fixtures.rs");

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: math_gallery INPUT.json OUTPUT_DIRECTORY".into());
    }
    let input: Vec<String> = serde_json::from_slice(&fs::read(&args[0])?)?;
    let output = PathBuf::from(&args[1]);
    fs::create_dir_all(output.join("svg"))?;
    let mut strict_fonts = FontSystem::new(false);
    let mut text_fonts = FontSystem::new(false);
    for (name, bytes) in [
        (
            "latinmodern",
            include_bytes!("../../../tests/fonts/math/latinmodern-math.otf").as_slice(),
        ),
        (
            "stix",
            include_bytes!("../../../tests/fonts/math/STIX2Math.otf").as_slice(),
        ),
        (
            "xits",
            include_bytes!("../../../tests/fonts/math/XITSMath-Regular.otf").as_slice(),
        ),
    ] {
        strict_fonts.register_font(&format!("/{name}.otf"), bytes.to_vec());
        text_fonts.register_font(&format!("/{name}.otf"), bytes.to_vec());
    }
    for (name, bytes) in TEXT_FONTS {
        text_fonts.register_font(name, bytes.to_vec());
    }
    let mut report = vec![];
    for (index, source) in input.iter().enumerate() {
        let mut record = json!({"source":source});
        for (policy, fonts) in [("strict", &mut strict_fonts), ("text", &mut text_fonts)] {
            let mut variants = serde_json::Map::new();
            for font in ["ratex-katex", "latinmodern", "stix", "xits"] {
                let mut styles = serde_json::Map::new();
                for style in ["inline", "display"] {
                    let request = if font == "ratex-katex" {
                        font.into()
                    } else {
                        format!("/{font}.otf")
                    };
                    let mut warnings = vec![];
                    let result = formula(
                        &json!({"source":source,"math_font":request,"font_size":5.,"style":style,
                        "math_text_fallback":policy=="text",
                        "font_family":if policy=="text"{json!(TEXT_FAMILY)}else{Value::Null}}),
                        fonts,
                        &mut warnings,
                        "/gallery.lay",
                        Loc::default(),
                    );
                    let row = match result {
                        Err(e) => json!({"error":e.message,"code":e.code}),
                        Ok(mut node) => 'render: {
                            let (w, h) = (jnum(&node, "width", 0.), jnum(&node, "height", 0.));
                            let invisible = node["items"].as_array().is_some_and(|a| a.is_empty());
                            let mut bounds = kurbo::Rect::new(0., 0., w.max(0.), h.max(0.));
                            let mut violations = vec![];
                            let mut glyph_fonts = BTreeSet::new();
                            let mut fallback_families = BTreeSet::new();
                            let used_text_fonts = node["mathTextFonts"].clone();
                            for face in used_text_fonts.as_array().into_iter().flatten() {
                                if let Some(name) = face["family"].as_str() {
                                    fallback_families.insert(name.to_owned());
                                }
                            }
                            let placeholders =
                                node["mathMissingGlyphs"].as_array().map_or(0, Vec::len);
                            for item in node["items"].as_array().ok_or("formula items absent")? {
                                let ink = if item["kind"] == "path" {
                                    kurbo::BezPath::from_svg(
                                        item["d"].as_str().ok_or("path absent")?,
                                    )?
                                    .bounding_box()
                                } else if item["kind"] == "rule" || item["kind"] == "box" {
                                    let x = jnum(item, "x", 0.);
                                    let y = jnum(item, "y", 0.);
                                    kurbo::Rect::new(
                                        x,
                                        y,
                                        x + jnum(item, "width", 0.),
                                        y + jnum(item, "height", 0.),
                                    )
                                } else if item["kind"] == "glyph" {
                                    // The default RaTeX backend has a text fallback
                                    // for characters outside its own math faces.
                                    // Keep the real exporter behavior and mark it.
                                    if let Some(name) = item["fontFamily"].as_str() {
                                        glyph_fonts.insert(name.to_string());
                                    }
                                    if let Some(name) = item["fontSystemFamily"].as_str() {
                                        fallback_families.insert(name.to_string());
                                    }
                                    continue;
                                } else {
                                    return Err(format!("unexpected formula item: {item}").into());
                                };
                                if ink.x0 < -1e-6
                                    || ink.y0 < -1e-6
                                    || ink.x1 > w + 1e-6
                                    || ink.y1 > h + 1e-6
                                {
                                    violations.push(format!("outline {ink:?} exceeds {w}x{h}"));
                                }
                                bounds = bounds.union(ink);
                            }
                            if ![w, h, bounds.width(), bounds.height()]
                                .iter()
                                .all(|v| v.is_finite())
                                || (!invisible && (w < 0. || h < 0.))
                            {
                                if font != "ratex-katex" {
                                    return Err(
                                        format!("invalid formula geometry: {source}").into()
                                    );
                                }
                                break 'render json!({"preview_error":format!("原排版返回无效尺寸：{w} × {h} mm"),
                                "preview_error_code":"E_GALLERY_GEOMETRY","layout_width":w,"layout_height":h,
                                "bounds_errors":["non-finite or negative dimensions"],"warnings":warnings});
                            }
                            if font != "ratex-katex" && !violations.is_empty() {
                                return Err(format!(
                                    "OpenType outline bounds: {source}: {violations:?}"
                                )
                                .into());
                            }
                            // Include the entire ink area, also for old KaTeX metrics
                            // with overhangs. This changes the viewport, never the layout.
                            let x = 0.5 - bounds.x0;
                            let y = 0.5 - bounds.y0;
                            let ascent = jnum(&node, "ascent", h) + y;
                            node["x"] = json!(x);
                            node["y"] = json!(y);
                            let scene = Scene {
                                schema_version: 1,
                                width: bounds.width() + 1.,
                                height: bounds.height() + 1.,
                                background: json!("transparent"),
                                layout_dpi: 96.,
                                canvas_unit: "mm".into(),
                                export_dpi: 1200.,
                                nodes: vec![node],
                                warnings: vec![],
                                fonts: fonts
                                    .assets
                                    .iter()
                                    .filter(|(key, _)| glyph_fonts.contains(*key))
                                    .map(|(key, value)| (key.clone(), value.clone()))
                                    .collect(),
                            };
                            let svg = laymesh_render::render_svg(&scene)?;
                            if font != "ratex-katex"
                                && (svg.contains("<text") || svg.contains("@font-face"))
                            {
                                return Err(
                                    format!("math export must contain outlines: {source}").into()
                                );
                            }
                            let file = format!("svg/{index}-{font}-{policy}-{style}.svg");
                            fs::write(output.join(&file), svg)?;
                            json!({"svg":file,"width":scene.width,"height":scene.height,"ascent":ascent,
                            "layout_width":w,"layout_height":h,"bounds_errors":violations,"warnings":warnings,
                            "text_fallback":!fallback_families.is_empty(),"fallback_families":fallback_families,
                            "text_fonts":used_text_fonts,"text_policy":policy,
                            "placeholder_glyphs":placeholders,"invisible":invisible})
                        }
                    };
                    styles.insert(style.into(), row);
                }
                variants.insert(font.into(), Value::Object(styles));
            }
            record[if policy == "text" {
                "text_variants"
            } else {
                "variants"
            }] = Value::Object(variants);
        }
        report.push(record);
        if (index + 1) % 250 == 0 {
            eprintln!("Exported {} / {} formulas", index + 1, input.len());
        }
    }
    fs::write(output.join("renders.json"), serde_json::to_vec(&report)?)?;
    println!(
        "Exported {} distinct formulas, four fonts, two styles and two text policies",
        input.len()
    );
    Ok(())
}
