//! Behavioral contracts retained from the original native/CLI test suite.
use kurbo::{Point, Shape};
use laymesh_core::{
    engine::compile_source,
    geometry::visible,
    model::{Host, Scene, jnum},
};
fn scene(source: &str) -> Scene {
    compile_source(source, "/fixtures/main.lay", Host::default()).unwrap()
}
fn close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 0.001, "{actual} != {expected}");
}
#[test]
fn physical_units_and_default_typographic_units() {
    let s = scene(
        "page=canvas(size=(15,10),unit=\"cm\")\npage.add(rect(size=(2,3),border_color=\"#222222\",border_width=1))\npage.add(rect(size=(1mm,1mm)),offset=(2inch,2cm))",
    );
    close(s.width, 150.);
    close(s.height, 100.);
    close(jnum(&s.nodes[0], "width", 0.), 20.);
    close(
        s.nodes[0]["strokeStyle"]["width"].as_f64().unwrap(),
        25.4 / 72.,
    );
    close(jnum(&s.nodes[1], "x", 0.), 50.8);
    close(jnum(&s.nodes[1], "y", 0.), 20.);
}
#[test]
fn independent_material_instances_keep_natural_auto_and_anchors() {
    let s = scene(
        "page=canvas(size=(100,80))\nbox=rect(size=(10,10),fill=\"#f00\")\na=page.add(box,size=(30,auto))\nb=page.add(box,target=a.top_right,offset=(2,0))\ng=group()\none=g.add(box)\ntwo=g.add(box,target=one.top_right,offset=(3,0))\npage.add(g,target=page.bottom_right,anchor=bottom_right)",
    );
    close(jnum(&s.nodes[0], "height", 0.), 10.);
    close(jnum(&s.nodes[1], "x", 0.), 32.);
    close(jnum(&s.nodes[2], "width", 0.), 23.);
    close(jnum(&s.nodes[2], "x", 0.), 77.);
    close(jnum(&s.nodes[2], "y", 0.), 70.);
}
#[test]
fn signed_line_endpoints_follow_rotation() {
    let s = scene(
        "page=canvas(size=(100,80))\na=page.add(line(dx=-20,dy=0),rotation=90deg,offset=(30,30))\npage.add(rect(size=(1,1)),target=a.start)\npage.add(rect(size=(1,1)),target=a.end)",
    );
    close(jnum(&s.nodes[1], "x", 0.), 30.);
    close(jnum(&s.nodes[1], "y", 0.), 50.);
    close(jnum(&s.nodes[2], "x", 0.), 30.);
    close(jnum(&s.nodes[2], "y", 0.), 30.);
}
#[test]
fn lexical_functions_control_flow_and_dimensional_arithmetic() {
    let s = scene(
        "function twice(v,factor=2){return v*factor}\npage=canvas(size=(100,70),layout_dpi=100)\nn=0\nfor i in range(4){if i==1{continue}\nn=n+i}\nwhile n<8{n=n+1\nif n==7{break}}\ngap=twice(2mm)\npage.add(rect(size=(gap+10px,n)))",
    );
    close(jnum(&s.nodes[0], "width", 0.), 6.54);
    close(jnum(&s.nodes[0], "height", 0.), 7.);
}
#[test]
fn unsafe_calls_and_execution_limits_have_diagnostics() {
    for (source, code) in [
        ("page=canvas(size=(20,20))\nx=process.exit()", "E_CALL"),
        ("page=canvas(size=(20,20))\nwhile true {}", "E_LIMIT"),
        (
            "page=canvas(size=(20,20))\nfunction f(){return f()}\nx=f()",
            "E_LIMIT",
        ),
        ("page=canvas(size=(20,20))\nx=1mm+2", "E_UNIT"),
        ("page=canvas(size=(20,20))\nx=1nm", "E_UNIT"),
    ] {
        let e = compile_source(source, "/test.lay", Host::default()).unwrap_err();
        assert_eq!(e.code, code);
        assert!(e.loc.line >= 2);
    }
}
#[test]
fn inline_styles_cascade_inherit_and_keep_physical_padding() {
    let s = scene(
        "style { text { border:1pt solid #111111; } .note {border-color:#ff0000;} text.note {border:2pt solid #004488;} }\npage=canvas(size=(30,30))\npage.add(text(\"A\",class=\"note\",padding=2mm,background=\"#fff\"))",
    );
    let b = &s.nodes[0]["children"][0];
    assert_eq!(b["strokeStyle"]["color"], "#004488");
    close(b["strokeStyle"]["width"].as_f64().unwrap(), 2. * 25.4 / 72.);
    close(jnum(&s.nodes[0]["children"][1], "x", 0.), 2.);
}
#[test]
fn image_crop_recalculates_auto_aspect() {
    let mut h = Host::default();
    h.files.insert(
        "/fixtures/photo.png".into(),
        include_bytes!("../../../examples/assets/photo.png").to_vec(),
    );
    let s=compile_source("page=canvas(size=(180,120))\np=image(\"photo.png\")\na=page.add(p,size=(80,auto))\npage.add(p,size=(40,auto),crop=box(offset=(0.5,0),size=(0.5,1)),target=a.top_right)","/fixtures/main.lay",h).unwrap();
    close(jnum(&s.nodes[0], "height", 0.), 40.);
    close(jnum(&s.nodes[1], "height", 0.), 40.);
    close(jnum(&s.nodes[1], "x", 0.), 80.);
}
#[test]
fn fused_hollow_shapes_preserve_holes() {
    let s = scene(
        "page=canvas(size=(80,50))\na=page.add(ring(outer_radius=10,inner_radius=5,fill=\"#f00\"),offset=(10,10))\nb=page.add(rect(size=(10,10),fill=\"#f00\"),offset=(40,15))\nf=page.fuse(a,b,bridge_width=3,fill=\"#f00\")",
    );
    assert_eq!(s.nodes.len(), 1);
    let p = visible(&s.nodes[0]);
    assert_eq!(p.winding(Point::new(20., 20.)), 0);
    assert_ne!(p.winding(Point::new(11., 20.)), 0);
    assert_ne!(p.winding(Point::new(45., 20.)), 0);
}
#[test]
fn fusion_consumes_handles_and_requires_a_bridge() {
    for (tail, code) in [
        ("f=page.fuse(a,b,fill=\"#f00\")", "E_FUSE"),
        ("f=page.fuse(a,b,bridge_width=0,fill=\"#f00\")", "E_VALUE"),
        (
            "f=page.fuse(a,b,bridge_width=2,fill=\"#f00\")\npage.add(box,target=a.top_left)",
            "E_FUSE",
        ),
    ] {
        let source = format!(
            "page=canvas(size=(80,50))\nbox=rect(size=(10,10),fill=\"#f00\")\na=page.add(box)\nb=page.add(box,offset=(20,0))\n{tail}"
        );
        assert_eq!(
            compile_source(&source, "/test.lay", Host::default())
                .unwrap_err()
                .code,
            code
        );
    }
}
#[test]
fn module_imports_are_relative_and_exported() {
    let mut h = Host::default();
    h.files.insert(
        "/fixtures/sub/part.lay".into(),
        b"gap=2mm\nexport function card(){g=group()\ng.add(rect(size=(gap*3,4)))\nreturn g}"
            .to_vec(),
    );
    let s = compile_source(
        "import { card } from './sub/part.lay'\npage=canvas(size=(20,20))\npage.add(card())",
        "/fixtures/main.lay",
        h,
    )
    .unwrap();
    close(jnum(&s.nodes[0], "width", 0.), 6.);
}

#[test]
fn string_equality_and_python_style_interpolation_preserve_units() {
    let source = r#"
page=canvas(size=(80,30))
if r"A" == "A" { page.add(rect(size=(1,1))) }
if str(1cm) == "10 mm" { page.add(rect(size=(2,2))) }
if f"{1cm:.2f} / {12:+05d} / {'A':*^5} / {'x'!r} / {2:.1e}" == '10.00 mm / +0012 / **A** / "x" / 2.0e+0' { page.add(rect(size=(3,3))) }
"#;
    assert_eq!(scene(source).nodes.len(), 3);
    for expression in [
        "f'{1.2:d}'",
        "f'{1:invalid}'",
        "f'{1:10001}'",
        "f'broken }'",
        "f'{1!x}'",
    ] {
        let source = format!("page=canvas(size=(20,20))\ns={expression}");
        assert_eq!(
            compile_source(&source, "/main.lay", Host::default())
                .unwrap_err()
                .code,
            "E_FORMAT"
        );
    }
}

#[test]
fn json_and_csv_reject_invalid_or_lossy_data() {
    for (kind, name, bytes) in [
        ("array", "data.json", "[]"),
        ("array", "data.json", "[[1,2],[3]]"),
        ("array", "data.json", "[true]"),
        ("array", "data.json", "[9007199254740993]"),
        ("table", "data.json", "{\"x\":[1],\"y\":[2,3]}"),
        ("table", "data.csv", "x,y\n"),
        ("table", "data.csv", "x,y\n1,2,3\n"),
        ("table", "data.csv", "x,x\n1,2\n"),
    ] {
        let mut host = Host::default();
        host.files
            .insert(format!("/{name}"), bytes.as_bytes().to_vec());
        let source = format!("page=canvas(size=(20,20))\nd={kind}(src=\"{name}\")");
        assert_eq!(
            compile_source(&source, "/main.lay", host).unwrap_err().code,
            "E_DATA",
            "{source} {bytes}"
        );
    }
    let mut host = Host::default();
    host.files.insert(
        "/data.csv".into(),
        b"\xef\xbb\xbfx,label\n2, A \n3, B\n".to_vec(),
    );
    let source = "page=canvas(size=(20,20))\nd=table(src=\"data.csv\")\npage.add(rect(size=(d[\"x\"][0],d[\"x\"][1])))";
    let s = compile_source(source, "/main.lay", host).unwrap();
    close(jnum(&s.nodes[0], "width", 0.), 2.);
}

#[test]
fn negative_group_bounds_and_path_resizing_match_visible_geometry() {
    let s = scene(
        "page=canvas(size=(100,100))\ng=group()\ng.add(rect(size=(4,6)),offset=(-10,-20))\npage.add(g)\np=path(commands=[move_to(0,0),line_to(4,0),line_to(4,6),close()],fill=\"#ff0000\")\npage.add(p,size=(40,12))",
    );
    close(jnum(&s.nodes[0], "width", 0.), 4.);
    close(jnum(&s.nodes[0], "height", 0.), 6.);
    close(visible(&s.nodes[1]).bounding_box().width(), 40.);
    close(visible(&s.nodes[1]).bounding_box().height(), 12.);
}

#[test]
fn nested_style_replay_preserves_fusion_and_following_anchors() {
    let s = scene(
        r##"page=canvas(size=(100,100))
style {group.red rect {fill:#ff0000;}}
g=group()
a=g.add(rect(size=(5,5)))
b=g.add(rect(size=(5,5)),offset=(10,0))
c=g.fuse(a,b,bridge_width=2,points=(a.middle_right,b.middle_left),fill="#00ff00",border_width=0)
g.add(rect(size=(2,2)),target=c.bottom_right)
outer=group(class="red")
outer.add(g)
page.add(outer,size=(34,14))"##,
    );
    let group = &s.nodes[0]["children"][0];
    assert_eq!(group["children"].as_array().unwrap().len(), 2);
    assert_eq!(group["children"][0]["fill"], "#00ff00");
    close(visible(&s.nodes[0]).bounding_box().width(), 34.);
    close(visible(&s.nodes[0]).bounding_box().height(), 14.);
}

#[test]
fn analytic_curve_recipes_survive_large_anisotropic_group_scales() {
    use laymesh_core::geometry::visible_transformed;
    let s = scene("page=canvas(size=(100,100))\npage.add(ellipse(size=(2,2),fill=\"#ff0000\"))");
    let p = visible_transformed(&s.nodes[0], kurbo::Affine::scale_non_uniform(500., 200.));
    let mut max_error: f64 = 0.;
    kurbo::flatten(p.iter(), 0.0001, |e| {
        if let kurbo::PathEl::LineTo(q) = e {
            let angle = ((q.y - 200.) / 200.).atan2((q.x - 500.) / 500.);
            let exact = Point::new(500. + 500. * angle.cos(), 200. + 200. * angle.sin());
            max_error = max_error.max(q.distance(exact));
        }
    });
    assert!(max_error <= 0.001, "physical error {max_error}mm");
}

#[test]
fn fused_circles_retain_analytic_sources_for_later_scaling() {
    use laymesh_core::geometry::visible_transformed;
    let s = scene(
        "page=canvas(size=(30,10))\na=page.add(ring(outer_radius=1,inner_radius=0.5,fill=\"#f00\"))\nb=page.add(ring(outer_radius=1,inner_radius=0.5,fill=\"#f00\"),offset=(10,0))\npage.fuse(a,b,bridge_width=0.2,fill=\"#f00\",border_width=0)",
    );
    let path = visible_transformed(&s.nodes[0], kurbo::Affine::scale(1000.));
    let mut points = 0;
    kurbo::flatten(path.iter(), 0.0001, |e| {
        if let kurbo::PathEl::LineTo(p) = e {
            if p.x < 999.9 {
                points += 1;
                let r = p.distance(Point::new(1000., 1000.));
                assert!(
                    (r - 1000.).abs().min((r - 500.).abs()) <= 0.001,
                    "radial error at {p:?}: {r}"
                );
            }
        }
    });
    assert!(points > 100);
    assert_eq!(path.winding(Point::new(1000., 1000.)), 0);
}

#[test]
fn short_inch_unit_and_fusion_without_border_preserve_semantics() {
    let s = scene(
        r##"page=canvas(size=(3in,2inch))
a=page.add(rect(size=(10,10),fill="#f00"))
b=page.add(rect(size=(10,10),fill="#f00"),offset=(20,0))
page.fuse(a,b,bridge_width=2,fill="#f00")"##,
    );
    close(s.width, 76.2);
    close(s.height, 50.8);
    assert_eq!(s.nodes.len(), 1);
    assert_eq!(s.nodes[0]["kind"], "path");
    assert_eq!(s.nodes[0]["strokeStyle"]["color"], "none");
}
