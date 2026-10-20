use laymesh_language::LanguageService;
fn service(source: &str) -> LanguageService {
    let mut s = LanguageService::new("en");
    s.update("/art.lay", source);
    s
}
#[test]
fn artistic_parameters_complete_hover_and_validate_without_rendering() {
    let source = "p=canvas(size=(100,80))\np.add(text(\"Art\",";
    let s = service(source);
    let completions = s.completions("/art.lay", source.len());
    for key in [
        "text_fill",
        "text_stroke_width",
        "path",
        "warp",
        "extrude",
        "effects",
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
    let source = "shadow(blur=1mm,mode=\"inner\")";
    let s = service(source);
    assert!(s.hover("/art.lay", 2).to_string().contains("shadow"));
    assert!(s.signature("/art.lay", 7).to_string().contains("blur"));
    assert_eq!(s.diagnostics("/art.lay"), serde_json::json!([]));
    for source in [
        "shadow(blur=-1mm)",
        "glow(spread=-2mm)",
        "text_extrude(depth=-1mm)",
        "text_warp(\"wave\",wavelength=0mm)",
        "text(\"A\",effects=shadow())",
    ] {
        let ds = service(source).diagnostics("/art.lay");
        assert!(
            ds.as_array()
                .unwrap()
                .iter()
                .any(|v| v["code"] == "E_EFFECT"),
            "{source}: {ds}"
        );
    }
}
#[test]
fn object_effects_and_3d_complete_in_source_and_lcss() {
    for (source, expected) in [
        ("rect(", vec!["three_d", "effects"]),
        (
            "three_d(",
            vec!["bevel_top", "projection", "material", "light_direction"],
        ),
        ("reflection(", vec!["length", "fade", "distance"]),
        ("soft_edge(", vec!["radius"]),
    ] {
        let s = service(source);
        let options = s.completions("/art.lay", source.len());
        for key in expected {
            assert!(
                options
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|v| v["label"] == key),
                "{source}: {options}"
            );
        }
    }
    let mut s = LanguageService::new("en");
    for (source, key) in [
        (".a{ ", "three-d"),
        (".a{three-d:three_d(", "bevel_top"),
        (".a{effects:[reflection(", "fade"),
    ] {
        s.update("/effect.lcss", source);
        assert!(
            s.completions("/effect.lcss", source.len())
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v["label"] == key),
            "{source}"
        );
    }
    let source = "p=canvas(stylesheet=\"@laymesh/ef";
    s.update("/art.lay", source);
    assert!(
        s.completions("/art.lay", source.len())
            .to_string()
            .contains("@laymesh/effects.css")
    );
}
