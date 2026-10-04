use laymesh_core::{
    engine::compile_source,
    model::{Host, jnum},
};
use serde_json::Value;
fn compile(s: &str) -> Value {
    let scene = compile_source(s, "/queries.lay", host()).unwrap();
    serde_json::to_value(scene).unwrap()
}
fn host() -> Host {
    let mut host = Host::default();
    host.files.insert(
        "/font.ttf".into(),
        include_bytes!("../../../tests/fonts/DejaVuSans.ttf").to_vec(),
    );
    host
}
fn xy(n: &Value) -> [f64; 2] {
    [jnum(n, "x", 0.), jnum(n, "y", 0.)]
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 0.0001, "{a} != {b}");
}
fn marker(s: &str) -> String {
    format!("page=canvas(size=(120mm,100mm))\ndot=rect(size=(1mm,1mm))\n{s}")
}
#[test]
fn rounded_rectangle_exposes_box_path_and_ink_separately() {
    let d = compile(&marker(
        r##"a=page.add(rect(size=(72mm,45mm),border_radius=10mm,fill="#fff",border_color="#000",border_width=2mm),offset=(20mm,20mm))
page.add(dot,target=a.bounds.top_left)
page.add(dot,target=a.path.nearest(to=a.bounds.top_left)[0])
page.add(dot,target=a.ink.bounds.top_left)
"##,
    ));
    let ns = d["nodes"].as_array().unwrap();
    close(xy(&ns[1])[0], 20.);
    close(xy(&ns[2])[0], 30. - 10. / 2f64.sqrt());
    close(xy(&ns[2])[1], 30. - 10. / 2f64.sqrt());
    close(xy(&ns[3])[0], 19.);
}
#[test]
fn source_path_anchor_and_target_frame_offset() {
    let d = compile(&marker(
        r##"a=page.add(line(dx=20mm,dy=0mm),offset=(20mm,20mm),rotation=90deg)
page.add(line(dx=5mm,dy=0mm),anchor=self.path.end,target=a.path.at(fraction=0.5),rotation=a.path.at(fraction=0.5).tangent_angle,offset=(2mm,3mm),offset_space="target")
"##,
    ));
    let b = &d["nodes"][1];
    let p = &b["endpoints"][1];
    let t = laymesh_core::geometry::node_transform(b)
        * kurbo::Point::new(p[0].as_f64().unwrap(), p[1].as_f64().unwrap());
    close(t.x, 23.);
    close(t.y, 32.);
}
#[test]
fn arclength_parameter_nodes_and_control_points_differ() {
    let d = compile(&marker(
        r##"a=page.add(path(commands=[move_to(0mm,0mm),cubic_to(0mm,0mm,0mm,0mm,80mm,0mm)],border_color="#000"))
page.add(dot,target=a.path.at(fraction=0.5))
page.add(dot,target=a.path.segments[0].at(t=0.5))
page.add(dot,target=a.path.nodes[1])
page.add(dot,target=a.path.segments[0].controls[0])
"##,
    ));
    close(xy(&d["nodes"][1])[0], 40.);
    close(xy(&d["nodes"][2])[0], 10.);
    close(xy(&d["nodes"][3])[0], 80.);
    close(xy(&d["nodes"][4])[0], 0.);
}
#[test]
fn parent_and_local_lengths_differ_after_nonuniform_resize() {
    let d = compile(&marker(
        r##"a=page.add(polyline(points=[(0mm,0mm),(10mm,0mm),(10mm,10mm)]),size=(40mm,10mm))
page.add(dot,target=a.path.at(fraction=0.5))
page.add(dot,target=a.path.in_space("local").at(fraction=0.5))
"##,
    ));
    close(xy(&d["nodes"][1])[0], 25.);
    close(xy(&d["nodes"][2])[0], 40.);
}
#[test]
fn searches_always_require_index_even_when_single() {
    for query in [
        "a.path.nearest(to=a.bounds.top_left)",
        "a.path.extrema(axis=\"y\")",
    ] {
        let s = marker(&format!(
            "a=page.add(ellipse(size=(30mm,20mm)))\npage.add(dot,target={query})"
        ));
        let e = compile_source(&s, "/q.lay", Host::default()).unwrap_err();
        assert_eq!(e.code, "E_ANCHOR");
        assert!(e.message.contains("[0]"));
    }
}
#[test]
fn features_intersections_and_closed_paths_are_ordered() {
    let d = compile(&marker(
        r##"a=page.add(path(commands=[move_to(0mm,20mm),cubic_to(20mm,-20mm,40mm,60mm,60mm,20mm)],border_color="#000"))
page.add(dot,target=a.path.extrema(axis="y")[0])
page.add(dot,target=a.path.extrema(axis="y")[1])
page.add(dot,target=a.path.inflections()[0])
page.add(dot,target=a.path.intersections(ray(origin=(0mm,20mm),direction=(1,0)))[0])
"##,
    ));
    assert!(xy(&d["nodes"][1])[0] < xy(&d["nodes"][2])[0]);
    close(xy(&d["nodes"][3])[0], 30.);
}
#[test]
fn between_nodes_and_corner_directions() {
    let d = compile(&marker(
        r##"a=page.add(polyline(points=[(0mm,0mm),(20mm,0mm),(20mm,20mm),(40mm,20mm)]))
r=a.path
page.add(dot,target=r.between(r.nodes[1],r.nodes[3]).at(fraction=0.5))
page.add(dot,target=r.corners()[0].with_side("incoming"),offset=(2mm,3mm),offset_space="target")
"##,
    ));
    close(xy(&d["nodes"][1])[0], 20.);
    close(xy(&d["nodes"][1])[1], 20.);
    close(xy(&d["nodes"][2])[0], 22.);
    close(xy(&d["nodes"][2])[1], -3.);
}
#[test]
fn composite_paths_require_explicit_route_and_infinite_solutions_fail() {
    for (shape, query, code) in [
        (
            "path(commands=[move_to(0,0),line_to(10,0),move_to(20,0),line_to(30,0)])",
            "a.path.at(fraction=0.5)",
            "E_PATH",
        ),
        (
            "ellipse(size=(20,20))",
            "a.path.nearest(to=a.bounds.center)[0]",
            "E_GEOMETRY",
        ),
        (
            "line(dx=20,dy=0)",
            "a.path.intersections(ray(origin=(0,0),direction=(1,0)))[0]",
            "E_GEOMETRY",
        ),
        (
            "ellipse(size=(30,20))",
            "a.path.inflections()[0]",
            "E_INDEX",
        ),
    ] {
        let e = compile_source(
            &marker(&format!(
                "a=page.add({shape})\npage.add(dot,target={query})"
            )),
            "/q.lay",
            Host::default(),
        )
        .unwrap_err();
        assert_eq!(e.code, code, "{e:?}");
    }
}
#[test]
fn chart_parts_data_and_axis_minimum_obey_mapping() {
    let d = compile(&marker(
        r##"p=plot(size=(100mm,80mm),plot_area=box(offset=(20mm,10mm),size=(60mm,50mm)),x=axis(range=(0,10),reverse=true,label="x"),y=axis(range=(0,100),label="y"))
p.line(x=[0,10],y=[0,100])
a=page.add(p,offset=(5mm,5mm))
page.add(dot,target=a.plot_area.bounds.top_left)
page.add(dot,target=a.axes["x"].min)
page.add(dot,target=a.axes["x"].spine.path.at(fraction=0.5))
page.add(dot,target=a.axes["x"].label.bounds.center)
page.add(dot,target=a.data(x=5,y=50))
"##,
    ));
    close(xy(&d["nodes"][1])[0], 25.);
    close(xy(&d["nodes"][2])[0], 85.);
    close(xy(&d["nodes"][3])[0], 55.);
    close(xy(&d["nodes"][5])[0], 55.);
    close(xy(&d["nodes"][5])[1], 40.);
}

#[test]
fn coincident_instances_cannot_supply_between_endpoints_or_fuse_points() {
    for tail in [
        "page.add(dot,target=a.path.between(c.path.start,b.path.end).at(fraction=0.5))",
        "page.fuse(a,b,points=(c.path.end,b.path.start),bridge_width=2)",
    ] {
        let source = marker(&format!(
            "a=page.add(line(dx=10,dy=0))\nb=page.add(line(dx=10,dy=0),offset=(20,0))\nc=page.add(line(dx=10,dy=0))\n{tail}"
        ));
        assert!(compile_source(&source, "/q.lay", host()).is_err());
    }
}

#[test]
fn closed_ranges_require_wrap_and_keep_traversal_direction() {
    let source = marker(
        "a=page.add(rect(size=(20,10)))\nr=a.path\npage.add(dot,target=r.between(r.nodes[3],r.nodes[1],wrap=true).at(fraction=0.5))\npage.add(dot,target=r.start)\npage.add(dot,target=r.end)",
    );
    let d = compile(&source);
    close(xy(&d["nodes"][1])[0], 5.);
    close(xy(&d["nodes"][1])[1], 0.);
    assert_eq!(xy(&d["nodes"][2]), xy(&d["nodes"][3]));
    assert!(compile_source(&source.replace(",wrap=true", ""), "/q.lay", host()).is_err());
}

#[test]
fn off_path_controls_do_not_claim_a_tangent_and_corners_require_a_side() {
    for query in [
        "a.path.nodes[1].tangent_angle",
        "a.path.corners()[0].tangent_angle",
    ] {
        let source = marker(&format!(
            "a=page.add(polyline(points=[(0,0),(20,0),(20,20)]))\nangle={query}"
        ));
        let e = compile_source(&source, "/q.lay", host()).unwrap_err();
        assert_eq!(e.code, "E_ANCHOR_DIRECTION");
    }
    let d = compile(&marker(
        "a=page.add(polyline(points=[(0,0),(20,0),(20,20)]))\npage.add(dot,target=a.path.nodes[1].with_side(\"outgoing\"),offset=(2,3),offset_space=\"target\")",
    ));
    close(xy(&d["nodes"][1])[0], 23.);
    close(xy(&d["nodes"][1])[1], 2.);
    let source = marker(
        "a=page.add(path(commands=[move_to(0,0),quad_to(10,10,20,0)]))\nangle=a.path.segments[0].controls[0].tangent_angle",
    );
    assert_eq!(
        compile_source(&source, "/q.lay", host()).unwrap_err().code,
        "E_ANCHOR_DIRECTION"
    );
}

#[test]
fn self_is_bound_after_resize_and_rotation_and_never_in_target() {
    let d = compile(&marker(
        "page.add(polyline(points=[(0,0),(10,0),(10,10)]),size=(40,10),anchor=self.path.at(fraction=0.5),offset=(50,30))",
    ));
    close(xy(&d["nodes"][0])[0], 25.);
    close(xy(&d["nodes"][0])[1], 30.);
    for tail in [
        "page.add(dot,target=self.path.start)",
        "page.add(dot,anchor=self.path.nearest(to=page.center)[0])",
    ] {
        assert_eq!(
            compile_source(&marker(tail), "/q.lay", host())
                .unwrap_err()
                .code,
            "E_ANCHOR"
        );
    }
}

#[test]
fn source_arc_indices_do_not_depend_on_rendering_subdivision() {
    let d = compile(&marker(
        "a=page.add(path(commands=[move_to(0,0),arc_to(rx=10,ry=10,x=20,y=0,sweep=true)]))\ncount=len(a.path.segments)\npage.add(dot,offset=(count,0))\npage.add(dot,target=a.path.nodes[1])\npage.add(dot,target=a.path.segments[0].at(t=0.5))",
    ));
    close(xy(&d["nodes"][1])[0], 1.);
    close(xy(&d["nodes"][2])[0], 20.);
    close(xy(&d["nodes"][3])[0], 10.);
}

#[test]
fn ellipse_nearest_ties_and_self_intersection_occurrences_are_retained() {
    let d = compile(&marker(
        "a=page.add(ellipse(size=(30,20)))\nfor p in a.path.nearest(to=a.center) { page.add(dot,target=p) }\npage.add(dot,offset=(len(a.path.nearest(to=a.center)),0))",
    ));
    close(xy(&d["nodes"][1])[1], 20.);
    close(xy(&d["nodes"][2])[1], 0.);
    close(xy(&d["nodes"][3])[0], 2.);
    let d = compile(&marker(
        "a=page.add(polyline(points=[(0,0),(20,20),(0,20),(20,0)]))\nfor p in a.path.nearest(to=(10,10)) {page.add(dot,target=p)}",
    ));
    assert_eq!(d["nodes"].as_array().unwrap().len(), 3);
    close(xy(&d["nodes"][1])[0], 10.);
    assert_eq!(xy(&d["nodes"][1]), xy(&d["nodes"][2]));
}

#[test]
fn line_and_arrow_centerlines_differ_from_caps_dashes_and_heads() {
    let d = compile(&marker(
        "a=page.add(line(dx=20,dy=0,line_width=4mm,line_cap=\"round\"),offset=(10,10))\npage.add(dot,target=a.path.bounds.top_left)\npage.add(dot,target=a.ink.bounds.top_left)\nb=page.add(line(end_head=head(),dx=20,dy=0,line_width=1mm),offset=(10,30))\npage.add(dot,target=b.path.end)\npage.add(dot,target=b.ink.bounds.bottom_center)\nc=page.add(line(dx=20,dy=0,line_width=2mm,line_dash=[4mm,4mm]))\npage.add(dot,target=c.ink.bounds.bottom_right)",
    ));
    close(xy(&d["nodes"][1])[0], 10.);
    close(xy(&d["nodes"][2])[0], 8.);
    close(xy(&d["nodes"][2])[1], 8.);
    close(xy(&d["nodes"][4])[0], 30.);
    assert!(xy(&d["nodes"][5])[1] > 30.);
    close(xy(&d["nodes"][7])[0], 20.);
}

#[test]
fn tight_rotated_ellipse_bounds_and_query_spaces() {
    let d = compile(&marker(
        "a=page.add(ellipse(size=(40,20)),rotation=45deg)\npage.add(dot,target=a.bounds.top_left)\npage.add(dot,target=a.path.bounds.top_left)\npage.add(dot,target=a.path.in_space(\"local\").bounds.top_left)",
    ));
    assert!(xy(&d["nodes"][2])[0] > xy(&d["nodes"][1])[0] + 1.);
    let d2 = compile(&marker(
        "a=page.add(rect(size=(20,20),border_radius=10))\npage.add(dot,target=a.path.nearest(to=a.bounds.top_left)[0])",
    ));
    close(xy(&d2["nodes"][1])[0], 10. - 10. / 2f64.sqrt());
}

#[test]
fn queries_reject_foreign_container_points_and_report_invalid_arguments() {
    for (tail, code) in [
        (
            "g=group()\na=g.add(line(dx=10,dy=0))\npage.add(dot,target=a.path.nearest(to=page.center)[0])",
            "E_LAYOUT",
        ),
        (
            "a=page.add(line(dx=10,dy=0))\nx=a.path.in_space(\"local\",extra=1)",
            "E_ARG",
        ),
        (
            "a=page.add(line(dx=10,dy=0))\nx=a.path.start.with_side(\"outgoing\",extra=1)",
            "E_ARG",
        ),
        ("a=page.add(line(dx=10,dy=0))\nx=a.path.at(t=0.5)", "E_PATH"),
        (
            "a=page.add(line(dx=10,dy=0))\nx=a.path.at(distance=11mm)",
            "E_PATH",
        ),
        (
            "a=page.add(line(dx=10,dy=0))\nx=a.path.intersections(ray(origin=(20,0),direction=(1,0)))\npage.add(dot,offset=(len(x),0))",
            "",
        ),
    ] {
        let result = compile_source(&marker(tail), "/q.lay", host());
        if code.is_empty() {
            assert!(result.is_ok(), "{result:?}");
        } else {
            assert_eq!(result.unwrap_err().code, code, "{tail}");
        }
    }
}

#[test]
fn path_ink_axis_and_data_dependencies_are_resolved_again_during_group_reflow() {
    let d = compile(
        r##"
style { group.grow { font-size:20pt; font-family:"/font.ttf"; } }
page=canvas(size=(120,100))
g=group()
p=plot(size=(90,70),font_family="/font.ttf",x=axis(range=(0,10),label="x"),y=axis(range=(0,10),label="y"))
p.line(x=[0,10],y=[0,10])
a=g.add(p)
g.add(rect(size=(1,1)),target=a.data(x=10,y=10))
g.add(rect(size=(1,1)),target=a.axis(name="x",anchor="center"))
g.add(rect(size=(1,1)),target=a.axes["x"].spine.path.at(fraction=0.5))
g.add(rect(size=(1,1)),target=a.axes["x"].label.bounds.center)
b=g.add(text("A",font_family="/font.ttf"),offset=(0,80))
g.add(rect(size=(1,1)),target=b.bounds.top_right)
parent=group(class="grow")
parent.add(g)
page.add(parent)
page.add(g)
"##,
    );
    let large = &d["nodes"][0]["children"][0]["children"];
    let small = &d["nodes"][1]["children"];
    let plot = &large[0];
    let area = &plot["plotBounds"];
    close(
        xy(&large[1])[0],
        jnum(area, "x", 0.) + jnum(area, "width", 0.),
    );
    close(xy(&large[1])[1], jnum(area, "y", 0.));
    for k in 0..2 {
        close(xy(&large[2])[k], xy(&large[3])[k]);
    }
    assert_ne!(xy(&large[1]), xy(&small[1]));
    close(xy(&large[6])[0], jnum(&large[5], "width", 0.));
}

#[test]
fn replay_preserves_new_fuse_and_boundary_query_identity() {
    let d = compile(
        r##"
style { group.grow { color:blue; } }
page=canvas(size=(80,60))
g=group()
a=g.add(rect(size=(10,10),fill="#ff0000"))
b=g.add(rect(size=(10,10),fill="#ff0000"),offset=(20,0))
f=g.fuse(a,b,points=(a.path.nearest(to=a.bounds.middle_right)[0],b.path.nearest(to=b.bounds.middle_left)[0]),bridge_width=2,fill="#ff0000")
g.add(rect(size=(1,1)),target=f.ink.boundary.nearest(to=f.bounds.top_right)[0])
parent=group(class="grow")
parent.add(g)
page.add(parent)
"##,
    );
    let ns = &d["nodes"][0]["children"][0]["children"];
    assert_eq!(ns.as_array().unwrap().len(), 2);
    close(xy(&ns[1])[0], 30.);
    close(xy(&ns[1])[1], 0.);
}

#[test]
fn replay_updates_direction_scalars_and_nested_measurement_arguments() {
    let d = compile(
        r##"
style { group.grow { font-size:20pt; font-family:"/font.ttf"; } }
page=canvas(size=(120,100))
g=group()
a=g.add(text("AAAA",font_family="/font.ttf"))
curve=g.add(polyline(points=[(0,0),(10,10)]),size=(a.bounds.width,20mm),offset=(0,20))
g.add(line(dx=3,dy=0),anchor=self.path.start,target=curve.path.at(fraction=0.5),rotation=curve.path.start.tangent_angle)
g.add(rect(size=(1,1)),target=curve.path.at(distance=a.bounds.width))
parent=group(class="grow")
parent.add(g)
page.add(parent)
page.add(g)
"##,
    );
    let large = &d["nodes"][0]["children"][0]["children"];
    let small = &d["nodes"][1]["children"];
    for ns in [large, small] {
        let w = jnum(&ns[1], "width", 0.);
        close(jnum(&ns[2], "rotation", 0.), 20f64.atan2(w).to_degrees());
        close(xy(&ns[3])[0], w * w / w.hypot(20.));
    }
    assert_ne!(large[2]["rotation"], small[2]["rotation"]);
    let d = compile(&marker(
        "page.add(line(dx=20,dy=0),anchor=self.path.at(distance=self.bounds.width),offset=(50,30))",
    ));
    close(xy(&d["nodes"][0])[0], 30.);
}

#[test]
fn ink_queries_keep_analytic_infinite_candidates_and_exclude_hidden_children() {
    for shape in [
        "ellipse(size=(20,20),fill=\"#ffffff\")",
        "ellipse(size=(20,20),border_color=\"#000000\",border_width=2mm)",
    ] {
        let source = marker(&format!(
            "a=page.add({shape})\npage.add(dot,target=a.ink.boundary.nearest(to=a.center)[0])"
        ));
        assert_eq!(
            compile_source(&source, "/q.lay", host()).unwrap_err().code,
            "E_GEOMETRY"
        );
    }
    let d = compile(&marker(
        "g=group()\ng.add(rect(size=(10,10),fill=\"#000000\"))\ng.add(rect(size=(10,10),fill=\"#000000\"),offset=(20,0),opacity=0)\na=page.add(g)\npage.add(dot,target=a.ink.bounds.bottom_right)",
    ));
    close(xy(&d["nodes"][1])[0], 10.);
}
