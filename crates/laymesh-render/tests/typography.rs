#![cfg(feature = "native")]
//! Behavioral ports of the pre-Rust typography/font-stack acceptance suite.
use laymesh_core::{
    Loc,
    engine::compile_source,
    model::{Host, Scene, jnum},
    text::{FontSystem, layout_text},
};
use laymesh_render::{render_pdf, render_png, render_svg};
use serde_json::json;
fn host() -> Host {
    let mut host = Host::default();
    host.files.insert(
        "/fixture/shaping.ttf".into(),
        include_bytes!("assets/ShapingTest.ttf").to_vec(),
    );
    host.files.insert(
        "/fixture/font.ttc".into(),
        include_bytes!("../../../tests/assets/LayMeshTest.ttc").to_vec(),
    );
    host.files.insert(
        "/fixture/font.otc".into(),
        include_bytes!("../../../tests/assets/LayMeshTest.otc").to_vec(),
    );
    host
}
fn compile(source: &str) -> Scene {
    compile_source(source, "/fixture/main.lay", host()).unwrap()
}
#[test]
fn rich_spans_reflow_per_placement_and_preserve_anchors() {
    let s = compile(
        r##"
page=canvas(size=(90 mm,70 mm),background="#ffffff")
body=text(spans=[span("Red words ",color="#ff0000"),span("Blue words and another line",color="#0000ff",font_size=9 pt)],font_family="./font.ttc#LayMeshTestFirst",font_size=8 pt,align=center)
narrow=page.add(body,size=(22 mm,auto),offset=(3 mm,3 mm))
wide=page.add(body,size=(55 mm,auto),target=narrow.bottom_left,offset=(0 mm,2 mm))
"##,
    );
    let (a, b) = (&s.nodes[0], &s.nodes[1]);
    assert_eq!(a["width"], 22.);
    assert_eq!(b["width"], 55.);
    assert!(jnum(a, "height", 0.) > jnum(b, "height", 0.));
    assert!((jnum(b, "y", 0.) - jnum(a, "y", 0.) - jnum(a, "height", 0.) - 2.).abs() < 1e-6);
    let runs = a["runs"].as_array().unwrap();
    assert!(runs.iter().any(|r| r["color"] == "#ff0000"));
    assert!(runs.iter().any(|r| r["color"] == "#0000ff"));
    assert!(runs.iter().any(|r| jnum(r, "x", 0.) > 0.));
    assert!(
        runs.iter()
            .filter(|r| r["color"] == "#0000ff")
            .all(|r| (jnum(r, "fontSize", 0.) - 9. * 25.4 / 72.).abs() < 1e-9)
    );
    let svg = render_svg(&s).unwrap();
    assert!(svg.contains("<text"));
    let png = render_png(&s, 144.).unwrap();
    let reader = png::Decoder::new(std::io::Cursor::new(png))
        .read_info()
        .unwrap();
    assert_eq!((reader.info().width, reader.info().height), (510, 397));
}
#[test]
fn right_and_justified_lines_do_not_stretch_the_last_line() {
    let s = compile(
        r#"page=canvas(size=(60 mm,40 mm))
a=text(size=(25 mm,auto),content="Right",font_family="./font.ttc",font_size=8 pt,align=right)
page.add(a)
b=text(size=(20 mm,auto),content="one two three four five six",font_family="./font.ttc",font_size=8 pt,align=justify)
page.add(b,offset=(0 mm,10 mm))"#,
    );
    assert!(jnum(&s.nodes[0]["runs"][0], "x", 0.) > 0.);
    let runs = s.nodes[1]["runs"].as_array().unwrap();
    let last = runs.last().unwrap()["baseline"].clone();
    let first = runs.iter().find(|r| r["baseline"] == last).unwrap();
    assert_eq!(first["x"], 0.);
    assert!(jnum(runs.last().unwrap(), "x", 0.) < 20.);
    assert!(
        runs.iter()
            .any(|r| r["content"] == " " && jnum(r, "x", 0.) > 0.)
    );
}
#[test]
fn selected_ttc_and_otc_faces_survive_all_exports() {
    for (ext, first, second, mime) in [
        ("ttc", "LayMeshTestFirst", "LayMeshTestSecond", "font/ttf"),
        ("otc", "LayMeshOTFFirst", "LayMeshOTFSecond", "font/otf"),
    ] {
        let make = |name: &str| {
            compile(&format!(
                "page=canvas(size=(40 mm,20 mm),background=\"#ffffff\")\na=text(size=(30 mm,auto),content=\"A\",font_family=\"./font.{ext}#{name}\",font_size=30 pt)\npage.add(a)"
            ))
        };
        let first = make(first);
        let second_scene = make(second);
        assert_eq!(second_scene.nodes[0]["runs"][0]["fontFace"], second);
        let font = second_scene.fonts.values().next().unwrap();
        assert!(!font.data.starts_with(b"ttcf"));
        if ext == "otc" {
            assert!(font.data.starts_with(b"OTTO"));
        }
        assert!(
            render_svg(&second_scene)
                .unwrap()
                .contains(&format!("data:{mime};base64,"))
        );
        assert_ne!(
            render_png(&first, 144.).unwrap(),
            render_png(&second_scene, 144.).unwrap()
        );
        assert!(render_pdf(&second_scene).unwrap().len() > 1000);
    }
}
#[test]
fn imported_span_paths_keep_defining_module_and_warning_locations() {
    let mut h = host();
    h.files.insert(
        "/fixture/parts/local.ttc".into(),
        include_bytes!("../../../tests/assets/LayMeshTest.ttc").to_vec(),
    );
    h.files.insert("/fixture/parts/title.lay".into(),b"export function title() { return text(spans=[span(\"A\",font_family=[\"./local.ttc#LayMeshTestSecond\"])],font_size=10 pt) }".to_vec());
    let s=compile_source("page=canvas(size=(40 mm,20 mm))\nimport {title} from \"./parts/title.lay\"\npage.add(title())","/fixture/main.lay",h).unwrap();
    assert_eq!(
        s.nodes[0]["runs"][0]["fontPath"],
        "/fixture/parts/local.ttc"
    );
    assert_eq!(s.nodes[0]["runs"][0]["fontFace"], "LayMeshTestSecond");
    assert!(s.warnings.is_empty());
}
#[test]
fn missing_glyphs_are_located_vector_boxes_in_all_exports() {
    let s=compile_source("page=canvas(size=(60 mm,20 mm))\na=text(content=\"A\u{10ffff}\u{10ffff}\u{10fffe}\",font_size=12 pt)\npage.add(a)","/no-fonts/main.lay",Host::default()).unwrap();
    assert!(s.fonts.is_empty());
    assert_eq!(
        s.nodes[0]["runs"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["kind"] == "box")
            .count(),
        4
    );
    assert_eq!(s.warnings.len(), 1);
    assert_eq!(s.warnings[0].code, "W_FONT");
    assert_eq!(s.warnings[0].file, "/no-fonts/main.lay");
    assert_eq!(s.warnings[0].loc.line, 2);
    assert!(s.warnings[0].message.contains("U+10FFFF"));
    assert!(s.warnings[0].message.contains("U+10FFFE"));
    for bytes in [
        render_svg(&s).unwrap().into_bytes(),
        render_png(&s, 144.).unwrap(),
        render_pdf(&s).unwrap(),
    ] {
        assert!(bytes.len() > 100);
    }
}
#[test]
fn font_stack_order_and_span_override_are_preserved() {
    let s = compile(
        r#"page=canvas(size=(60 mm,20 mm))
a=text(spans=[span("A e\u0301"),span("B",font_family=["LayMesh Test Second","LayMesh Test First"])],font_family=["LayMesh Test First","DejaVu Sans","LayMesh Test Second"],font_size=12 pt)
page.add(a)"#,
    );
    let runs = s.nodes[0]["runs"].as_array().unwrap();
    assert_eq!(runs[0]["fontSystemFamily"], "LayMesh Test First");
    assert_eq!(
        runs.last().unwrap()["fontSystemFamily"],
        "LayMesh Test Second"
    );
    assert!(runs.iter().any(|r| {
        r["content"]
            .as_str()
            .is_some_and(|s| s.contains("e\u{301}"))
    }));
    assert!(s.warnings.is_empty());
}
#[test]
fn shaped_ligatures_and_mixed_direction_measurements_are_stable() {
    let mut fs = FontSystem::new(false);
    fs.register_font("test", include_bytes!("assets/ShapingTest.ttf").to_vec());
    let mut w = vec![];
    let mut lay = |text: &str| {
        layout_text(
            &json!({"content":text,"font_family":["DejaVu Sans"],"font_size":5.}),
            None,
            &mut fs,
            &mut w,
            "x",
            Loc::default(),
        )
        .unwrap()
    };
    let lig = lay("ffi");
    let individual = lay("f")["width"].as_f64().unwrap() * 2. + lay("i")["width"].as_f64().unwrap();
    assert!(jnum(&lig, "width", 0.) < individual);
    let bidi = lay("abc אבג 123 xyz");
    let runs = bidi["runs"].as_array().unwrap();
    assert!(
        runs.iter()
            .any(|r| r["content"].as_str().is_some_and(|s| s.contains("אבג")))
    );
    let total = runs.iter().map(|r| jnum(r, "width", 0.)).sum::<f64>();
    assert!((total - jnum(&bidi, "width", 0.)).abs() < 1e-6);
    for adjacent in runs.windows(2) {
        assert!(
            (jnum(&adjacent[0], "x", 0.) + jnum(&adjacent[0], "width", 0.)
                - jnum(&adjacent[1], "x", 0.))
            .abs()
                < 1e-6
        );
    }
}

#[test]
fn inline_formula_error_points_to_the_string_delimiter() {
    let source = "page=canvas(size=(100,80))\np=plot(size=(90,70),x=axis(\n label=\"missing $x\"))\np.line(x=[0,1],y=[1,2])\npage.add(p)";
    let e = compile_source(source, "/fixture/labels.lay", host()).unwrap_err();
    assert_eq!(e.code, "E_FORMULA");
    assert_eq!((e.loc.line, e.loc.column), (3, 17));
}
#[test]
fn span_returned_from_another_module_keeps_its_font_origin() {
    let mut h = host();
    h.files.insert(
        "/fixture/parts/local.ttc".into(),
        include_bytes!("../../../tests/assets/LayMeshTest.ttc").to_vec(),
    );
    h.files.insert("/fixture/parts/label.lay".into(),b"export function label() { return span(\"A\",font_family=[\"./local.ttc#LayMeshTestSecond\"]) }".to_vec());
    let s=compile_source("page=canvas(size=(40 mm,20 mm))\nimport {label} from \"./parts/label.lay\"\npage.add(text(spans=[label()],font_size=10 pt))","/fixture/main.lay",h).unwrap();
    assert_eq!(
        s.nodes[0]["runs"][0]["fontPath"],
        "/fixture/parts/local.ttc"
    );
    assert!(s.warnings.is_empty());
}

#[test]
fn missing_font_and_nonmatching_weight_warn_without_stopping_exports() {
    let s=compile_source("page=canvas(size=(40,20))\npage.add(text(content=\"A\",font_family=[\"./missing.ttf\",\"./font.ttc#LayMeshTestFirst\"],font_weight=500))","/fixture/main.lay",host()).unwrap();
    assert!(
        s.warnings
            .iter()
            .any(|w| w.code == "W_FONT" && w.message.contains("missing.ttf") && w.loc.line == 2)
    );
    assert!(
        s.warnings
            .iter()
            .any(|w| w.code == "W_FONT" && w.message.contains("字重") && w.loc.line == 2)
    );
    assert_eq!(s.nodes[0]["runs"][0]["kind"], "glyph");
    assert!(render_svg(&s).unwrap().contains("<text"));
    assert!(render_png(&s, 144.).unwrap().len() > 100);
    assert!(render_pdf(&s).unwrap().len() > 1000);
}

#[test]
fn embedding_restricted_fonts_never_reach_any_export() {
    for flag in [2u16, 0x100, 0x200] {
        let mut bytes = include_bytes!("../../../tests/fonts/DejaVuSans.ttf").to_vec();
        let count = u16::from_be_bytes([bytes[4], bytes[5]]) as usize;
        let record = (0..count)
            .map(|i| 12 + i * 16)
            .find(|&i| &bytes[i..i + 4] == b"OS/2")
            .unwrap();
        let offset =
            u32::from_be_bytes(bytes[record + 8..record + 12].try_into().unwrap()) as usize;
        bytes[offset + 8..offset + 10].copy_from_slice(&flag.to_be_bytes());
        let mut host = Host::default();
        host.files.insert("/restricted.ttf".into(), bytes.clone());
        let scene = compile_source("page=canvas(size=(30mm,20mm))\npage.add(text(\"A中文\",font_family=[\"/restricted.ttf\"],font_size=4mm))", "/source.lay", host).unwrap();
        assert!(scene.fonts.is_empty());
        assert!(
            scene
                .warnings
                .iter()
                .any(|w| w.code == "W_FONT" && w.message.contains("嵌入"))
        );
        let svg = render_svg(&scene).unwrap();
        let excludes_body_font = |output: &str| !output.contains("data:font/");
        assert!(excludes_body_font(&svg));
        assert!(render_png(&scene, 144.).unwrap().len() > 100);
        let pdf = render_pdf(&scene).unwrap();
        assert!(pdf.len() > 100);
        assert!(!String::from_utf8_lossy(&pdf).contains("DejaVuSans"));
        // Fault injection: obtain a real shaped scene with an unrestricted
        // face, then replace only its embedded bytes with the restricted face.
        // This reproduces the old selector admitting fsType-restricted fonts;
        // it is distinct from merely supplying a forbidden input fixture.
        let mut allowed = bytes.clone();
        allowed[offset + 8..offset + 10].copy_from_slice(&0u16.to_be_bytes());
        let mut host = Host::default();
        host.files.insert("/restricted.ttf".into(), allowed);
        let mut old = compile_source("page=canvas(size=(30mm,20mm))\npage.add(text(\"A中文\",font_family=[\"/restricted.ttf\"],font_size=4mm))", "/source.lay", host).unwrap();
        assert!(!old.fonts.is_empty());
        for font in old.fonts.values_mut() {
            font.data = bytes.clone();
        }
        assert!(
            !excludes_body_font(&render_svg(&old).unwrap()),
            "the export check must reject admitted restricted face {flag:#x}"
        );
    }
}
