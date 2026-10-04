use laymesh_core::{
    color::Color,
    engine::compile_source,
    model::{Host, jnum},
};
fn scene(code: &str) -> laymesh_core::model::Scene {
    compile_source(
        &format!("page=canvas(size=(120mm,90mm))\n{code}"),
        "/test.lay",
        Host::default(),
    )
    .unwrap()
}
#[test]
fn polar_lines_and_fixed_head_only_geometry() {
    let s = scene(
        "a=page.add(line(length=0mm,angle=30deg,end_head=head(size=(8mm,6mm))),anchor=self.path.end,offset=(30mm,30mm))\npage.add(rect(size=(1mm,1mm)),target=a.path.end,offset=(5mm,2mm),offset_space=\"target\")\npage.add(rect(size=(1mm,1mm)),target=a.ink.bounds.top_left)",
    );
    assert!(jnum(&s.nodes[0], "width", 0.) > 6.);
    assert!(jnum(&s.nodes[0], "height", 0.) > 6.);
    assert!(
        (jnum(&s.nodes[1], "x", 0.)
            - (30. + 5. * 30f64.to_radians().cos() + 2. * 30f64.to_radians().sin()))
        .abs()
            < 1e-4
    );
    assert!(jnum(&s.nodes[2], "x", 0.) < 25.);
}
#[test]
fn displacement_matches_length_angle() {
    let s =
        scene("page.add(line(dx=30mm,dy=40mm))\npage.add(line(length=50mm,angle=53.130102354deg))");
    for key in ["width", "height"] {
        assert!((jnum(&s.nodes[0], key, 0.) - jnum(&s.nodes[1], key, 0.)).abs() < 1e-6)
    }
}
#[test]
fn zero_line_frames_follow_heads_and_caps_without_moving_selected_endpoints() {
    let s = scene(
        "for size in [(4mm,3mm),(8mm,6mm)] { a=page.add(line(length=0mm,angle=30deg,end_head=head(size=size)),anchor=self.path.end,offset=(25mm,20mm),rotation=90deg)\npage.add(rect(size=(1,1)),target=a.path.start)\npage.add(rect(size=(1,1)),target=a.path.end) }\npage.add(line(length=0mm,angle=0deg,line_width=2mm))\npage.add(line(length=0mm,angle=0deg,line_width=2mm,end_cap=\"round\"))\npage.add(line(length=0mm,angle=0deg,line_width=2mm,end_head=head(shape=\"dot\",size=(6mm,4mm),opacity=0)))",
    );
    for key in ["width", "height"] {
        assert!((jnum(&s.nodes[3], key, 0.) / jnum(&s.nodes[0], key, 0.) - 2.).abs() < 0.0001);
    }
    for i in [1, 2, 4, 5] {
        assert!((jnum(&s.nodes[i], "x", 0.) - 25.).abs() < 1e-7);
        assert!((jnum(&s.nodes[i], "y", 0.) - 20.).abs() < 1e-7);
    }
    for i in [6, 7] {
        assert!((jnum(&s.nodes[i], "width", 0.) - 2.).abs() < 0.0001);
        assert!((jnum(&s.nodes[i], "height", 0.) - 2.).abs() < 0.0001);
    }
    assert!((jnum(&s.nodes[8], "width", 0.) - 6.).abs() < 0.0001);
    assert!((jnum(&s.nodes[8], "height", 0.) - 4.).abs() < 0.0001);
    assert!(laymesh_core::geometry::visible(&s.nodes[6]).is_empty());
    assert!(laymesh_core::geometry::visible(&s.nodes[8]).is_empty());
}
#[test]
fn shapes_caps_and_open_curve_preserve_path() {
    for shape in ["triangle", "open", "stealth", "dot", "diamond", "bar"] {
        let s = scene(&format!(
            "a=page.add(line(length=30mm,line_width=2mm,line_cap=\"round\",end_head=head(shape=\"{shape}\",size=(6mm,5mm))))\npage.add(rect(size=(1mm,1mm)),target=a.path.end)"
        ));
        assert_eq!(jnum(&s.nodes[1], "x", 0.), 30.);
        assert!(
            !laymesh_core::geometry::visible(&s.nodes[0])
                .elements()
                .is_empty()
        );
    }
    let s = scene(
        "page.add(path(commands=[move_to(0,0),cubic_to(c1x=5,c1y=0,c2x=20,c2y=10,x=25,y=10)],border_color=\"#000\",border_width=1mm,end_head=head()))",
    );
    assert!(s.nodes[0]["geometryRecipe"].is_object());
}
#[test]
fn clear_invalid_endpoint_diagnostics() {
    for code in [
        "line(length=0)",
        "line(length=1,dx=1,dy=0)",
        "line(length=-1)",
        "line(length=1,angle=2mm)",
        "line(length=1,end_head=rect(size=(2,2)))",
        "path(commands=[move_to(0,0),line_to(2,2),close()],end_head=head())",
        "arrow(dx=1,dy=0)",
        "path(commands=[move_to(0,0),cubic_to(0,0,0,0,0,0)],end_head=head())",
    ] {
        let full = format!("page=canvas(size=(20,20))\npage.add({code})");
        assert!(
            compile_source(&full, "/invalid.lay", Host::default()).is_err(),
            "{code}"
        );
    }
}
#[test]
fn rgba_hsv_oklch_conversion_and_source_retention() {
    let c = Color::parse("#ffffff90").unwrap();
    assert!((c.alpha - 144. / 255.).abs() < 1e-12);
    let c = Color::new("hsv", [120., 1., 1.], 0.3).unwrap();
    assert_eq!(c.rgba, [0., 1., 0., 0.3]);
    assert_eq!(
        Color::new("oklch", [0.7, 0.9, 200.], 1.).unwrap().space,
        "oklch"
    );
    assert!(Color::new("oklch", [0.7, 0.9, 200.], 1.).unwrap().mapped);
    for s in [
        "rgb(255 0 0 / 0.5)",
        "hsv(120deg 1 1 / 0.6)",
        "oklch(0.7 0.15 200 / 0.8)",
        "#1234",
        "#12345678",
        "hsv(-1.5707963267948966rad 1 1 / 0.7)",
    ] {
        let c = Color::parse(s).unwrap();
        assert!(Color::parse(&c.css()).is_ok());
    }
    let s = scene(
        "page.add(rect(size=(10,10),fill=rgb(255,0,0,alpha=0.5)))\npage.add(rect(size=(10,10),fill=hsv(120deg,1,1)))\npage.add(rect(size=(10,10),fill=oklch(0.7,0.15,200deg)))",
    );
    assert_eq!(s.nodes.len(), 3);
}
#[test]
fn lcss_endpoint_and_color_configuration() {
    let s = scene(
        "style { .pointer { end-head: head(shape=\"triangle\",size=(5mm,4mm),fill=rgb(255,0,0,alpha=0.5)); start-cap: round; line-color: hsv(120 1 1 / 0.5); } }\npage.add(line(length=30mm,class=\"pointer\"))",
    );
    assert!(s.nodes[0]["endpointRecipe"]["end_head"].is_object());
    assert!(
        s.nodes[0]["strokeStyle"]["color"]
            .as_str()
            .unwrap()
            .starts_with("rgba")
    );
}

#[test]
fn zero_path_keeps_identity_continuous_points_and_transformed_direction() {
    let s = scene(
        "a=page.add(line(length=0mm,angle=0deg,end_head=head()),anchor=self.path.start,offset=(20mm,20mm),rotation=90deg)\nfor p in [a.path.start,a.path.end,a.path.nodes[0],a.path.at(fraction=0.75),a.path.at(distance=0mm)] { page.add(rect(size=(1,1)),target=p) }\npage.add(rect(size=(1,1)),target=a.path.start,offset=(3mm,2mm),offset_space=\"target\")",
    );
    for n in &s.nodes[1..6] {
        assert!((jnum(n, "x", 0.) - 20.).abs() < 1e-7);
        assert!((jnum(n, "y", 0.) - 20.).abs() < 1e-7);
    }
    assert!((jnum(&s.nodes[6], "x", 0.) - 22.).abs() < 1e-7);
    assert!((jnum(&s.nodes[6], "y", 0.) - 23.).abs() < 1e-7);
    for (query, code) in [
        ("a.path.at(distance=1mm)", "E_PATH"),
        ("a.path.at(distance=0.000001mm)", "E_PATH"),
        ("a.path.nearest(to=(0mm,0mm))[0]", "E_GEOMETRY"),
    ] {
        let src = format!(
            "page=canvas(size=(20,20))\na=page.add(line(length=0mm,angle=0deg))\nx={query}"
        );
        assert_eq!(
            compile_source(&src, "/zero.lay", Host::default())
                .unwrap_err()
                .code,
            code
        );
    }
}

#[test]
fn caps_short_lines_overlaps_and_multiple_subpaths_have_vector_ink() {
    use kurbo::{Point, Shape};
    for (cap, area) in [
        ("butt", 0.),
        ("round", std::f64::consts::PI),
        ("square", 4.),
    ] {
        let s = scene(&format!(
            "page.add(line(length=0mm,angle=45deg,line_width=2mm,start_cap=\"{cap}\",end_cap=\"{cap}\"))"
        ));
        let ink = laymesh_core::geometry::visible(&s.nodes[0]);
        assert!(
            (ink.area().abs() - area).abs() < 0.002,
            "{cap} area {}",
            ink.area()
        );
    }
    let s = scene(
        "a=page.add(line(length=1mm,start_head=head(size=(8mm,6mm)),end_head=head(size=(8mm,6mm))))\npage.add(rect(size=(1,1)),target=a.path.end)\npage.add(path(commands=[move_to(0mm,10mm),line_to(20mm,10mm),move_to(0mm,20mm),line_to(20mm,20mm)],border_color=\"#000\",end_head=head(size=(6mm,5mm))))",
    );
    assert_eq!(jnum(&s.nodes[1], "x", 0.), 1.);
    let ink = laymesh_core::geometry::visible(&s.nodes[0]);
    assert!(ink.bounding_box().width() > 10.);
    let ink = laymesh_core::geometry::visible(&s.nodes[2]);
    assert!(ink.winding(Point::new(17., 9.)) != 0);
    assert!(ink.winding(Point::new(17., -1.)) != 0);
}

#[test]
fn dashes_keep_source_phase_and_closed_routes_do_not_close_each_dash() {
    use kurbo::{Point, Shape};
    let s = scene(
        "page.add(line(length=30mm,line_width=1mm,line_dash=[4mm,4mm],start_head=head(shape=\"bar\",size=(10mm,3mm))))\npage.add(path(commands=[move_to(0,10),line_to(20,10),line_to(20,30),line_to(0,30),close()],border_color=\"#000\",border_width=1mm,border_dash=[4mm,4mm],start_cap=\"round\"))",
    );
    let ink = laymesh_core::geometry::visible(&s.nodes[0]);
    assert_eq!(ink.winding(Point::new(6., 0.)), 0);
    assert_ne!(ink.winding(Point::new(9., 0.)), 0);
    let ink = laymesh_core::geometry::visible(&s.nodes[1]);
    assert_eq!(ink.winding(Point::new(10., 10.)), 0);
}

#[test]
fn endpoint_recipes_survive_group_replay_nonuniform_scaling_and_fusion() {
    use kurbo::Shape;
    let s = scene(
        "style { .wide {line-width:2mm;} }\ng=group()\na=g.add(line(length=20mm,end_head=head(shape=\"dot\",size=(6mm,4mm))),class=\"wide\")\ng.add(rect(size=(1,1)),target=a.path.end)\ng.add(rect(size=(1,1)),target=a.ink.bounds.bottom_right)\npage.add(g,size=(50mm,10mm),rotation=30deg)\npage.add(g)\nb=page.add(polyline(points=[(0mm,0mm),(10mm,10mm)],end_head=head(size=(6mm,4mm))),size=(30mm,10mm))\npage.add(rect(size=(1,1)),target=b.path.end)",
    );
    for group in &s.nodes[..2] {
        let children = group["children"].as_array().unwrap();
        assert!(children[0]["endpointRecipe"].is_object());
        assert!((jnum(&children[1], "x", 0.) - 20.).abs() < 1e-7);
        assert!(
            laymesh_core::geometry::visible(&children[0])
                .bounding_box()
                .width()
                >= 23.
        );
    }
    assert!(
        (jnum(&s.nodes[3], "x", 0.) - 30.).abs() < 1e-7,
        "{}",
        s.nodes[3]
    );
    assert!(
        (jnum(&s.nodes[3], "y", 0.) - 10.).abs() < 1e-7,
        "{}",
        s.nodes[3]
    );
    let s = scene(
        "a=page.add(line(length=10mm,end_head=head(shape=\"dot\",size=(6mm,4mm))))\nb=page.add(line(length=10mm,end_head=head(shape=\"bar\",size=(2mm,4mm))),offset=(20mm,0mm))\npage.fuse(a,b,points=(a.path.end,b.path.start),bridge_width=2mm,fill=\"#000\")",
    );
    assert_eq!(s.nodes.len(), 1);
    assert!(!laymesh_core::geometry::visible(&s.nodes[0]).is_empty());
}

#[test]
fn head_only_groups_keep_ink_and_each_route_direction() {
    let s = scene(
        "g=group()\ng.add(line(length=0mm,angle=0deg,end_head=head(shape=\"dot\",size=(6mm,4mm))))\na=page.add(g,anchor=self.path.start,offset=(20mm,20mm),rotation=90deg)\npage.add(rect(size=(1,1)),target=a.path.start,offset=(3mm,2mm),offset_space=\"target\")\npage.add(rect(size=(1,1)),target=a.ink.bounds.top_left)\ng2=group()\ng2.add(line(length=0mm,angle=0deg,end_head=head()),anchor=self.path.start)\ng2.add(line(length=0mm,angle=90deg,end_head=head()),anchor=self.path.start,offset=(0mm,10mm))\nb=page.add(g2,anchor=self.path.subpaths[0].start,offset=(40mm,10mm))\npage.add(rect(size=(1,1)),target=b.path.subpaths[0].start,offset=(2mm,1mm),offset_space=\"target\")\npage.add(rect(size=(1,1)),target=b.path.subpaths[1].start,offset=(2mm,1mm),offset_space=\"target\")",
    );
    for (i, x, y) in [(1, 22., 23.), (2, 18., 17.), (4, 42., 9.), (5, 41., 22.)] {
        assert!(
            (jnum(&s.nodes[i], "x", 0.) - x).abs() < 0.0001,
            "{}",
            s.nodes[i]
        );
        assert!(
            (jnum(&s.nodes[i], "y", 0.) - y).abs() < 0.0001,
            "{}",
            s.nodes[i]
        );
    }
    assert!(!laymesh_core::geometry::visible(&s.nodes[0]).is_empty());
}
