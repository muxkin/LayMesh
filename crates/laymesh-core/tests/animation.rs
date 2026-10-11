use laymesh_core::{
    animation::{css, values},
    engine::compile_source,
    model::Host,
};
const LIB: &str = include_str!("../../../tests/animation/vendor/animate.css");
fn host() -> Host {
    Host {
        files: [("/animate.css".into(), LIB.as_bytes().to_vec())]
            .into_iter()
            .collect(),
        ..Host::default()
    }
}
#[test]
fn original_animate_css_all_97_effects_compile_and_seek() {
    let (_, sheet) = css::extract(
        LIB,
        "/animate.css",
        laymesh_core::Loc {
            line: 1,
            column: 1,
            offset: 0,
        },
        None,
    )
    .unwrap();
    let names: Vec<_> = sheet
        .keyframes
        .iter()
        .filter(|k| !k.prefixed)
        .map(|k| k.name.clone())
        .collect();
    assert_eq!(names.len(), 97);
    for name in names {
        let src = format!(
            "page=canvas(size=(100,80),stylesheet=\"animate.css\")\na=page.add(rect(size=(20,15),fill=\"#ff0000\"))\ns=page.step(name=\"{name}\")\ns.animate(a,class=\"animate__animated animate__{name}\")"
        );
        let scene =
            compile_source(&src, "/main.lay", host()).unwrap_or_else(|e| panic!("{name}: {e}"));
        let anim = scene.animation.unwrap();
        assert_eq!(anim.normal.steps.len(), 1);
        assert!(anim.normal.duration > 0.);
        for fraction in [0., 0.1, 0.25, 0.5, 0.75, 0.99, 1.] {
            let result = anim.normal.sample(fraction * anim.normal.steps[0].duration);
            assert!(result.contains_key("a"), "{name}: {result:?}");
            for style in result.values() {
                assert!(
                    style
                        .transform
                        .is_none_or(|m| m.iter().all(|v| v.is_finite())),
                    "{name}"
                );
            }
        }
    }
}
#[test]
fn css_defaults_and_instance_opacity_replace() {
    let src = "style {@keyframes fade {from{opacity:0}to{opacity:1}} .a{animation:fade 2s linear both}}\np=canvas(size=(30,20))\na=p.add(rect(size=(10,10),class=\"a\"),opacity=0.4)\ns=p.step()\ns.animate(a)";
    let scene = compile_source(src, "/main.lay", Host::default()).unwrap();
    let a = scene.animation.unwrap();
    assert!(
        (a.normal.sample(1.)["a"].opacity.unwrap() - 0.5).abs() < 1e-6,
        "{a:?}"
    );
}
#[test]
fn typed_calc_and_css_pixels() {
    assert!((values::time("calc(800ms * 0.75)").unwrap() - 0.6).abs() < 1e-7);
    assert!((values::length("96px", 0.).unwrap() - 25.4).abs() < 1e-8);
    assert!(values::time("calc(1s + 2mm)").is_err());
}
#[test]
fn infinite_needs_step_window() {
    let src = "p=canvas(size=(30,20),stylesheet=\"animate.css\")\na=p.add(rect(size=(10,10)))\ns=p.step()\ns.animate(a,class=\"animate__animated animate__pulse animate__infinite\")";
    let err = compile_source(src, "/main.lay", host()).unwrap_err();
    assert!(err.message.contains("Infinite"), "{err}");
    let scene = compile_source(
        &src.replace("s=p.step()", "s=p.step(duration=3s)"),
        "/main.lay",
        host(),
    )
    .unwrap();
    assert_eq!(scene.animation.unwrap().normal.duration, 4.);
}
fn css_scene(css: &str, action: &str) -> laymesh_core::model::Scene {
    let src = format!(
        "p=canvas(size=(60,40),stylesheet=\"test.css\")\na=p.add(rect(size=(25.4,19.05)),opacity=0.4)\ns=p.step()\ns.animate(a,{action})"
    );
    compile_source(
        &src,
        "/main.lay",
        Host {
            files: [("/test.css".into(), css.as_bytes().to_vec())]
                .into_iter()
                .collect(),
            ..Host::default()
        },
    )
    .unwrap()
}
#[test]
fn css_shorthand_resets_longhands_and_resolves_same_element_variables() {
    let scene = css_scene(
        "@keyframes f{from{opacity:0}to{opacity:1}}.a{animation-duration:9s;animation:f var(--d) linear both;--d:2s}",
        "class=\"a\",vars={\"--d\":4s}",
    );
    let a = scene.animation.unwrap();
    assert_eq!(a.normal.steps[0].duration, 4.);
    assert!((a.normal.sample(2.)["a"].opacity.unwrap() - 0.5).abs() < 1e-6);
    let scene = css_scene(
        "@keyframes f{from{opacity:0}to{opacity:1}}.a{animation-duration:9s;animation:f}",
        "class=\"a\"",
    );
    let c = scene.animation.unwrap().normal.steps[0].clips[0].clone();
    assert_eq!(c.duration, 0.);
    assert_eq!(c.fill, "none");
    assert_eq!(c.progress(0.), None);
}
#[test]
fn directions_negative_delay_fractional_and_zero_iterations() {
    for (direction, expected) in [
        ("normal", 0.25),
        ("reverse", 0.75),
        ("alternate", 0.75),
        ("alternate-reverse", 0.25),
    ] {
        let s = css_scene(
            "@keyframes f{from{opacity:0}to{opacity:1}}.a{animation:f 1s linear -1s 2.5 both}",
            &format!("class=\"a\",direction=\"{direction}\""),
        );
        let a = s.animation.unwrap();
        assert!((a.normal.sample(0.25)["a"].opacity.unwrap() - expected).abs() < 1e-6);
        assert!((a.normal.sample(1.5)["a"].opacity.unwrap() - 0.5).abs() < 1e-6);
    }
    let s = css_scene(
        "@keyframes f{from{opacity:0}to{opacity:1}}.a{animation:f 1s linear 0 both}",
        "class=\"a\"",
    );
    assert_eq!(
        s.animation.unwrap().normal.sample(0.)["a"].opacity,
        Some(0.)
    );
}
#[test]
fn duplicate_frames_missing_endpoints_and_segment_easing() {
    let s = css_scene(
        "@keyframes f{0%{opacity:0;animation-timing-function:steps(2,end)}50%{opacity:0.8}50%{opacity:1}100%{transform:translateX(96px)}}.a{animation:f 2s linear both}",
        "class=\"a\"",
    );
    let a = s.animation.unwrap();
    let at = |t| a.normal.sample(t)["a"].opacity.unwrap();
    assert!((at(0.4) - 0.).abs() < 1e-6);
    assert!((at(0.5) - 0.5).abs() < 1e-6);
    assert!((at(1.5) - 0.7).abs() < 1e-6);
    assert!((a.normal.sample(2.)["a"].transform.unwrap()[3] - 25.4).abs() < 1e-6);
}
#[test]
fn important_media_prefix_alias_and_list_composition() {
    let s = css_scene(
        "@charset \"UTF-8\";:root{--d:2s}@-webkit-keyframes f{from{opacity:0.9}to{opacity:0.9}}@keyframes f{from{opacity:0}to{opacity:1}}@keyframes g{from{opacity:1}to{opacity:0}}.a{-webkit-animation:f var(--d) linear both;animation:f var(--d) linear both,g 1s linear none}@media (prefers-reduced-motion: reduce){.a{animation-duration:0.001s!important;opacity:0.2!important}}@media print{.a{animation-duration:999s!important}}",
        "class=\"a\"",
    );
    let a = s.animation.unwrap();
    assert_eq!(a.normal.steps[0].clips.len(), 2);
    assert!((a.normal.sample(0.25)["a"].opacity.unwrap() - 0.75).abs() < 1e-6);
    assert!((a.normal.sample(1.5)["a"].opacity.unwrap() - 0.75).abs() < 1e-6);
    assert!((a.reduced.sample(0.)["a"].opacity.unwrap() - 0.2).abs() < 1e-6);
    assert!(a.reduced.steps[0].duration < 0.002);
}
#[test]
fn css_steps_before_flag_and_lay_steps_are_separate() {
    let s = css_scene(
        "@keyframes f{from{opacity:0}to{opacity:1}}.a{animation:f 1s steps(4,start) 1s both}",
        "class=\"a\"",
    );
    let a = s.animation.unwrap();
    assert_eq!(a.normal.sample(0.5)["a"].opacity, Some(0.));
    assert_eq!(a.normal.sample(1.)["a"].opacity, Some(0.25));
}
#[test]
fn duplicate_rule_easings_and_implicit_endpoint_match_browser() {
    let s = css_scene(
        "@keyframes f{from,to{animation-timing-function:ease-out}from{opacity:0}to{opacity:1}}.a{animation:f 1s linear both}",
        "class=\"a\"",
    );
    assert!(
        (s.animation.unwrap().normal.sample(0.2)["a"]
            .opacity
            .unwrap()
            - 0.2)
            .abs()
            < 1e-6
    );
    let s = css_scene(
        "@keyframes f{0%{animation-timing-function:steps(2,end)}100%{opacity:1}}.a{animation:f 1s linear both}",
        "class=\"a\"",
    );
    assert!(
        (s.animation.unwrap().normal.sample(0.2)["a"]
            .opacity
            .unwrap()
            - 0.4)
            .abs()
            < 1e-6
    );
}
#[test]
fn media_environment_and_time_arithmetic() {
    let s = css_scene(
        "@keyframes f{from{opacity:0}to{opacity:1}}.a{animation:f 2s both}@media print{.a{animation-duration:1ms!important}}",
        "class=\"a\"",
    );
    let a = s.animation.unwrap();
    assert_eq!(a.normal.steps[0].duration, 2.);
    assert!((a.print.steps[0].duration - 0.001).abs() < 1e-9);
    let s = css_scene(
        "@keyframes f{from{opacity:0}to{opacity:1}}",
        "name=\"f\",duration=1s + 2 * 200ms",
    );
    assert!((s.animation.unwrap().normal.steps[0].duration - 1.4).abs() < 1e-9);
}

#[test]
fn linked_actions_follow_computed_css_end_and_delay() {
    let src = r#"style {
      @keyframes f { from { opacity: 0; } to { opacity: 1; } }
      .multi { animation: f 2s .5s 1.5 both, f 1s .2s both; }
      @media (prefers-reduced-motion: reduce) { .multi { animation-duration: .1s; } }
    }
    p=canvas(size=(40,30))
    a=p.add(rect(size=(5,5)))
    b=p.add(rect(size=(5,5)))
    c=p.add(rect(size=(5,5)))
    s=p.step(start="after-previous",hold=0s)
    s.animate(a,class="multi")
    s.animate(b,name="f",duration=1s,start="after-previous",delay=.2s,fill="both")
    s.animate(c,name="f",duration=1s,start="with-previous",fill="both")
    "#;
    let a = compile_source(src, "/main.lay", Host::default())
        .unwrap()
        .animation
        .unwrap();
    let normal = &a.normal.steps[0];
    assert_eq!(normal.trigger, "after-previous");
    assert!((normal.clips[2].delay - 3.7).abs() < 1e-8);
    assert!((normal.clips[3].delay - 3.7).abs() < 1e-8);
    assert!((normal.duration - 4.7).abs() < 1e-8);
    assert!((a.reduced.steps[0].clips[2].delay - 0.85).abs() < 1e-8);
}

#[test]
fn linked_actions_reject_unbounded_predecessor_and_invalid_trigger() {
    let src = "p=canvas(size=(30,20),stylesheet=\"animate.css\")\na=p.add(rect(size=(10,10)))\ns=p.step(duration=3s)\ns.animate(a,name=\"pulse\",duration=1s,iterations=\"infinite\")\ns.animate(a,name=\"fadeOut\",duration=1s,start=\"after-previous\")";
    assert!(
        compile_source(src, "/main.lay", host())
            .unwrap_err()
            .message
            .contains("after-previous")
    );
    assert!(
        compile_source(
            &src.replace("after-previous", "invalid"),
            "/main.lay",
            host()
        )
        .unwrap_err()
        .message
        .contains("animate start")
    );
    assert!(
        compile_source(
            "p=canvas(size=(30,20))\ns=p.step(start=\"invalid\")",
            "/main.lay",
            host()
        )
        .unwrap_err()
        .message
        .contains("step start")
    );
}

#[test]
fn linked_negative_delay_and_empty_action_have_finite_boundaries() {
    let src = "p=canvas(size=(30,20),stylesheet=\"animate.css\")\na=p.add(rect(size=(10,10)))\ns=p.step()\ns.animate(a,name=\"pulse\",duration=2s,delay=-.5s,iterations=.5)\ns.animate(a,name=\"none\",start=\"after-previous\")\ns.animate(a,name=\"fadeOut\",duration=1s,start=\"after-previous\",fill=\"both\")";
    let a = compile_source(src, "/main.lay", host())
        .unwrap()
        .animation
        .unwrap();
    assert!((a.normal.steps[0].clips[1].delay - 0.5).abs() < 1e-8);
    assert!((a.normal.steps[0].duration - 1.5).abs() < 1e-8);
}
