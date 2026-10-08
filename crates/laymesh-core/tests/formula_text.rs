use kurbo::Shape;
use laymesh_core::{
    Loc,
    engine::compile_source,
    model::{Host, jnum},
    text::{FontSystem, formula},
};
use serde_json::{Value, json};
include!("../../../tests/fonts/text/fixtures.rs");
const MATH: &[(&str, &[u8])] = &[
    (
        "latinmodern",
        include_bytes!("../../../tests/fonts/math/latinmodern-math.otf"),
    ),
    (
        "stix",
        include_bytes!("../../../tests/fonts/math/STIX2Math.otf"),
    ),
    (
        "xits",
        include_bytes!("../../../tests/fonts/math/XITSMath-Regular.otf"),
    ),
];
fn fonts() -> FontSystem {
    let mut f = FontSystem::new(false);
    for (name, data) in MATH {
        f.register_font(&format!("/{name}.otf"), data.to_vec());
    }
    for (name, data) in TEXT_FONTS {
        f.register_font(name, data.to_vec());
    }
    f
}
fn render(backend: &str, source: &str, size: f64, display: bool) -> Value {
    let mut f = fonts();
    let mut warnings = vec![];
    let node = formula(
        &json!({"source":source,"math_font":backend,"font_family":TEXT_FAMILY,
        "font_size":size,"style":if display{"display"}else{"inline"}}),
        &mut f,
        &mut warnings,
        "/text.lay",
        Loc::default(),
    )
    .unwrap_or_else(|e| panic!("{backend} {source}: {}", e.message));
    assert!(warnings.is_empty(), "{source}: {warnings:?}");
    assert!(
        f.assets.is_empty(),
        "{backend} {source}: outlined text embedded {:?}",
        f.assets.keys().collect::<Vec<_>>()
    );
    node
}
fn backends() -> impl Iterator<Item = String> {
    std::iter::once("ratex-katex".into()).chain(MATH.iter().map(|(n, _)| format!("/{n}.otf")))
}
fn text_ink(node: &Value) -> kurbo::Rect {
    let mut bounds = None;
    for item in node["items"].as_array().unwrap() {
        if item["kind"] == "path" && item["mathCodepoint"].is_null() {
            let path = kurbo::BezPath::from_svg(item["d"].as_str().unwrap()).unwrap();
            let b = path.bounding_box();
            bounds = Some(bounds.map_or(b, |r: kurbo::Rect| r.union(b)));
        }
    }
    bounds.unwrap()
}
#[test]
fn multilingual_text_chemistry_physics_proofs_and_styles_have_real_outlines() {
    let cases: Vec<(String, String)> = serde_json::from_str(include_str!(
        "../../../experiments/opentype-math/text-fallback-cases.json"
    ))
    .unwrap();
    let mut checked = 0;
    for backend in backends() {
        for (_, source) in &cases {
            for display in [false, true] {
                for size in [2., 5., 12.] {
                    let n = render(&backend, source, size, display);
                    assert!(
                        !n["mathTextFonts"].as_array().unwrap().is_empty(),
                        "{source}"
                    );
                    let (w, h) = (jnum(&n, "width", 0.), jnum(&n, "height", 0.));
                    assert!(w.is_finite() && h.is_finite() && w >= 0. && h >= 0.);
                    for p in n["items"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .filter(|p| p["kind"] == "path")
                    {
                        let b = kurbo::BezPath::from_svg(p["d"].as_str().unwrap())
                            .unwrap()
                            .bounding_box();
                        assert!(
                            b.x0 >= -1e-6 && b.y0 >= -1e-6 && b.x1 <= w + 1e-6 && b.y1 <= h + 1e-6,
                            "{backend} {source}: {b:?} exceeds {w}x{h}"
                        );
                    }
                    assert!(
                        n["items"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .all(|p| p["kind"] != "box" && p["kind"] != "glyph")
                    );
                    checked += 1;
                }
            }
        }
    }
    assert_eq!(checked, 24 * 4 * 2 * 3);
}
#[test]
fn text_advance_spaces_baselines_and_script_scaling_use_actual_font_metrics() {
    let cjk = ttf_parser::Face::parse(TEXT_FONTS[4].1, 0).unwrap();
    let expected =
        5. * f64::from(
            cjk.glyph_hor_advance(cjk.glyph_index('中').unwrap())
                .unwrap(),
        ) / f64::from(cjk.units_per_em());
    let space =
        5. * f64::from(
            cjk.glyph_hor_advance(cjk.glyph_index(' ').unwrap())
                .unwrap(),
        ) / f64::from(cjk.units_per_em());
    for backend in backends() {
        let a = render(&backend, r"\text{中文}", 5., false);
        assert!(
            (jnum(&a, "width", 0.) - 2. * expected).abs() < 1e-6,
            "{backend}: {a}"
        );
        let mut f = fonts();
        let a=formula(&json!({"source":r"\text{中 文}","font_size":5.,"math_font":backend,"font_family":[TEXT_FAMILY[1]]}),&mut f,&mut vec![],"/test.lay",Loc::default()).unwrap();
        assert!((jnum(&a, "width", 0.) - 2. * expected - space).abs() < 1e-6);
        if backend != "ratex-katex" {
            let base = render(&backend, r"\text{中文}", 5., false);
            let script = render(&backend, r"x_{\text{中文}}", 5., false);
            let second = render(&backend, r"x_{i_{\text{中文}}}", 5., false);
            let bytes = MATH
                .iter()
                .find(|(n, _)| backend == format!("/{n}.otf"))
                .unwrap()
                .1;
            let face = ttf_parser::Face::parse(bytes, 0).unwrap();
            let c = face.tables().math.unwrap().constants.unwrap();
            let first_scale = f64::from(c.script_percent_scale_down()) / 100.;
            let second_scale = f64::from(c.script_script_percent_scale_down()) / 100.;
            assert!(
                (text_ink(&script).height() / text_ink(&base).height() - first_scale).abs() < 1e-6
            );
            assert!(
                (text_ink(&second).height() / text_ink(&base).height() - second_scale).abs() < 1e-6
            );
            let mixed = render(&backend, r"\text{中文}x", 5., false);
            let x = mixed["items"]
                .as_array()
                .unwrap()
                .iter()
                .find(|p| p["mathCodepoint"] == json!(0x1D465))
                .unwrap();
            assert!((jnum(x, "baseline", 0.) - jnum(&mixed, "ascent", 0.)).abs() < 1e-6);
        }
    }
}
#[test]
fn bold_italic_ligatures_and_combining_marks_are_shaped_as_text() {
    for backend in backends() {
        let regular = render(&backend, r"\text{中文}", 5., false);
        let bold = render(&backend, r"\textbf{中文}", 5., false);
        assert_eq!(bold["mathTextFonts"][0]["weight"], 700);
        assert_ne!(regular["items"][0]["d"], bold["items"][0]["d"]);
        let combined = render(&backend, r"\textbf{\textit{Italic}}", 5., false);
        assert_eq!(combined["mathTextFonts"][0]["weight"], 700);
        assert_eq!(combined["mathTextFonts"][0]["italic"], true);
        let ligature = render(&backend, r"\text{office}", 5., false);
        assert!(
            ligature["mathTextFonts"][0]["glyph_count"]
                .as_u64()
                .unwrap()
                < 6
        );
        let mark = render(&backend, "\\text{a\u{0301}}", 5., false);
        assert_eq!(mark["mathTextFonts"][0]["glyph_count"], 1);
        assert_eq!(mark["mathTextFonts"][0]["character_count"], 2);
        let joined = render(&backend, r"\text{مرحبا}", 5., false);
        let isolated = render(&backend, r"\text{م ر ح ب ا}", 5., false);
        assert_ne!(joined["width"], isolated["width"]);
    }
}

#[test]
fn fallback_is_explicit_configurable_and_never_changes_math_alphabets() {
    for backend in backends() {
        let mut f = fonts();
        let mut warnings = vec![];
        let base = json!({"source":r"\text{中文}","math_font":backend,"font_family":TEXT_FAMILY});
        let mut strict = base.clone();
        strict["math_text_fallback"] = json!(false);
        let result = formula(&strict, &mut f, &mut warnings, "/test.lay", Loc::default());
        let n = result.unwrap();
        assert_eq!(n["mathTextFallback"], false);
        assert_eq!(n["mathMissingGlyphs"].as_array().unwrap().len(), 2);
        assert!(
            warnings
                .iter()
                .any(|d| d.code == "W_FONT" && d.message.contains("U+4E2D"))
        );
        assert!(f.assets.is_empty());
        let mut narrow = base.clone();
        narrow["font_family"] = json!([TEXT_FAMILY[0]]);
        let n = formula(&narrow, &mut f, &mut vec![], "/test.lay", Loc::default()).unwrap();
        assert_eq!(n["mathMissingGlyphs"], json!(['中' as u32, '文' as u32]));
        let mut missing = base.clone();
        missing["source"] = json!("\\text{\u{10ffff}}");
        let n = formula(&missing, &mut f, &mut vec![], "/test.lay", Loc::default()).unwrap();
        assert_eq!(n["mathMissingGlyphs"], json!([0x10ffff]));
        // A real math expression must remain byte-identical when text fallback changes.
        let mut math = base.clone();
        math["source"] = json!(r"\mathbf{A}+\bm{\alpha x}+\frac{x}{2}");
        let enabled = formula(&math, &mut f, &mut vec![], "/test.lay", Loc::default()).unwrap();
        math["math_text_fallback"] = json!(false);
        let disabled = formula(&math, &mut f, &mut vec![], "/test.lay", Loc::default()).unwrap();
        assert_eq!(enabled["items"], disabled["items"]);
        assert_eq!(enabled["width"], disabled["width"]);
        assert!(enabled["mathTextFonts"].as_array().unwrap().is_empty());
    }
}

fn host() -> Host {
    let mut h = Host::default();
    h.files.insert("/fonts/math.otf".into(), MATH[2].1.to_vec());
    for (i, (_, bytes)) in TEXT_FONTS.iter().enumerate() {
        h.files
            .insert(format!("/fonts/text-{i}.otf"), bytes.to_vec());
    }
    h
}

#[test]
fn inline_and_lcss_inheritance_resolve_text_font_paths_and_share_the_baseline() {
    let s=compile_source(r#"page=canvas(size=(120mm,40mm),math_font="./fonts/math.otf",font_family=["./fonts/text-0.otf","./fonts/text-4.otf"],math_text_fallback=true)
page.add(text(spans=[span("A "),formula(r"\text{中文}x_2"),span(" B")],font_size=12pt))"#,"/main.lay",host()).unwrap();
    assert!(s.warnings.is_empty(), "{:?}", s.warnings);
    let runs = s.nodes[0]["runs"].as_array().unwrap();
    let m = runs.iter().find(|n| n["kind"] == "formula").unwrap();
    assert_eq!(m["mathFont"], "XITS Math");
    assert_eq!(m["mathTextFonts"][0]["family"], TEXT_FAMILY[1]);
    assert!(
        (jnum(m, "y", 0.) + jnum(m, "ascent", 0.) - jnum(&runs[0], "baseline", 0.)).abs() < 1e-9
    );
    // Formula outlines do not embed the CJK face; ordinary text still embeds its Latin font.
    assert_eq!(s.fonts.len(), 1);
    assert_eq!(s.fonts.values().next().unwrap().family, TEXT_FAMILY[0]);
    let mut h = host();
    h.files.insert("/styles/theme.lcss".into(),br#"formula { math_font: "../fonts/math.otf"; font_family: "../fonts/text-0.otf", "../fonts/text-4.otf"; math_text_fallback: true; }"#.to_vec());
    let s=compile_source("page=canvas(size=(120mm,40mm),stylesheet=\"./styles/theme.lcss\")\npage.add(formula(r\"\\text{中文}\"))","/main.lay",h).unwrap();
    assert!(s.warnings.is_empty(), "{:?}", s.warnings);
    assert!(s.fonts.is_empty());
    assert_eq!(s.nodes[0]["mathTextFonts"][0]["family"], TEXT_FAMILY[1]);
    for value in [json!("true"), json!(1), json!([])] {
        let mut f = fonts();
        let e = formula(
            &json!({"source":"x","math_text_fallback":value}),
            &mut f,
            &mut vec![],
            "/test.lay",
            Loc::default(),
        )
        .unwrap_err();
        assert_eq!(e.code, "E_FONT");
    }
}

#[test]
fn isolated_text_leaves_and_explicit_body_styles_use_measured_text() {
    for backend in backends() {
        for source in [
            r"中文_i+\hat{文}",
            r"\raisebox{0.5em}{\text{中文}}+\llap{\text{文}}x",
        ] {
            for display in [false, true] {
                let n = render(&backend, source, 5., display);
                assert!(!n["mathTextFonts"].as_array().unwrap().is_empty());
            }
        }
        let mut f = fonts();
        let n=formula(&json!({"source":r"\text{ABC}","math_font":backend,"font_weight":700,"font_style":"italic"}),&mut f,&mut vec![],"/test.lay",Loc::default()).unwrap();
        assert_eq!(n["mathTextFonts"][0]["weight"], 700);
        assert_eq!(n["mathTextFonts"][0]["italic"], true);
        assert!(f.assets.is_empty());
        for invalid in [json!({"font_weight":"bold"}), json!({"font_style":42})] {
            let mut spec = json!({"source":"x","math_font":backend});
            spec.as_object_mut()
                .unwrap()
                .extend(invalid.as_object().unwrap().clone());
            assert_eq!(
                formula(&spec, &mut f, &mut vec![], "/test.lay", Loc::default())
                    .unwrap_err()
                    .code,
                "E_FONT"
            );
        }
    }
}

#[test]
fn missing_glyphs_preserve_formula_structure_color_scripts_and_exportable_ink() {
    let cases = [
        (r"\text{Energy 能量}=mc^2", 2),
        (r"E^{你好}_{世界！}", 5),
        (r"\frac{\text{中文}}{x_2}+\sqrt{\text{日文}}", 4),
        (r"\ce{A ->[中文] B}", 2),
        (
            r"\begin{prooftree}\AxiomC{\text{前提}}\UnaryInfC{\text{结论}}\end{prooftree}",
            4,
        ),
        ("\\text{A\u{10ffff}B}+x", 1),
    ];
    for backend in backends() {
        for (source, count) in cases {
            for enabled in [true, false] {
                for display in [true, false] {
                    for size in [2., 5., 12.] {
                        let mut f = fonts();
                        let mut warnings = vec![];
                        let n = formula(
                            &json!({"source":source,"math_font":backend,"font_family":[TEXT_FAMILY[0]],
                                "math_text_fallback":enabled,"color":"#b03050","font_size":size,
                                "style":if display {"display"} else {"inline"}}),
                            &mut f, &mut warnings, "/missing.lay", Loc {line:7,column:3,offset:0},
                        ).unwrap_or_else(|e| panic!("{backend} {source}: {e:?}"));
                        assert_eq!(
                            n["mathMissingGlyphs"].as_array().unwrap().len(),
                            count,
                            "{backend} {source}"
                        );
                        let warning = warnings
                            .iter()
                            .find(|w| w.message.contains("使用矢量方框替代"))
                            .unwrap();
                        assert_eq!(warning.code, "W_FONT");
                        assert_eq!(warning.file, "/missing.lay");
                        assert_eq!(warning.loc.line, 7);
                        assert!(
                            n["items"].as_array().unwrap().len() > 1,
                            "remaining formula vanished"
                        );
                        let (w, h) = (jnum(&n, "width", 0.), jnum(&n, "height", 0.));
                        for item in n["items"].as_array().unwrap() {
                            assert!(item["kind"] == "path" || item["kind"] == "rule");
                            if item["kind"] == "path" {
                                assert!(item["fill"] == "#b03050" || item["stroke"] == "#b03050");
                                let r = kurbo::BezPath::from_svg(item["d"].as_str().unwrap())
                                    .unwrap()
                                    .bounding_box();
                                assert!(
                                    r.x0 >= -1e-6
                                        && r.y0 >= -1e-6
                                        && r.x1 <= w + 1e-6
                                        && r.y1 <= h + 1e-6,
                                    "{backend} {source}: {r:?} outside {w}x{h}"
                                );
                            }
                        }
                        assert!(f.assets.is_empty());
                    }
                }
            }
        }
    }
}

#[test]
fn placeholder_shapes_are_identical_across_formula_fonts_and_text_policies() {
    let canonical = kurbo::BezPath::from_svg(
        "M0 0L0.65 0L0.65 0.8L0 0.8ZM0.045 0.045L0.045 0.755L0.605 0.755L0.605 0.045Z",
    )
    .unwrap();
    for backend in backends() {
        for enabled in [true, false] {
            let n = formula(&json!({"source":"\u{10ffff}","math_font":backend,"math_text_fallback":enabled,"font_size":5.}),
                &mut fonts(), &mut vec![], "/missing.lay", Loc::default()).unwrap();
            assert_eq!(n["mathMissingGlyphs"], json!([0x10ffff]));
            let items = n["items"].as_array().unwrap();
            assert_eq!(items.len(), 1);
            let mut actual = kurbo::BezPath::from_svg(items[0]["d"].as_str().unwrap()).unwrap();
            let bounds = actual.bounding_box();
            actual.apply_affine(kurbo::Affine::new([
                0.2,
                0.,
                0.,
                -0.2,
                -bounds.x0 / 5.,
                bounds.y1 / 5.,
            ]));
            for (a, b) in actual.elements().iter().zip(canonical.elements()) {
                assert_eq!(std::mem::discriminant(a), std::mem::discriminant(b));
                if let (Some(a), Some(b)) = (a.end_point(), b.end_point()) {
                    assert!(a.distance(b) < 1e-6);
                }
            }
            assert_eq!(actual.elements().len(), canonical.elements().len());
        }
    }
}

#[test]
fn a_text_face_with_cmap_coverage_but_no_vector_outlines_uses_boxes() {
    let mut bytes = TEXT_FONTS[0].1.to_vec();
    let count = u16::from_be_bytes(bytes[4..6].try_into().unwrap()) as usize;
    let at = (0..count)
        .map(|i| 12 + i * 16)
        .find(|&i| &bytes[i..i + 4] == b"glyf")
        .unwrap();
    bytes[at..at + 4].copy_from_slice(b"TEST");
    let face = ttf_parser::Face::parse(&bytes, 0).unwrap();
    assert!(face.glyph_index('A').is_some());
    assert!(
        face.glyph_bounding_box(face.glyph_index('A').unwrap())
            .is_none()
    );
    for backend in backends() {
        let mut f = fonts();
        f.register_font("/no-outlines.ttf", bytes.clone());
        let mut warnings = vec![];
        let n = formula(
            &json!({"source":r"\text{A B}+x^2","math_font":backend,
            "font_family":["/no-outlines.ttf"],"font_size":5.}),
            &mut f,
            &mut warnings,
            "/missing.lay",
            Loc::default(),
        )
        .unwrap();
        assert_eq!(n["mathMissingGlyphs"], json!(['A' as u32, 'B' as u32]));
        assert!(
            warnings
                .iter()
                .any(|w| w.code == "W_FONT" && w.message.contains("U+0041"))
        );
        assert!(jnum(&n, "width", 0.) >= 2. * 0.8 * 5.);
    }
}
