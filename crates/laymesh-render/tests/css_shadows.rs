#![cfg(feature = "native")]
use laymesh_core::{engine::compile_source, model::Host};
use laymesh_render::{ExportOptions, render_export, render_png, render_svg};
fn compile(css: &str, material: &str) -> laymesh_core::model::Scene {
    compile_source(&format!("p=canvas(size=(40mm,30mm))\nstyle {{.a {{{css}}}}}\np.add({material},offset=(10mm,8mm))"),"/css.lay",Host::default()).unwrap()
}
fn pixel(png: &[u8], x: u32, y: u32) -> [u8; 4] {
    image::load_from_memory(png)
        .unwrap()
        .to_rgba8()
        .get_pixel(x, y)
        .0
}
#[test]
fn standard_box_uses_the_box_not_ellipse_alpha_and_knocks_out_inside() {
    let scene = compile(
        "box-shadow:4mm 0 0 blue",
        "ellipse(size=(12mm,12mm),class=\"a\")",
    );
    let png = render_png(&scene, 254.).unwrap();
    // Ellipse is transparent, but CSS shadow comes from its border box.
    assert_eq!(pixel(&png, 110, 90)[3], 0); // knockout inside original box
    assert!(pixel(&png, 235, 90)[2] > 240);
    assert!(pixel(&png, 235, 90)[3] > 240);
    let svg = render_svg(&scene).unwrap();
    assert!(svg.contains("operator='out'"));
    assert!(svg.contains("fill=\"white\""));
}
#[test]
fn front_first_layers_negative_spread_and_inner_shadows() {
    let scene = compile(
        "box-shadow:4mm 0 0 red,4mm 0 0 blue",
        "rect(size=(12mm,12mm),class=\"a\")",
    );
    let png = render_png(&scene, 254.).unwrap();
    assert!(pixel(&png, 235, 140)[0] > 240);
    assert!(pixel(&png, 235, 140)[2] < 10);
    let scene = compile(
        "background-color:red;box-shadow:inset 4mm 0 0 blue",
        "rect(size=(12mm,12mm),class=\"a\")",
    );
    let png = render_png(&scene, 254.).unwrap();
    assert!(pixel(&png, 115, 140)[2] > 240);
    assert!(pixel(&png, 180, 140)[0] > 240);
    assert_eq!(pixel(&png, 235, 140)[3], 0);
    let scene = compile(
        "box-shadow:4mm 0 0 -2mm blue",
        "rect(size=(12mm,12mm),class=\"a\")",
    );
    let png = render_png(&scene, 254.).unwrap();
    assert_eq!(pixel(&png, 235, 90)[3], 0);
    assert!(pixel(&png, 235, 140)[2] > 240);
    // Signed spread also works in PDF's effect-only raster tiles.
    let pdf = render_export(
        &scene,
        "pdf",
        &ExportOptions {
            dpi: Some(127.),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(pdf.starts_with(b"%PDF"));
}
#[test]
fn css_blur_half_radius_pdf_vectors_and_export_dpi_independence() {
    let source = "p=canvas(size=(40mm,30mm),layout_dpi=192)\nstyle {.a{background-color:red;box-shadow:2mm 0 24px blue;box-shadow-blur:2mm}}\np.add(rect(size=(24px,24px),class=\"a\"),offset=(10mm,8mm))";
    let scene = compile_source(source, "/css.lay", Host::default()).unwrap();
    assert_eq!(scene.nodes[0]["width"], 3.175);
    let svg = render_svg(&scene).unwrap();
    assert!(svg.contains("stdDeviation='1'"));
    for dpi in [96., 192.] {
        let pdf = render_export(
            &scene,
            "pdf",
            &ExportOptions {
                dpi: Some(dpi),
                ..Default::default()
            },
        )
        .unwrap();
        let doc = lopdf::Document::load_mem(&pdf).unwrap();
        let page = doc.get_pages()[&1];
        let body = doc.get_page_content(page).unwrap();
        let ops = lopdf::content::Content::decode(&body).unwrap();
        assert!(
            ops.operations
                .iter()
                .any(|op| matches!(op.operator.as_str(), "m" | "re")),
            "PDF subject remains a path"
        );
        let box_ = doc
            .get_object(page)
            .unwrap()
            .as_dict()
            .unwrap()
            .get(b"MediaBox")
            .unwrap()
            .as_array()
            .unwrap();
        let width = box_[2].as_float().unwrap();
        assert!((width - 40. * 72. / 25.4).abs() < 0.001);
    }
}

#[test]
fn text_and_drop_shadow_use_content_alpha_and_distinct_blur_units() {
    let scene = compile(
        "filter:drop-shadow(4mm 0 2mm blue)",
        r##"ellipse(size=(12mm,12mm),fill="#ff0000",class="a")"##,
    );
    let svg = render_svg(&scene).unwrap();
    assert!(svg.contains("stdDeviation='2'"));
    let png = render_png(&scene, 254.).unwrap();
    assert!(pixel(&png, 240, 140)[2] > 0);
    let scene = compile(
        "text-shadow:0 1mm 2mm blue",
        r#"text("Glyph",font_size=5mm,class="a")"#,
    );
    let svg = render_svg(&scene).unwrap();
    assert!(svg.contains("stdDeviation='1'"));
    assert!(!svg.contains("operator='out'"));
    let scene = compile(
        "box-shadow:2mm 0 0 blue",
        r##"rect(size=(12mm,12mm),fill="#ff0000",border_color="#000000",border_width=1mm,effects=[shadow(blur=3mm)],class="a")"##,
    );
    // A standard outer shadow preserves the original fill/stroke effect source.
    let original = &scene.nodes[0]["children"][2];
    assert_eq!(original["kind"], "path");
    assert_eq!(original["strokeStyle"]["color"], "#000000");
    assert_eq!(original["effects"][0]["blur"], 3.);
    let error = render_export(&scene, "pdf", &Default::default()).unwrap_err();
    assert!(error.message.contains("64MB"));
    render_export(
        &scene,
        "pdf",
        &ExportOptions {
            dpi: Some(254.),
            ..Default::default()
        },
    )
    .unwrap();
}

#[test]
fn bootstrap_percentage_radius_uses_both_box_axes() {
    let scene = compile(
        "border-radius:50%;background:red;box-shadow:2mm 0 0 blue",
        r#"rect(size=(12mm,6mm),class="a")"#,
    );
    let png = render_png(&scene, 254.).unwrap();
    assert_eq!(pixel(&png, 101, 81)[3], 0);
    assert!(pixel(&png, 160, 110)[0] > 240);
    let svg = render_svg(&scene).unwrap();
    assert!(svg.contains("operator='out'"));
}
