use kurbo::Shape;
use laymesh_core::{
    Loc,
    engine::compile_source,
    model::{Host, Scene, jnum},
    text::{FontSystem, formula},
};
use serde_json::{Value, json};

const FONTS: [(&str, &[u8]); 4] = [
    (
        "Latin Modern Math",
        include_bytes!("../../../tests/fonts/math/latinmodern-math.otf"),
    ),
    (
        "STIX Two Math",
        include_bytes!("../../../tests/fonts/math/STIX2Math.otf"),
    ),
    (
        "XITS Math",
        include_bytes!("../../../tests/fonts/math/XITSMath-Regular.otf"),
    ),
    (
        "XITS Math Bold",
        include_bytes!("../../../tests/fonts/math/XITSMath-Bold.otf"),
    ),
];
const CORPUS: [&str; 14] = [
    r"E=mc^2",
    r"x=\frac{-b\pm\sqrt{b^2-4ac}}{2a}",
    r"\frac{1}{1+\frac{1}{x}}",
    r"\sum_{i=1}^{n} x_i^2",
    r"\int_0^1 e^{-x^2}\,dx",
    r"\left(\frac{a}{b}\right)",
    r"\begin{pmatrix}a&b\\c&d\end{pmatrix}",
    r"\sqrt[3]{\frac{a}{b}}",
    r"\binom{n}{k}",
    r"\bm{\alpha}+\mathbf{A}\bm{x}",
    r"\mathcal{F}+\mathbb{R}+\mathfrak{g}+\mathsf{A}+\mathtt{B}",
    r"\widehat{ABC}+\vec{x}",
    r"\overline{a+b}+\underline{x}",
    r"\sin(x)+\operatorname{erf}(x)",
];
fn render_formula(bytes: &[u8], source: &str, display: bool) -> Value {
    let mut fonts = FontSystem::new(false);
    fonts.register_font("/math.otf", bytes.to_vec());
    let mut warnings = vec![];
    let result = formula(
        &json!({"source":source,"math_font":"/math.otf","font_size":5.,
        "style":if display{"display"}else{"inline"}}),
        &mut fonts,
        &mut warnings,
        "/main.lay",
        Loc::default(),
    )
    .unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    assert!(
        fonts.assets.is_empty(),
        "outlined math must not embed a redundant full font"
    );
    result
}
fn glyphs(node: &Value) -> Vec<&Value> {
    node["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| v["kind"] == "path")
        .collect()
}
fn host() -> Host {
    let mut h = Host::default();
    for (i, (_, data)) in FONTS.iter().enumerate() {
        h.files.insert(format!("/fonts/{i}.otf"), data.to_vec());
    }
    h.files.insert(
        "/text.ttf".into(),
        include_bytes!("../../../tests/fonts/DejaVuSans.ttf").to_vec(),
    );
    h
}
fn compile(source: &str) -> Scene {
    compile_source(source, "/main.lay", host()).unwrap()
}
#[test]
fn common_formulas_use_only_the_selected_font_and_fit_their_measured_bounds() {
    for (name, bytes) in &FONTS[..3] {
        for source in CORPUS {
            for display in [false, true] {
                let node = render_formula(bytes, source, display);
                assert_eq!(node["mathFontBackend"], "opentype-math");
                let width = jnum(&node, "width", 0.);
                let height = jnum(&node, "height", 0.);
                assert!(
                    width.is_finite() && width > 0. && height.is_finite() && height > 0.,
                    "{name}: {source}"
                );
                for item in node["items"].as_array().unwrap() {
                    if item["kind"] == "path" {
                        let p = kurbo::BezPath::from_svg(item["d"].as_str().unwrap()).unwrap();
                        let r = p.bounding_box();
                        assert!(
                            r.x0 >= -1e-6
                                && r.y0 >= -1e-6
                                && r.x1 <= width + 1e-6
                                && r.y1 <= height + 1e-6,
                            "{name}: {source}: {r:?} exceeds {width} x {height}"
                        );
                    } else {
                        assert_eq!(item["kind"], "rule");
                        assert!(jnum(item, "height", 0.) > 0.);
                    }
                }
            }
        }
    }
}
fn table_offset(data: &[u8], tag: &[u8; 4]) -> usize {
    let count = u16::from_be_bytes(data[4..6].try_into().unwrap()) as usize;
    (0..count)
        .find_map(|i| {
            let at = 12 + i * 16;
            (&data[at..at + 4] == tag)
                .then(|| u32::from_be_bytes(data[at + 8..at + 12].try_into().unwrap()) as usize)
        })
        .unwrap()
}
#[test]
fn changing_only_math_constants_changes_rules_and_script_geometry() {
    // Fault injection: identical glyph outlines, different OpenType MATH
    // data. A backend which only swaps font files cannot pass this test.
    let bytes = FONTS[2].1;
    let offset = table_offset(bytes, b"MATH");
    let constants = offset
        + usize::from(u16::from_be_bytes(
            bytes[offset + 4..offset + 6].try_into().unwrap(),
        ));
    let mut changed = bytes.to_vec();
    let original = ttf_parser::Face::parse(bytes, 0).unwrap();
    let c = original.tables().math.unwrap().constants.unwrap();
    // MathConstants offsets from the OpenType specification.
    changed[constants + 144..constants + 146]
        .copy_from_slice(&(c.fraction_rule_thickness().value * 2).to_be_bytes());
    changed[constants..constants + 2].copy_from_slice(&50_i16.to_be_bytes());
    changed[constants + 2..constants + 4].copy_from_slice(&30_i16.to_be_bytes());
    let a = render_formula(bytes, r"\frac{a}{b}", true);
    let b = render_formula(&changed, r"\frac{a}{b}", true);
    let rule = |n: &Value| {
        n["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["kind"] == "rule")
            .unwrap()["height"]
            .as_f64()
            .unwrap()
    };
    assert!((rule(&b) - 2. * rule(&a)).abs() < 1e-10);
    let a = render_formula(bytes, "x^2", true);
    let b = render_formula(&changed, "x^2", true);
    let script = |n: &Value| {
        glyphs(n)
            .into_iter()
            .find(|g| g["mathCodepoint"] == json!('2' as u32))
            .unwrap()["fontSize"]
            .as_f64()
            .unwrap()
    };
    assert_ne!(script(&a), script(&b));
    assert!((script(&b) - 2.5).abs() < 1e-10);
    assert_ne!(a["height"], b["height"]);
}
#[test]
fn bold_italic_and_alphabet_commands_select_actual_unicode_glyphs() {
    let normal = render_formula(FONTS[2].1, "x", false);
    let bold = render_formula(FONTS[2].1, r"\mathbf{x}", false);
    let bm = render_formula(FONTS[2].1, r"\bm{x}", false);
    assert_eq!(glyphs(&normal)[0]["mathCodepoint"], json!(0x1D465));
    assert_eq!(glyphs(&bold)[0]["mathCodepoint"], json!(0x1D431));
    assert_eq!(glyphs(&bm)[0]["mathCodepoint"], json!(0x1D499));
    assert_ne!(glyphs(&normal)[0]["d"], glyphs(&bold)[0]["d"]);
    assert_ne!(glyphs(&normal)[0]["d"], glyphs(&bm)[0]["d"]);
    let special = render_formula(FONTS[2].1, r"\mathbb{R}+\mathcal{F}+\mathit{h}", false);
    let cps: Vec<_> = glyphs(&special)
        .iter()
        .filter_map(|g| g["mathCodepoint"].as_u64())
        .collect();
    assert!(cps.contains(&0x211D) && cps.contains(&0x2131) && cps.contains(&0x210E));
}
#[test]
fn tall_delimiters_use_the_fonts_assembly_parts() {
    let source = r"\left(\begin{matrix}a\\b\\c\\d\\e\\f\\g\\h\\i\\j\end{matrix}\right)";
    for (name, bytes) in &FONTS[..3] {
        let n = render_formula(bytes, source, true);
        let count = glyphs(&n)
            .iter()
            .filter(|g| g["mathCodepoint"].is_null())
            .count();
        assert!(
            count >= 6,
            "{name}: tall parentheses must use multiple actual MATH assembly glyphs"
        );
        assert!(jnum(&n, "height", 0.) > 40.);
    }
}
#[test]
fn unknown_fonts_missing_math_glyphs_and_unsupported_commands_are_explicit_errors() {
    let mut fonts = FontSystem::new(false);
    fonts.register_font(
        "/ordinary.ttf",
        include_bytes!("../../../tests/assets/GFSNeohellenic.otf").to_vec(),
    );
    fonts.register_font("/math.otf", FONTS[2].1.to_vec());
    for (font, source, code, fragment) in [
        ("Unavailable Math", "x", "E_MATH_FONT", "不可用"),
        ("/ordinary.ttf", "x", "E_MATH_FONT", "MATH"),
        ("/math.otf", r"\text{中文}", "E_FORMULA", "U+"),
        ("/math.otf", r"\def\a{x}\a", "E_FORMULA", "不允许"),
    ] {
        let e = formula(
            &json!({"source":source,"math_font":font}),
            &mut fonts,
            &mut vec![],
            "/main.lay",
            Loc {
                line: 4,
                column: 9,
                offset: 0,
            },
        )
        .unwrap_err();
        assert_eq!(e.code, code);
        assert!(e.message.contains(fragment), "{e}");
        assert_eq!(e.loc.line, 4);
    }
}
#[test]
fn module_relative_paths_and_in_memory_family_names_work_without_system_fonts() {
    let mut h = host();
    h.files
        .insert("/module/font.otf".into(), FONTS[2].1.to_vec());
    h.files.insert(
        "/module/label.lay".into(),
        br#"export label=formula(r"\frac{a}{b}",math_font="./font.otf")"#.to_vec(),
    );
    let s=compile_source("import {label} from \"./module/label.lay\"\npage=canvas(size=(60mm,40mm))\npage.add(label)","/main.lay",h).unwrap();
    assert_eq!(s.nodes[0]["mathFont"], "XITS Math");
    let mut fonts = FontSystem::new(false);
    fonts.register_font("/xits.otf", FONTS[2].1.to_vec());
    let n = formula(
        &json!({"source":"x","math_font":"XITS Math"}),
        &mut fonts,
        &mut vec![],
        "/main.lay",
        Loc::default(),
    )
    .unwrap();
    assert_eq!(n["mathFont"], "XITS Math");
}
#[test]
fn inline_math_inherits_the_selected_math_font_and_keeps_a_shared_baseline() {
    let s = compile(
        r#"page=canvas(size=(100mm,40mm),math_font="./fonts/2.otf",font_family="./text.ttf")
page.add(text(spans=[span("A "),formula(r"\frac{x}{2}"),span(" B")],font_size=12pt))"#,
    );
    let runs = s.nodes[0]["runs"].as_array().unwrap();
    let math = runs.iter().find(|v| v["kind"] == "formula").unwrap();
    assert_eq!(math["mathFont"], "XITS Math");
    let baseline = runs[0]["baseline"].as_f64().unwrap();
    assert!((jnum(math, "y", 0.) + jnum(math, "ascent", 0.) - baseline).abs() < 1e-9);
    let s = compile(
        r#"page=canvas(size=(100mm,40mm))
page.add(text("A $x^2$ B",math_font="./fonts/1.otf",font_family="./text.ttf",font_size=12pt))"#,
    );
    assert!(
        s.nodes[0]["runs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["mathFont"] == "STIX Two Math")
    );
}
#[test]
fn lcss_math_font_paths_are_relative_to_the_stylesheet() {
    let mut h = host();
    h.files.insert(
        "/styles/theme.lcss".into(),
        br#"formula { math_font: "../fonts/2.otf"; }"#.to_vec(),
    );
    let s=compile_source("page=canvas(size=(50mm,40mm),stylesheet=\"./styles/theme.lcss\")\npage.add(formula(\"x\"))","/main.lay",h).unwrap();
    assert_eq!(s.nodes[0]["mathFont"], "XITS Math");
}
#[test]
fn registering_replacement_font_bytes_invalidates_the_math_face_cache() {
    let mut fonts = FontSystem::new(false);
    fonts.register_font("/math.otf", FONTS[0].1.to_vec());
    let spec = json!({"source":"x","math_font":"/math.otf"});
    let a = formula(&spec, &mut fonts, &mut vec![], "/main.lay", Loc::default()).unwrap();
    fonts.register_font("/math.otf", FONTS[2].1.to_vec());
    let b = formula(&spec, &mut fonts, &mut vec![], "/main.lay", Loc::default()).unwrap();
    assert_eq!(b["mathFont"], "XITS Math");
    assert_ne!(a["items"], b["items"]);
}
#[test]
fn xits_bold_face_changes_outlines_and_reports_its_incomplete_stretch_coverage() {
    let normal = render_formula(FONTS[2].1, "E=mc^2", true);
    let bold = render_formula(FONTS[3].1, "E=mc^2", true);
    assert_eq!(normal["mathFontFace"], "XITSMath-Regular");
    assert_eq!(bold["mathFontFace"], "XITSMath-Bold");
    assert_ne!(normal["items"], bold["items"]);
    let mut fonts = FontSystem::new(false);
    fonts.register_font("/bold.otf", FONTS[3].1.to_vec());
    let e = formula(
        &json!({"source":r"\sqrt{\frac{a}{b}}","math_font":"/bold.otf","style":"display"}),
        &mut fonts,
        &mut vec![],
        "/main.lay",
        Loc::default(),
    )
    .unwrap_err();
    assert!(e.message.contains("伸缩构造"));
}
#[test]
fn combining_accents_attach_to_the_ink_without_an_extra_baseline_gap() {
    for (_, bytes) in &FONTS[..3] {
        let n = render_formula(bytes, r"\vec{x}", false);
        let glyph = glyphs(&n);
        let ink = |g: &Value| {
            kurbo::BezPath::from_svg(g["d"].as_str().unwrap())
                .unwrap()
                .bounding_box()
        };
        let gap = ink(glyph[0]).y0 - ink(glyph[1]).y1;
        assert!(
            gap > 0. && gap < 0.6,
            "accent gap {gap} must stay near the letter at 5 mm type size"
        );
    }
}
#[test]
fn math_fonts_obey_embedding_restrictions_and_bad_math_data_fails() {
    for flags in [2_u16, 0x100, 0x200] {
        let mut bytes = FONTS[2].1.to_vec();
        let at = table_offset(&bytes, b"OS/2");
        bytes[at + 8..at + 10].copy_from_slice(&flags.to_be_bytes());
        let mut fonts = FontSystem::new(false);
        fonts.register_font("/restricted.otf", bytes);
        let e = formula(
            &json!({"source":"x","math_font":"/restricted.otf"}),
            &mut fonts,
            &mut vec![],
            "/main.lay",
            Loc::default(),
        )
        .unwrap_err();
        assert_eq!(e.code, "E_MATH_FONT");
        assert!(e.message.contains("嵌入"));
    }
    let mut bytes = FONTS[2].1.to_vec();
    let at = table_offset(&bytes, b"MATH");
    let constants = at
        + usize::from(u16::from_be_bytes(
            bytes[at + 4..at + 6].try_into().unwrap(),
        ));
    bytes[constants..constants + 2].fill(0);
    let mut fonts = FontSystem::new(false);
    fonts.register_font("/bad.otf", bytes);
    let e = formula(
        &json!({"source":"x^2","math_font":"/bad.otf"}),
        &mut fonts,
        &mut vec![],
        "/main.lay",
        Loc::default(),
    )
    .unwrap_err();
    assert!(e.message.contains("无效"));
}
#[test]
fn plot_labels_load_math_fonts_from_the_in_memory_host() {
    let s = compile(
        r#"page=canvas(size=(110mm,80mm))
p=plot(size=(100mm,70mm),style=plot_style(math_font="./fonts/2.otf"),
       x=axis(range=(0,1),label="$x^2$"),y=axis(range=(0,1)))
p.line(x=[0,1],y=[0,1])
page.add(p)"#,
    );
    fn has_math(v: &Value) -> bool {
        match v {
            Value::Object(map) => v["mathFont"] == "XITS Math" || map.values().any(has_math),
            Value::Array(items) => items.iter().any(has_math),
            _ => false,
        }
    }
    assert!(has_math(&json!(s.nodes)));
}
#[test]
fn operator_limits_respect_font_baseline_rise_and_drop_minima() {
    for (_, bytes) in &FONTS[..3] {
        let n = render_formula(bytes, r"\sum_i^n", true);
        let gs = glyphs(&n);
        let find = |cp| gs.iter().find(|g| g["mathCodepoint"] == json!(cp)).unwrap();
        let op = find(0x2211);
        let bounds = kurbo::BezPath::from_svg(op["d"].as_str().unwrap())
            .unwrap()
            .bounding_box();
        let font = ttf_parser::Face::parse(bytes, 0).unwrap();
        let c = font.tables().math.unwrap().constants.unwrap();
        let unit = 5. / f64::from(font.units_per_em());
        assert!(
            bounds.y0 - jnum(find(0x1D45B), "baseline", 0.) + 1e-9
                >= f64::from(c.upper_limit_baseline_rise_min().value) * unit
        );
        assert!(
            jnum(find(0x1D456), "baseline", 0.) - bounds.y1 + 1e-9
                >= f64::from(c.lower_limit_baseline_drop_min().value) * unit
        );
    }
}

#[test]
fn chemistry_and_units_keep_upright_elements_and_real_script_sizes() {
    for (_, bytes) in &FONTS[..3] {
        let n = render_formula(
            bytes,
            r"\ce{^{227}_{90}Th+ ->[H2O] ThO2} + \pu{1.2e3 kJ//mol}",
            true,
        );
        let g = glyphs(&n);
        assert!(g.iter().any(|v| v["mathCodepoint"] == u32::from('T')));
        assert!(!g.iter().any(|v| v["mathCodepoint"] == 0x1D447_u32));
        let sizes: Vec<_> = g.iter().map(|v| jnum(v, "fontSize", 0.)).collect();
        assert!(sizes.iter().any(|s| *s < 4.) && sizes.iter().any(|s| (*s - 5.).abs() < 1e-6));
    }
}
#[test]
fn proof_rules_labels_and_root_direction_are_preserved() {
    let bytes = FONTS[2].1;
    let root =
        r"\begin{prooftree}\AxiomC{P}\LeftLabel{cut}\RightLabel{r1}\UnaryInfC{Q}\end{prooftree}";
    let bottom = render_formula(bytes, root, true);
    let top = render_formula(
        bytes,
        &root.replace("\\UnaryInfC", "\\rootAtTop\\UnaryInfC"),
        true,
    );
    let y = |node: &Value, ch: char| {
        glyphs(node)
            .iter()
            .find(|v| v["mathCodepoint"] == u32::from(ch))
            .unwrap()["baseline"]
            .as_f64()
            .unwrap()
    };
    assert!(y(&bottom, 'P') < y(&bottom, 'Q'));
    assert!(y(&top, 'P') > y(&top, 'Q'));
    let rules = |n: &Value| {
        n["items"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| v["kind"] == "rule")
            .count()
    };
    assert_eq!(rules(&bottom), 1);
    assert_eq!(
        rules(&render_formula(
            bytes,
            &root.replace("\\UnaryInfC", "\\noLine\\UnaryInfC"),
            true
        )),
        0
    );
    assert!(
        rules(&render_formula(
            bytes,
            &root.replace("\\UnaryInfC", "\\dashedLine\\UnaryInfC"),
            true
        )) > 2
    );
}
#[test]
fn aligned_equalities_share_a_column_and_equation_tags_render() {
    for (_, bytes) in &FONTS[..3] {
        let n = render_formula(
            bytes,
            r"\begin{aligned}x&=a+b\\longname&=c\end{aligned}",
            true,
        );
        let eq: Vec<_> = glyphs(&n)
            .into_iter()
            .filter(|v| v["mathCodepoint"] == u32::from('='))
            .collect();
        assert_eq!(eq.len(), 2);
        assert!((jnum(eq[0], "x", 0.) - jnum(eq[1], "x", 0.)).abs() < 1e-6);
        let n = render_formula(
            bytes,
            r"\begin{align}x&=a\tag{A}\\y&=b\tag{B}\end{align}",
            true,
        );
        for ch in ['A', 'B'] {
            assert!(
                glyphs(&n)
                    .iter()
                    .any(|v| v["mathCodepoint"] == u32::from(ch))
            );
        }
    }
}
#[test]
fn deep_delimiter_nesting_does_not_repeat_layout_exponentially() {
    let source = format!("{}x{}", r"\left(".repeat(30), r"\right)".repeat(30));
    for (_, bytes) in &FONTS[..3] {
        render_formula(bytes, &source, true);
    }
    render_formula(
        FONTS[2].1,
        r"\left\{x\in\mathbb{R}\middle|\left(\frac{x}{2}\right)>0\right\}",
        true,
    );
}
#[test]
fn unicode_variants_and_negations_do_not_require_katex_private_use_glyphs() {
    for (_, bytes) in &FONTS[1..3] {
        let n = render_formula(bytes, r"\imath+\jmath\neq\varsubsetneq\ngeqslant", true);
        assert!(glyphs(&n).iter().all(|v| {
            v["mathCodepoint"]
                .as_u64()
                .is_none_or(|cp| !(0xE000..=0xF8FF).contains(&cp))
        }));
    }
}
#[test]
fn zero_width_break_controls_do_not_change_negation_or_atom_spacing() {
    for (_, bytes) in &FONTS[..3] {
        let a = render_formula(bytes, "a+b", true);
        let b = render_formula(bytes, r"a\allowbreak+\nobreak b", true);
        assert!((jnum(&a, "width", 0.) - jnum(&b, "width", 0.)).abs() < 1e-8);
        let eq = render_formula(bytes, "=", true);
        let ne = render_formula(bytes, r"\neq", true);
        assert!(
            (jnum(&eq, "width", 0.) - jnum(&ne, "width", 0.)).abs() < 1e-8,
            "eq {} ne {}; equals x {:?}",
            eq["width"],
            ne["width"],
            glyphs(&ne)
                .iter()
                .map(|v| (&v["mathCodepoint"], &v["x"]))
                .collect::<Vec<_>>()
        );
    }
}
#[test]
fn raw_dsl_strings_preserve_nested_mhchem_math_islands() {
    let source = r"\ce{CH4 + 2 $\left( \ce{O2 + 79/21 N2} \right)$}";
    let s = compile(&format!(
        "page=canvas(size=(120mm,40mm))\npage.add(formula(r\"{source}\",math_font=\"/fonts/2.otf\"))"
    ));
    assert_eq!(s.nodes[0]["source"], source);
    assert!(
        !glyphs(&s.nodes[0])
            .iter()
            .any(|v| v["mathCodepoint"] == 0x1D459_u32)
    ); // no literal italic l from "left"
}
#[test]
fn bold_commands_include_real_bold_digits_and_letters() {
    for (_, bytes) in &FONTS[..3] {
        let n = render_formula(bytes, r"\bm{1\alpha x}+\mathbf{Ax1}", true);
        let codes: Vec<_> = glyphs(&n)
            .iter()
            .filter_map(|v| v["mathCodepoint"].as_u64())
            .collect();
        for cp in [0x1D7CF, 0x1D736, 0x1D499, 0x1D400, 0x1D431] {
            assert!(codes.contains(&cp), "missing {cp:X}: {codes:X?}");
        }
    }
}

#[test]
fn inner_math_alphabet_commands_override_outer_commands() {
    for (_, bytes) in &FONTS[..3] {
        for (inner, ch) in [
            ("mathcal", "A"),
            ("mathfrak", "g"),
            ("mathsfit", "x"),
            ("mathsf", "x"),
            ("mathbf", "A"),
            ("mathit", "x"),
            ("mathrm", "x"),
            ("mathnormal", "x"),
        ] {
            let expected = render_formula(bytes, &format!("\\{inner}{{{ch}}}"), true);
            for outer in ["mathbf", "mathcal", "mathit", "mathsf", "bm"] {
                let source = format!("\\{outer}{{\\{inner}{{{ch}}}}}");
                let n = render_formula(bytes, &source, true);
                assert_eq!(n["items"], expected["items"], "{source}");
                assert_eq!(n["width"], expected["width"], "{source}");
            }
        }
    }
}

#[test]
fn styled_unicode_letters_keep_their_native_math_glyphs() {
    for (_, bytes) in &FONTS[..3] {
        for cp in [0x1D400, 0x1D4D0, 0x1D58C, 0x1D639, 0x1D66D, 0x1D7CF] {
            let source = char::from_u32(cp).unwrap().to_string();
            let n = render_formula(bytes, &source, true);
            assert_eq!(glyphs(&n)[0]["mathCodepoint"], cp);
            let face = ttf_parser::Face::parse(bytes, 0).unwrap();
            assert_eq!(
                glyphs(&n)[0]["glyphId"],
                face.glyph_index(char::from_u32(cp).unwrap()).unwrap().0
            );
        }
    }
}

#[test]
fn chemistry_bonds_use_math_minus_and_text_hyphens_in_separate_layers() {
    for (font, bytes) in &FONTS[..3] {
        let minus = render_formula(bytes, "-", true);
        let hyphen = render_formula(bytes, r"\text{-}", true);
        assert_eq!(glyphs(&minus)[0]["mathCodepoint"], 0x2212_u32);
        assert_eq!(glyphs(&hyphen)[0]["mathCodepoint"], 0x2D_u32);
        for (bond, solid_count) in [("~-", 1), ("~--", 2), ("~=", 2), ("-~-", 2)] {
            let n = render_formula(bytes, &format!(r"\ce{{A\bond{{{bond}}}B}}"), true);
            let items = glyphs(&n);
            let solid: Vec<_> = items
                .iter()
                .filter(|v| v["mathCodepoint"] == 0x2212_u32)
                .collect();
            let dashed: Vec<_> = items
                .iter()
                .filter(|v| v["mathCodepoint"] == 0x2D_u32)
                .collect();
            assert_eq!(solid.len(), solid_count, "{bond}");
            assert_eq!(dashed.len(), 3, "{bond}");
            for s in &solid {
                assert_eq!(s["glyphId"], glyphs(&minus)[0]["glyphId"], "{bond}");
                assert_eq!(s["x"], solid[0]["x"], "{bond}");
            }
            let b = items
                .iter()
                .find(|v| v["mathCodepoint"] == u32::from('B'))
                .unwrap();
            let next_ink = kurbo::BezPath::from_svg(b["d"].as_str().unwrap())
                .unwrap()
                .bounding_box();
            for s in solid.iter().chain(dashed.iter()) {
                let bounds = kurbo::BezPath::from_svg(s["d"].as_str().unwrap())
                    .unwrap()
                    .bounding_box();
                assert!(
                    bounds.x1 <= next_ink.x0 + 1e-6,
                    "{font} {bond}: U+{:X} bounds {bounds:?}, next atom ink {next_ink:?}",
                    s["mathCodepoint"].as_u64().unwrap(),
                );
            }
        }
    }
}
