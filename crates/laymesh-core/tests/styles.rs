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
    let err = e.styled("rect", &Args::new(), &Args::new()).unwrap_err();
    assert_eq!(
        (
            err.code.as_str(),
            err.file.as_str(),
            err.loc.line,
            err.loc.column
        ),
        ("E_LCSS", "/style.lcss", 3, 3)
    );
}
#[test]
fn variable_cycles_invalid_selectors_and_trailing_junk_are_errors() {
    let e = engine("rect {--a:var(--b);--b:var(--a);fill:var(--a);}");
    assert!(
        e.styled("rect", &Args::new(), &Args::new())
            .unwrap_err()
            .message
            .contains("循环")
    );
    for css in [
        "rect:hover{fill:red}",
        "#name{fill:red}",
        "rect::unknown{fill:red}",
        "rect > {fill:red}",
        "rect {fill:red} trailing",
        "rect { fill:red !important; }",
        "rect {width:2mm}",
        "/* unclosed",
    ] {
        let mut e = Engine::new(Host::default());
        assert_eq!(
            e.stylesheet(css, "/bad.lcss", loc()).unwrap_err().code,
            "E_LCSS",
            "{css}"
        );
    }
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
    let err = compile_source(
        "\n\n\npage=canvas(size=(20,20),stylesheet=\"theme.lcss\")\npage.add(rect(size=(10,10)))",
        "/main.lay",
        host,
    )
    .unwrap_err();
    assert_eq!(
        (err.file.as_str(), err.loc.line, err.loc.column),
        ("/theme.lcss", 3, 3)
    );
    let err=compile_source("style { rect { fill:var(--missing); } }\npage=canvas(size=(20,20))\npage.add(rect(size=(10,10)))","/main.lay",Host::default()).unwrap_err();
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
