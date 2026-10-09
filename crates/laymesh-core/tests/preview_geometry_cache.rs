use laymesh_core::{asset_cache, engine::compile_source, model::Host};

#[test]
fn font_host_binding_observes_buffers_added_after_engine_creation() {
    let mut engine = laymesh_core::engine::Engine::new(Host::default());
    engine.host.files.insert(
        "/new.ttf".into(),
        include_bytes!("../../../tests/fonts/DejaVuSans.ttf").to_vec(),
    );
    let scene = engine
        .compile(
            "p=canvas(size=(30mm,20mm))\np.add(text(\"Hello\",font_family=\"/new.ttf\"))",
            "/main.lay",
        )
        .unwrap();
    assert_eq!(scene.fonts.len(), 1);
    assert!(!scene.warnings.iter().any(|w| w.code == "W_FONT"));
}

#[test]
fn repeated_edits_and_changed_geometry_match_uncached_compilation() {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            asset_cache::preview_mode(false);
        }
    }
    let _reset = Reset;
    for round in 0..2 {
        for (radius, width, rotation, x) in
            [(1., 2., 0., 25.), (0.5, 3., 31., 28.), (1., 2., 0., 25.)]
        {
            let source = format!(
                r##"page=canvas(size=(80,40))
a=page.add(rect(size=(10,10),fill="#09a"),rotation={rotation},offset=(5,10))
b=page.add(rect(size=(10,10),fill="#09a"),offset=({x},10))
page.fuse(a,b,bridge_width={width},radius={radius},junction="round",fill="#09a")
# edit {round}"##
            );
            asset_cache::preview_mode(false);
            let reference = compile_source(&source, "/preview.lay", Host::default()).unwrap();
            asset_cache::preview_mode(true);
            let actual = compile_source(&source, "/preview.lay", Host::default()).unwrap();
            assert_eq!(
                serde_json::to_value(reference).unwrap(),
                serde_json::to_value(actual).unwrap()
            );
        }
    }
}
