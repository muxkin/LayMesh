#![cfg(feature = "native")]
//! Assertion-level ports of acceptance/font-stack/typography/vector/outline at 78db22d.
//! Captures supply source and immutable resources, never the expected answer.
use laymesh_core::{
    engine::compile_source,
    model::{Host, Scene, jnum},
};
use laymesh_render::{render_pdf, render_png, render_svg};
use serde_json::{Value, json};
use std::{io::Cursor, path::PathBuf, process::Command};
fn capture(set: &str, line: usize) -> Value {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../migration/corpus");
    serde_json::from_str(
        std::fs::read_to_string(root.join(format!("cases-{set}.jsonl")))
            .unwrap()
            .lines()
            .nth(line - 1)
            .unwrap(),
    )
    .unwrap()
}
fn result(set: &str, line: usize) -> laymesh_core::Result<Scene> {
    let c = capture(set, line);
    let mut host = Host::default();
    let blobs = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../migration/corpus/blobs");
    for (name, hash) in c["files"].as_object().unwrap() {
        host.files.insert(
            name.clone(),
            std::fs::read(blobs.join(hash.as_str().unwrap())).unwrap(),
        );
    }
    compile_source(
        &laymesh_core::migration::migrate_arrows(c["source"].as_str().unwrap()),
        c["file"].as_str().unwrap(),
        host,
    )
}
fn scene(set: &str, line: usize) -> Scene {
    result(set, line).unwrap()
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-6, "{a} != {b}");
}
fn ids(s: &Scene) -> Vec<&str> {
    s.nodes.iter().map(|n| n["id"].as_str().unwrap()).collect()
}
fn png(s: &Scene, dpi: f64) -> (png::OutputInfo, Vec<u8>, Option<png::PixelDimensions>) {
    let mut reader = png::Decoder::new(Cursor::new(render_png(s, dpi).unwrap()))
        .read_info()
        .unwrap();
    let dims = reader.info().pixel_dims;
    let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut pixels).unwrap();
    (info, pixels, dims)
}
fn pixel(s: &Scene, dpi: f64, x: f64, y: f64) -> [u8; 3] {
    let (i, b, _) = png(s, dpi);
    let p = ((y * dpi / 25.4).round() as usize * i.width as usize
        + (x * dpi / 25.4).round() as usize)
        * 4;
    [b[p], b[p + 1], b[p + 2]]
}
fn pdf(s: &Scene) -> String {
    String::from_utf8_lossy(&render_pdf(s).unwrap()).into_owned()
}
fn no_images(s: &Scene) {
    assert!(
        !pdf(s)
            .split_whitespace()
            .collect::<String>()
            .contains("/Subtype/Image")
    );
}
fn all_exports(s: &Scene) {
    assert!(render_svg(s).unwrap().contains("<path "));
    assert!(render_pdf(s).unwrap().len() > 1000);
    assert!(render_png(s, 96.).unwrap().len() > 100);
}
fn located(set: &str, rows: &[(usize, &str)], exact_line: Option<usize>) {
    for &(line, code) in rows {
        let e = result(set, line).unwrap_err();
        assert_eq!(e.code, code, "{set}:{line}");
        assert_eq!(e.file, capture(set, line)["file"].as_str().unwrap());
        assert!(e.loc.line > 0 && e.loc.column > 0);
        if let Some(line) = exact_line {
            assert_eq!(e.loc.line, line);
        }
    }
}
#[test]
fn legacy_reused_images_preserve_crop_size_anchors_order_and_opacity() {
    let s = scene("1044526", 1);
    assert_eq!(s.schema_version, 8);
    assert_eq!(s.nodes.len(), 2);
    for (i, values) in [[8., 12., 80., 40., 32., 1.], [92., 12., 40., 40., 16., 0.5]]
        .iter()
        .enumerate()
    {
        for (k, v) in ["x", "y", "width", "height", "intrinsicWidth", "opacity"]
            .iter()
            .zip(values)
        {
            close(jnum(&s.nodes[i], k, 1.), *v);
        }
    }
    let svg = render_svg(&s).unwrap();
    assert!(svg.find("data-id=\"first\"").unwrap() < svg.find("data-id=\"second\"").unwrap());
    assert!(pdf(&s).contains("/ca 0.5"));
}
#[test]
fn legacy_bare_placements_and_group_anchors() {
    let s = scene("1044526", 2);
    assert_eq!(ids(&s), ["@1", "named", "@2", "@4"]);
    assert_eq!(s.nodes[2]["x"], 90.);
    assert_eq!(s.nodes[2]["y"], 12.);
    assert_eq!(s.nodes[3]["children"][0]["id"], "@3");
    located("1044526", &[(3, "E_STATEMENT")], Some(2));
    for (row, right) in [(6, 22.), (7, 32.)] {
        let s = scene("1044526", row);
        assert_eq!(s.nodes[1]["x"], right);
        assert_eq!(s.nodes[2]["width"], 23.);
        assert_eq!(s.nodes[2]["x"], 77.);
        assert_eq!(s.nodes[2]["children"][1]["x"], 13.);
    }
}
#[test]
fn legacy_canvas_background_and_physical_output_dimensions() {
    let s = scene("1044526", 4);
    assert_eq!(s.background, "#ffffff");
    assert!(s.nodes.is_empty());
    let svg = render_svg(&s).unwrap();
    assert!(svg.contains("<rect width='10' height='10' fill='#ffffff'"));
    let (_, bytes, _) = png(&s, 96.);
    assert_eq!(&bytes[..4], &[255, 255, 255, 255]);
    assert!(pdf(&s).contains("/MediaBox"));
    located("1044526", &[(5, "E_COLOR")], Some(1));
    let mut s = scene("1044526", 8);
    close(jnum(&s.nodes[0], "width", 0.), 25.4);
    assert!(
        render_svg(&s)
            .unwrap()
            .contains("width=\"180mm\" height=\"120mm\"")
    );
    let structure = pdf(&s);
    let at = structure.find("/MediaBox").unwrap();
    let bbox = &structure[at..];
    let bbox = &bbox[bbox.find('[').unwrap() + 1..bbox.find(']').unwrap()];
    let values: Vec<f64> = bbox
        .split_whitespace()
        .map(|v| v.parse().unwrap())
        .collect();
    assert_eq!(&values[..2], &[0., 0.]);
    assert!((values[2] - 180. * 72. / 25.4).abs() < 0.001);
    assert!((values[3] - 120. * 72. / 25.4).abs() < 0.001);
    let (i, _, dims) = png(&s, 300.);
    assert_eq!((i.width, i.height), (2126, 1417));
    assert!((dims.unwrap().xppu as f64 * 0.0254 - 300.).abs() <= 1.);
    close(jnum(&s.nodes[0], "width", 0.), 25.4);
    s.width = 1.;
    s.height = 3.;
    s.nodes.clear();
    let (i, _, _) = png(&s, 96.);
    assert_eq!((i.width, i.height), (4, 11));
}
#[test]
fn legacy_svg_input_and_body_text_stay_vector_and_all_image_formats_load() {
    let s = scene("1044526", 9);
    let svg = render_svg(&s).unwrap();
    assert!(svg.contains("<text ") && svg.contains("Select me"));
    assert!(svg.contains("<path "));
    assert!(pdf(&s).contains("/FontFile2"));
    no_images(&s);
    for row in 10..=14 {
        let s = scene("1044526", row);
        assert_eq!(s.nodes[0]["kind"], "image");
        assert!(!s.nodes[0]["data"].as_str().unwrap().is_empty());
    }
}
#[test]
fn legacy_asset_and_syntax_failures_keep_diagnostics() {
    located(
        "1044526",
        &[
            (15, "E_TIFF"),
            (16, "E_TIFF"),
            (17, "E_ASSET"),
            (18, "E_UNIT"),
            (19, "E_NAME"),
            (20, "E_CALL"),
            (21, "E_SYNTAX"),
            (22, "E_SVG"),
            (23, "E_SVG"),
        ],
        Some(2),
    );
}
#[test]
fn legacy_ordered_font_stacks_cover_chinese_and_combining_graphemes() {
    let s = scene("1044527", 1);
    let runs = s.nodes[0]["runs"].as_array().unwrap();
    assert_eq!(runs[0]["fontSystemFamily"], "DejaVu Sans");
    assert!(
        runs.iter()
            .any(|r| r["content"].as_str().unwrap_or("").contains("中文")
                && r["fontSystemFamily"] == "Noto Sans CJK SC")
    );
    assert!(
        runs.iter()
            .any(|r| r["content"].as_str().unwrap_or("").contains("e\u{301}")
                && r["fontSystemFamily"] == "DejaVu Sans")
    );
    assert_eq!(s.warnings.iter().filter(|w| w.code == "W_FONT").count(), 0);
}
#[test]
fn legacy_missing_glyphs_are_vector_boxes_and_import_paths_and_lists_are_checked() {
    let s = scene("1044527", 2);
    let runs = s.nodes[0]["runs"].as_array().unwrap();
    assert_eq!(
        runs.iter()
            .map(|r| r["content"].as_str().unwrap_or(""))
            .collect::<String>(),
        "A□□□"
    );
    assert_eq!(runs.iter().filter(|r| r["kind"] == "box").count(), 3);
    assert_eq!(s.warnings.len(), 1);
    assert!(
        s.warnings[0].message.contains("U+10FFFF") && s.warnings[0].message.contains("U+10FFFE")
    );
    assert!(
        s.warnings[0].message.find("U+10FFFF").unwrap()
            < s.warnings[0].message.find("U+10FFFE").unwrap()
    );
    assert_eq!(s.warnings[0].loc.line, 2);
    assert!(jnum(&s.nodes[0], "width", 0.) > 0.);
    for bytes in [
        render_svg(&s).unwrap().into_bytes(),
        render_pdf(&s).unwrap(),
        render_png(&s, 96.).unwrap(),
    ] {
        assert!(bytes.len() > 100);
    }
    let s = scene("1044527", 3);
    let file = capture("1044527", 3)["file"].as_str().unwrap().to_owned();
    assert_eq!(
        s.nodes[0]["runs"][0]["fontPath"],
        PathBuf::from(file)
            .parent()
            .unwrap()
            .join("parts/font.ttf")
            .to_str()
            .unwrap()
            .replace('\\', "/")
    );
    located(
        "1044527",
        &[(4, "E_FONT"), (5, "E_FONT"), (6, "E_FONT")],
        Some(2),
    );
    let s = scene("1044527", 7);
    assert!(
        s.warnings
            .iter()
            .any(|w| w.code == "W_FONT" && w.message.contains("Unavailable Test Font"))
    );
    assert!(render_svg(&s).unwrap().contains("时间"));
}
#[test]
fn legacy_rich_spans_reflow_with_color_and_measured_alignment() {
    let s = scene("1044611", 1);
    assert_eq!(s.schema_version, 8);
    let (a, b) = (&s.nodes[0], &s.nodes[1]);
    assert_eq!(a["width"], 22.);
    assert_eq!(b["width"], 55.);
    assert!(jnum(a, "height", 0.) > jnum(b, "height", 0.));
    close(
        jnum(b, "y", 0.),
        jnum(a, "y", 0.) + jnum(a, "height", 0.) + 2.,
    );
    let runs = a["runs"].as_array().unwrap();
    let mut colors = vec![];
    for r in runs {
        if r["kind"] == "glyph" && !colors.contains(&r["color"]) {
            colors.push(r["color"].clone());
        }
    }
    assert_eq!(colors, [json!("#ff0000"), json!("#0000ff")]);
    assert!(
        runs.iter()
            .any(|r| r["kind"] == "glyph" && jnum(r, "x", 0.) > 0.)
    );
    let svg = render_svg(&s).unwrap();
    assert!(svg.contains("<text") && svg.contains("#ff0000"));
    let (i, _, _) = png(&s, 144.);
    assert_eq!((i.width, i.height), (510, 397));
    let s = scene("1044611", 3);
    assert!(jnum(&s.nodes[0]["runs"][0], "x", 0.) > 0.);
    let runs = s.nodes[1]["runs"].as_array().unwrap();
    let first = &runs[0]["baseline"];
    let last = &runs.last().unwrap()["baseline"];
    assert_ne!(first, last);
    assert!(
        runs.iter()
            .any(|r| &r["baseline"] == first && r["content"] == " " && jnum(r, "x", 0.) > 0.)
    );
    let last: Vec<_> = runs.iter().filter(|r| &r["baseline"] == last).collect();
    assert_eq!(last[0]["x"], 0.);
    assert!(jnum(last.last().unwrap(), "x", 0.) < 20.);
}
#[test]
fn legacy_unavailable_fonts_warn_and_ttc_otc_faces_are_embedded_correctly() {
    let s = scene("1044611", 2);
    assert!(s.nodes.iter().all(|n| {
        n["kind"] == "text"
            && n["runs"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["kind"] == "glyph" && r["fontPath"].is_string())
    }));
    assert!(s.warnings.iter().any(|w| w.code == "W_FONT"
        && w.file == capture("1044611", 2)["file"].as_str().unwrap()
        && w.loc.line == 5
        && w.message.contains("Definitely Missing Font Family")));
    assert!(render_pdf(&s).unwrap().len() > 1000);
    for (second, first, name, family, mime) in [
        (4, 5, "LayMeshTestSecond", "LayMesh Test Second", "font/ttf"),
        (6, 7, "LayMeshOTFSecond", "LayMesh OTF Second", "font/otf"),
    ] {
        let s = scene("1044611", second);
        let r = &s.nodes[0]["runs"][0];
        assert_eq!(r["fontFace"], name);
        if second == 4 {
            assert_eq!(r["fontSystemFamily"], family);
        }
        let font = &s.fonts[r["fontFamily"].as_str().unwrap()];
        assert!(font.data.len() > 1000);
        if second == 6 {
            assert!(font.data.starts_with(b"OTTO"));
        }
        assert!(
            render_svg(&s)
                .unwrap()
                .contains(&format!("data:{mime};base64"))
        );
        assert!(render_pdf(&s).unwrap().len() > 1000);
        assert_ne!(
            render_png(&s, 144.).unwrap(),
            render_png(&scene("1044611", first), 144.).unwrap()
        );
        let (i, bytes, _) = png(&s, 144.);
        assert!(i.width > 200);
        assert!(bytes.chunks_exact(4).any(|p| p[0] < 100));
    }
    let s = scene("1044611", 8);
    let file = capture("1044611", 8)["file"].as_str().unwrap().to_owned();
    assert_eq!(
        s.nodes[0]["runs"][0]["fontPath"],
        PathBuf::from(file)
            .parent()
            .unwrap()
            .join("组件/fonts/local.ttc")
            .to_str()
            .unwrap()
            .replace('\\', "/")
    );
    assert!(s.warnings.is_empty());
}
fn with_pdf(s: &Scene, check: impl FnOnce(&std::path::Path)) {
    let path = std::env::temp_dir().join(format!(
        "laymesh-legacy-export-{}-{}.pdf",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::write(&path, render_pdf(s).unwrap()).unwrap();
    check(&path);
    std::fs::remove_file(path).unwrap();
}
#[test]
fn legacy_formulas_keep_vectors_source_selection_and_following_terms() {
    let s = scene("1044611", 9);
    assert!(
        s.nodes[0]["runs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["kind"] == "formula")
    );
    assert_eq!(s.nodes[1]["kind"], "formula");
    let svg = render_svg(&s).unwrap();
    assert!(svg.contains(r#"data-latex-source="\frac{a}{\sqrt{b^2}}""#));
    assert!(svg.contains("<path "));
    no_images(&s);
    with_pdf(&s, |path| {
        let extracted = Command::new("pdftotext")
            .arg("-raw")
            .arg(path)
            .arg("-")
            .output()
            .expect("pdftotext is required for formula source extraction regression checks");
        assert!(extracted.status.success());
        let text = String::from_utf8_lossy(&extracted.stdout);
        for value in [
            r"\frac{a}{\sqrt{b^2}}",
            "follows",
            r"\begin{matrix}a&b\\c&d\end{matrix}",
        ] {
            assert!(text.contains(value), "{text}");
        }
        let out = Command::new("mutool")
            .args(["draw", "-F", "stext"])
            .arg(path)
            .output()
            .expect("mutool is required for PDF formula character-position regression checks");
        assert!(out.status.success());
        let text = String::from_utf8_lossy(&out.stdout);
        let document = roxmltree::Document::parse(&text).unwrap();
        let mut found = 0;
        for font in document.descendants().filter(|n| n.has_tag_name("font")) {
            let chars: Vec<_> = font.children().filter(|n| n.has_tag_name("char")).collect();
            let value = chars
                .iter()
                .filter_map(|n| n.attribute("c"))
                .collect::<String>();
            if value.contains("\\frac") || value.contains("\\begin") {
                found += 1;
                let mut positions: Vec<_> = chars.iter().filter_map(|n| n.attribute("x")).collect();
                positions.sort();
                positions.dedup();
                assert!(positions.len() > 5);
            }
        }
        assert_eq!(found, 2);
    });
    let (i, _, _) = png(&s, 144.);
    assert_eq!(i.width, 425);
    let s = scene("1044611", 10);
    for n in &s.nodes {
        assert_eq!(n["kind"], "formula");
        assert!(jnum(n, "width", 0.) > 4.);
        assert!(n["items"].as_array().unwrap().len() >= 4);
    }
    assert_eq!(s.nodes[0]["source"], "I-I_0");
    assert_eq!(s.nodes[1]["source"], r"t\;(\mathrm{s})");
    assert!(
        render_svg(&s)
            .unwrap()
            .contains("data-latex-source=\"I-I_0\"")
    );
    render_png(&s, 180.).unwrap();
    render_pdf(&s).unwrap();
}
#[test]
fn legacy_formula_failures_and_math_font_mapping_are_located() {
    located("1044611", &[(11, "E_FORMULA"), (14, "E_FORMULA")], Some(2));
    let s = scene("1044611", 12);
    assert_eq!(s.nodes[0]["mathFont"], "ratex-katex");
    assert!(
        s.warnings
            .iter()
            .any(|w| w.code == "W_FONT" && w.loc.line == 2)
    );
    let s = scene("1044611", 13);
    assert_eq!(s.nodes[0]["kind"], "formula");
    assert!(render_png(&s, 96.).unwrap().len() > 100);
}
#[test]
fn legacy_vector_presets_holes_gradients_dashes_and_selectable_spans() {
    let s = scene("1044977", 1);
    assert_eq!(s.schema_version, 8);
    assert_eq!(s.nodes.len(), 9);
    assert!(
        s.nodes[..8].iter().all(|n| n["kind"] == "path"),
        "{:?}",
        s.nodes[..8]
            .iter()
            .map(|n| (&n["kind"], &n["strokeStyle"], &n["children"]))
            .collect::<Vec<_>>()
    );
    assert_eq!(s.nodes[0]["fillRule"], "evenodd");
    assert_eq!(s.nodes[0]["strokeStyle"]["dash"], json!([2., 1., 0.5, 1.]));
    assert_eq!(
        s.nodes[8]["runs"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["kind"] == "glyph")
            .map(|r| r["color"].clone())
            .collect::<Vec<_>>(),
        [json!("#ff0000"), json!("#0000ff")]
    );
    let svg = render_svg(&s).unwrap();
    for value in [
        "<linearGradient",
        "<radialGradient",
        "fill-rule=\"evenodd\"",
        "stroke-dasharray=\"2 1 0.5 1\"",
        "fill=\"#ff0000\"",
    ] {
        assert!(svg.contains(value), "{value}");
    }
    let document = roxmltree::Document::parse(&svg).unwrap();
    assert!(
        document
            .descendants()
            .any(|n| n.has_tag_name("text") && n.attribute("fill") == Some("#ff0000"))
    );
    assert!(pdf(&s).contains("/FontFile2"));
    no_images(&s);
    let (i, _, _) = png(&s, 144.);
    assert_eq!((i.width, i.height), (567, 454));
    assert_eq!(pixel(&s, 144., 15., 15.), [255, 255, 255]);
}
#[test]
fn legacy_fuse_replaces_only_selected_instances_and_remains_reusable() {
    let a = scene("1044977", 2);
    let b = scene("1044977", 3);
    let c = scene("1044977", 4);
    assert_eq!(ids(&a), ["spare", "merged", "after"]);
    assert_eq!(a.nodes[0]["kind"], "path");
    assert_eq!(a.nodes[0]["geometryRecipe"]["kind"], "rect");
    assert_eq!(a.nodes[1]["kind"], "path");
    assert_eq!(c.nodes[1]["kind"], "path");
    assert_ne!(a.nodes[1]["d"], b.nodes[1]["d"]);
    assert_ne!(a.nodes[1]["d"], c.nodes[1]["d"]);
    assert_ne!(render_png(&a, 96.).unwrap(), render_png(&c, 96.).unwrap());
    for s in [&a, &b, &c] {
        all_exports(s);
    }
    let s = scene("1044977", 5);
    assert_eq!(s.nodes.len(), 2);
    for n in &s.nodes {
        assert_eq!(
            n["children"]
                .as_array()
                .unwrap()
                .iter()
                .map(|n| n["id"].as_str().unwrap())
                .collect::<Vec<_>>(),
            ["joint"]
        );
    }
    assert!(render_png(&s, 96.).unwrap().len() > 100);
    for row in [6, 7] {
        let s = scene("1044977", row);
        assert_eq!(s.nodes.len(), 1);
        assert_eq!(s.nodes[0]["kind"], "path");
        assert!(render_png(&s, 96.).unwrap().len() > 100);
    }
}
#[test]
fn legacy_fuse_and_path_errors_keep_codes_files_and_locations() {
    located(
        "1044977",
        &[
            (8, "E_FUSE"),
            (9, "E_VALUE"),
            (10, "E_FUSE"),
            (11, "E_FUSE"),
            (12, "E_FUSE"),
            (13, "E_PATH"),
            (14, "E_STROKE"),
            (15, "E_FUSE"),
        ],
        None,
    );
}
#[test]
fn legacy_reusable_compound_outlines_keep_opacity_paths_and_dpi() {
    let s = scene("1044558", 1);
    assert_eq!(s.nodes.len(), 4);
    assert_eq!(
        s.nodes[..3]
            .iter()
            .map(|n| n["strokeStyle"]["compound"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["double", "double", "triple"]
    );
    assert_eq!(s.nodes[0]["strokeStyle"]["opacity"], 0.7);
    let outlines: Vec<_> = s.nodes[..3]
        .iter()
        .map(|n| {
            laymesh_core::geometry::compound_outline(
                &kurbo::BezPath::from_svg(n["d"].as_str().unwrap()).unwrap(),
                &n["strokeStyle"],
            )
            .to_svg()
        })
        .collect();
    assert_eq!(outlines[0], outlines[1]);
    assert!(outlines.iter().all(|d| d.contains('M')));
    assert!(s.nodes[3].get("outlineD").is_none());
    let svg = render_svg(&s).unwrap();
    assert!(svg.contains("fill-opacity=\"0.7\""));
    assert!(svg.contains("fill-rule=\"evenodd\""));
    assert_eq!(
        svg.matches("<path ").count(),
        7,
        "{}",
        &svg[..svg.len().min(1500)]
    );
    assert!(render_pdf(&s).unwrap().len() > 1000);
    no_images(&s);
    let (i, _, _) = png(&s, 150.);
    assert_eq!((i.width, i.height), (709, 325));
}
#[test]
fn legacy_double_outline_has_a_transparent_gap() {
    let s = scene("1044558", 2);
    for (x, y, rgb) in [
        (20., 7., [255, 0, 0]),
        (20., 7.8, [255, 255, 255]),
        (20., 9., [255, 0, 0]),
        (20., 11., [0, 0, 255]),
    ] {
        assert_eq!(pixel(&s, 300., x, y), rgb);
    }
}
#[test]
fn legacy_dashes_and_compound_outlines_survive_fusion() {
    let s = scene("1044558", 3);
    assert_eq!(ids(&s), ["@1", "@2", "spare", "merged"]);
    assert_eq!(s.nodes[0]["strokeStyle"]["dash"], json!([0.01, 1.5]));
    assert_eq!(s.nodes[0]["strokeStyle"]["cap"], "round");
    assert_eq!(s.nodes[0]["strokeStyle"]["opacity"], 0.5);
    assert_eq!(s.nodes[1]["strokeStyle"]["dash"], json!([4., 2., 0.01, 2.]));
    assert_eq!(s.nodes[2]["kind"], "path");
    let n = &s.nodes[3];
    assert!(
        !laymesh_core::geometry::compound_outline(
            &kurbo::BezPath::from_svg(n["d"].as_str().unwrap()).unwrap(),
            &n["strokeStyle"]
        )
        .is_empty()
    );
    assert!(render_png(&s, 96.).unwrap().len() > 100);
    assert!(render_pdf(&s).unwrap().len() > 1000);
}
#[test]
fn legacy_module_borders_and_invalid_styles_keep_diagnostics() {
    let s = scene("1044558", 4);
    assert_eq!(s.nodes[0]["strokeStyle"]["compound"], "double");
    located(
        "1044558",
        &[
            (5, "E_ARG"),
            (6, "E_STROKE"),
            (7, "E_STROKE"),
            (8, "E_STROKE"),
            (9, "E_NAME"),
            (10, "E_API_MIGRATION"),
            (11, "E_API_MIGRATION"),
        ],
        Some(2),
    );
}
