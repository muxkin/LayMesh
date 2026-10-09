use laymesh_language::LanguageService;
use serde_json::{Value, json};
fn service(src: &str) -> LanguageService {
    let mut s = LanguageService::new("zh-CN");
    s.update("/arrows.lay", src);
    s
}
fn diagnostics(src: &str) -> Value {
    service(src).diagnostics("/arrows.lay")
}
#[test]
fn constructors_and_anchor_queries_have_no_false_diagnostics() {
    for src in [
        "page=canvas(size=(100,80))\na=page.add(arrow(),start=(10,20),end=(60,30))\npage.add(rect(size=(1,1)),target=a.centerline.at(fraction=0.5))",
        "page=canvas(size=(100,80))\nx=arrow.arc(sweep_angle=-240deg)\npage.add(x,start=(10,20),end=(60,30))",
        "page=canvas(size=(100,80))\npage.add(arrow.path(path=path(commands=[move_to(0,0),line_to(50,0)])))",
        "function arrow(size) {return rect(size=(size,size))}\nx=arrow(5)",
    ] {
        assert_eq!(diagnostics(src), json!([]), "{src}");
    }
}
#[test]
fn namespace_completion_signature_and_hover_are_specific() {
    let src = "page=canvas(size=(100,80))\na=arrow.";
    let s = service(src);
    let cs = s.completions("/arrows.lay", src.len());
    for name in ["arc", "bent", "uturn", "chevron", "path"] {
        assert!(
            cs.as_array().unwrap().iter().any(|x| x["label"] == name),
            "{cs}"
        );
    }
    assert!(
        !cs.as_array()
            .unwrap()
            .iter()
            .any(|x| x["label"] == "circular")
    );
    let src = "page=canvas(size=(100,80))\na=arrow.arc(";
    let s = service(src);
    let sig = s.signature("/arrows.lay", src.len());
    assert_eq!(sig["name"], "arrow.arc");
    assert!(
        sig["parameters"]
            .as_array()
            .unwrap()
            .iter()
            .any(|x| x["name"] == "sweep_angle")
    );
    assert!(
        !sig["parameters"]
            .as_array()
            .unwrap()
            .iter()
            .any(|x| x["name"] == "span")
    );
    let h = s.hover("/arrows.lay", src.find(".arc").unwrap() + 2);
    assert!(h["contents"].as_str().unwrap().contains("arrow.arc"));
    let src = "a=arrow(length=40)\np=canvas(size=(100,80))\nx=p.add(a)\nx.centerline.";
    let cs = service(src).completions("/arrows.lay", src.len());
    assert!(
        cs.as_array().unwrap().iter().any(|x| x["label"] == "at"),
        "{cs}"
    );
}
#[test]
fn shape_calls_never_receive_legacy_line_fixes() {
    for src in [
        "a=arrow(length=40)",
        "a=arrow.arc(radius=-30)",
        "a=arrow(shaft_width=(2,5))",
        "function arrow(line_color){return line(line_color=line_color)}\na=arrow(line_color=\"red\")",
    ] {
        assert!(
            !diagnostics(src)
                .as_array()
                .unwrap()
                .iter()
                .any(|x| x["code"] == "E_API_MIGRATION"),
            "{src}"
        );
    }
    let src = "a=arrow(dx=40,dy=0,line_color=\"red\")";
    assert!(diagnostics(src).as_array().unwrap().iter().any(|x| {
        x["replacement"]
            .as_str()
            .is_some_and(|x| x.starts_with("line("))
    }));
}
#[test]
fn deferred_arrow_geometry_is_reported_at_placement_only() {
    for src in ["a=arrow()", "a=arrow.arc(sweep_angle=90deg)"] {
        assert_eq!(diagnostics(src), json!([]));
    }
    for call in [
        "arrow()",
        "arrow.arc(radius=30)",
        "arrow.arc(sweep_angle=90deg)",
    ] {
        let src = format!("page=canvas(size=(100,80))\npage.add({call})");
        let ds = diagnostics(&src);
        assert!(
            ds.as_array()
                .unwrap()
                .iter()
                .any(|d| d["from"] == src.find("page.add").unwrap()),
            "{ds}"
        );
    }
}
