use laymesh_core::{engine::compile_source, model::Host};
fn compile(css: &str) -> laymesh_core::Result<laymesh_core::model::Scene> {
    compile_source(
        &format!("p=canvas(size=(80mm,45mm))\nstyle {{ canvas {{ {css} }} }}"),
        "test.lay",
        Host::default(),
    )
}
#[test]
fn defaults_parameters_and_presets() {
    assert!(compile("").unwrap().page_transition.is_none());
    assert!(
        compile("-laymesh-transition:none")
            .unwrap()
            .page_transition
            .is_none()
    );
    let t = compile("-laymesh-transition:InvertedPageCurl")
        .unwrap()
        .page_transition
        .unwrap();
    assert!((t.duration_ms - 700.).abs() < 1e-9);
    let t=compile("-laymesh-transition:cube;-laymesh-transition-duration:0.25s;-laymesh-transition-param-persp:0.6;-laymesh-transition-timing-function:linear").unwrap().page_transition.unwrap();
    assert_eq!(t.duration_ms, 250.);
    assert_eq!(t.params["persp"], serde_json::json!(0.6));
    let s=compile_source("p=canvas(size=(80mm,45mm),stylesheet=\"@laymesh/gl-transitions.css\",class=\"gl-transition-InvertedPageCurl\")", "test.lay",Host::default()).unwrap();
    assert!(s.warnings.is_empty(), "{:?}", s.warnings);
    assert_eq!(s.page_transition.unwrap().name, "InvertedPageCurl");
    let t=compile("-laymesh-transition:GridFlip;-laymesh-transition-param-size:6 4;-laymesh-transition-param-bgcolor:0,0,0,1").unwrap().page_transition.unwrap();
    assert_eq!(t.params["size"], serde_json::json!([6., 4.]));
    let t = compile("-laymesh-transition:circleopen;-laymesh-transition-param-opening:false")
        .unwrap()
        .page_transition
        .unwrap();
    assert_eq!(t.params["opening"], serde_json::json!(false));
}
#[test]
fn invalid_and_missing_inputs_are_diagnosed() {
    for css in [
        "-laymesh-transition:typo",
        "-laymesh-transition:fade;-laymesh-transition-duration:-1s",
        "-laymesh-transition:fade;-laymesh-transition-param-nope:1",
        "-laymesh-transition:cube;-laymesh-transition-param-persp:NaN",
        "-laymesh-transition:luma",
        "-laymesh-transition:fade;-laymesh-transition-timing-function:invalid",
    ] {
        assert_eq!(compile(css).unwrap_err().code, "E_TRANSITION", "{css}");
    }
}
#[test]
fn catalog_all_presets_compile_without_missing_texture_effects() {
    for t in laymesh_core::page_transition::CATALOG.iter() {
        if t["textures"].as_array().unwrap().is_empty() {
            let css = format!("-laymesh-transition:{}", t["name"].as_str().unwrap());
            assert!(
                compile(&css)
                    .unwrap_or_else(|e| panic!("{css}: {e:?}"))
                    .page_transition
                    .is_some()
            );
        }
    }
}
#[test]
fn texture_urls_are_relative_to_stylesheet() {
    let mut host = Host::default();
    host.files.insert("/styles/theme.lcss".into(),b"canvas {-laymesh-transition:displacement; -laymesh-transition-texture-displacement-map:url('../map.png')}".to_vec());
    host.files.insert(
        "/map.png".into(),
        include_bytes!("../../../examples/gallery/typography/font-preview.png").to_vec(),
    );
    let s = compile_source(
        "p=canvas(size=(80mm,45mm),stylesheet=\"styles/theme.lcss\")",
        "/test.lay",
        host,
    )
    .unwrap();
    assert!(
        s.page_transition
            .unwrap()
            .assets
            .contains_key("displacementMap")
    );
}

#[test]
fn unavailable_texture_preserves_lazy_resource_request() {
    let error = compile("-laymesh-transition:luma; -laymesh-transition-texture-luma:url('missing.png')").unwrap_err();
    assert_eq!(error.code, "E_ASSET");
    assert!(error.message.contains("missing.png"));
}
