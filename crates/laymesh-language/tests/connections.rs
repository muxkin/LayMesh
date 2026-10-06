use laymesh_language::LanguageService;
use serde_json::Value;
fn service(source: &str) -> LanguageService {
    let mut s = LanguageService::new("zh-CN");
    s.update("/connections.lay", source);
    s
}
fn diagnostics(source: &str) -> Value {
    service(source).diagnostics("/connections.lay")
}
#[test]
fn deferred_line_geometry_is_valid_including_aliases_and_multiline_definitions() {
    for source in [
        "page=canvas(size=(100,80))\nunused=line()",
        "page=canvas(size=(100,80))\nl=line()\npage.add(l,start=(10,20),end=(50,30))",
        "page=canvas(size=(100,80))\nl=line(\n line_width=0.5pt\n)\nalias=l\npage.add(alias,start=page.center,end=(50,30))",
        "page=canvas(size=(100,80))\npage.add(line(end_head=head()),start=(10,20),end=(50,30))",
        "page=canvas(size=(100,80))\nl=line()\nfunction f(l) { page.add(l) }",
        "page=canvas(size=(100,80))\nl=line()\nfunction f() { l=line(dx=2,dy=0) page.add(l) }",
    ] {
        assert_eq!(diagnostics(source), serde_json::json!([]), "{source}");
    }
}
#[test]
fn missing_geometry_diagnostic_points_to_add_for_variables_and_aliases() {
    for definition in ["l=line()", "a=line()\nl=a", "l=line(\n line_width=0.5pt\n)"] {
        let source = format!("page=canvas(size=(100,80))\n{definition}\npage.add(l)");
        let ds = diagnostics(&source);
        let d = ds
            .as_array()
            .unwrap()
            .iter()
            .find(|d| d["message"].as_str().unwrap().contains("未定义几何"))
            .unwrap();
        assert_eq!(d["from"], source.find("page.add").unwrap());
        assert_eq!(d["code"], "E_ARG");
    }
}
#[test]
fn connection_rules_are_diagnosed_without_executing_code() {
    for call in [
        "page.add(line(),start=(1,2))",
        "page.add(line(),start_offset=(1,2))",
        "page.add(rect(size=(2,2)),start=(1,2),end=(3,4))",
        "page.add(line(),start=(1,2),end=(3,4),rotation=0deg)",
        "page.add(line(),start=(1,2),end=(3,4),offset_space=\"target\")",
        "page.add(line(),start=(1,2),end=(3,4),end_offset_space=\"data\")",
        "line(dx=2)",
        "line(angle=20deg)",
    ] {
        let source = format!("page=canvas(size=(100,80))\n{call}");
        assert!(
            !diagnostics(&source).as_array().unwrap().is_empty(),
            "{source}"
        );
    }
}
#[test]
fn connection_parameters_complete_and_describe_units_and_frames() {
    let source = "page=canvas(size=(100,80))\npage.add(line(),";
    let s = service(source);
    let completions = s.completions("/connections.lay", source.len());
    for key in [
        "start",
        "end",
        "start_offset",
        "end_offset",
        "start_offset_space",
        "end_offset_space",
    ] {
        assert!(
            completions
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v["label"] == key),
            "{key}: {completions}"
        );
    }
    let signature = s.signature("/connections.lay", source.len());
    assert!(
        signature["parameters"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["name"] == "start")
    );
    let source =
        "page=canvas(size=(100,80))\npage.add(line(),start=(1,2),end=(3,4),start_offset_space=";
    let completions = service(source).completions("/connections.lay", source.len());
    for value in ["container", "target"] {
        assert!(
            completions.as_array().unwrap().iter().any(|v| v["label"]
                .as_str()
                .unwrap()
                .trim_matches('"')
                == value),
            "{completions}"
        );
    }
}
