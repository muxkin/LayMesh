use laymesh_core::{
    Loc,
    engine::compile_source,
    model::{Host, jnum},
    text::{FontSystem, formula, layout_text},
};
use serde_json::json;
fn restricted(flags: u16) -> Vec<u8> {
    let mut bytes = include_bytes!("../../../tests/fonts/DejaVuSans.ttf").to_vec();
    let count = u16::from_be_bytes([bytes[4], bytes[5]]) as usize;
    let record = (0..count)
        .map(|i| 12 + i * 16)
        .find(|&i| &bytes[i..i + 4] == b"OS/2")
        .unwrap();
    let offset = u32::from_be_bytes(bytes[record + 8..record + 12].try_into().unwrap()) as usize;
    bytes[offset + 8..offset + 10].copy_from_slice(&flags.to_be_bytes());
    bytes
}
#[test]
fn restricted_fonts_warn_then_use_only_permitted_fallbacks() {
    for flags in [2, 0x100, 0x200] {
        let mut fonts = FontSystem::new(false);
        fonts.register_font("/restricted.ttf", restricted(flags));
        fonts.register_font("/allowed.ttf", restricted(0));
        let mut warnings = vec![];
        let loc = Loc {
            line: 9,
            column: 7,
            offset: 36,
        };
        let result=layout_text(&json!({"content":"ABC","font_size":4.,"font_family":["/restricted.ttf","/allowed.ttf"]}),None,&mut fonts,&mut warnings,"/test.lay",loc).unwrap();
        assert_eq!(result["runs"][0]["fontPath"], "/allowed.ttf");
        assert_eq!(fonts.assets.len(), 1);
        assert!(warnings.iter().any(|w| w.code == "W_FONT"
            && w.file == "/test.lay"
            && w.loc == loc
            && w.message.contains("嵌入")));
        let mut fonts = FontSystem::new(false);
        fonts.register_font("/restricted.ttf", restricted(flags));
        let mut warnings = vec![];
        let result = layout_text(
            &json!({"content":"A中","font_size":4.,"font_family":["/restricted.ttf"]}),
            None,
            &mut fonts,
            &mut warnings,
            "/test.lay",
            loc,
        )
        .unwrap();
        assert!(fonts.assets.is_empty());
        assert!(
            result["runs"]
                .as_array()
                .unwrap()
                .iter()
                .all(|r| r["kind"] == "box")
        );
        assert!(warnings.iter().any(|w| w.message.contains("U+4E2D")));
    }
}
#[test]
fn positive_kerning_cannot_silently_overflow_a_text_frame() {
    let mut host = Host::default();
    host.files.insert("/font.ttf".into(), restricted(0));
    let error=compile_source("page=canvas(size=(30 mm,20 mm))\npage.add(text(\"AA\",font_family=[\"/font.ttf\"],font_size=10 mm,size=(13.8 mm,auto)))","/test.lay",host.clone()).unwrap_err();
    assert_eq!(error.code, "E_LAYOUT");
    assert_eq!(error.loc.line, 2);
    assert!(error.message.contains("排版后文字"));
    let scene=compile_source("page=canvas(size=(30 mm,20 mm))\npage.add(text(\"AA\",font_family=[\"/font.ttf\"],font_size=10 mm,size=(14 mm,auto)))","/test.lay",host).unwrap();
    let fits = |node: &serde_json::Value| {
        node["runs"].as_array().unwrap().iter().all(|run| {
            jnum(run, "x", 0.) + jnum(run, "width", 0.) <= jnum(node, "width", 0.) + 0.05
        })
    };
    assert!(fits(&scene.nodes[0]));
    // Fault injection: keep the genuinely shaped glyph run, but accept the old
    // nominal-advance frame that is too narrow after positive kerning.
    let mut old = scene.nodes[0].clone();
    old["width"] = json!(13.8);
    assert!(
        !fits(&old),
        "the same width check must reject the old accepted frame"
    );
}
#[test]
fn ratex_display_list_units_lines_and_rectangles_are_preserved() {
    let mut line_count = 0;
    let mut dash_count = 0;
    let mut rect_count = 0;
    for (source, style, size) in [
        (
            r"\frac{a+b}{\sqrt{x}}",
            ratex_types::MathStyle::Text,
            11. * 25.4 / 72.,
        ),
        (
            r"\int_0^1 x^2\,dx = \frac{1}{3}",
            ratex_types::MathStyle::Display,
            18. * 25.4 / 72.,
        ),
        (
            r"\begin{array}{c}a\\\hdashline b\end{array}",
            ratex_types::MathStyle::Display,
            4.,
        ),
        (r"\colorbox{red}{x}", ratex_types::MathStyle::Text, 4.),
    ] {
        let ast = ratex_parser::parse(source).unwrap();
        let dl = ratex_layout::to_display_list(&ratex_layout::layout(
            &ast,
            &ratex_layout::LayoutOptions {
                style,
                ..Default::default()
            },
        ));
        let node=formula(&json!({"source":source,"font_size":size,"style":if style==ratex_types::MathStyle::Display{"display"}else{"inline"}}),&mut FontSystem::new(false),&mut vec![],"x",Loc::default()).unwrap();
        assert!((jnum(&node, "width", 0.) - dl.width * size).abs() < 1e-12);
        assert!((jnum(&node, "height", 0.) - (dl.height + dl.depth) * size).abs() < 1e-12);
        assert!((jnum(&node, "ascent", 0.) - dl.height * size).abs() < 1e-12);
        assert_eq!(node["items"].as_array().unwrap().len(), dl.items.len());
        for (raw, out) in dl.items.iter().zip(node["items"].as_array().unwrap()) {
            match raw {
                ratex_types::DisplayItem::Line {
                    x,
                    y,
                    width,
                    thickness,
                    dashed,
                    ..
                } => {
                    line_count += 1;
                    if *dashed {
                        dash_count += 1;
                    }
                    let center_matches = |item: &serde_json::Value| {
                        let center = jnum(item, "y", 0.)
                            + if *dashed {
                                0.
                            } else {
                                jnum(item, "height", 0.) / 2.
                            };
                        (center - y * size).abs() < 1e-12
                    };
                    assert!(center_matches(out), "{source}");
                    if !*dashed {
                        // Fault injection: the old adapter treated Line.y as
                        // a rectangle top instead of the RaTeX line center.
                        let mut old = out.clone();
                        old["y"] = json!(y * size);
                        assert!(
                            !center_matches(&old),
                            "old Line top must be rejected: {source}"
                        );
                    }
                    assert!((jnum(out, "x", 0.) - x * size).abs() < 1e-12);
                    assert!((jnum(out, "width", 0.) - width * size).abs() < 1e-12);
                    assert!((jnum(out, "height", 0.) - thickness * size).abs() < 1e-12);
                }
                ratex_types::DisplayItem::Rect {
                    x,
                    y,
                    width,
                    height,
                    ..
                } => {
                    rect_count += 1;
                    for (field, value) in [("x", x), ("y", y), ("width", width), ("height", height)]
                    {
                        assert!((jnum(out, field, 0.) - value * size).abs() < 1e-12);
                    }
                }
                _ => {}
            }
        }
    }
    assert!(line_count >= 4);
    assert!(dash_count > 0);
    assert!(rect_count > 0);
}

#[derive(Default)]
struct GlyphOutline(kurbo::BezPath);
impl ttf_parser::OutlineBuilder for GlyphOutline {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.move_to((f64::from(x), f64::from(y)));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.0.line_to((f64::from(x), f64::from(y)));
    }
    fn quad_to(&mut self, x: f32, y: f32, a: f32, b: f32) {
        self.0
            .quad_to((f64::from(x), f64::from(y)), (f64::from(a), f64::from(b)));
    }
    fn curve_to(&mut self, x: f32, y: f32, a: f32, b: f32, c: f32, d: f32) {
        self.0.curve_to(
            (f64::from(x), f64::from(y)),
            (f64::from(a), f64::from(b)),
            (f64::from(c), f64::from(d)),
        );
    }
    fn close(&mut self) {
        self.0.close_path();
    }
}
fn paths_equal(actual: &kurbo::BezPath, expected: &kurbo::BezPath) -> bool {
    use kurbo::PathEl;
    let point =
        |a: kurbo::Point, b: kurbo::Point| (a.x - b.x).abs() < 1e-9 && (a.y - b.y).abs() < 1e-9;
    actual.elements().len() == expected.elements().len()
        && actual
            .elements()
            .iter()
            .zip(expected.elements())
            .all(|(a, b)| match (*a, *b) {
                (PathEl::MoveTo(a), PathEl::MoveTo(b)) | (PathEl::LineTo(a), PathEl::LineTo(b)) => {
                    point(a, b)
                }
                (PathEl::QuadTo(a, c), PathEl::QuadTo(b, d)) => point(a, b) && point(c, d),
                (PathEl::CurveTo(a, c, e), PathEl::CurveTo(b, d, f)) => {
                    point(a, b) && point(c, d) && point(e, f)
                }
                (PathEl::ClosePath, PathEl::ClosePath) => true,
                _ => false,
            })
}
#[test]
fn ratex_operator_subscript_and_spaced_unit_keep_every_glyph_and_exact_scale() {
    use kurbo::Shape;
    for (source, expected) in [("I-I_0", "I−I0"), (r"t\;(\mathrm{s})", "t(s)")] {
        let size = 12. * 25.4 / 72.;
        let ast = ratex_parser::parse(source).unwrap();
        let dl = ratex_layout::to_display_list(&ratex_layout::layout(
            &ast,
            &ratex_layout::LayoutOptions::default(),
        ));
        let out = formula(
            &json!({"source":source,"font_size":size}),
            &mut FontSystem::new(false),
            &mut vec![],
            "formula.lay",
            Loc::default(),
        )
        .unwrap();
        assert_eq!(out["source"], source);
        assert!((jnum(&out, "width", 0.) - dl.width * size).abs() < 1e-12);
        assert_eq!(out["items"].as_array().unwrap().len(), dl.items.len());
        let chars: String = dl
            .items
            .iter()
            .filter_map(|item| match item {
                ratex_types::DisplayItem::GlyphPath { char_code, .. } => char::from_u32(*char_code),
                _ => None,
            })
            .collect();
        assert_eq!(chars, expected);
        let mut subscript = false;
        for (raw, item) in dl.items.iter().zip(out["items"].as_array().unwrap()) {
            let ratex_types::DisplayItem::GlyphPath {
                x,
                y,
                scale,
                font,
                char_code,
                ..
            } = raw
            else {
                continue;
            };
            let data = ratex_katex_fonts::ttf_bytes(&format!("KaTeX_{font}.ttf")).unwrap();
            let face = ttf_parser::Face::parse(&data, 0).unwrap();
            let mut glyph = GlyphOutline::default();
            face.outline_glyph(
                face.glyph_index(char::from_u32(*char_code).unwrap())
                    .unwrap(),
                &mut glyph,
            )
            .unwrap();
            let factor = size * scale / f64::from(face.units_per_em());
            let exact_path =
                kurbo::Affine::new([factor, 0., 0., -factor, x * size, y * size]) * glyph.0;
            let actual_path = kurbo::BezPath::from_svg(item["d"].as_str().unwrap()).unwrap();
            assert!(
                paths_equal(&actual_path, &exact_path),
                "every command and control point: {source} glyph {char_code}"
            );
            // A displaced internal control point can preserve the whole bounding
            // box; compare path commands themselves rather than bounds alone.
            let mut corrupt = actual_path.elements().to_vec();
            let target = corrupt.iter_mut().find(|element| {
                matches!(
                    element,
                    kurbo::PathEl::QuadTo(..) | kurbo::PathEl::CurveTo(..)
                )
            });
            if let Some(element) = target {
                match element {
                    kurbo::PathEl::QuadTo(control, _) | kurbo::PathEl::CurveTo(control, _, _) => {
                        control.x += 0.01
                    }
                    _ => unreachable!(),
                }
                assert!(!paths_equal(
                    &kurbo::BezPath::from_vec(corrupt),
                    &exact_path
                ));
            }
            let exact = exact_path.bounding_box();
            let actual = kurbo::BezPath::from_svg(item["d"].as_str().unwrap())
                .unwrap()
                .bounding_box();
            for (a, b) in [
                (actual.x0, exact.x0),
                (actual.y0, exact.y0),
                (actual.x1, exact.x1),
                (actual.y1, exact.y1),
            ] {
                assert!((a - b).abs() < 1e-9, "{source}: {actual:?} != {exact:?}");
            }
            if *char_code == u32::from('0') {
                subscript = *scale < 1. && *y > 0.;
            }
        }
        if source == "I-I_0" {
            assert!(subscript);
        }
    }
}

#[test]
fn font_selection_and_missing_glyphs_retain_distinct_source_locations() {
    let source = "page=canvas(size=(50mm,20mm))\na=text(content=\"中\",font_family=[\"Unavailable face\"])\npage.add(a)";
    let scene = compile_source(source, "/source.lay", Host::default()).unwrap();
    let unavailable = scene
        .warnings
        .iter()
        .find(|w| w.message.contains("Unavailable face"))
        .unwrap();
    let missing = scene
        .warnings
        .iter()
        .find(|w| w.message.contains("U+4E2D"))
        .unwrap();
    assert_eq!((unavailable.loc.line, unavailable.loc.column), (2, 3));
    assert_eq!((missing.loc.line, missing.loc.column), (2, 17));
    assert_eq!(unavailable.file, "/source.lay");
    assert_eq!(missing.file, "/source.lay");
    assert!(scene.fonts.is_empty());
    assert_eq!(scene.nodes[0]["runs"][0]["kind"], "box");
}
