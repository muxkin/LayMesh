use laymesh_language::LanguageService;
fn labels(service: &LanguageService, source: &str) -> Vec<String> {
    service
        .completions("/test.lay", source.len())
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["label"].as_str().unwrap().to_string())
        .collect()
}
#[test]
fn dictionary_and_cmap_members_signature_and_names() {
    let mut s = LanguageService::new("en");
    for (source, expected) in [
        ("d={\"a\":1}\nx=d.", "items"),
        ("cm=cmap(\"viridis\")\nx=cm.", "sample"),
        ("x={}.get(", "default"),
        ("x=cmap(\"vir", "viridis"),
        ("x=color_scale(cmap=\"tw", "twilight"),
    ] {
        s.update("/test.lay", source);
        if expected == "default" {
            assert!(
                s.signature("/test.lay", source.len())["label"]
                    .as_str()
                    .unwrap()
                    .contains("default")
            );
        } else {
            assert!(
                labels(&s, source).iter().any(|n| n == expected),
                "{source}: {:?}",
                labels(&s, source)
            );
        }
    }
    for (source, label) in [
        ("d={\"a\":1}\nx=d.get(", "dict.get("),
        ("cm=cmap()\nx=cm.colors(", "cmap.colors("),
        ("x=cmap(\"viridis\").sample(", "cmap.sample("),
    ] {
        s.update("/test.lay", source);
        assert!(
            s.signature("/test.lay", source.len())["label"]
                .as_str()
                .unwrap()
                .starts_with(label)
        );
    }
}
#[test]
fn nested_loop_symbols_and_dictionary_braces_have_no_false_diagnostics() {
    let source = "d={\"b\":[1,2],\"a\":[3,4]}\ncm=cmap(\"viridis\")\nfor i,(name,ys) in enumerate(d.items()) {\ncolor=cm.sample(0.5)\nx=ys[0]\n}\nx=d.get(\"a\")";
    let mut s = LanguageService::new("en");
    s.update("/test.lay", source);
    assert_eq!(s.diagnostics("/test.lay"), serde_json::json!([]));
    let cursor = source.find("color=cm").unwrap();
    let suggestions = s.completions("/test.lay", cursor);
    for n in ["i", "name", "ys"] {
        assert!(
            suggestions
                .as_array()
                .unwrap()
                .iter()
                .any(|o| o["label"] == n),
            "{n}"
        );
    }
    let suggestions = s.completions("/test.lay", source.len());
    assert!(
        !suggestions
            .as_array()
            .unwrap()
            .iter()
            .any(|o| o["label"] == "ys")
    );
    for input in ["d={\"x\":missing}", "d={}\nd[\"x\"]=missing"] {
        s.update("/test.lay", input);
        assert!(
            s.diagnostics("/test.lay")
                .as_array()
                .unwrap()
                .iter()
                .any(|d| d["code"] == "E_NAME")
        );
    }
}
