use laymesh_language::LanguageService;
use serde_json::Value;
fn service(tail: &str) -> (LanguageService, String) {
    let source = format!("page=canvas(size=(100,80))\na=page.add(rect(size=(20,10)))\n{tail}");
    let mut service = LanguageService::new("en");
    service.update("geometry.lay", &source);
    (service, source)
}
fn options(tail: &str) -> Value {
    let (s, text) = service(tail);
    s.completions("geometry.lay", text.len())
}
fn has(tail: &str, label: &str) {
    assert!(
        options(tail)
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["label"] == label),
        "{tail}: {}",
        options(tail)
    );
}
#[test]
fn geometry_members_after_arbitrary_indices_calls_and_aliases() {
    for (tail, label) in [
        ("a.", "bounds"),
        ("a.path.", "nearest"),
        ("a.ink.", "boundary"),
        ("a.path.subpaths[0].segments[1].", "controls"),
        ("a.path.nearest(to=a.bounds.center)[0].", "tangent_angle"),
        ("a.axes[\"x\"].spine.path.", "at"),
        ("a.axes[\"x\"].label.bounds.", "top_left"),
        ("route=a.path.subpaths[0]\nroute.", "between"),
        ("point=a.path.at(fraction=0.5)\npoint.", "tangent_angle"),
        ("self.path.", "start"),
    ] {
        has(tail, label);
    }
    assert!(
        !options("a.path.nearest(to=a.center).")
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["label"] == "tangent_angle")
    );
    assert_eq!(
        options("a.")
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["label"] == "path")
            .unwrap()["detail"],
        "geometry_path"
    );
    let (s, text) = service("a.ink.bounds");
    assert!(
        s.hover("geometry.lay", text.len() - 2)["contents"]
            .as_str()
            .unwrap()
            .contains("Box corners")
    );
    // Incomplete, cyclic aliases must remain a finite static query.
    let (s, text) = service("left=right.path\nright=left.path\nleft.");
    assert!(s.completions("geometry.lay", text.len()).is_array());
}
#[test]
fn chained_method_signatures_parameters_and_documentation() {
    for (tail, signature, parameter) in [
        ("a.path.subpaths[0].at(dis", "path.at(", "distance"),
        ("a.ink.boundary.nearest(to=", "path.nearest(", "to"),
        (
            "a.path.corners()[0].with_side(",
            "anchor.with_side(",
            "side",
        ),
        (
            "a.axes[\"x\"].spine.path.in_space(",
            "path.in_space(",
            "space",
        ),
    ] {
        let (s, text) = service(tail);
        let sig = s.signature("geometry.lay", text.len());
        assert!(
            sig["label"].as_str().unwrap().starts_with(signature),
            "{sig}"
        );
        assert!(
            sig["parameters"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v["name"] == parameter)
        );
    }
    let (s, text) = service("a.path.segments[0].at(t=0.5)");
    assert!(
        s.hover("geometry.lay", text.find("at(t").unwrap() + 1)["contents"]
            .as_str()
            .unwrap()
            .contains("arc-length")
    );
    has("page.add(rect(size=(1,1)),offset_sp", "offset_space");
}
#[test]
fn chained_queries_diagnostics_accept_self_and_check_methods() {
    let (s, _) = service(
        "page.add(line(dx=20,dy=0),anchor=self.path.start,target=a.path.nearest(to=a.bounds.top_left)[0],rotation=a.path.start.tangent_angle,offset_space=\"target\")",
    );
    assert_eq!(s.diagnostics("geometry.lay"), serde_json::json!([]));
    for tail in [
        "v=a.path.at(distance=10deg)",
        "v=a.path.subpaths[0].at(unknown=1)",
        "v=a.path.at(fraction=0.5).with_side(side=\"guess\")",
    ] {
        let (s, _) = service(tail);
        assert!(
            !s.diagnostics("geometry.lay").as_array().unwrap().is_empty(),
            "{tail}"
        );
    }
}
