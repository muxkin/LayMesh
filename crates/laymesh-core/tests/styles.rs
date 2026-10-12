//! LCSS cascade contracts: resolution depends on the target and its ancestors,
//! while font/image paths and diagnostics retain the defining stylesheet.
use laymesh_core::{
    Loc,
    engine::{Engine, compile_source},
    model::{Args, Host, V},
};
fn loc() -> Loc {
    Loc {
        line: 1,
        column: 1,
        offset: 0,
    }
}
fn args(class: &str) -> Args {
    [("class".into(), V::text(class))].into_iter().collect()
}
fn engine(css: &str) -> Engine {
    let mut e = Engine::new(Host::default());
    e.file = "/main.lay".into();
    e.stylesheet(css, "/style.lcss", loc()).unwrap();
    e
}
#[test]
fn variables_are_selector_specific_and_inherited_before_substitution() {
    let e = engine(
        "group.red {--ink:#ff0000;} group.blue {--ink:#0000ff;} rect {fill:var(--ink);border-color:var(--absent,var(--ink));} .irrelevant {--ink:#00ff00;}",
    );
    for (class, color) in [("red", "#ff0000"), ("blue", "#0000ff")] {
        let parent = e.styled("group", &args(class), &Args::new()).unwrap();
        let child = e.styled("rect", &Args::new(), &parent).unwrap();
        assert_eq!(child["fill"].as_str(), color);
        assert_eq!(child["border_color"].as_str(), color);
    }
}
#[test]
fn all_variable_declarations_cascade_before_property_evaluation() {
    let e = engine(
        "rect {fill:var(--ink);--ink:var(--alias);--alias:#ff0000;} rect.active {--alias:#0000ff;}",
    );
    let styled = e.styled("rect", &args("active"), &Args::new()).unwrap();
    assert_eq!(styled["fill"].as_str(), "#0000ff");
}
#[test]
fn detached_groups_measure_with_canvas_variables_and_font_before_placement() {
    let scene = compile_source(
        r#"page=canvas(size=(100mm,60mm),stylesheet="@laymesh/open-props.css")
style {
 :root {font-size:5mm; --ink:#123456;}
 .tile {background:var(--gradient-7);border-radius:var(--radius-3);}
 .label {color:var(--ink);font-size:1em;}
 group.local {--ink:#654321;}
}
function card() {
 g=group(class="local")
 g.add(rect(size=(30mm,20mm),class="tile"))
 g.add(text("Label",class="label"),offset=(2mm,2mm))
 return g
}
page.add(card(),offset=(10mm,10mm))"#,
        "/main.lay",
        Host::default(),
    )
    .unwrap();
    assert!(
        scene
            .warnings
            .iter()
            .all(|w| w.code != "W_CSS" || w.file != "/main.lay"),
        "{:?}",
        scene.warnings
    );
    let children = scene.nodes[0]["children"].as_array().unwrap();
    assert_eq!(children[0]["fill"]["kind"], "linearGradient");
    assert_eq!(children[0]["geometryRecipe"]["radius"], 5.);
    for run in children[1]["runs"].as_array().unwrap() {
        assert_eq!(run["color"], "#654321");
        assert_eq!(run["fontSize"], 5.);
    }
}
#[test]
fn specificity_source_order_shorthands_and_explicit_args_are_preserved() {
    let e = engine(
        "rect {border:1pt solid red;} .note {border-color:green;} rect.note {border:2pt double blue;} rect.note {border-width:3pt;}",
    );
    let mut explicit = args("note");
    explicit.insert("border_color".into(), V::text("#aabbcc"));
    let a = e.styled("rect", &explicit, &Args::new()).unwrap();
    assert_eq!(a["border_color"].as_str(), "#aabbcc");
    assert_eq!(a["border_style"].as_str(), "double");
    assert!(matches!(&a["border_width"], V::Number(value,unit) if *value == 3. && unit == "pt"));
}
#[test]
fn descendants_and_direct_children_use_the_full_ancestor_chain() {
    let e = engine(
        "group.outer rect {fill:red;} group.outer > rect {fill:blue;} group.inner > rect {border-color:green;}",
    );
    let outer = e.styled("group", &args("outer"), &Args::new()).unwrap();
    let direct = e.styled("rect", &Args::new(), &outer).unwrap();
    assert_eq!(direct["fill"].as_str(), "#0000ff");
    let inner = e.styled("group", &args("inner"), &outer).unwrap();
    let deep = e.styled("rect", &Args::new(), &inner).unwrap();
    assert_eq!(deep["fill"].as_str(), "#ff0000");
    assert_eq!(deep["border_color"].as_str(), "#008000");
}
#[test]
fn module_rules_do_not_leak_and_external_rules_override_module_defaults() {
    let mut h = Host::default();
    h.files.insert("/parts/module.lay".into(),b"style { rect.local { fill: red; } }\nexport function card(){return rect(size=(10,10),class=\"local\")}".to_vec());
    h.files
        .insert("/theme.lcss".into(), b"rect {border-color:blue;}".to_vec());
    let source = "import {card} from \"./parts/module.lay\"\npage=canvas(size=(60,40),stylesheet=\"./theme.lcss\")\npage.add(card())\npage.add(rect(size=(10,10),class=\"local\"),offset=(12,0))";
    let s = compile_source(source, "/main.lay", h.clone()).unwrap();
    assert_eq!(s.nodes[0]["fill"], "#ff0000");
    assert_eq!(s.nodes[1]["fill"], "none");
    h.files
        .insert("/theme.lcss".into(), b"rect {fill:blue;}".to_vec());
    let s = compile_source(source, "/main.lay", h).unwrap();
    assert_eq!(s.nodes[0]["fill"], "#0000ff");
    assert_eq!(s.nodes[1]["fill"], "#0000ff");
}
#[test]
fn imported_styles_keep_scope_relative_font_paths_and_declaration_locations() {
    let mut e = Engine::new(Host::default());
    e.file = "/main.lay".into();
    e.host.files.insert(
        "/theme/font.lcss".into(),
        b"text {font-family: './fonts/a.ttc#Face';color:var(--ink);}".to_vec(),
    );
    e.stylesheet_scoped(
        "@import url('font.lcss'); text {--ink:blue;}",
        "/theme/main.lcss",
        loc(),
        Some("/main.lay"),
    )
    .unwrap();
    let a = e.styled("text", &Args::new(), &Args::new()).unwrap();
    assert_eq!(a["color"].as_str(), "#0000ff");
    assert_eq!(
        a["font_family"].list()[0].as_str(),
        "/theme/fonts/a.ttc#Face"
    );
    e.file = "/other.lay".into();
    assert!(
        !e.styled("text", &Args::new(), &Args::new())
            .unwrap()
            .contains_key("color")
    );
    let e = engine("/* comment */\nrect {\n  fill:var(--missing);\n}");
    e.styled("rect", &Args::new(), &Args::new()).unwrap();
    let err = e.css_diagnostics().remove(0);
    assert_eq!(
        (
            err.code.as_str(),
            err.file.as_str(),
            err.loc.line,
            err.loc.column
        ),
        ("W_CSS", "/style.lcss", 3, 3)
    );
}
#[test]
fn variable_cycles_invalid_selectors_and_trailing_junk_are_recovered() {
    let e = engine("rect {--a:var(--b);--b:var(--a);fill:var(--a);border-color:blue;}");
    let a = e.styled("rect", &Args::new(), &Args::new()).unwrap();
    assert_eq!(a["border_color"].as_str(), "#0000ff");
    assert!(e.css_diagnostics()[0].message.contains("循环"));
    for css in [
        "rect:hover{fill:red}",
        "#name{fill:red}",
        "rect::unknown{fill:red}",
        "rect > {fill:red}",
        "rect {fill:red} trailing",
        "rect {width:2mm}",
    ] {
        let e = engine(css);
        assert_eq!(e.css_diagnostics()[0].code, "W_CSS", "{css}");
    }
    let e = engine("rect { fill:red !important; }");
    assert_eq!(
        e.styled("rect", &Args::new(), &Args::new()).unwrap()["fill"].as_str(),
        "#ff0000"
    );
}

#[test]
fn nested_groups_are_restyled_for_placement_ancestry() {
    let source = "style { group.outer {--ink:#ff0000;} group.outer rect {fill:var(--ink);} group.outer > rect {fill:#0000ff;} }\npage=canvas(size=(60,40))\ninner=group()\ninner.add(rect(size=(10,10)))\nouter=group(class=\"outer\")\nouter.add(inner)\nouter.add(rect(size=(10,10)),offset=(12,0))\npage.add(outer)";
    let s = compile_source(source, "/main.lay", Host::default()).unwrap();
    assert_eq!(s.nodes[0]["children"][0]["children"][0]["fill"], "#ff0000");
    assert_eq!(s.nodes[0]["children"][1]["fill"], "#0000ff");
}

#[test]
fn stylesheet_diagnostics_use_the_actual_file_and_inline_content_positions() {
    let mut host = Host::default();
    host.files.insert(
        "/theme.lcss".into(),
        b"/* font-independent theme */\nrect {\n  fill:var(--missing);\n}".to_vec(),
    );
    let scene = compile_source(
        "\n\n\npage=canvas(size=(20,20),stylesheet=\"theme.lcss\")\npage.add(rect(size=(10,10)))",
        "/main.lay",
        host,
    )
    .unwrap();
    let err = scene.warnings.iter().find(|w| w.code == "W_CSS").unwrap();
    assert_eq!(
        (err.file.as_str(), err.loc.line, err.loc.column),
        ("/theme.lcss", 3, 3)
    );
    let scene=compile_source("style { rect { fill:var(--missing); } }\npage=canvas(size=(20,20))\npage.add(rect(size=(10,10)))","/main.lay",Host::default()).unwrap();
    let err = scene.warnings.iter().find(|w| w.code == "W_CSS").unwrap();
    assert_eq!(
        (err.file.as_str(), err.loc.line, err.loc.column),
        ("/main.lay", 1, 16)
    );
}

#[test]
fn stylesheet_import_cycle_points_to_the_import_that_closes_the_cycle() {
    let mut e = Engine::new(Host::default());
    e.host.files.insert(
        "/base.lcss".into(),
        b"/* base */\n@import 'paper.lcss';".to_vec(),
    );
    e.host
        .files
        .insert("/paper.lcss".into(), b"@import 'base.lcss';".to_vec());
    let err = e
        .stylesheet("@import 'base.lcss';", "/paper.lcss", loc())
        .unwrap_err();
    assert_eq!(
        (err.code.as_str(), err.file.as_str(), err.loc.line),
        ("E_LCSS", "/base.lcss", 2)
    );
}

#[test]
fn class_attribute_selectors_share_matching_and_specificity_with_animations() {
    use laymesh_core::animation::css;
    for (selector, class, yes) in [
        ("rect.note[class*='Out']", "note animate__fadeOut", true),
        ("rect.note[class*='Out']", "note animate__fadeIn", false),
        ("[class~='note']", "noteworthy note", true),
        ("[class='note']", "note other", false),
        ("[class^=note]", "note other", true),
        ("[class$='Out' i]", "fadeout", true),
        ("[class|=note]", "note-card", true),
        ("[class*='']", "note", false),
        ("[class*='a > b']", "a > b", true),
    ] {
        let sheet = format!("{selector}{{fill:red}} rect{{fill:blue}}");
        let e = engine(&sheet);
        let styled = e.styled("rect", &args(class), &Args::new()).unwrap();
        assert_eq!(
            styled["fill"].as_str(),
            if yes { "#ff0000" } else { "#0000ff" },
            "{selector}"
        );
        assert_eq!(
            css::matches(
                selector,
                &[css::Element {
                    kind: "rect".into(),
                    class: class.into()
                }]
            ),
            yes,
            "{selector}"
        );
    }
    // A dot in a quoted attribute value must not add class specificity.
    assert_eq!(css::specificity("rect.note[class*='a.b']"), 201);
    assert!(
        engine("[data-bs-theme=dark]{fill:red}")
            .styled("rect", &Args::new(), &Args::new())
            .unwrap()
            .get("fill")
            .is_none()
    );
}
