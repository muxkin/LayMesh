//! Anchor dependencies refer to instances, not coincident coordinates.
use laymesh_core::{
    engine::compile_source,
    geometry::node_transform,
    model::{Host, jnum},
};
use serde_json::Value;

fn host() -> Host {
    let mut host = Host::default();
    host.files.insert(
        "/fixtures/font.ttf".into(),
        include_bytes!("../../../tests/fonts/DejaVuSans.ttf").to_vec(),
    );
    host
}

#[test]
fn definitions_and_groups_do_not_expose_placed_instance_geometry() {
    for (source, code) in [
        ("shape=rect(size=(10,10))\nx=shape.width", "E_TYPE"),
        ("shape=rect(size=(10,10))\nx=shape.top_left", "E_TYPE"),
        ("g=group()\nx=g.width", "E_TYPE"),
        ("g=group()\nx=g.center", "E_LAYOUT"),
        (
            "a=page.add(rect(size=(10,10)))\nx=a.plot_top_left",
            "E_LAYOUT",
        ),
    ] {
        let source = format!("page=canvas(size=(40,30))\n{source}");
        let error = compile_source(&source, "/fixtures/main.lay", host()).unwrap_err();
        assert_eq!(error.code, code, "{source}");
        assert_eq!(error.loc.line, 3);
    }
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-8, "{a} != {b}");
}
fn children(node: &Value) -> &[Value] {
    node["children"].as_array().unwrap()
}

#[test]
fn coincident_boxes_do_not_steal_a_text_anchor_after_inherited_reflow() {
    let scene = compile_source(
        r##"
style { group.grow { font-size:20pt; } }
page=canvas(size=(80mm,40mm))
g=group()
a=g.add(text("A",font_family="/fixtures/font.ttf"))
b=g.add(rect(size=(a.width,a.height),fill="none"))
g.add(rect(size=(1mm,1mm),fill="#ff0000"),target=a.top_right)
parent=group(class="grow")
parent.add(g)
page.add(parent)
page.add(g,offset=(0,20))
"##,
        "/fixtures/main.lay",
        host(),
    )
    .unwrap();
    let large = children(&children(&scene.nodes[0])[0]);
    let small = children(&scene.nodes[1]);
    assert_eq!(large[0]["id"], "a");
    assert_eq!(large[1]["id"], "b");
    assert_eq!(small[0]["id"], "a");
    close(jnum(&large[2], "x", 0.), jnum(&large[0], "width", 0.));
    close(jnum(&small[2], "x", 0.), jnum(&small[0], "width", 0.));
    close(
        jnum(&large[0], "width", 0.),
        2. * jnum(&small[0], "width", 0.),
    );
    // b's size was evaluated when the original group was built, so it stays fixed.
    close(jnum(&large[1], "width", 0.), jnum(&small[1], "width", 0.));
}

#[test]
fn fuse_rejects_a_different_instance_even_when_its_anchor_coincides() {
    let error = compile_source(
        r##"
page=canvas(size=(60,30))
a=page.add(rect(size=(10,10),fill="#ff0000"))
b=page.add(rect(size=(10,10),fill="#ff0000"),offset=(20,0))
c=page.add(rect(size=(10,10),fill="#ff0000"))
page.fuse(a,b,points=(c.middle_right,b.middle_left),bridge_width=2,fill="#ff0000")
"##,
        "/fixtures/main.lay",
        host(),
    )
    .unwrap_err();
    assert_eq!(error.code, "E_FUSE");
    assert_eq!(error.loc.line, 6);
}

#[test]
fn group_replay_resolves_named_line_endpoints_after_rotation() {
    let scene = compile_source(
        r##"
style { group.grow { color:blue; } }
page=canvas(size=(80,60))
g=group()
a=g.add(line(dx=-20,dy=8),rotation=37deg,offset=(30,20))
g.add(rect(size=(1,1),fill="#ff0000"),target=a.start)
g.add(rect(size=(1,1),fill="#0000ff"),target=a.end)
parent=group(class="grow")
parent.add(g)
page.add(parent)
"##,
        "/fixtures/main.lay",
        host(),
    )
    .unwrap();
    let nodes = children(&children(&scene.nodes[0])[0]);
    for i in 0..2 {
        let point = &nodes[0]["endpoints"][i];
        let q = node_transform(&nodes[0])
            * kurbo::Point::new(point[0].as_f64().unwrap(), point[1].as_f64().unwrap());
        close(jnum(&nodes[i + 1], "x", 0.), q.x);
        close(jnum(&nodes[i + 1], "y", 0.), q.y);
    }
}

#[test]
fn fuse_operations_and_their_dependents_keep_identity_during_replay() {
    let scene = compile_source(
        r##"
style { group.grow { color:blue; } }
page=canvas(size=(80,60))
g=group()
a=g.add(rect(size=(10,10),fill="#ff0000"))
b=g.add(rect(size=(10,10),fill="#ff0000"),offset=(20,0))
f=g.fuse(a,b,points=(a.middle_right,b.middle_left),bridge_width=2,fill="#ff0000")
g.add(rect(size=(1,1)),target=f.top_right)
parent=group(class="grow")
parent.add(g)
page.add(parent)
"##,
        "/fixtures/main.lay",
        host(),
    )
    .unwrap();
    let nodes = children(&children(&scene.nodes[0])[0]);
    assert_eq!(nodes.len(), 2);
    assert_eq!(nodes[0]["id"], "f");
    close(jnum(&nodes[1], "x", 0.), jnum(&nodes[0], "width", 0.));
    close(jnum(&nodes[1], "y", 0.), 0.);
}

#[test]
fn plot_area_anchor_tracks_the_replayed_plot_margins() {
    let scene = compile_source(r##"
style { group.grow { font-size:20pt; font-family:"/fixtures/font.ttf"; } }
page=canvas(size=(120,100))
g=group()
p=plot(size=(90,70),font_family="/fixtures/font.ttf",x=axis(range=(0,10),label="x"),y=axis(range=(0,10),label="y"))
p.line(x=[0,10],y=[0,10])
a=g.add(p)
g.add(rect(size=(1,1),fill="#ff0000"),target=a.plot_top_right)
parent=group(class="grow")
parent.add(g)
page.add(parent)
"##, "/fixtures/main.lay", host()).unwrap();
    let nodes = children(&children(&scene.nodes[0])[0]);
    let bounds = &nodes[0]["plotBounds"];
    let q = laymesh_core::plot::local_to_parent(
        &nodes[0],
        [
            jnum(bounds, "x", 0.) + jnum(bounds, "width", 0.),
            jnum(bounds, "y", 0.),
        ],
    );
    close(jnum(&nodes[1], "x", 0.), q[0]);
    close(jnum(&nodes[1], "y", 0.), q[1]);
}

#[test]
fn repeated_named_placements_and_bare_calls_have_stable_unique_scene_ids() {
    let scene = compile_source(
        r#"page=canvas(size=(50,30))
for i in range(3) { a=page.add(rect(size=(2,2)),offset=(i*3,0)) }
page.add(rect(size=(2,2)))
unused=text("not placed")
page.add(rect(size=(2,2)))"#,
        "/ids.lay",
        host(),
    )
    .unwrap();
    let ids: Vec<_> = scene
        .nodes
        .iter()
        .map(|n| n["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["a", "a#2", "a#3", "@1", "@2"]);
}
