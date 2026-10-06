#![cfg(feature = "native")]
use laymesh_core::{engine::Engine, model::Host};
use laymesh_render::{ExportOptions, render_export, render_png, render_svg};
fn engine() -> Engine {
    let mut host = Host::default();
    host.files.insert(
        "/test-font.ttf".into(),
        include_bytes!("../../../tests/fonts/DejaVuSans.ttf").to_vec(),
    );
    Engine::new(host)
}
fn compile(s: &str) -> laymesh_core::model::Scene {
    engine().compile(s, "art-test.lay").unwrap()
}
fn pixel(png: &[u8], x: u32, y: u32) -> [u8; 4] {
    let im = image::load_from_memory(png).unwrap().to_rgba8();
    im.get_pixel(x, y).0
}
#[test]
fn shadows_follow_alpha_and_inner_effects_do_not_leak() {
    let base = "p=canvas(size=(40mm,30mm))\np.add(ellipse(size=(12mm,12mm),fill=\"#ff0000\"";
    let outer = compile(&format!(
        "{base},effects=[shadow(blur=0mm,offset=(4mm,0mm),color=\"#0000ff\",opacity=1)]),offset=(10mm,8mm))"
    ));
    let inner = compile(&format!(
        "{base},effects=[shadow(mode=\"inner\",blur=0mm,offset=(4mm,0mm),color=\"#0000ff\",opacity=1)]),offset=(10mm,8mm))"
    ));
    let a = render_png(&outer, 254.).unwrap();
    let b = render_png(&inner, 254.).unwrap();
    assert!(pixel(&a, 235, 140)[2] > 200);
    assert_eq!(pixel(&b, 235, 140)[3], 0);
    assert!(pixel(&b, 115, 140)[2] > 200);
    assert!(pixel(&b, 180, 140)[0] > 200);
    // A corner inside the image/layout frame but outside the ellipse must remain transparent.
    assert_eq!(pixel(&a, 105, 85)[3], 0);
}
#[test]
fn decorated_text_keeps_vectors_and_span_overrides() {
    let s = compile(
        r##"p=canvas(size=(90mm,35mm))
 p.add(text(spans=[span("AV",text_fill="#ff0000",text_stroke_color="#000000",text_stroke_width=1pt),span("fi",color="#0000ff")],font_family="DejaVu Sans",font_size=24pt,text_fill=linear_gradient(stops=[(0,"#00ff00"),(1,"#ffff00")]),extrude=text_extrude(depth=1mm),effects=[glow(color="#00ff00",blur=0.4mm)]),offset=(8mm,8mm))"##,
    );
    let svg = render_svg(&s).unwrap();
    assert!(svg.contains("#ff0000"));
    assert!(svg.contains("#0000ff"));
    assert!(svg.contains("aria-label=\"AVfi\""));
    assert!(svg.contains("<path"));
    assert!(!svg.contains("<text "));
    let pdf = render_export(
        &s,
        "pdf",
        &ExportOptions {
            dpi: Some(100.),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(pdf.starts_with(b"%PDF"));
    assert!(pdf.len() > 1000);
    let runs = s.nodes[0]["runs"].as_array().unwrap();
    assert!(
        runs.iter()
            .any(|r| r["strokeStyle"]["width"].as_f64().unwrap_or(0.) > 0.3)
    );
}
#[test]
fn upright_arc_and_perspective_reject_invalid_geometry() {
    let s = compile(
        r##"p=canvas(size=(100mm,40mm))
 p.add(text("ARC",font_family="DejaVu Sans",font_size=30pt,warp=text_warp("arc",angle=40deg)),offset=(10mm,10mm))"##,
    );
    // Counter winding survives the warp: the A must remain upright.
    let n = &s.nodes[0];
    let paths = n["runs"].as_array().unwrap();
    let a = kurbo::BezPath::from_svg(paths[0]["d"].as_str().unwrap()).unwrap();
    assert!(kurbo::Shape::bounding_box(&a).height() > 5.);
    let plain = compile(
        r##"p=canvas(size=(100mm,40mm))
p.add(text("ARC",font_family="DejaVu Sans",font_size=30pt,text_fill="#000000"))"##,
    );
    let original =
        kurbo::BezPath::from_svg(plain.nodes[0]["runs"][0]["d"].as_str().unwrap()).unwrap();
    assert!(
        kurbo::Shape::area(&a) * kurbo::Shape::area(&original) > 0.,
        "Arc must preserve glyph orientation"
    );
    for bad in [
        r##"warp=text_warp("perspective",corners=[(0mm,0mm),(10mm,0mm),(0mm,0mm),(0mm,10mm)])"##,
        r##"warp=text_warp("wave",wavelength=0mm)"##,
        r##"effects=[shadow(blur=-1mm)]"##,
        r##"effects=shadow()"##,
        r##"path=text_path(line(length=1mm))"##,
        r##"path=text_path(line(length=100mm)),content="A\nB""##,
    ] {
        let input = if bad.contains("content=") {
            format!("p=canvas(size=(100mm,30mm))\np.add(text({bad}))")
        } else {
            format!("p=canvas(size=(100mm,30mm))\np.add(text(\"long enough\",{bad}))")
        };
        let err = engine().compile(&input, "bad.lay").unwrap_err();
        assert!(["E_EFFECT", "E_ARG"].contains(&err.code.as_str()), "{err}");
    }
}
#[test]
fn lcss_group_scaling_and_overflow_preserve_layout() {
    let s = compile(
        r##"style { .title { text-fill: linear-gradient(to right, #ff0000, #0000ff); text-stroke-color: #ffffff; text-stroke-width: 0.5pt; effects: [shadow(blur=1mm, offset=(2mm,0mm))]; warp: text_warp("wave", amplitude=1mm, wavelength=20mm); } }
 p=canvas(size=(40mm,30mm))
 g=group()
 g.add(text("Style",class="title",font_family="DejaVu Sans",font_size=18pt))
 p.add(g,size=(60mm,30mm),offset=(0mm,5mm))"##,
    );
    assert!(s.warnings.iter().any(|w| w.code == "W_EFFECT_OVERFLOW"));
    assert_eq!(s.nodes[0]["width"], 60.);
    let svg = render_svg(&s).unwrap();
    assert!(svg.contains("gradientUnits='userSpaceOnUse'"));
    assert!(svg.contains("feGaussianBlur"));
}

#[test]
fn transparent_image_contours_and_missing_chinese_glyphs_are_preserved() {
    let mut im = image::RgbaImage::new(20, 20);
    for y in 5..15 {
        for x in 5..15 {
            im.put_pixel(x, y, image::Rgba([255, 0, 0, 255]));
        }
    }
    let mut bytes = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(im)
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
    let mut e = engine();
    e.host.files.insert("/alpha.png".into(), bytes.into_inner());
    let s=e.compile(r##"p=canvas(size=(40mm,30mm))
    p.add(image(src="/alpha.png",effects=[shadow(blur=0mm,offset=(2mm,0mm),color="#0000ff",opacity=1)]),size=(20mm,20mm),offset=(5mm,5mm))"##,"image.lay").unwrap();
    let png = render_png(&s, 254.).unwrap();
    assert_eq!(pixel(&png, 60, 60)[3], 0);
    assert!(pixel(&png, 210, 150)[2] > 200);
    assert_eq!(pixel(&png, 250, 150)[3], 0);
    let cn = compile(
        r##"p=canvas(size=(80mm,30mm))
    p.add(text("中文 AV fi",font_family="DejaVu Sans",font_size=20pt,text_fill="#0000ff",warp=text_warp("wave",amplitude=1mm)),offset=(8mm,8mm))"##,
    );
    assert_eq!(cn.nodes[0]["artText"], "中文 AV fi");
    assert!(render_svg(&cn).unwrap().contains("中文 AV fi"));
    assert!(render_png(&cn, 100.).unwrap().len() > 100);
}
