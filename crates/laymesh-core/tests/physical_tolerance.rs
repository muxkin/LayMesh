//! Boolean inputs are flattened in physical output space, including shear from
//! nested nonuniform scales separated by rotations.
use kurbo::{Affine, BezPath, ParamCurve, ParamCurveNearest, Point};
use laymesh_core::geometry::{compound_outline_transformed, node_transform, visible};
use serde_json::json;
fn boundary_distance(path: &BezPath, point: Point) -> f64 {
    path.segments()
        .map(|segment| segment.nearest(point, 1e-10).distance_sq.sqrt())
        .fold(f64::INFINITY, f64::min)
}
#[test]
fn nested_scaled_cubic_fill_stays_within_one_micron() {
    let mut curve = BezPath::new();
    curve.move_to((0., 0.));
    curve.curve_to((0., 0.002), (0.002, 0.002), (0.002, 0.));
    curve.close_path();
    let leaf = json!({"kind":"path","d":curve.to_svg(),"width":0.002,"height":0.002,"fill":"#000","strokeStyle":{"color":"none","width":0.}});
    let inner = json!({"kind":"group","width":2.,"height":0.2,"contentWidth":0.002,"contentHeight":0.002,"rotation":31.,"children":[leaf]});
    let outer = json!({"kind":"group","width":200.,"height":200.,"contentWidth":2.,"contentHeight":0.2,"children":[inner]});
    let transform = node_transform(&outer)
        * Affine::scale_non_uniform(100., 1000.)
        * node_transform(&outer["children"][0])
        * Affine::scale_non_uniform(1000., 100.);
    let expected = transform * curve;
    let actual = visible(&outer);
    let curve = expected.segments().next().unwrap();
    let max = (0..1001)
        .map(|i| boundary_distance(&actual, curve.eval(i as f64 / 1000.)))
        .fold(0., f64::max);
    assert!(max <= 0.001, "final physical boundary error {max}mm");
}
#[test]
fn magnified_compound_round_caps_preserve_bands_and_submicron_boundaries() {
    let mut path = BezPath::new();
    path.move_to((0., 0.));
    path.line_to((0.002, 0.));
    let transform = Affine::rotate(0.37) * Affine::scale_non_uniform(100000., 50000.);
    let result = compound_outline_transformed(
        &path,
        &json!({"width":0.0001,"cap":"round","join":"round","compound":"double"}),
        transform,
    );
    let mut max: f64 = 0.;
    for radius in [0.00005, 0.00005 / 3.] {
        for i in 0..101 {
            let angle = std::f64::consts::FRAC_PI_2 + i as f64 / 100. * std::f64::consts::PI;
            let point = transform * Point::new(radius * angle.cos(), radius * angle.sin());
            max = max.max(boundary_distance(&result, point));
        }
    }
    assert!(max <= 0.001, "compound cap boundary error {max}mm");
}

#[test]
fn small_round_joins_prune_disks_only_below_the_physical_sagitta_budget() {
    for theta in [0.001_f64, 0.01, 0.1, 1.] {
        let mut path = BezPath::new();
        path.move_to((-100., 0.));
        path.line_to((0., 0.));
        path.line_to((100. * theta.cos(), 100. * theta.sin()));
        let result = compound_outline_transformed(
            &path,
            &json!({"width":10.,"cap":"butt","join":"round"}),
            Affine::IDENTITY,
        );
        let max = (0..101)
            .map(|i| {
                let angle = -std::f64::consts::FRAC_PI_2 + theta * i as f64 / 100.;
                boundary_distance(&result, Point::new(5. * angle.cos(), 5. * angle.sin()))
            })
            .fold(0., f64::max);
        assert!(
            max <= 0.001,
            "round join angle {theta}: boundary error {max}mm"
        );
    }
}

#[test]
fn magnified_polar_ink_and_clips_retain_final_physical_precision() {
    use laymesh_core::{
        engine::compile_source,
        geometry::resolved_path,
        model::{Host, jnum},
    };
    use serde_json::Value;
    let scene=compile_source(r#"page=canvas(size=(60,60))
p=plot(size=(40,40),plot_area=box(offset=(10,10),size=(20,20)),projection="polar",inner_radius=3mm,theta=axis(ticks=[]),r=axis(range=(0,1),ticks=[1],grid="major",tick_labels=false))
p.line(theta=[0,90],r=[1,1])
page.add(p)"#, "/polar-precision.lay",Host::default()).unwrap();
    fn collect<'a>(n: &'a Value, out: &mut Vec<&'a Value>) {
        if n["geometryRecipe"]["kind"] == "polar" {
            out.push(n);
        }
        if n["clipPath"]["geometryRecipe"]["kind"] == "polar" {
            out.push(&n["clipPath"]);
        }
        for c in n["children"].as_array().into_iter().flatten() {
            collect(c, out);
        }
    }
    let mut recipes = vec![];
    collect(&scene.nodes[0], &mut recipes);
    assert!(
        recipes.len() >= 3,
        "data, grid/ring and clipping boundary must retain recipes"
    );
    let transform = Affine::rotate(0.41) * Affine::scale_non_uniform(1000., 3000.);
    let mut tested_modes = std::collections::BTreeSet::new();
    for node in recipes {
        let recipe = &node["geometryRecipe"];
        let mode = recipe["mode"].as_str().unwrap();
        if !["projected", "ring", "boundary"].contains(&mode) {
            continue;
        }
        let m = &recipe["m"];
        let center = Point::new(
            m["center"][0].as_f64().unwrap(),
            m["center"][1].as_f64().unwrap(),
        );
        let r = jnum(m, "outerRadius", 0.);
        let actual = resolved_path(node, transform);
        let original = transform * BezPath::from_svg(node["d"].as_str().unwrap()).unwrap();
        let (mut refined_error, mut original_error) = (0_f64, 0_f64);
        for i in 0..=127 {
            let angle = std::f64::consts::FRAC_PI_2 * (i as f64 + 0.37) / 128.;
            let point =
                transform * Point::new(center.x + r * angle.cos(), center.y - r * angle.sin());
            refined_error = refined_error.max(boundary_distance(&actual, point));
            original_error = original_error.max(boundary_distance(&original, point));
        }
        assert!(refined_error <= 0.001, "{mode}: {refined_error}mm");
        // Fault injection: remove retained source geometry, as in the first rewrite.
        // Magnifying that already flattened path must fail the same error contract.
        assert!(
            original_error > 0.01,
            "{mode}: test must detect the old coarse polygon ({original_error})"
        );
        tested_modes.insert(mode);
    }
    for mode in ["projected", "ring", "boundary"] {
        assert!(tested_modes.contains(mode), "missing {mode}");
    }
}

#[test]
fn magnified_polar_hatches_are_reclipped_with_library_against_refined_rings() {
    use laymesh_core::{engine::compile_source, geometry::resolved_path, model::Host};
    use serde_json::Value;
    let scene=compile_source(r#"page=canvas(size=(60,60))
p=plot(size=(40,40),plot_area=box(offset=(10,10),size=(20,20)),projection="polar",inner_radius=3mm,theta=axis(ticks=[]),r=axis(range=(0,1),ticks=[]))
p.bar(positions=[180],values=[1],angle_width=360,hatch="cross",hatch_spacing=0.7mm)
page.add(p)"#, "/polar-hatch.lay",Host::default()).unwrap();
    fn find(n: &Value) -> Option<&Value> {
        if n["geometryRecipe"]["kind"] == "polar_hatch" {
            return Some(n);
        }
        n["children"]
            .as_array()
            .into_iter()
            .flatten()
            .find_map(find)
    }
    let node =
        find(&scene.nodes[0]).expect("polar hatches retain source region and physical pattern");
    let transform = Affine::rotate(0.41) * Affine::scale_non_uniform(1000., 3000.);
    let actual = resolved_path(node, transform);
    let original = transform * BezPath::from_svg(node["d"].as_str().unwrap()).unwrap();
    let error = |path: &BezPath| {
        // FloatClip may split crossing hatch lines at their intersections.
        // Only degree-one vertices are endpoints on the region boundary.
        let mut vertices = std::collections::BTreeMap::<(i64, i64), (usize, Point)>::new();
        for segment in path.segments() {
            for point in [segment.start(), segment.end()] {
                let key = (
                    (point.x * 1e5).round() as i64,
                    (point.y * 1e5).round() as i64,
                );
                let entry = vertices.entry(key).or_insert((0, point));
                entry.0 += 1;
            }
        }
        let endpoints: Vec<_> = vertices
            .values()
            .filter(|(degree, _)| *degree == 1)
            .map(|(_, p)| *p)
            .collect();
        assert!(
            endpoints.len() > 20,
            "enough independent region intersections"
        );
        endpoints
            .into_iter()
            .map(|point| {
                let local = transform.inverse() * point;
                let radius = local.distance(Point::new(20., 20.));
                (radius - 3.).abs().min((radius - 10.).abs()) * 3000.
            })
            .fold(0_f64, f64::max)
    };
    assert!(
        actual.segments().count() > 20,
        "nonempty intersected hatch bands"
    );
    assert!(
        error(&actual) <= 0.001,
        "final hatch intersection error {}mm",
        error(&actual)
    );
    assert!(
        error(&original) > 0.01,
        "fault injection must detect previously clipped endpoints"
    );
}
