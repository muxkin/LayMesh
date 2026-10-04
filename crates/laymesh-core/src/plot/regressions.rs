//! Behavioral regressions found by auditing the original plot implementation.
use super::*;
fn args(v: Json) -> Args {
    if let V::Map(a) = V::from_json(&v) {
        a
    } else {
        panic!("args")
    }
}
fn engine() -> Engine {
    Engine::new(Host::default())
}
fn layer(kind: &str, a: Json) -> Layer {
    build(
        &mut engine(),
        kind,
        &args(a),
        0,
        &Args::new(),
        "cartesian",
        &Args::new(),
        Loc::default(),
    )
    .unwrap()
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}
#[test]
fn step_defaults_to_post_and_preserves_explicit_modes() {
    for (where_, expected) in [
        (None, vec![[0., 0.], [2., 0.], [2., 1.]]),
        (Some("pre"), vec![[0., 0.], [0., 1.], [2., 1.]]),
        (Some("mid"), vec![[0., 0.], [1., 0.], [1., 1.], [2., 1.]]),
        (Some("post"), vec![[0., 0.], [2., 0.], [2., 1.]]),
    ] {
        let mut a = json!({"x":[0,2],"y":[0,1]});
        if let Some(v) = where_ {
            a["where"] = json!(v);
        }
        assert_eq!(layer("step", a).primitives[0].points, expected);
    }
}
#[test]
fn area_uses_every_baseline_and_breaks_on_missing_rows() {
    let l = layer("area", json!({"x":[0,1,2],"y":[3,1,4],"baseline":[1,2,3]}));
    assert_eq!(
        l.primitives[0].points,
        vec![[0., 1.], [1., 1.], [2., 3.], [2., 4.], [1., 2.], [0., 3.]]
    );
    let l = layer(
        "area",
        json!({"x":[0,1,2],"y":[3,3,3],"baseline":[1,null,2]}),
    );
    assert_eq!(l.primitives.len(), 2);
    assert_eq!(l.primitives[1].points, vec![[2., 2.], [2., 3.]]);
    let err = build(
        &mut engine(),
        "area",
        &args(json!({"x":[0,1],"y":[3,3],"baseline":[1]})),
        0,
        &Args::new(),
        "cartesian",
        &Args::new(),
        Loc::default(),
    )
    .err()
    .unwrap();
    assert!(err.message.contains("baseline"));
}
#[test]
fn errors_and_missing_color_invalidate_whole_rows() {
    let mut e = engine();
    let a = args(json!({"x":[1,2],"y":[1,2],"xerr":0.1,"yerr":[0.1,null],"marker":"circle"}));
    let l = build(
        &mut e,
        "errorbar",
        &a,
        0,
        &Args::new(),
        "cartesian",
        &Args::new(),
        Loc::default(),
    )
    .unwrap();
    assert_eq!(l.primitives.len(), 3);
    assert_eq!(l.primitives.last().unwrap().points, vec![[1., 1.]]);
    assert_eq!(
        e.warnings
            .iter()
            .filter(|w| w.code == "W_PLOT_MISSING")
            .count(),
        1
    );
    let all = args(json!({"x":[1],"y":[1],"yerr":[null]}));
    assert!(
        build(
            &mut e,
            "errorbar",
            &all,
            0,
            &Args::new(),
            "cartesian",
            &Args::new(),
            Loc::default()
        )
        .is_err()
    );
}
#[test]
fn polar_default_bar_width_is_unit_invariant_and_limited() {
    for (unit, expected) in [("deg", 20.), ("rad", std::f64::consts::PI / 9.)] {
        let plot = args(json!({"angle_unit":unit}));
        let a = args(json!({"positions":[0],"values":[2]}));
        let l = build(
            &mut engine(),
            "bar",
            &a,
            0,
            &Args::new(),
            "polar",
            &plot,
            Loc::default(),
        )
        .unwrap();
        close(
            l.primitives[0].points[1][0] - l.primitives[0].points[0][0],
            expected,
        );
        let bad =
            args(json!({"positions":[0],"values":[2],"angle_width":if unit=="deg"{361.}else{7.}}));
        assert!(
            build(
                &mut engine(),
                "bar",
                &bad,
                0,
                &Args::new(),
                "polar",
                &plot,
                Loc::default()
            )
            .is_err()
        );
    }
}
#[test]
fn constant_histogram_range_and_degenerate_violin_retain_defaults() {
    let h = layer(
        "hist",
        json!({"values":[100,100],"bins":2,"stat":"density"}),
    );
    assert_eq!(h.statistics["edges"], json!([95., 100., 105.]));
    assert_eq!(h.statistics["counts"], json!([0., 2.]));
    close(h.statistics["heights"][1].as_f64().unwrap() * 5., 1.);
    let v = layer(
        "violin",
        json!({"values":[2],"position":3,"data_width":0.8}),
    );
    close(
        v.primitives[0].points[1][0] - v.primitives[0].points[0][0],
        0.8,
    );
}
#[test]
fn ecdf_reaches_visible_axis_endpoints() {
    let e = engine();
    let l = layer("ecdf", json!({"values":[1,2,2,3]}));
    let mut x = Axis::new(
        &e,
        args(json!({"range":[0,4]})),
        "x",
        "bottom",
        0.,
        &[],
        Loc::default(),
    )
    .unwrap();
    let mut y = Axis::new(
        &e,
        args(json!({"range":[0,1]})),
        "y",
        "left",
        0.,
        &[],
        Loc::default(),
    )
    .unwrap();
    x.map_segments(&e, 80., Loc::default()).unwrap();
    y.map_segments(&e, 40., Loc::default()).unwrap();
    let nodes = cartesian_layer(&e, &l, &x, &y, 80., 40.).unwrap();
    let d = nodes[0]["children"][0]["d"].as_str().unwrap();
    let p = BezPath::from_svg(d).unwrap();
    assert_eq!(
        p.elements().first(),
        Some(&kurbo::PathEl::MoveTo((0., 40.).into()))
    );
    assert_eq!(
        p.elements().last(),
        Some(&kurbo::PathEl::LineTo((80., 0.).into()))
    );
}
#[test]
fn hatch_physical_spacing_opacity_holes_and_limits() {
    let mut l = layer(
        "bar",
        json!({"positions":[1],"values":[3],"hatch":"slash","hatch_spacing":1.5,"opacity":0.25}),
    );
    let p = &l.primitives[0];
    let rings = vec![
        vec![[0., 0.], [10., 0.], [10., 10.], [0., 10.]],
        vec![[3., 3.], [7., 3.], [7., 7.], [3., 7.]],
    ];
    let nodes = hatch(&engine(), p, &rings, &l.args, 10., 10.).unwrap();
    assert_eq!(nodes[0]["opacity"], json!(0.25));
    let path = BezPath::from_svg(nodes[0]["d"].as_str().unwrap()).unwrap();
    let mut intercepts = vec![];
    for pair in path.elements().chunks_exact(2) {
        if let (kurbo::PathEl::MoveTo(a), kurbo::PathEl::LineTo(b)) = (pair[0], pair[1]) {
            intercepts.push((a.x + a.y) / std::f64::consts::SQRT_2);
            let m = a.midpoint(b);
            assert!(!(m.x > 3. && m.x < 7. && m.y > 3. && m.y < 7.));
        }
    }
    intercepts.sort_by(f64::total_cmp);
    intercepts.dedup_by(|a, b| (*a - *b).abs() < 1e-8);
    for pair in intercepts.windows(2) {
        close(pair[1] - pair[0], 1.5);
    }
    l.args.insert("hatch_spacing".into(), V::mm(0.000001));
    assert_eq!(
        hatch(&engine(), &l.primitives[0], &rings, &l.args, 10., 10.)
            .unwrap_err()
            .code,
        "E_LIMIT"
    );
}
#[test]
fn marching_squares_has_no_triangle_diagonal_vertex() {
    let l = layer(
        "contourf",
        json!({"z":[[0,0],[1,1.15]],"x":[0,45],"y":[0,1],"levels":[0,0.5,2]}),
    );
    let band = &l.primitives[0];
    let points = std::iter::once(&band.points)
        .chain(&band.holes)
        .flatten()
        .collect::<Vec<_>>();
    assert!(
        points
            .iter()
            .any(|p| (p[0] - 0.).abs() < 1e-9 && (p[1] - 0.5).abs() < 1e-9)
    );
    assert!(
        points
            .iter()
            .any(|p| (p[0] - 45.).abs() < 1e-9 && (p[1] - 0.5 / 1.15).abs() < 1e-9)
    );
    assert!(
        !points
            .iter()
            .any(|p| (p[0] - 45. * 0.5 / 1.15).abs() < 1e-9)
    );
}

#[test]
fn legacy_statistics_reference_results_do_not_mutate_inputs() {
    let input = json!({"values":[-1,0,0.5,1,2,3,4],"bins":[0,1,3],"weights":[1,2,3,4,5,6,7],"stat":"density"});
    let snapshot = input.clone();
    let h = layer("hist", input.clone());
    assert_eq!(input, snapshot);
    assert_eq!(h.statistics["counts"], json!([5., 15.]));
    assert_eq!(h.statistics["excluded"], json!(2));
    close(h.statistics["total"].as_f64().unwrap(), 20.);
    let v = h.statistics["heights"].as_array().unwrap();
    close(v[0].as_f64().unwrap() + v[1].as_f64().unwrap() * 2., 1.);
    let b = layer("boxplot", json!({"values":[100,1,2,3,4,5,6,7,8]}));
    for (k, v) in [
        ("q1", 3.),
        ("median", 5.),
        ("q3", 7.),
        ("lower", 1.),
        ("upper", 8.),
    ] {
        close(b.statistics[k].as_f64().unwrap(), v);
    }
    assert_eq!(b.statistics["outliers"], json!([100.]));
    let ecdf = layer("ecdf", json!({"values":[2,1,2,3]}));
    assert_eq!(
        ecdf.primitives[0].points,
        vec![
            [1., 0.],
            [1., 0.25],
            [2., 0.25],
            [2., 0.75],
            [3., 0.75],
            [3., 1.]
        ]
    );
    let kde = layer("violin", json!({"values":[-1,1],"bandwidth":1,"points":3}));
    assert_eq!(kde.statistics["positions"], json!([-1., 0., 1.]));
    close(
        kde.statistics["density"][1].as_f64().unwrap(),
        (-0.5f64).exp() / std::f64::consts::TAU.sqrt(),
    );
    let auto = layer("violin", json!({"values":[-1,1]}));
    close(
        auto.statistics["bandwidth"].as_f64().unwrap(),
        2f64.sqrt() * 2f64.powf(-0.2),
    );
    assert_eq!(layer("violin", json!({"values":[2,2]})).primitives.len(), 1);
}
#[test]
fn color_normalizations_under_over_boundaries_and_inverse_retain_reference_values() {
    let mut s = args(
        json!({"norm":"linear","vmin":0,"vmax":10,"cmap":["#000000","#ffffff"],"center":2,"constant":1}),
    );
    assert_eq!(scale_color(&s, 5.), "#808080");
    assert_eq!(scale_color(&s, -5.), "#000000");
    assert_eq!(scale_color(&s, f64::NAN), "none");
    s.insert("under".into(), V::text("#ff0000"));
    assert_eq!(scale_color(&s, -1.), "#ff0000");
    s.insert("over".into(), V::text("#0000ff"));
    assert_eq!(scale_color(&s, 11.), "#0000ff");
    for (mut s, x) in [
        (args(json!({"norm":"log","vmin":1,"vmax":100})), 10.),
        (
            args(json!({"norm":"centered","vmin":-4,"vmax":10,"center":0})),
            0.,
        ),
        (args(json!({"norm":"symlog","vmin":-9,"vmax":9})), 0.),
    ] {
        close(color_position(&s, x), 0.5);
        close(color_value(&s, 0.5), x);
        s.insert("constant".into(), V::num(1.));
        close(color_value(&s, color_position(&s, x - 0.1)), x - 0.1);
    }
    let discrete = args(json!({"norm":"boundary","boundaries":[0,1,10],"cmap":["#f00","#00f"]}));
    assert_eq!(scale_color(&discrete, 0.99), "#f00");
    assert_eq!(scale_color(&discrete, 1.), "#00f");
    assert_eq!(scale_color(&discrete, 10.), "#00f");
}
#[test]
fn missing_grid_masks_cover_every_valid_nonuniform_cell_exactly_once() {
    let mut z = vec![vec![1.; 16]; 12];
    for row in &mut z[4..7] {
        for v in &mut row[6..9] {
            *v = f64::NAN;
        }
    }
    let x: Vec<_> = (0..16).map(|i| (i * i) as f64).collect();
    let y: Vec<_> = (0..12).map(|i| (i * 2) as f64).collect();
    let a = args(
        json!({"z":z.iter().map(|row|row.iter().map(|v|if v.is_finite(){json!(v)}else{Json::Null}).collect::<Vec<_>>()).collect::<Vec<_>>(),"x":x,"y":y,"levels":[0,2]}),
    );
    let l = build(
        &mut engine(),
        "contourf",
        &a,
        0,
        &Args::new(),
        "cartesian",
        &Args::new(),
        Loc::default(),
    )
    .unwrap();
    let masks = l.masks.unwrap();
    assert!(masks.len() <= 4);
    for j in 0..11 {
        for i in 0..15 {
            let (cx, cy) = ((x[i] + x[i + 1]) / 2., (y[j] + y[j + 1]) / 2.);
            let count = masks
                .iter()
                .filter(|[a, b, c, d]| *a < cx && cx < *b && *c < cy && cy < *d)
                .count();
            assert_eq!(
                count,
                usize::from(
                    [z[j][i], z[j][i + 1], z[j + 1][i], z[j + 1][i + 1]]
                        .iter()
                        .all(|v| v.is_finite())
                )
            );
        }
    }
}
#[test]
fn polar_degree_radian_paths_shortest_raw_chord_and_physical_accuracy() {
    let m = json!({"kind":"polar","center":[55.,45.],"outerRadius":30.,"innerRadius":0.,"angleUnit":"deg","thetaZero":0.,"direction":1.,"theta":[0.,360.],"radial":{"scale":"linear","domain":[0.,10.],"constant":1.,"reverse":false}});
    let mut rad = m.clone();
    rad["angleUnit"] = json!("rad");
    rad["theta"] = json!([0., std::f64::consts::TAU]);
    for angle in [-720., -20., 0., 90., 350., 800.] {
        for r in [-5., 0., 8.] {
            let a = projection::polar_point(&m, angle, r);
            let b = projection::polar_point(&rad, angle.to_radians(), r);
            close(a[0], b[0]);
            close(a[1], b[1]);
        }
    }
    assert_eq!(
        projection::unwrap_angle_points(&[[350., 5.], [10., 5.]], 360.),
        vec![[350., 5.], [370., 5.]]
    );
    assert_eq!(
        projection::unwrap_angle_points(&[[0., 5.], [180., 5.]], 360.),
        vec![[0., 5.], [180., 5.]]
    );
    let mut l = layer("line", json!({"x":[350,10],"y":[5,5]}));
    let short = projection::projected_points(&l.primitives[0], &l, &m);
    assert!(short.iter().all(|p| p[0] > 69.));
    l.args.insert("wrap".into(), V::text("raw"));
    let raw = projection::projected_points(&l.primitives[0], &l, &m);
    assert!(raw.iter().any(|p| p[0] < 41.));
    l.primitives[0].points = vec![[0., 5.], [360., 5.]];
    let circle = projection::projected_points(&l.primitives[0], &l, &m);
    assert!(circle.len() > 100);
    for p in &circle {
        close((p[0] - 55.).hypot(p[1] - 45.), 15.);
    }
    for q in circle.windows(2) {
        assert!(
            15. - ((q[0][0] + q[1][0]) / 2. - 55.).hypot((q[0][1] + q[1][1]) / 2. - 45.) <= 0.001
        );
    }
    l.primitives[0].points = vec![[350., 5.], [370., 5.]];
    l.args.insert("interpolation".into(), V::text("chord"));
    let chord = projection::projected_points(&l.primitives[0], &l, &m);
    assert_eq!(chord.len(), 2);
    close(chord[0][0], chord[1][0]);
    assert!((chord[0][0] + chord[1][0]) / 2. < 70.);
}
#[test]
fn angular_histogram_weighted_mass_is_equal_in_degrees_and_radians() {
    for (unit, factor) in [("deg", 1.), ("rad", std::f64::consts::PI / 180.)] {
        let a = args(
            json!({"values":[-10.*factor,0.,360.*factor,370.*factor],"bins":[0.,180.*factor,360.*factor],"weights":[1,2,3,4],"stat":"density"}),
        );
        let before = a
            .iter()
            .map(|(k, v)| (k.clone(), v.json()))
            .collect::<BTreeMap<_, _>>();
        let mut e = engine();
        let l = build(
            &mut e,
            "hist",
            &a,
            0,
            &Args::new(),
            "polar",
            &args(json!({"angle_unit":unit})),
            Loc::default(),
        )
        .unwrap();
        assert_eq!(l.statistics["counts"], json!([9., 1.]));
        close(l.statistics["total"].as_f64().unwrap(), 10.);
        close(
            l.statistics["heights"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_f64().unwrap())
                .sum::<f64>()
                * 180.
                * factor,
            1.,
        );
        assert_eq!(
            before,
            a.iter()
                .map(|(k, v)| (k.clone(), v.json()))
                .collect::<BTreeMap<_, _>>()
        );
        assert!(e.warnings.is_empty());
    }
}

#[test]
fn polar_log_symlog_and_negative_crossing_reference_values() {
    let mut m = json!({"kind":"polar","center":[50.,50.],"outerRadius":30.,"innerRadius":0.,"angleUnit":"deg","thetaZero":0.,"direction":1.,"theta":[0.,360.],"radial":{"scale":"log","domain":[1.,1000.],"constant":1.,"reverse":false}});
    close(projection::polar_point(&m, 0., 10.)[0] - 50., 10.);
    m["radial"] = json!({"scale":"symlog","domain":[0.,9.],"constant":1.});
    close(
        projection::polar_point(&m, 0., 10f64.sqrt() - 1.)[0] - 50.,
        15.,
    );
    m["radial"] = json!({"scale":"linear","domain":[0.,5.]});
    let l = layer("line", json!({"x":[0,0],"y":[-5,5]}));
    let p = projection::projected_points(&l.primitives[0], &l, &m);
    let zero = p
        .iter()
        .position(|p| (p[0] - 50.).hypot(p[1] - 50.) < 1e-9)
        .unwrap();
    assert_eq!(p[..=zero].last(), p[zero..].first());
    assert!(zero > 0 && zero + 1 < p.len());
    let half = layer("line", json!({"x":[0,180],"y":[5,5]}));
    let p = projection::projected_points(&half.primitives[0], &half, &m);
    assert!(p.iter().all(|q| q[1] <= 50. + 1e-9));
}
#[test]
fn missing_color_invalidates_line_rows_and_ticks_retain_exact_endpoints() {
    let mut a = args(json!({"x":[0,1,2],"y":[1,2,3],"c":[0,null,1]}));
    a.insert(
        "color_scale".into(),
        V::Object(Rc::new(RefCell::new(Object {
            kind: "color_scale".into(),
            args: args(json!({"vmin":0,"vmax":1})),
            file: String::new(),
            loc: Loc::default(),
            id: 0,
            nodes: vec![],
            layers: vec![],
            sealed: false,
            width: 0.,
            height: 0.,
            node: None,
            parent: 0,
        }))),
    );
    let l = build(
        &mut engine(),
        "scatter",
        &a,
        0,
        &Args::new(),
        "cartesian",
        &Args::new(),
        Loc::default(),
    )
    .unwrap();
    assert_eq!(l.primitives.len(), 2);
    assert!(
        l.primitives
            .iter()
            .all(|p| !p.points.iter().any(|p| p[0] == 1.))
    );
    assert_eq!(ticks([0., 1.2], 5), vec![0., 0.2, 0.4, 0.6, 0.8, 1., 1.2]);
}

#[test]
fn layer_contracts_reject_previously_observed_step_baseline_and_missing_row_mutations() {
    let actual = layer("step", json!({"x":[0,2],"y":[0,1]}));
    let post_contract = |p: &Primitive| p.points == vec![[0., 0.], [2., 0.], [2., 1.]];
    assert!(post_contract(&actual.primitives[0]));
    let mut midpoint = actual.primitives[0].clone();
    midpoint.points = vec![[0., 0.], [1., 0.], [1., 1.], [2., 1.]];
    assert!(
        !post_contract(&midpoint),
        "the former mid-step default must be rejected"
    );

    let actual = layer("area", json!({"x":[0,1,2],"y":[3,4,5],"baseline":[1,2,3]}));
    let baseline_contract = |p: &Primitive| {
        p.points == vec![[0., 1.], [1., 2.], [2., 3.], [2., 5.], [1., 4.], [0., 3.]]
    };
    assert!(baseline_contract(&actual.primitives[0]));
    let mut scalar_baseline = actual.primitives[0].clone();
    for p in &mut scalar_baseline.points[..3] {
        p[1] = 0.;
    }
    assert!(
        !baseline_contract(&scalar_baseline),
        "discarding a baseline array must be rejected"
    );

    let actual = layer(
        "errorbar",
        json!({"x":[1,2],"y":[1,2],"xerr":0.1,"yerr":[0.1,null],"marker":"circle"}),
    );
    let missing_contract = |l: &Layer| {
        l.primitives.len() == 3
            && l.primitives
                .iter()
                .all(|p| p.points.iter().all(|p| p[0] < 1.2))
    };
    assert!(missing_contract(&actual));
    let mut partial_row = actual.clone();
    let mut horizontal_error = actual.primitives[0].clone();
    horizontal_error.points = vec![[1.9, 2.], [2.1, 2.]];
    partial_row.primitives.push(horizontal_error);
    let mut marker = actual.primitives.last().unwrap().clone();
    marker.points = vec![[2., 2.]];
    partial_row.primitives.push(marker);
    assert!(
        !missing_contract(&partial_row),
        "retaining x-error and marker when y-error is missing must be rejected"
    );

    let mut a = args(json!({"x":[0,1,2],"y":[1,2,3],"c":[0,null,1]}));
    a.insert(
        "color_scale".into(),
        V::Object(Rc::new(RefCell::new(Object {
            kind: "color_scale".into(),
            args: args(json!({"vmin":0,"vmax":1})),
            file: String::new(),
            loc: Loc::default(),
            id: 0,
            nodes: vec![],
            layers: vec![],
            sealed: false,
            width: 0.,
            height: 0.,
            node: None,
            parent: 0,
        }))),
    );
    let actual = build(
        &mut engine(),
        "scatter",
        &a,
        0,
        &Args::new(),
        "cartesian",
        &Args::new(),
        Loc::default(),
    )
    .unwrap();
    let color_contract = |l: &Layer| {
        l.primitives.len() == 2
            && l.primitives
                .iter()
                .all(|p| p.points.iter().all(|p| p[0] != 1.))
    };
    assert!(color_contract(&actual));
    let mut retained_missing_color = actual.clone();
    let mut bad = actual.primitives[0].clone();
    bad.points = vec![[1., 2.]];
    retained_missing_color.primitives.push(bad);
    assert!(
        !color_contract(&retained_missing_color),
        "missing colors invalidate the entire point, not just its paint"
    );
}
#[test]
fn hatch_contract_rejects_lost_opacity_and_unnormalized_diagonal_spacing() {
    let l = layer(
        "bar",
        json!({"positions":[1],"values":[3],"hatch":"slash","hatch_spacing":1.5,"opacity":0.25}),
    );
    let rings = vec![vec![[0., 0.], [10., 0.], [10., 10.], [0., 10.]]];
    let actual = hatch(&engine(), &l.primitives[0], &rings, &l.args, 10., 10.).unwrap();
    let contract = |nodes: &[Json]| {
        if nodes.len() != 1 || (jnum(&nodes[0], "opacity", 1.) - 0.25).abs() > 1e-9 {
            return false;
        }
        let p = BezPath::from_svg(jstr(&nodes[0], "d", "")).unwrap();
        let mut intercepts = vec![];
        for pair in p.elements().chunks_exact(2) {
            if let [kurbo::PathEl::MoveTo(a), kurbo::PathEl::LineTo(b)] = pair {
                if ((a.x - a.y) - (b.x - b.y)).abs() < 1e-9 {
                    return false;
                }
                intercepts.push((a.x + a.y) / std::f64::consts::SQRT_2);
            }
        }
        intercepts.sort_by(f64::total_cmp);
        intercepts.dedup_by(|a, b| (*a - *b).abs() < 1e-8);
        intercepts.len() > 3
            && intercepts
                .windows(2)
                .all(|q| (q[1] - q[0] - 1.5).abs() < 1e-8)
    };
    assert!(contract(&actual));
    let mut opaque = actual.clone();
    opaque[0]["opacity"] = json!(1.);
    assert!(!contract(&opaque));
    let mut old_spacing = actual.clone();
    let d = BezPath::from_svg(jstr(&old_spacing[0], "d", "")).unwrap();
    old_spacing[0]["d"] = json!((kurbo::Affine::scale(1. / std::f64::consts::SQRT_2) * d).to_svg());
    assert!(
        !contract(&old_spacing),
        "1.5 intercept spacing is not 1.5mm perpendicular spacing"
    );
}
#[test]
fn range_contracts_reject_degree_width_in_radians_narrow_constant_histogram_and_short_ecdf() {
    let actual = build(
        &mut engine(),
        "bar",
        &args(json!({"positions":[0],"values":[2]})),
        0,
        &Args::new(),
        "polar",
        &args(json!({"angle_unit":"rad"})),
        Loc::default(),
    )
    .unwrap();
    let radial_contract = |l: &Layer| {
        let width = l.primitives[0].points[1][0] - l.primitives[0].points[0][0];
        (width - std::f64::consts::PI / 9.).abs() < 1e-9 && width <= std::f64::consts::TAU
    };
    assert!(radial_contract(&actual));
    let mut degrees_as_radians = actual.clone();
    degrees_as_radians.primitives[0].points[0][0] = -10.;
    degrees_as_radians.primitives[0].points[1][0] = 10.;
    assert!(!radial_contract(&degrees_as_radians));

    let actual = layer(
        "hist",
        json!({"values":[100,100],"bins":2,"stat":"density"}),
    );
    let histogram_contract = |l: &Layer| {
        l.statistics["edges"] == json!([95., 100., 105.])
            && l.statistics["counts"] == json!([0., 2.])
            && (l.statistics["heights"][1].as_f64().unwrap() * 5. - 1.).abs() < 1e-9
    };
    assert!(histogram_contract(&actual));
    let mut narrow = actual.clone();
    narrow.statistics["edges"] = json!([99.5, 100., 100.5]);
    narrow.statistics["heights"] = json!([0., 2.]);
    assert!(!histogram_contract(&narrow));

    let e = engine();
    let l = layer("ecdf", json!({"values":[1,2,2,3]}));
    let mut x = Axis::new(
        &e,
        args(json!({"range":[0,4]})),
        "x",
        "bottom",
        0.,
        &[],
        Loc::default(),
    )
    .unwrap();
    let mut y = Axis::new(
        &e,
        args(json!({"range":[0,1]})),
        "y",
        "left",
        0.,
        &[],
        Loc::default(),
    )
    .unwrap();
    x.map_segments(&e, 80., Loc::default()).unwrap();
    y.map_segments(&e, 40., Loc::default()).unwrap();
    let actual = cartesian_layer(&e, &l, &x, &y, 80., 40.).unwrap();
    let ecdf_contract = |nodes: &[Json]| {
        let d = BezPath::from_svg(jstr(&nodes[0]["children"][0], "d", "")).unwrap();
        d.elements().first() == Some(&kurbo::PathEl::MoveTo((0., 40.).into()))
            && d.elements().last() == Some(&kurbo::PathEl::LineTo((80., 0.).into()))
    };
    assert!(ecdf_contract(&actual));
    let mut cropped = actual.clone();
    let mut d = BezPath::from_svg(jstr(&cropped[0]["children"][0], "d", ""))
        .unwrap()
        .elements()
        .to_vec();
    d[0] = kurbo::PathEl::MoveTo((20., 40.).into());
    *d.last_mut().unwrap() = kurbo::PathEl::LineTo((60., 0.).into());
    cropped[0]["children"][0]["d"] = json!(BezPath::from_vec(d).to_svg());
    assert!(!ecdf_contract(&cropped));
}
