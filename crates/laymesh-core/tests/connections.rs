use kurbo::Point;
use laymesh_core::{
    engine::compile_source,
    geometry::node_transform,
    model::{Host, Scene, jnum},
};
use serde_json::Value;

fn host() -> Host {
    let mut host = Host::default();
    host.files.insert(
        "/font.ttf".into(),
        include_bytes!("../../../tests/fonts/DejaVuSans.ttf").to_vec(),
    );
    host
}
fn scene(source: &str) -> Scene {
    compile_source(
        &format!("page=canvas(size=(160,120))\n{source}"),
        "/connections.lay",
        host(),
    )
    .unwrap()
}
fn close(a: Point, b: Point) {
    assert!(a.distance(b) < 1e-7, "{a:?} != {b:?}");
}
fn endpoints(node: &Value) -> [Point; 2] {
    [0, 1].map(|i| {
        node_transform(node)
            * Point::new(
                node["endpoints"][i][0].as_f64().unwrap(),
                node["endpoints"][i][1].as_f64().unwrap(),
            )
    })
}
fn children(node: &Value) -> &[Value] {
    node["children"].as_array().unwrap()
}

#[test]
fn geometry_free_definitions_and_inline_connections_are_valid() {
    let s = scene(
        "unused=line()\nwire=line(line_width=0.5pt,end_head=head(shape=\"open\"))\na=page.add(wire,start=(10,20),end=(50,30))\npage.add(line(),start=(80,40),end=(60,10))\npage.add(rect(size=(1,1)),target=a.path.start)\npage.add(rect(size=(1,1)),target=a.end)",
    );
    let [start, end] = endpoints(&s.nodes[0]);
    close(start, Point::new(10., 20.));
    close(end, Point::new(50., 30.));
    close(endpoints(&s.nodes[1])[1], Point::new(60., 10.));
    close(
        Point::new(jnum(&s.nodes[2], "x", 0.), jnum(&s.nodes[2], "y", 0.)),
        start,
    );
    close(
        Point::new(jnum(&s.nodes[3], "x", 0.), jnum(&s.nodes[3], "y", 0.)),
        end,
    );
    assert_eq!(s.nodes[0]["endpointRecipe"]["end_head"]["shape"], "open");
}

#[test]
fn existing_material_geometry_and_style_are_not_mutated() {
    let s = scene(
        r##"
style { line.wire { line-color:#cc0000; line-width:1pt; } }
wire=line(length=20mm,angle=90deg,class="wire",end_head=head())
page.add(wire,start=(40,30),end=(10,10),line_color="#0000ff",opacity=0.4)
page.add(wire,anchor="start",offset=(5,6))
page.add(wire,start=(10,50),end=(90,50))
"##,
    );
    close(endpoints(&s.nodes[0])[0], Point::new(40., 30.));
    close(endpoints(&s.nodes[0])[1], Point::new(10., 10.));
    close(endpoints(&s.nodes[1])[0], Point::new(5., 6.));
    close(endpoints(&s.nodes[1])[1], Point::new(5., 26.));
    assert_eq!(s.nodes[0]["strokeStyle"]["color"], "#0000ff");
    assert_eq!(s.nodes[1]["strokeStyle"]["color"], "#cc0000");
    assert_eq!(s.nodes[0]["opacity"], 0.4);
}

#[test]
fn physical_offsets_units_and_reverse_vectors_are_exact() {
    let s = compile_source(
        r#"page=canvas(size=(16cm,12cm),unit="cm")
wire=line()
page.add(wire,start=(1,2),end=(5,2),start_offset=(2mm,1mm),end_offset=(-2mm,1mm),offset=(1mm,3mm))
page.add(wire,start=(8,7),end=(8,3))
page.add(wire,start=(-2mm,-3mm),end=(5mm,6mm))"#,
        "/units.lay",
        host(),
    )
    .unwrap();
    close(endpoints(&s.nodes[0])[0], Point::new(13., 24.));
    close(endpoints(&s.nodes[0])[1], Point::new(49., 24.));
    close(endpoints(&s.nodes[1])[0], Point::new(80., 70.));
    close(endpoints(&s.nodes[1])[1], Point::new(80., 30.));
    close(endpoints(&s.nodes[2])[0], Point::new(-2., -3.));
}

#[test]
fn both_endpoint_offsets_can_use_transformed_path_frames() {
    let s = scene(
        r#"
a=page.add(line(dx=20,dy=0),anchor="start",offset=(20,10),rotation=90deg)
page.add(line(),start=a.path.start,end=a.path.end,
 start_offset=(2,3),end_offset=(-2,-3),start_offset_space="target",end_offset_space="target",offset=(1,2))
"#,
    );
    close(endpoints(&s.nodes[1])[0], Point::new(24., 14.));
    close(endpoints(&s.nodes[1])[1], Point::new(18., 30.));
}

#[test]
fn box_data_and_geometry_anchors_can_be_mixed_after_chart_rotation() {
    let s = scene(
        r#"
p=plot(size=(80,60),plot_area=box(offset=(10,10),size=(60,40)),font_family="/font.ttf",x=axis(range=(0,10)),y=axis(range=(0,10)))
p.line(x=[0,10],y=[0,10])
chart=page.add(p,rotation=30deg,offset=(20,20))
a=page.add(rect(size=(8,6)),offset=(5,5))
page.add(line(),start=a.middle_right,end=chart.data(x=2,y=3),end_offset=(0,-2))
page.add(line(),start=page.bottom_left,end=a.path.nearest(to=(20,8))[0])
"#,
    );
    close(endpoints(&s.nodes[2])[0], Point::new(13., 8.));
    let expected = laymesh_core::plot::local_to_parent(&s.nodes[0], [22., 38.]);
    close(
        endpoints(&s.nodes[2])[1],
        Point::new(expected[0], expected[1] - 2.),
    );
    close(endpoints(&s.nodes[3])[0], Point::new(0., 120.));
    close(endpoints(&s.nodes[3])[1], Point::new(13., 8.));
}

#[test]
fn replay_rebinds_named_and_data_endpoints_after_inherited_layout_changes() {
    let s = scene(
        r##"
style { group.grow { font-size:20pt; font-family:"/font.ttf"; } }
g=group(font_family="/font.ttf")
a=g.add(text("A"),offset=(5,5))
coincident=g.add(rect(size=(a.width,a.height)),offset=(5,5))
p=plot(size=(90,70),font_family="/font.ttf",x=axis(range=(0,10),label="x"),y=axis(range=(0,10),label="y"))
p.line(x=[0,10],y=[0,10])
chart=g.add(p,offset=(30,10))
c=g.add(line(end_head=head()),start=a.top_right,end=chart.data(x=2,y=3))
g.add(rect(size=(1,1)),target=c.path.start)
g.add(rect(size=(1,1)),target=c.end)
parent=group(class="grow")
parent.add(g)
page.add(parent)
page.add(g,offset=(0,80))
"##,
    );
    let large = children(&children(&s.nodes[0])[0]);
    let small = children(&s.nodes[1]);
    for nodes in [large, small] {
        let expected_start = Point::new(
            jnum(&nodes[0], "x", 0.) + jnum(&nodes[0], "width", 0.),
            jnum(&nodes[0], "y", 0.),
        );
        close(endpoints(&nodes[3])[0], expected_start);
        let bounds = &nodes[2]["plotBounds"];
        let expected = laymesh_core::plot::local_to_parent(
            &nodes[2],
            [
                jnum(bounds, "x", 0.) + 0.2 * jnum(bounds, "width", 0.),
                jnum(bounds, "y", 0.) + 0.7 * jnum(bounds, "height", 0.),
            ],
        );
        close(
            endpoints(&nodes[3])[1],
            Point::new(expected[0], expected[1]),
        );
        close(
            Point::new(jnum(&nodes[4], "x", 0.), jnum(&nodes[4], "y", 0.)),
            expected_start,
        );
        close(
            Point::new(jnum(&nodes[5], "x", 0.), jnum(&nodes[5], "y", 0.)),
            Point::new(expected[0], expected[1]),
        );
    }
    assert!(jnum(&large[0], "width", 0.) > jnum(&small[0], "width", 0.) * 1.5);
}

#[test]
fn connections_and_dependent_markers_follow_nonuniform_group_scaling() {
    let s = scene(
        r#"
g=group()
a=g.add(rect(size=(10,10)),offset=(5,5))
b=g.add(rect(size=(10,10)),offset=(35,25))
c=g.add(line(),start=a.middle_right,end=b.middle_left,start_offset=(2,0))
g.add(rect(size=(1,1)),target=c.path.start)
g.add(rect(size=(1,1)),target=c.path.end)
placed=page.add(g,size=(100,35),rotation=30deg,offset=(20,30))
page.add(rect(size=(1,1)),target=placed.path.subpaths[2].start)
page.add(rect(size=(1,1)),target=placed.path.subpaths[2].end)
"#,
    );
    let nodes = children(&s.nodes[0]);
    let bounds = Point::new(
        jnum(&nodes[0], "x", 0.) + jnum(&nodes[0], "width", 0.) + 2.,
        jnum(&nodes[0], "y", 0.) + jnum(&nodes[0], "height", 0.) / 2.,
    );
    close(endpoints(&nodes[2])[0], bounds);
    let frame = node_transform(&s.nodes[0])
        * kurbo::Affine::scale_non_uniform(
            jnum(&s.nodes[0], "width", 0.) / jnum(&s.nodes[0], "contentWidth", 1.),
            jnum(&s.nodes[0], "height", 0.) / jnum(&s.nodes[0], "contentHeight", 1.),
        );
    for i in 0..2 {
        let expected = frame * endpoints(&nodes[2])[i];
        close(
            Point::new(
                jnum(&s.nodes[i + 1], "x", 0.),
                jnum(&s.nodes[i + 1], "y", 0.),
            ),
            expected,
        );
    }
}

#[test]
fn missing_geometry_is_reported_at_placement_and_invalid_definitions_stay_invalid() {
    let error = compile_source(
        "page=canvas(size=(20,20))\nl=line()\npage.add(l)",
        "/missing.lay",
        host(),
    )
    .unwrap_err();
    assert_eq!(error.code, "E_ARG");
    assert_eq!(error.loc.line, 3);
    assert_eq!(
        error.message,
        laymesh_core::endpoints::MISSING_LINE_GEOMETRY
    );
    for bad in [
        "line(dx=1)",
        "line(angle=30deg)",
        "line(length=1,dx=1,dy=0)",
        "line(length=0)",
        "line(line_width=-1)",
        "line(line_color=\"invalid\")",
    ] {
        let error = compile_source(
            &format!("page=canvas(size=(20,20))\nl={bad}"),
            "/bad.lay",
            host(),
        )
        .unwrap_err();
        assert_eq!(error.loc.line, 2, "{bad}");
    }
}

#[test]
fn invalid_connections_have_located_diagnostics() {
    for (call, code) in [
        ("page.add(line(),start=(1,2))", "E_ARG"),
        ("page.add(line(),start_offset=(1,2))", "E_ARG"),
        ("page.add(rect(size=(1,1)),start=(1,2),end=(3,4))", "E_ARG"),
        (
            "page.add(line(),start=(1,2),end=(3,4),target=page.center)",
            "E_ARG",
        ),
        (
            "page.add(line(),start=(1,2),end=(3,4),anchor=\"start\")",
            "E_ARG",
        ),
        ("page.add(line(),start=(1,2),end=(3,4),size=(2,2))", "E_ARG"),
        (
            "page.add(line(),start=(1,2),end=(3,4),rotation=0deg)",
            "E_ARG",
        ),
        (
            "page.add(line(),start=(1,2),end=(3,4),offset_space=\"target\")",
            "E_ARG",
        ),
        (
            "page.add(line(),start=(1,2),end=(3,4),start_offset_space=\"data\")",
            "E_ARG",
        ),
        (
            "page.add(line(),start=(1,2),end=(3,4),end_offset_space=\"target\")",
            "E_ANCHOR_DIRECTION",
        ),
        (
            "page.add(line(),start=page.center,end=(3,4),start_offset_space=\"target\")",
            "E_ANCHOR_DIRECTION",
        ),
        (
            "page.add(line(),start=(1,2),end=(1,2))",
            "E_ANCHOR_DIRECTION",
        ),
        (
            "page.add(line(),start=(1,2),end=(3,4),end_offset=(-2,-2))",
            "E_ANCHOR_DIRECTION",
        ),
        ("page.add(line(),start=(1deg,2),end=(3,4))", "E_UNIT"),
        (
            "page.add(line(),start=(1,2),end=(3,4),offset=(1deg,0))",
            "E_UNIT",
        ),
        (
            "page.add(line(),start=(1,2),end=(3,4),start_offset=(1,))",
            "E_ARG",
        ),
        ("page.add(line(),start=(1,),end=(3,4))", "E_ARG"),
        (
            "page.add(line(),start=self.path.start,end=(3,4))",
            "E_ANCHOR",
        ),
    ] {
        let error = compile_source(
            &format!("page=canvas(size=(20,20))\n{call}"),
            "/bad.lay",
            host(),
        )
        .unwrap_err();
        assert_eq!(error.code, code, "{call}: {error:?}");
        assert_eq!(error.loc.line, 2, "{call}");
    }
}

#[test]
fn foreign_anchors_collections_and_undefined_material_geometry_are_rejected() {
    for (source, code) in [
        (
            "g=group()\na=g.add(rect(size=(2,2)))\npage.add(line(),start=a.center,end=(3,4))",
            "E_LAYOUT",
        ),
        (
            "a=page.add(ellipse(size=(10,10)))\npage.add(line(),start=a.path.nearest(to=(0,0)),end=(20,20))",
            "E_ANCHOR",
        ),
        (
            "a=rect(size=(10,10))\npage.add(line(),start=a.center,end=(20,20))",
            "E_TYPE",
        ),
        (
            "a=page.add(polyline(points=[(0,0),(10,0),(10,10)]))\npage.add(line(),start=a.path.nodes[1],end=(20,20),start_offset_space=\"target\")",
            "E_ANCHOR_DIRECTION",
        ),
    ] {
        let error = compile_source(
            &format!("page=canvas(size=(30,30))\n{source}"),
            "/bad.lay",
            host(),
        )
        .unwrap_err();
        assert_eq!(error.code, code, "{source}: {error:?}");
    }
    scene(
        "a=page.add(polyline(points=[(0,0),(10,0),(10,10)]))\npage.add(line(),start=a.path.nodes[1].with_side(\"outgoing\"),end=(20,20),start_offset_space=\"target\",start_offset=(2,0))",
    );
}
