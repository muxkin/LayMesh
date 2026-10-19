use laymesh_core::{
    animation::values::Context,
    backgrounds,
    engine::compile_source,
    gradients::{self, Geometry, Resolved},
    model::Host,
};
fn resolve(s: &str, w: f64, h: f64) -> Resolved {
    let p = gradients::parse(s, Context::default(), "#000").unwrap();
    gradients::from_json(&p).unwrap().resolve([0., 0., w, h])
}
fn sample(r: &Resolved, t: f64) -> [f64; 4] {
    let i = r.stops.partition_point(|s| s.0 <= t);
    if i == 0 {
        return r.stops[0].1;
    }
    if i == r.stops.len() {
        return r.stops[i - 1].1;
    }
    let (a, ca) = r.stops[i - 1];
    let (b, cb) = r.stops[i];
    let u = (t - a) / (b - a);
    std::array::from_fn(|i| ca[i] + (cb[i] - ca[i]) * u)
}
fn close(a: [f64; 4], b: [f64; 4]) {
    for i in 0..4 {
        assert!((a[i] - b[i]).abs() < 0.004, "{a:?} != {b:?}");
    }
}
#[test]
fn independent_standard_color_interpolation_values() {
    close(
        sample(&resolve("linear-gradient(red, blue)", 100., 50.), 0.5),
        [0.5, 0., 0.5, 1.],
    );
    close(
        sample(
            &resolve("linear-gradient(in srgb-linear, black, white)", 100., 50.),
            0.5,
        ),
        [0.735357, 0.735357, 0.735357, 1.],
    );
    close(
        sample(
            &resolve("linear-gradient(in oklab, red, blue)", 100., 50.),
            0.5,
        ),
        [0.550441, 0.32562, 0.636501, 1.],
    );
    close(
        sample(
            &resolve("linear-gradient(red, transparent)", 100., 50.),
            0.5,
        ),
        [1., 0., 0., 0.5],
    );
    close(
        sample(
            &resolve("linear-gradient(in hsl longer hue, red, blue)", 100., 50.),
            0.5,
        ),
        [0., 1., 0., 1.],
    );
    close(
        sample(
            &resolve("linear-gradient(in hsl shorter hue, red, blue)", 100., 50.),
            0.5,
        ),
        [1., 0., 1., 1.],
    );
    close(
        sample(
            &resolve(
                "linear-gradient(in srgb, rgb(none 0 0), rgb(255 0 0))",
                100.,
                50.,
            ),
            0.5,
        ),
        [1., 0., 0., 1.],
    );
}
#[test]
fn all_color4_spaces_parse_without_clipping_source_colors() {
    for space in [
        "srgb",
        "srgb-linear",
        "display-p3",
        "display-p3-linear",
        "a98-rgb",
        "prophoto-rgb",
        "rec2020",
        "xyz",
        "xyz-d50",
        "xyz-d65",
        "lab",
        "lch",
        "oklab",
        "oklch",
        "hsl",
        "hwb",
    ] {
        let r = resolve(
            &format!("linear-gradient(in {space},color(display-p3 1 0 0),oklch(50% .2 240))"),
            100.,
            50.,
        );
        assert!(
            r.stops
                .iter()
                .all(|(_, c)| c.iter().all(|v| v.is_finite() && (0. ..=1.).contains(v))),
            "{space}"
        );
    }
}
#[test]
fn hint_is_halfway_color_not_an_extra_color_stop() {
    let r = resolve("linear-gradient(to right, red, 25%, blue)", 100., 10.);
    close(sample(&r, 0.25), [0.5, 0., 0.5, 1.]);
    let r = resolve("linear-gradient(to right, red, 10%, blue, lime)", 100., 10.);
    close(sample(&r, 0.55), [0., 0., 1., 1.]);
}
#[test]
fn hard_stops_negative_positions_and_lengths_survive_resolution() {
    let r = resolve(
        "linear-gradient(to right, red -10mm 20mm, blue 20mm 120mm)",
        100.,
        40.,
    );
    assert_eq!(r.range, [-0.1, 1.2]);
    assert!(
        r.stops
            .windows(2)
            .any(|p| p[0].0 == 0.2 && p[1].0 == 0.2 && p[0].1 != p[1].1)
    );
    close(sample(&r, 0.19999), [1., 0., 0., 1.]);
    close(sample(&r, 0.20001), [0., 0., 1., 1.]);
    let r = resolve(
        "linear-gradient(to right, red calc(10% + 10mm), blue 100%)",
        100.,
        50.,
    );
    assert!((r.range[0] - 0.2).abs() < 1e-7);
}
#[test]
fn radial_shapes_and_outside_centers_use_the_final_box() {
    for (css, radii) in [
        (
            "radial-gradient(circle closest-side at 25% 50%,red,blue)",
            [25., 25.],
        ),
        (
            "radial-gradient(ellipse farthest-side at 25% 50%,red,blue)",
            [75., 30.],
        ),
        (
            "radial-gradient(20mm 50% at right -10mm bottom 5mm,red,blue)",
            [20., 30.],
        ),
    ] {
        let r = resolve(css, 100., 60.);
        let Geometry::Radial { radii: actual, .. } = r.geometry else {
            panic!()
        };
        assert_eq!(actual, radii);
    }
    let r = resolve(
        "conic-gradient(from .25turn at -10% 120%,red,blue)",
        100.,
        50.,
    );
    let Geometry::Conic { center, angle } = r.geometry else {
        panic!()
    };
    assert!((center[0] + 10.).abs() < 1e-5 && (center[1] - 60.).abs() < 1e-5);
    assert!((angle - std::f64::consts::FRAC_PI_2).abs() < 1e-6);
}
#[test]
fn repetitions_keep_phase_and_hard_seams() {
    let r = resolve(
        "repeating-conic-gradient(red 0deg 90deg, blue 90deg 180deg)",
        100.,
        50.,
    );
    close(sample(&r, 0.1), [1., 0., 0., 1.]);
    close(sample(&r, 0.4), [0., 0., 1., 1.]);
    close(sample(&r, 0.6), [1., 0., 0., 1.]);
    close(sample(&r, 0.9), [0., 0., 1., 1.]);
    let r = resolve(
        "repeating-linear-gradient(to right,red 10px,blue 20px)",
        100.,
        50.,
    );
    assert!(r.repeating);
    assert!((r.range[0] - 25.4 / 960.).abs() < 1e-6);
    let r = resolve("repeating-linear-gradient(red 20%,blue 20%)", 100., 50.);
    assert!(!r.repeating);
    assert!(r.stops.iter().all(|s| s.1 == [0.5, 0., 0.5, 1.]));
}
#[test]
fn invalid_grammar_does_not_advertise_support() {
    for s in [
        "linear-gradient(red,20%,30%,blue)",
        "linear-gradient(red,blue,30%)",
        "linear-gradient(to left right,red,blue)",
        "linear-gradient(in srgb longer hue,red,blue)",
        "conic-gradient(red 10px,blue)",
        "radial-gradient(circle 50%,red,blue)",
        "radial-gradient(ellipse 10px,red,blue)",
        "linear-gradient(red 10%,blue),linear-gradient(red,blue)",
        "linear-gradient(rgb(10 20 30 .5),blue)",
        "linear-gradient(lab(50%,20,30),blue)",
        "linear-gradient(color(oklab .5 .1 .2),blue)",
    ] {
        assert!(
            gradients::parse(s, Context::default(), "black").is_err(),
            "accepted {s}"
        );
    }
}
#[test]
fn linear_p3_missing_components_are_resolved_in_the_interpolation_space() {
    let expected = resolve(
        "linear-gradient(in display-p3-linear,color(display-p3-linear .5 .1 .1),color(display-p3-linear .5 .1 .1))",
        100.,
        50.,
    );
    let missing = resolve(
        "linear-gradient(in display-p3-linear,color(display-p3-linear none .1 .1),color(display-p3-linear .5 .1 .1))",
        100.,
        50.,
    );
    for t in [0., 0.1, 0.5, 1.] {
        close(sample(&missing, t), sample(&expected, t));
    }
    let a = resolve(
        "linear-gradient(in display-p3-linear,color(display-p3-linear .1 .2 .3),color(display-p3-linear .5 .4 .3))",
        100.,
        50.,
    );
    let mid = resolve(
        "linear-gradient(color(display-p3-linear .3 .3 .3))",
        100.,
        50.,
    );
    close(sample(&a, 0.5), sample(&mid, 0.5));
}
#[test]
fn background_lists_keep_color_and_repeat_shorter_longhands() {
    let ctx = Context::default();
    let parts=backgrounds::shorthand("linear-gradient(red,transparent) right bottom / 20px 30px no-repeat, linear-gradient(blue,transparent) white",ctx,"black").unwrap();
    assert_eq!(
        parts.iter().find(|p| p.0 == "background_color").unwrap().1,
        "white"
    );
    assert!(backgrounds::shorthand("red,linear-gradient(red,blue)", ctx, "black").is_err());
    let src = "p=canvas(size=(100mm,60mm))\nstyle{.a{background-color:white;background-image:linear-gradient(red,transparent),linear-gradient(blue,transparent);background-size:20mm 10mm;background-position:right bottom;background-repeat:no-repeat}.a{background-size:-1mm}}\np.add(rect(size=(100mm,60mm),class=\"a\"))";
    let scene = compile_source(src, "/gradient.lay", Host::default()).unwrap();
    let fill = &scene.nodes[0]["fill"];
    let bg: backgrounds::Background = serde_json::from_value(fill["background"].clone()).unwrap();
    assert_eq!(bg.color, "#ffffff");
    assert_eq!(bg.layers.len(), 2);
    for layer in &bg.layers {
        let t = layer.layout(&bg.units, [0., 0., 100., 60.]).unwrap();
        assert_eq!(t.origin, [80., 50.]);
        assert_eq!(t.size, [20., 10.]);
    }
    assert_eq!(
        scene.warnings.iter().filter(|w| w.code == "W_CSS").count(),
        1
    );
}
#[test]
fn constructors_return_reusable_paints() {
    let src = "p=canvas(size=(40mm,20mm))\ng=gradient(css=\"repeating-conic-gradient(red 0deg 90deg,blue 90deg 180deg)\")\np.add(rect(size=(20mm,20mm),fill=g))\np.add(ellipse(size=(20mm,20mm),fill=conic_gradient([(0,\"#f00\"),(1,\"#00f\")],angle=.25turn)),offset=(20mm,0mm))";
    let scene = compile_source(src, "/dsl.lay", Host::default()).unwrap();
    assert!(scene.nodes[0]["fill"]["gradient"].is_object());
    assert!((scene.nodes[1]["fill"]["angle"].as_f64().unwrap() - 90.).abs() < 1e-6);
}

#[test]
fn degenerate_geometry_and_calc_angles_follow_images_standard() {
    let r = resolve(
        "repeating-linear-gradient(red 0px,white 0px,blue 0px)",
        100.,
        50.,
    );
    close(r.stops[0].1, [0.75, 0.5, 0.75, 1.]);
    let r = resolve("radial-gradient(0px 20px,red 0px,blue 80px)", 100., 50.);
    let Geometry::Radial { radii, .. } = r.geometry else {
        panic!()
    };
    assert!(radii[1] > 1e9);
    let r = resolve("repeating-radial-gradient(20px 0px,red,blue)", 100., 50.);
    close(r.stops[0].1, [0.5, 0., 0.5, 1.]);
    let r = resolve("conic-gradient(red calc(25% - 45deg),blue 100%)", 100., 50.);
    close(sample(&r, 0.125), [1., 0., 0., 1.]);
    let r = resolve("radial-gradient(calc(50% - 10mm) 20mm,red,blue)", 100., 50.);
    let Geometry::Radial { radii, .. } = r.geometry else {
        panic!()
    };
    assert_eq!(radii, [40., 20.]);
}

#[test]
fn images4_single_color_gradients_are_solid() {
    for s in [
        "linear-gradient(red)",
        "conic-gradient(red 50%)",
        "radial-gradient(in oklab,red)",
    ] {
        let r = resolve(s, 100., 50.);
        assert!(
            r.stops
                .iter()
                .all(|s| s.1[0] > 0.999 && s.1[1] < 0.001 && s.1[2] < 0.001)
        );
    }
}

#[test]
fn numeric_source_colors_retain_sub_byte_precision() {
    let r = resolve(
        "LINEAR-GRADIENT(TO RIGHT, rgb(calc(1.5) 0 0),rgb(calc(2.5) 0 0))",
        100.,
        50.,
    );
    assert!((r.stops[0].1[0] - 1.5 / 255.).abs() < 1e-7);
    assert!((r.stops.last().unwrap().1[0] - 2.5 / 255.).abs() < 1e-7);
}
