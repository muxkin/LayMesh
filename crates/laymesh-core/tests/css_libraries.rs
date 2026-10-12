//! Raw vendor CSS, project physical units, CSS recovery and coordinated shadows.
use laymesh_core::{
    Loc,
    engine::{Engine, compile_source},
    model::{Args, Host, V},
    style,
};
fn loc() -> Loc {
    Loc {
        line: 1,
        column: 1,
        offset: 0,
    }
}
fn classes(s: &str) -> Args {
    [("class".into(), V::text(s))].into_iter().collect()
}
fn library(name: &str, extra: &str) -> Engine {
    let mut e = Engine::new(Host::default());
    e.file = "/main.lay".into();
    let path = format!("@laymesh/{name}.css");
    e.stylesheet(style::bundled_stylesheet(&path).unwrap(), &path, loc())
        .unwrap();
    e.stylesheet_scoped(extra, "/main.lay", loc(), Some("/main.lay"))
        .unwrap();
    e
}
#[test]
fn original_bootstrap_appearance_and_partial_important_override() {
    let e = library("bootstrap", ".shadow.custom{box-shadow-blur:2mm!important}");
    let root = e.styled("canvas", &Args::new(), &Args::new()).unwrap();
    let normal = e
        .styled(
            "rect",
            &classes("bg-primary rounded border shadow fw-bold fst-italic"),
            &root,
        )
        .unwrap();
    let custom = e
        .styled(
            "rect",
            &classes("bg-primary rounded border shadow custom"),
            &root,
        )
        .unwrap();
    assert_eq!(normal["fill"].as_str(), "#0d6efd");
    assert_eq!(normal["font_weight"].json().as_f64(), Some(700.));
    assert_eq!(normal["font_style"].as_str(), "italic");
    assert_eq!(normal["border_style"].as_str(), "solid");
    let old = normal["__css_box_shadows"].json();
    let new = custom["__css_box_shadows"].json();
    assert_eq!(new.as_array().unwrap().len(), 1);
    for key in ["color", "offset", "spread", "mode"] {
        assert_eq!(old[0][key], new[0][key], "{key}");
    }
    assert_eq!(new[0]["blur"], 1.);
    assert_eq!(e.css_diagnostics().len(), 1); // full Bootstrap's unsupported layout is summarized once.
}

#[test]
fn bootstrap_font_stack_uses_registered_fonts_without_system_fonts() {
    let mut host = Host::default();
    host.files.insert(
        "/preview.ttf".into(),
        include_bytes!("../../../tests/fonts/DejaVuSans.ttf").to_vec(),
    );
    let scene = compile_source(
        r#"p=canvas(size=(80mm,50mm),stylesheet="@laymesh/bootstrap.css")
p.add(text("Offline CSS libraries",class="text-primary"))"#,
        "/font-stack.lay",
        host,
    )
    .unwrap();
    assert!(
        scene.nodes[0]["runs"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["kind"] == "glyph")
    );
    assert!(!scene.warnings.iter().any(|w| w.code == "W_FONT"));
}
#[test]
fn original_open_props_empty_variable_hsl_calc_gradient_and_multilayer_shadows() {
    let e = library(
        "open-props",
        ".card{color:var(--blue-7);border-radius:var(--radius-2);background:var(--gradient-1);box-shadow:var(--shadow-2);font-size:var(--font-size-2)}",
    );
    let root = e.styled("canvas", &Args::new(), &Args::new()).unwrap();
    let a = e.styled("rect", &classes("card"), &root).unwrap();
    assert!(a["color"].as_str().starts_with('#'));
    assert_eq!(a["fill"].json()["kind"], "linearGradient");
    assert_eq!(a["fill"].json()["stops"].as_array().unwrap().len(), 8);
    assert_eq!(a["__css_box_shadows"].json().as_array().unwrap().len(), 2);
    assert!(
        !root["__style_variables"].json()["--gradient-space"]
            .as_str()
            .unwrap()
            .contains("oklab")
    );
}
#[test]
fn coordinated_lists_shorthand_reset_and_none() {
    let e = library(
        "bootstrap",
        ".layers{box-shadow:1mm 2mm 3mm -1mm red,inset 4mm 5mm 6mm 2mm blue}.layers.custom{box-shadow-blur:2mm;box-shadow-spread:-2mm,3mm;box-shadow-offset:7mm 8mm,9mm;box-shadow-color:green;box-shadow-position:outset,inset}.layers.reset{box-shadow:0 0 4mm purple}.layers.clear{box-shadow:none}.only{box-shadow-blur:2mm}",
    );
    let root = e.styled("canvas", &Args::new(), &Args::new()).unwrap();
    let a = e.styled("rect", &classes("layers custom"), &root).unwrap()["__css_box_shadows"].json();
    assert_eq!(a.as_array().unwrap().len(), 2);
    assert_eq!(a[0]["blur"], 1.);
    assert_eq!(a[1]["blur"], 1.);
    assert_eq!(a[0]["spread"], -2.);
    assert_eq!(a[1]["spread"], 3.);
    assert_eq!(a[1]["offset"], serde_json::json!([9, 9]));
    assert_eq!(a[1]["color"], "#008000");
    assert_eq!(a[1]["mode"], "inner");
    let reset =
        e.styled("rect", &classes("layers reset"), &root).unwrap()["__css_box_shadows"].json();
    assert_eq!(reset.as_array().unwrap().len(), 1);
    assert_eq!(reset[0]["spread"], 0.);
    for class in ["layers clear", "only"] {
        assert!(
            !e.styled("rect", &classes(class), &root)
                .unwrap()
                .contains_key("__css_box_shadows")
        );
    }
}
#[test]
fn physical_units_layout_dpi_relative_math_media_supports_and_recovery() {
    let source = r#"p=canvas(size=(80mm,50mm),layout_dpi=192)
style {
 :root{font-size:20px;--empty: ;--ink:rebeccapurple}
 .a{width:80mm;bad declaration;opacity:bad;color:var(--missing,var(--ink));box-shadow:1inch 1in 1cm 1pt currentColor;box-shadow-blur:calc(1mm + 1mm);border-radius:24px;font-size:clamp(1mm,2rem,40mm);padding:1em 2vw}
 @media screen and (min-width:70mm){.a{background-color:hsl(120deg 100% 25% / 50%)}}
 @supports (background:linear-gradient(to right in oklab,red,blue)){.a{color:red}}
 @supports (box-shadow:0 0 2mm blue){.a{border-color:rgb(100% 0% 0%)}}
}
p.add(rect(size=(24px,24px),class="a"))"#;
    let s = compile_source(source, "/units.lay", Host::default()).unwrap();
    assert!((s.nodes[0]["width"].as_f64().unwrap() - 3.175).abs() < 1e-6);
    assert_eq!(s.layout_dpi, 192.);
    let warnings: Vec<_> = s.warnings.iter().filter(|w| w.code == "W_CSS").collect();
    assert_eq!(warnings.len(), 1);
    let e = library(
        "bootstrap",
        ".shadow.custom{box-shadow-blur:2mm}.a{fill:red !important}",
    );
    let root = e.styled("canvas", &Args::new(), &Args::new()).unwrap();
    let original = e.styled("rect", &classes("shadow"), &root).unwrap()["__css_box_shadows"].json();
    let custom =
        e.styled("rect", &classes("shadow custom"), &root).unwrap()["__css_box_shadows"].json();
    assert_eq!(original, custom);
    let mut explicit = classes("a");
    explicit.insert("fill".into(), V::text("#abcdef"));
    assert_eq!(
        e.styled("rect", &explicit, &root).unwrap()["fill"].as_str(),
        "#abcdef"
    );
}
#[test]
fn all_project_units_math_context_and_animation_pixels_are_shared() {
    use laymesh_core::animation::values::{self, Context};
    let ctx = Context {
        dpi: 192.,
        font: 5.,
        root_font: 4.,
        viewport: [80., 50.],
        reference: 10.,
    };
    for (css, expected) in [
        ("2mm", 2.),
        ("2cm", 20.),
        ("2in", 50.8),
        ("2inch", 50.8),
        ("72pt", 25.4),
        ("192px", 25.4),
        ("2em", 10.),
        ("2rem", 8.),
        ("2vw", 1.6),
        ("2vh", 1.),
        ("calc(2mm + 1cm)", 12.),
        ("min(3mm,2mm)", 2.),
        ("max(3mm,2mm)", 3.),
        ("clamp(1mm,2mm,3mm)", 2.),
        ("clamp(10%,40%,80%)", 4.),
    ] {
        assert!(
            (values::length_context(css, ctx).unwrap() - expected).abs() < 1e-5,
            "{css}"
        );
    }
    assert!((values::angle("180deg").unwrap() - std::f64::consts::PI).abs() < 1e-8);
    assert!((values::angle("3.14159265rad").unwrap() - std::f64::consts::PI).abs() < 1e-6);
    assert!((values::time("1500ms").unwrap() - 1.5).abs() < 1e-8);
    assert_eq!(values::time("1.5s").unwrap(), 1.5);
    let scene = compile_source(
        r#"p=canvas(size=(80mm,50mm),layout_dpi=192)
style{@keyframes move {from{transform:translateX(0px)}to{transform:translateX(192px)}}}
a=p.add(rect(size=(192px,10mm)))
s=p.step()
s.animate(a,name="move",duration=1000ms,fill="both",easing="linear")"#,
        "/units.lay",
        Host::default(),
    )
    .unwrap();
    assert_eq!(scene.nodes[0]["width"], 25.4);
    let state = scene.animation.unwrap().normal.sample(1.);
    let translated = state.values().next().unwrap().transform.unwrap();
    assert!((translated[3] - 25.4).abs() < 1e-8);
}
#[test]
fn single_offset_preserves_layers_and_em_shorthand_uses_computed_font() {
    let e = library(
        "open-props",
        ".a{font-size:10mm;box-shadow:1em 2em 3em red,2em 3em 4em blue}.a.tuned{box-shadow-offset:2mm}.a.bad{box-shadow-blur:-2mm;box-shadow-spread:bogus}",
    );
    let root = e.styled("canvas", &Args::new(), &Args::new()).unwrap();
    let a = e.styled("rect", &classes("a"), &root).unwrap()["__css_box_shadows"].json();
    assert_eq!(a[0]["offset"][0].as_f64(), Some(10.));
    assert_eq!(a[0]["blur"].as_f64(), Some(15.));
    let b = e.styled("rect", &classes("a tuned"), &root).unwrap()["__css_box_shadows"].json();
    assert_eq!(b.as_array().unwrap().len(), 2);
    for layer in b.as_array().unwrap() {
        assert_eq!(layer["offset"], serde_json::json!([2, 2]));
    }
    let c = e.styled("rect", &classes("a bad"), &root).unwrap()["__css_box_shadows"].json();
    assert_eq!(a, c);
}
#[test]
fn pinned_libraries_are_unmodified_and_imports_work_without_a_native_host() {
    use sha2::{Digest, Sha256};
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../assets/styles/manifest.json")).unwrap();
    for item in manifest.as_array().unwrap() {
        let path = format!("@laymesh/{}.css", item["name"].as_str().unwrap());
        let data = Host::default().read(&path, "/main.lay", loc()).unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(data)),
            item["sha256"].as_str().unwrap()
        );
    }
    let source = r#"p=canvas(size=(80mm,50mm))
style{@import "@laymesh/bootstrap.css";@import "@laymesh/open-props.css"; .a{background-color:var(--blue-7)}}
p.add(rect(size=(30mm,20mm),class="a rounded shadow"),offset=(20mm,15mm))"#;
    assert!(compile_source(source, "/main.lay", Host::default()).is_ok());
    assert_eq!(
        compile_source(
            "p=canvas(size=(20,20),stylesheet=\"missing.css\")",
            "/main.lay",
            Host::default()
        )
        .unwrap_err()
        .code,
        "E_ASSET"
    );
    assert!(
        compile_source(
            "p=canvas(size=(20,20))\np.add(rect(size=(-1mm,2mm)))",
            "/main.lay",
            Host::default()
        )
        .is_err()
    );
}

#[test]
fn animation_uses_computed_element_and_root_fonts() {
    let source = r#"p=canvas(size=(80mm,50mm),layout_dpi=192)
style {:root{font-size:4mm}.a{font-size:6mm}@keyframes move{from{transform:translate(0,0)}to{transform:translate(1em,1rem)}}}
a=p.add(rect(size=(10mm,10mm),class="a"))
s=p.step()
s.animate(a,name="move",duration=1s,easing="linear",fill="both")"#;
    let scene = compile_source(source, "/font.lay", Host::default()).unwrap();
    let anim = scene.animation.unwrap();
    let value = anim.normal.sample(1.);
    let transform = value["a"].transform.unwrap();
    assert!((transform[3] - 6.).abs() < 1e-8);
    assert!((transform[7] - 4.).abs() < 1e-8);
}
#[test]
fn single_radial_open_props_gradients_and_physical_linear_angles() {
    let e = library(
        "open-props",
        ".a{background:var(--gradient-13)}.b{background:var(--gradient-16)}",
    );
    let root = e.styled("canvas", &Args::new(), &Args::new()).unwrap();
    for class in ["a", "b"] {
        let paint = e.styled("rect", &classes(class), &root).unwrap()["fill"].json();
        assert_eq!(paint["kind"], "radialGradient");
        let computed = laymesh_core::css_values::gradient_box(&paint, [0., 0., 30., 20.]);
        assert!(computed["radius"].as_f64().unwrap() > 1.);
    }
    let paint = laymesh_core::css_values::gradient(
        "linear-gradient(45deg,red,blue)",
        Default::default(),
        "black",
    )
    .unwrap();
    let p = laymesh_core::css_values::gradient_box(&paint, [0., 0., 30., 10.]);
    let dx = (p["end"][0].as_f64().unwrap() - p["start"][0].as_f64().unwrap()) * 30.;
    let dy = (p["end"][1].as_f64().unwrap() - p["start"][1].as_f64().unwrap()) * 10.;
    assert!((dx + dy).abs() < 1e-8);
}

#[test]
fn invalid_animation_css_recovers_while_lay_overrides_remain_errors() {
    let source = r#"p=canvas(size=(30mm,20mm))
style{.a{animation-duration:bad;animation-direction:sideways}@keyframes move{from{opacity:broken;left:0px}to{opacity:50%;transform:translateX(2mm)}}}
a=p.add(rect(size=(10mm,10mm),class="a"))
s=p.step()
s.animate(a,name="move",duration=1s,fill="both",easing="linear")"#;
    let scene = compile_source(source, "/recover.lay", Host::default()).unwrap();
    assert_eq!(
        scene.warnings.iter().filter(|w| w.code == "W_CSS").count(),
        1
    );
    assert!(
        (scene.animation.unwrap().normal.sample(1.)["a"]
            .opacity
            .unwrap()
            - 0.5)
            .abs()
            < 1e-8
    );
    assert!(
        compile_source(
            &source.replace("duration=1s", "duration=-1s"),
            "/recover.lay",
            Host::default()
        )
        .is_err()
    );
}

#[test]
fn css_shadows_preserve_plot_anchors_line_endpoints_and_source_paths() {
    let source = r##"page=canvas(size=(100mm,75mm))
style{.a{box-shadow:1mm 2mm 3mm blue}.b{box-shadow:inset 1mm 1mm 2mm blue}}
p=plot(size=(40mm,30mm),x=axis(range=(0,2)),y=axis(range=(0,3)),class="a")
p.line(x=[0,1,2],y=[1,2,1])
chart=page.add(p)
page.add(rect(size=(1mm,1mm)),target=chart.plot_top_left)
l=page.add(line(dx=10mm,dy=5mm,class="a"),offset=(50mm,0mm))
page.add(rect(size=(1mm,1mm)),target=l.end)
r=page.add(rect(size=(12mm,8mm),fill="#ff0000",class="b"),offset=(50mm,20mm))
page.add(rect(size=(1mm,1mm)),target=r.path.subpaths[0].start)
"##;
    let scene = compile_source(source, "/geometry.lay", Host::default()).unwrap();
    assert!(scene.nodes[0]["plotBounds"].is_object());
    assert!(scene.nodes[2]["endpoints"].is_array());
    assert_eq!(
        compile_source(
            &source.replace("r.path.subpaths[0]", "r.path.subpaths[1]"),
            "/geometry.lay",
            Host::default()
        )
        .unwrap_err()
        .code,
        "E_INDEX"
    );
}
