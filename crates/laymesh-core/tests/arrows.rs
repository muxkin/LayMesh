use kurbo::{Affine, BezPath, Point, Shape};
use laymesh_core::{
    engine::compile_source,
    geometry::{node_transform, resolved_path},
    model::{Host, Scene, jnum},
};
use serde_json::Value;
fn scene(code: &str) -> Scene {
    compile_source(
        &format!("page=canvas(size=(200,160))\n{code}"),
        "/arrows.lay",
        Host::default(),
    )
    .unwrap_or_else(|e| panic!("{code}\n{e:?}"))
}
fn point(node: &Value, i: usize) -> Point {
    node_transform(node)
        * Point::new(
            node["endpoints"][i][0].as_f64().unwrap(),
            node["endpoints"][i][1].as_f64().unwrap(),
        )
}
fn close(a: Point, b: Point) {
    assert!(a.distance(b) < 1e-7, "{a:?} != {b:?}");
}

// Constructors without a closing parenthesis, so each case can share physical
// dimensions and decoration while exercising a different connection solver.
const ANCHORED_TEMPLATES: [&str; 5] = [
    "arrow(length=50",
    "arrow.arc(sweep_angle=90deg",
    "arrow.bent(span=(40,25),corner_radius=8",
    "arrow.uturn(span=(40,25),corner_radius=8",
    "arrow.chevron(length=50,notch_depth=3",
];

#[test]
fn every_template_connects_coordinates_layout_data_and_directed_path_anchors() {
    let tangent = kurbo::Vec2::new(100., 40.).normalize();
    let normal = kurbo::Vec2::new(tangent.y, -tangent.x);
    let cases = [
        (
            "",
            "start=(20,30),end=(100,70)",
            Point::new(20., 30.),
            Point::new(100., 70.),
        ),
        (
            "a=page.add(rect(size=(10,10)),offset=(10,25))\nb=page.add(rect(size=(10,10)),offset=(100,65))",
            "start=a.middle_right,end=b.middle_left,start_offset=(2,3),end_offset=(-4,1),offset=(1,2)",
            Point::new(23., 35.),
            Point::new(97., 73.),
        ),
        (
            "guide=page.add(line(dx=100,dy=40),anchor=start,offset=(20,30))",
            "start=guide.path.at(fraction=0.1),end=guide.path.at(fraction=0.9),start_offset=(3,2),end_offset=(-4,1),start_offset_space=target,end_offset_space=target,offset=(2,5)",
            Point::new(32., 39.) + tangent * 3. + normal * 2.,
            Point::new(112., 71.) - tangent * 4. + normal,
        ),
        (
            "p=plot(size=(140,100),plot_area=box(offset=(10,10),size=(120,80)),x=axis(range=(0,10)),y=axis(range=(0,10)))\np.line(x=[0,10],y=[0,10])\nchart=page.add(p,offset=(5,5))",
            "start=chart.data(x=1,y=2),end=chart.data(x=8,y=7)",
            Point::new(27., 79.),
            Point::new(111., 39.),
        ),
    ];
    for template in ANCHORED_TEMPLATES {
        for heads in ["start", "end", "both"] {
            if template.starts_with("arrow.chevron") && heads == "both" {
                continue;
            }
            for (setup, placement, start, end) in cases {
                let s = scene(&format!(
                    r##"{setup}
material={template},heads={heads},shaft_width=(2mm,4mm),head_size=(7mm,9mm),fill=linear_gradient(stops=[(0,"#087f8c"),(1,"#5273c5")]),border_color="#ffffff",border_width=0.4mm,effects=[shadow(blur=1mm,offset=(1mm,1mm))])
connected=page.add(material,{placement})
page.add(rect(size=(1,1)),target=connected.start)
page.add(rect(size=(1,1)),target=connected.end)
page.add(rect(size=(1,1)),target=connected.centerline.start)
page.add(rect(size=(1,1)),target=connected.centerline.end)
"##
                ));
                let ns = &s.nodes[s.nodes.len() - 5..];
                close(point(&ns[0], 0), start);
                close(point(&ns[0], 1), end);
                for (n, expected) in ns[1..].iter().zip([start, end, start, end]) {
                    close(Point::new(jnum(n, "x", 0.), jnum(n, "y", 0.)), expected);
                }
                assert_eq!(ns[0]["fill"]["kind"], "linear_gradient");
                assert_eq!(ns[0]["strokeStyle"]["color"], "#ffffff");
                assert_eq!(ns[0]["strokeStyle"]["width"], 0.4);
                assert_eq!(
                    ns[0]["geometryRecipe"]["widths"],
                    serde_json::json!([[0., 2.], [1., 4.]])
                );
                assert_eq!(ns[0]["effects"].as_array().unwrap().len(), 1);
            }
        }
    }
}

#[test]
fn every_template_single_anchor_survives_scaling_rotation_and_material_reuse() {
    for template in ANCHORED_TEMPLATES {
        let template = template.replace("arrow.arc(", "arrow.arc(radius=30mm,");
        for (anchor, endpoint) in [("start", 0), ("end", 1)] {
            for rotation in [-37, 0, 53] {
                let s = scene(&format!(
                    "material={template},shaft_width=3mm,head_size=(7mm,9mm))\noriginal=page.add(material)\ntarget_box=page.add(rect(size=(10,10)),offset=(110,55))\nx=page.add(material,anchor={anchor},target=target_box.middle_left,size=(80,40),rotation={rotation}deg)\npage.add(rect(size=(1,1)),target=x.centerline.{anchor})\npage.add(material)"
                ));
                close(point(&s.nodes[2], endpoint), Point::new(110., 60.));
                close(
                    Point::new(jnum(&s.nodes[3], "x", 0.), jnum(&s.nodes[3], "y", 0.)),
                    Point::new(110., 60.),
                );
                // Ordinary placement must not retain a previous instance's fit.
                for key in ["d", "width", "height", "endpoints", "geometryRecipe"] {
                    assert_eq!(s.nodes[0][key], s.nodes[4][key], "{template}: {key}");
                }
            }
        }
    }
}

#[test]
fn every_template_group_replay_reconnects_after_inherited_text_reflow() {
    for template in ANCHORED_TEMPLATES {
        let source = format!(
            r##"
page=canvas(size=(240,180))
style {{ group.grow {{font-size:24pt;}} }}
g=group()
a=g.add(text("Anchor",font_family="/font.ttf"),offset=(5,40))
b=g.add(rect(size=(10,10)),offset=(120,55))
c=g.add({template},shaft_width=2mm,head_size=(6mm,8mm)),start=a.middle_right,end=b.middle_left)
g.add(rect(size=(1,1)),target=c.centerline.start)
g.add(rect(size=(1,1)),target=c.end)
large=group(class="grow")
large.add(g)
page.add(large,size=(170,80),rotation=17deg)
page.add(g,offset=(0,95))
"##
        );
        let mut host = Host::default();
        host.files.insert(
            "/font.ttf".into(),
            include_bytes!("../../../tests/fonts/DejaVuSans.ttf").to_vec(),
        );
        let s = compile_source(&source, "/reflow.lay", host).unwrap();
        let large = &s.nodes[0]["children"][0]["children"];
        let small = &s.nodes[1]["children"];
        assert!(jnum(&large[0], "width", 0.) > jnum(&small[0], "width", 0.));
        for ns in [large, small] {
            let start = node_transform(&ns[0])
                * Point::new(jnum(&ns[0], "width", 0.), jnum(&ns[0], "height", 0.) / 2.);
            let end = node_transform(&ns[1]) * Point::new(0., jnum(&ns[1], "height", 0.) / 2.);
            close(point(&ns[2], 0), start);
            close(point(&ns[2], 1), end);
            close(
                Point::new(jnum(&ns[3], "x", 0.), jnum(&ns[3], "y", 0.)),
                start,
            );
            close(
                Point::new(jnum(&ns[4], "x", 0.), jnum(&ns[4], "y", 0.)),
                end,
            );
        }
    }
}

#[test]
fn every_template_rejects_conflicting_or_invalid_anchor_connections() {
    for template in ANCHORED_TEMPLATES {
        for placement in [
            "start=(20,30)",
            "start=(20,30),end=(20,30)",
            "start=(20,30),end=(100,70),anchor=start",
            "start=(20,30),end=(100,70),target=box_ref.center",
            "start=(20,30),end=(100,70),size=(80,40)",
            "start=(20,30),end=(100,70),rotation=15deg",
            "start=(20,30),end=(100,70),start_offset_space=target",
        ] {
            let source = format!(
                "page=canvas(size=(200,160))\nbox_ref=page.add(rect(size=(10,10)))\npage.add({template}),{placement})"
            );
            let error =
                compile_source(&source, "/invalid.lay", Host::default()).expect_err(&source);
            assert!(
                matches!(error.code.as_str(), "E_ARG" | "E_ANCHOR_DIRECTION"),
                "{error:?}"
            );
            assert_eq!(error.loc.line, 3);
        }
    }
}
#[test]
fn namespace_templates_are_closed_single_materials() {
    for expr in [
        "arrow(length=50)",
        "arrow(dx=40,dy=-20,heads=both)",
        "arrow.arc(radius=30,start_angle=15deg,sweep_angle=260deg)",
        "arrow.bent()",
        "arrow.uturn(heads=both)",
        "arrow.chevron(length=45)",
        "arrow.chevron(length=45,heads=start)",
        "arrow.path(path=path(commands=[move_to(0,0),cubic_to(20,0,20,25,50,25)]))",
    ] {
        let s = scene(&format!("page.add({expr})"));
        let n = &s.nodes[0];
        let p = BezPath::from_svg(n["d"].as_str().unwrap()).unwrap();
        assert_eq!(
            p.elements()
                .iter()
                .filter(|e| matches!(e, kurbo::PathEl::MoveTo(_)))
                .count(),
            1,
            "{expr}"
        );
        assert!(
            matches!(p.elements().last(), Some(kurbo::PathEl::ClosePath)),
            "{expr}"
        );
        assert!(p.area().abs() > 10., "{expr}");
        assert_eq!(n["geometryRecipe"]["kind"], "arrow");
        assert!(n.get("endpointRecipe").is_none());
    }
}
#[test]
fn signed_arc_connections_and_centerline_queries() {
    for config in [
        "sweep_angle=90deg",
        "sweep_angle=-90deg",
        "sweep_angle=270deg",
        "sweep_angle=-270deg",
        "radius=30mm",
        "radius=-30mm",
        "radius=20mm",
        "radius=-20mm",
    ] {
        let s = scene(&format!(
            "a=page.add(arrow.arc({config}),start=(20,40),end=(60,40))\npage.add(rect(size=(1,1)),target=a.centerline.start)\npage.add(rect(size=(1,1)),target=a.centerline.end)\npage.add(rect(size=(1,1)),target=a.centerline.at(fraction=0.5))"
        ));
        close(point(&s.nodes[0], 0), Point::new(20., 40.));
        close(point(&s.nodes[0], 1), Point::new(60., 40.));
        for (i, p) in [(1, Point::new(20., 40.)), (2, Point::new(60., 40.))] {
            close(
                Point::new(jnum(&s.nodes[i], "x", 0.), jnum(&s.nodes[i], "y", 0.)),
                p,
            );
        }
        let y = jnum(&s.nodes[3], "y", 0.);
        assert!(
            if config.contains('-') {
                y > 40.
            } else {
                y < 40.
            },
            "{config}: {y}"
        );
    }
}
#[test]
fn widths_heads_styles_and_connections_do_not_mutate_materials() {
    let s = scene(
        r##"
 style { arrow.route { fill:#123456; border-color:#ffffff; } }
 a=arrow(length=50,shaft_width=[(0,2mm),(0.4,6mm),(1,3mm)],heads=both,start_head_size=(7,8),end_head_size=(10,12),class="route")
 x=page.add(a,start=(20,20),end=(90,30),start_offset=(2,0),offset=(1,2),opacity=0.5)
 page.add(a,anchor=start,offset=(10,70))
 page.add(rect(size=(1,1)),target=x.start)
 page.add(rect(size=(1,1)),target=x.centerline.end)
 "##,
    );
    close(point(&s.nodes[0], 0), Point::new(23., 22.));
    close(point(&s.nodes[0], 1), Point::new(91., 32.));
    close(point(&s.nodes[1], 0), Point::new(10., 70.));
    close(point(&s.nodes[1], 1), Point::new(60., 70.));
    assert_eq!(s.nodes[0]["fill"], "#123456");
    assert_eq!(s.nodes[0]["strokeStyle"]["color"], "#ffffff");
    assert_eq!(
        s.nodes[0]["geometryRecipe"]["widths"],
        serde_json::json!([[0., 2.], [0.4, 6.], [1., 3.]])
    );
    assert_eq!(s.nodes[0]["opacity"], 0.5);
}
#[test]
fn custom_centerline_preserves_source_segments_and_transforms() {
    let s = scene(
        "p=path(commands=[move_to(0,0),quad_to(20,0,30,20),cubic_to(40,40,60,10,80,20)])\na=page.add(arrow.path(path=p,shaft_width=(2,5)),start=(15,25),end=(135,75))\npage.add(rect(size=(1,1)),target=a.centerline.segments[0].controls[0])\npage.add(rect(size=(1,1)),target=a.centerline.end)",
    );
    close(point(&s.nodes[0], 0), Point::new(15., 25.));
    close(point(&s.nodes[0], 1), Point::new(135., 75.));
    assert_eq!(
        s.nodes[0]["geometryRecipe"]["centerline"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert_eq!(
        s.nodes[0]["geometryRecipe"]["widths"],
        serde_json::json!([[0., 2.], [1., 5.]])
    );
}
#[test]
fn impossible_geometry_has_located_errors() {
    for code in [
        "page.add(arrow())",
        "page.add(arrow.arc(radius=10),start=(0,0),end=(50,0))",
        "page.add(arrow.arc(radius=20,sweep_angle=90deg),start=(0,0),end=(50,0))",
        "arrow.arc(sweep_angle=0deg)",
        "arrow.arc(sweep_angle=360deg)",
        "arrow(shaft_width=0)",
        "arrow(shaft_width=[(0,2),(0.5,4),(0.4,3),(1,2)])",
        "arrow(shaft_width=[(0mm,2),(1,3)])",
        "arrow.chevron(heads=both)",
        "page.add(arrow(length=5,head_size=(10,8)))",
        "page.add(arrow.arc(radius=10,sweep_angle=270deg,shaft_width=24,head_size=(5,26)))",
        "page.add(arrow.path(path=path(commands=[move_to(0,0),line_to(30,0),close()])))",
        "page.add(arrow.path(path=path(commands=[move_to(0,0),line_to(40,40),line_to(0,40),line_to(40,0)])))",
    ] {
        let error = compile_source(
            &format!("page=canvas(size=(200,160))\n{code}"),
            "/bad.lay",
            Host::default(),
        )
        .expect_err(code);
        assert_eq!(error.loc.line, 2, "{code}: {error}");
    }
}
#[test]
fn group_replay_and_scaled_endpoint_aliases_follow_centerline() {
    let s = scene(
        "g=group()\na=g.add(rect(size=(10,10)),offset=(5,5))\nb=g.add(rect(size=(10,10)),offset=(60,30))\nc=g.add(arrow.arc(sweep_angle=90deg),start=a.middle_right,end=b.middle_left)\ng.add(rect(size=(1,1)),target=c.centerline.end)\npage.add(g,size=(140,60),rotation=15deg)\npage.add(g,offset=(0,80))\nx=page.add(arrow(length=40),size=(80,20),offset=(100,110))\npage.add(rect(size=(1,1)),target=x.end)\npage.add(rect(size=(1,1)),target=x.centerline.end)",
    );
    for g in &s.nodes[..2] {
        let ns = g["children"].as_array().unwrap();
        let expected = Point::new(jnum(&ns[1], "x", 0.), jnum(&ns[1], "y", 0.) + 5.);
        close(point(&ns[2], 1), expected);
        close(
            Point::new(jnum(&ns[3], "x", 0.), jnum(&ns[3], "y", 0.)),
            expected,
        );
    }
    close(
        Point::new(jnum(&s.nodes[3], "x", 0.), jnum(&s.nodes[3], "y", 0.)),
        Point::new(jnum(&s.nodes[4], "x", 0.), jnum(&s.nodes[4], "y", 0.)),
    );
}
#[test]
fn retained_circle_offsets_obey_final_physical_tolerance() {
    let s =
        scene("page.add(arrow.arc(radius=30,sweep_angle=180deg,shaft_width=4,head_size=(6,8)))");
    let n = &s.nodes[0];
    let tr = Affine::rotate(0.4) * Affine::scale_non_uniform(200., 80.);
    let p = resolved_path(n, tr);
    let o = &n["geometryRecipe"]["origin"];
    let origin = Point::new(o[0].as_f64().unwrap(), o[1].as_f64().unwrap());
    use kurbo::ParamCurveNearest;
    for radius in [28., 32.] {
        for i in 0..101 {
            let a = 0.2 + i as f64 / 100. * 2.3;
            let q = tr
                * Point::new(
                    30. + radius * a.cos() - origin.x,
                    30. + radius * a.sin() - origin.y,
                );
            let d = p
                .segments()
                .map(|s| s.nearest(q, 1e-10).distance_sq.sqrt())
                .fold(f64::INFINITY, f64::min);
            assert!(d < 0.001, "boundary error {d}");
        }
    }
}
#[test]
fn only_identifiable_legacy_calls_migrate() {
    let new = "arrow(length=40)\narrow.arc(radius=-30)\narrow(shaft_width=2)";
    assert_eq!(laymesh_core::migration::migrate_arrows(new), new);
    assert!(
        laymesh_core::migration::migrate_arrows("arrow(dx=40,dy=0,line_color=\"red\")")
            .starts_with("line(")
    );
    let s = scene("function arrow(foo) { return rect(size=(foo,foo)) }\npage.add(arrow(12))");
    assert_eq!(s.nodes[0]["width"], 12.);
}

#[test]
fn measured_shaft_width_interpolates_after_excluding_heads() {
    let s = scene("page.add(arrow(length=80,shaft_width=[(0,2),(0.5,6),(1,4)],head_size=(12,10)))");
    let n = &s.nodes[0];
    let o = &n["geometryRecipe"]["origin"];
    let p = Affine::translate((o[0].as_f64().unwrap(), o[1].as_f64().unwrap()))
        * BezPath::from_svg(n["d"].as_str().unwrap()).unwrap();
    for (x, w) in [(17., 4.), (34., 6.), (51., 5.)] {
        assert_ne!(p.winding(Point::new(x, w / 2. - 0.01)), 0);
        assert_eq!(p.winding(Point::new(x, w / 2. + 0.01)), 0);
        assert_ne!(p.winding(Point::new(x, -w / 2. + 0.01)), 0);
        assert_eq!(p.winding(Point::new(x, -w / 2. - 0.01)), 0);
    }
}
#[test]
fn sharp_templates_and_css_profiles_have_valid_silhouettes() {
    for expr in [
        "arrow.bent(corner_radius=0)",
        "arrow.uturn(corner_radius=0,heads=both)",
        "arrow.path(path=polyline(points=[(0,40),(0,0),(50,0)]))",
    ] {
        scene(&format!("page.add({expr})"));
    }
    let s = scene(
        "style { arrow.curve { shaft-width:[(0,2mm),(0.5,5mm),(1,3mm)];head-size:8mm 10mm;fill:#123456; } }\na=arrow.arc(radius=30,sweep_angle=180deg,class=\"curve\")\npage.add(a)\npage.add(a,shaft_width=4mm)",
    );
    assert_eq!(s.nodes[0]["fill"], "#123456");
    assert_eq!(
        s.nodes[0]["geometryRecipe"]["widths"],
        serde_json::json!([[0., 2.], [0.5, 5.], [1., 3.]])
    );
    assert_eq!(
        s.nodes[1]["geometryRecipe"]["widths"],
        serde_json::json!([[0., 4.], [1., 4.]])
    );
}
#[test]
fn centerline_lengths_use_source_geometry_and_query_space() {
    let s = scene(
        "a=page.add(arrow.arc(radius=30,sweep_angle=180deg),size=(80,40))\nb=page.add(rect(size=(a.centerline.in_space(space=\"local\").length,1)))\npage.add(rect(size=(a.centerline.length,1)),offset=(0,10))",
    );
    let expected = 30. * (std::f64::consts::PI - (6_f64 / 30.).atan()) + 6.;
    assert!((jnum(&s.nodes[1], "width", 0.) - expected).abs() < 1e-6);
    assert!(jnum(&s.nodes[2], "width", 0.) > jnum(&s.nodes[1], "width", 0.));
}

fn segment_ends(s: &Value) -> (Point, Point, kurbo::Vec2, kurbo::Vec2) {
    let tr = Affine::new(std::array::from_fn(|i| s["transform"][i].as_f64().unwrap()));
    let point = |p: &Value| Point::new(p[0].as_f64().unwrap(), p[1].as_f64().unwrap());
    if let Some(ps) = s["points"].as_array() {
        let a = tr * point(&ps[0]);
        let b = tr * point(ps.last().unwrap());
        (
            a,
            b,
            (tr * point(&ps[1]) - a).normalize(),
            (b - tr * point(&ps[ps.len() - 2])).normalize(),
        )
    } else {
        let c = point(&s["center"]);
        let r = s["radii"][0].as_f64().unwrap();
        let start = s["start"].as_f64().unwrap();
        let sweep = s["sweep"].as_f64().unwrap();
        let at = |a: f64| c + kurbo::Vec2::new(a.cos(), a.sin()) * r;
        let tangent = |a: f64| {
            let p = at(a);
            (tr * (p + kurbo::Vec2::new(-a.sin(), a.cos()) * sweep.signum()) - tr * p).normalize()
        };
        (
            tr * at(start),
            tr * at(start + sweep),
            tangent(start),
            tangent(start + sweep),
        )
    }
}
fn assert_centered_heads(n: &Value) {
    let recipe = &n["geometryRecipe"];
    assert_eq!(recipe["appendedHeads"], true);
    let segments = recipe["centerline"].as_array().unwrap();
    let origin = &recipe["origin"];
    let p = Affine::translate((origin[0].as_f64().unwrap(), origin[1].as_f64().unwrap()))
        * BezPath::from_svg(n["d"].as_str().unwrap()).unwrap();
    let vertices: Vec<_> = p
        .elements()
        .iter()
        .filter_map(|el| match el {
            kurbo::PathEl::MoveTo(p) | kurbo::PathEl::LineTo(p) => Some(*p),
            _ => None,
        })
        .collect();
    for start in [true, false] {
        let h = &recipe[if start { "startHead" } else { "endHead" }];
        let length = h[0].as_f64().unwrap();
        if length == 0. {
            continue;
        }
        let (a, b, t0, t1) = segment_ends(&segments[if start { 0 } else { segments.len() - 1 }]);
        let (neck, tip, axis, tangent, width) = if start {
            let body = segment_ends(&segments[1]);
            close(b, body.0);
            (b, a, t0, body.2, recipe["widths"][0][1].as_f64().unwrap())
        } else {
            let body = segment_ends(&segments[segments.len() - 2]);
            close(a, body.1);
            (
                a,
                b,
                t1,
                body.3,
                recipe["widths"].as_array().unwrap().last().unwrap()[1]
                    .as_f64()
                    .unwrap(),
            )
        };
        assert!((tip.distance(neck) - length).abs() < 1e-8);
        assert!((axis - tangent).hypot() < 1e-8);
        let normal = kurbo::Vec2::new(-axis.y, axis.x);
        // Both shaft corners and both head shoulders are centered on the same neck.
        for distance in [
            -width / 2.,
            width / 2.,
            -h[1].as_f64().unwrap() / 2.,
            h[1].as_f64().unwrap() / 2.,
        ] {
            let expected = neck + normal * distance;
            assert!(
                vertices.iter().any(|p| p.distance(expected) < 1e-7),
                "missing neck/shoulder {expected:?}"
            );
        }
        assert!(vertices.iter().any(|p| p.distance(tip) < 1e-7));
    }
}
#[test]
fn curved_heads_are_centered_on_necks_and_follow_body_tangents() {
    for heads in ["end", "start", "both"] {
        for sweep in [90, -90, 250, -250] {
            let s = scene(&format!(
                "page.add(arrow.arc(radius=15,start_angle=160deg,sweep_angle={sweep}deg,shaft_width=(4,3),heads={heads},start_head_size=(7,9),end_head_size=(8,10)))"
            ));
            assert_centered_heads(&s.nodes[0]);
        }
        let s = scene(&format!(
            "curve=path(commands=[move_to(0,20),cubic_to(18,20,10,0,28,0),cubic_to(44,0,36,20,54,20)])\npage.add(arrow.path(path=curve,shaft_width=[(0,2),(0.5,6),(1,3)],heads={heads},head_size=(8,10)))"
        ));
        assert_centered_heads(&s.nodes[0]);
        let segments = s.nodes[0]["geometryRecipe"]["centerline"]
            .as_array()
            .unwrap();
        let i = usize::from(heads != "end");
        let first = segment_ends(&segments[i]);
        let last = segment_ends(&segments[i + 1]);
        close(first.0, Point::new(0., 20.));
        close(last.1, Point::new(54., 20.));
        assert_eq!(
            segments[i]["points"],
            serde_json::json!([[0., 20.], [18., 20.], [10., 0.], [28., 0.]])
        );
        assert_eq!(
            segments[i + 1]["points"],
            serde_json::json!([[28., 0.], [44., 0.], [36., 20.], [54., 20.]])
        );
    }
}
#[test]
fn total_arc_sweep_includes_unequal_tangent_heads() {
    for heads in ["end", "start", "both"] {
        for sweep in [120_f64, -120., 290., -290.] {
            let s = scene(&format!(
                "a=page.add(arrow.arc(radius=30,start_angle=25deg,sweep_angle={sweep}deg,heads={heads},start_head_size=(7,9),end_head_size=(11,12)))\npage.add(rect(size=(a.centerline.length,1)))"
            ));
            let n = &s.nodes[0];
            let segments = n["geometryRecipe"]["centerline"].as_array().unwrap();
            let h0 = if heads == "end" { 0_f64 } else { 7. };
            let h1 = if heads == "start" { 0_f64 } else { 11. };
            let body = &segments[usize::from(h0 > 0.)];
            let beta = sweep.abs().to_radians() - (h0 / 30.).atan() - (h1 / 30.).atan();
            assert!((body["sweep"].as_f64().unwrap() - sweep.signum() * beta).abs() < 1e-10);
            let c = Point::new(30., 30.);
            let start = 25_f64.to_radians();
            close(
                segment_ends(&segments[0]).0,
                c + kurbo::Vec2::new(start.cos(), start.sin()) * 30_f64.hypot(h0),
            );
            let end = start + sweep.to_radians();
            close(
                segment_ends(segments.last().unwrap()).1,
                c + kurbo::Vec2::new(end.cos(), end.sin()) * 30_f64.hypot(h1),
            );
            assert!((jnum(&s.nodes[1], "width", 0.) - (30. * beta + h0 + h1)).abs() < 1e-6);
        }
    }
}
#[test]
fn arc_connections_solve_head_aware_chords_and_new_semicircle_boundary() {
    let radius = 20_f64;
    let heads = [7_f64, 11_f64];
    let a = radius.hypot(heads[0]);
    let b = radius.hypot(heads[1]);
    for sweep in [90_f64, -90., 180., -180., 270., -270.] {
        let distance = (a - b).hypot(2. * (a * b).sqrt() * (sweep.to_radians() / 2.).sin());
        for config in [
            format!("sweep_angle={sweep}deg"),
            format!("radius=20,sweep_angle={sweep}deg"),
        ] {
            let s = scene(&format!(
                "page.add(arrow.arc({config},heads=both,start_head_size=(7,9),end_head_size=(11,12)),start=(12,25),end=({},25))",
                12. + distance
            ));
            let n = &s.nodes[0];
            close(point(n, 0), Point::new(12., 25.));
            close(point(n, 1), Point::new(12. + distance, 25.));
            assert!(
                (n["geometryRecipe"]["centerline"][1]["radii"][0]
                    .as_f64()
                    .unwrap()
                    - radius)
                    .abs()
                    < 1e-6
            );
            assert_centered_heads(n);
        }
    }
    for sign in [1., -1.] {
        let d = a + b;
        let s = scene(&format!(
            "page.add(arrow.arc(radius={},heads=both,start_head_size=(7,9),end_head_size=(11,12)),start=(0,0),end=({d},0))",
            sign * radius
        ));
        close(point(&s.nodes[0], 1), Point::new(d, 0.));
        let actual = s.nodes[0]["geometryRecipe"]["centerline"][1]["sweep"]
            .as_f64()
            .unwrap();
        assert!(
            (actual
                - sign
                    * (std::f64::consts::PI
                        - (heads[0] / radius).atan()
                        - (heads[1] / radius).atan()))
            .abs()
                < 1e-7
        );
        let bad = format!(
            "page=canvas(size=(200,100))\npage.add(arrow.arc(radius={},heads=both,start_head_size=(7,9),end_head_size=(11,12)),start=(0,0),end=({},0))",
            sign * radius,
            d + 0.01
        );
        assert!(compile_source(&bad, "/bad.lay", Host::default()).is_err());
    }
}
#[test]
fn connected_custom_paths_keep_physical_heads_and_larger_positive_scale() {
    for heads in ["start", "end", "both"] {
        let s = scene(&format!(
            "p=path(commands=[move_to(0,0),quad_to(0,20,30,20)])\na=arrow.path(path=p,heads={heads},shaft_width=(1,2),start_head_size=(7,9),end_head_size=(11,12))\npage.add(a,start=(15,20),end=(120,65))\npage.add(a,anchor=start,offset=(5,100))"
        ));
        close(point(&s.nodes[0], 0), Point::new(15., 20.));
        close(point(&s.nodes[0], 1), Point::new(120., 65.));
        close(point(&s.nodes[1], 0), Point::new(5., 100.));
        for n in &s.nodes {
            assert_centered_heads(n);
        }
    }
    // The endpoint equation has two positive scales. Both point backwards at
    // the head; choose the larger body rather than silently folding it smaller.
    let s = scene(
        "p=path(commands=[move_to(0,0),cubic_to(-10,0,40,10,30,10)])\npage.add(arrow.path(path=p,shaft_width=0.01,head_size=(10,0.04)),start=(0,0),end=(5,0))",
    );
    let n = &s.nodes[0];
    close(point(n, 1), Point::new(5., 0.));
    assert_centered_heads(n);
    let body = &n["geometryRecipe"]["centerline"][0];
    let tr = Affine::new(std::array::from_fn(|i| {
        body["transform"][i].as_f64().unwrap()
    }));
    let expected = (300_f64 / 1000_f64.sqrt() + (25_f64 - 10.).sqrt()) / 1000_f64.sqrt();
    assert!(((tr * Point::new(1., 0.)).distance(tr * Point::ORIGIN) - expected).abs() < 1e-8);
}
#[test]
fn appended_heads_reject_impossible_body_constraints() {
    for call in [
        "page.add(arrow.arc(radius=10,sweep_angle=10deg,head_size=(8,10)))",
        "page.add(arrow.arc(sweep_angle=90deg,head_size=(10,12)),start=(0,0),end=(5,0))",
        "page.add(arrow.arc(sweep_angle=350deg,shaft_width=0.1,head_size=(8,0.2)),start=(0,0),end=(1,0))",
        "page.add(arrow.arc(radius=20,sweep_angle=180deg),start=(0,0),end=(40,0))",
        "page.add(arrow.path(path=line(length=20),head_size=(10,12)),start=(0,0),end=(8,0))",
        "page.add(arrow.path(path=path(commands=[move_to(0,0),quad_to(0,20,30,20)]),heads=both,head_size=(20,22)),start=(0,0),end=(1,0))",
    ] {
        let error = compile_source(
            &format!("page=canvas(size=(200,160))\n{call}"),
            "/bad.lay",
            Host::default(),
        )
        .expect_err(call);
        assert_eq!(error.code, "E_ARROW");
        assert_eq!(error.loc.line, 2);
    }
}
#[test]
fn major_arc_solver_selects_larger_radius_branch() {
    let s = scene(
        "page.add(arrow.arc(sweep_angle=350deg,shaft_width=0.1,head_size=(8,0.2)),start=(0,0),end=(6,0))",
    );
    let n = &s.nodes[0];
    close(point(n, 1), Point::new(6., 0.));
    assert_centered_heads(n);
    let sine = 350_f64.to_radians().sin();
    let u = 36. - sine * sine * 64.;
    let expected =
        ((u + (u * u - sine * sine * (36_f64 - 64.).powi(2)).sqrt()) / (2. * sine * sine)).sqrt();
    assert!(
        (n["geometryRecipe"]["centerline"][0]["radii"][0]
            .as_f64()
            .unwrap()
            - expected)
            .abs()
            < 1e-7
    );
}
