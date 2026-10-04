use laymesh_core::{
    engine::{compile_source, predefined_variables},
    model::Host,
};
#[test]
fn strings_and_predefined_variables_have_identical_runtime_values() {
    let bare = "page=canvas(size=(80mm,60mm))\nopts=[round,triangle,top_left]\nfunction cap(value=round) { return value }\nif opts[0]==\"round\" { page.add(line(length=20mm,start_cap=cap(),end_head=head(shape=opts[1],size=(4mm,3mm))),anchor=opts[2]) }";
    let quoted = bare
        .replace(
            "opts=[round,triangle,top_left]",
            "opts=[\"round\",\"triangle\",\"top_left\"]",
        )
        .replace("value=round", "value=\"round\"");
    let a = compile_source(bare, "/test.lay", Host::default()).unwrap();
    let b = compile_source(&quoted, "/test.lay", Host::default()).unwrap();
    assert_eq!(a.nodes, b.nodes);
    let a=compile_source("page=canvas(size=(80mm,60mm))\nround=\"square\"\npage.add(line(length=20mm,start_cap=round))","/test.lay",Host::default()).unwrap();
    let b = compile_source(
        "page=canvas(size=(80mm,60mm))\npage.add(line(length=20mm,start_cap=\"square\"))",
        "/test.lay",
        Host::default(),
    )
    .unwrap();
    assert_eq!(a.nodes, b.nodes);
}
#[test]
fn registry_variables_are_unique_and_aliases_keep_string_values() {
    let mut names = std::collections::BTreeSet::new();
    for v in predefined_variables() {
        assert!(names.insert(v["name"].as_str().unwrap()));
        assert_eq!(v["type"], "string");
    }
    assert_eq!(
        laymesh_core::engine::predefined_value("ratex_katex"),
        Some("ratex-katex")
    );
    let e = compile_source(
        "page=canvas(size=(80mm,60mm))\nvalue=not_a_builtin",
        "/test.lay",
        Host::default(),
    )
    .unwrap_err();
    assert_eq!(e.code, "E_NAME");
}

#[test]
fn module_functions_share_predefined_values_and_keep_local_scope() {
    let mut host = Host::default();
    host.files.insert(
        "/parts.lay".into(),
        b"export caps=[round,square]\nexport function cap(value=round) { return value }".to_vec(),
    );
    let source = "import {caps,cap} from \"parts.lay\"\npage=canvas(size=(80mm,60mm))\npage.add(line(length=20mm,start_cap=cap(caps[1])))";
    let scene = compile_source(source, "/test.lay", host).unwrap();
    assert_eq!(scene.nodes.len(), 1);
}
