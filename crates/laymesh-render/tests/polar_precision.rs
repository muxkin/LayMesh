#![cfg(feature = "native")]
//! The native/WASM SVG traversal must refine clipping paths as well as ink.
use laymesh_core::{engine::compile_source, model::Host};
use laymesh_render::render_svg;
use serde_json::{Value, json};
#[test]
fn final_svg_refines_polar_clip_paths_after_group_scaling() {
    let mut scene=compile_source(r#"page=canvas(size=(60,60))
p=plot(size=(40,40),plot_area=box(offset=(10,10),size=(20,20)),projection="polar",inner_radius=3mm,theta=axis(ticks=[]),r=axis(range=(0,1),ticks=[]))
p.line(theta=[0,90],r=[1,1])
page.add(p)"#, "/polar-svg.lay",Host::default()).unwrap();
    scene.nodes = vec![
        json!({"kind":"group","id":"scaled","width":4000.,"height":8000.,"contentWidth":40.,"contentHeight":40.,"children":scene.nodes}),
    ];
    let refined = render_svg(&scene).unwrap();
    fn strip(n: &mut Value) {
        if let Some(object) = n.as_object_mut() {
            object.remove("geometryRecipe");
            for value in object.values_mut() {
                strip(value);
            }
        } else if let Some(array) = n.as_array_mut() {
            for value in array {
                strip(value);
            }
        }
    }
    for n in &mut scene.nodes {
        strip(n);
    }
    let coarse = render_svg(&scene).unwrap();
    let vertices = |svg: &str| {
        let doc = roxmltree::Document::parse(svg).unwrap();
        doc.descendants()
            .filter(|n| n.has_tag_name("clipPath"))
            .flat_map(|n| n.children())
            .filter_map(|n| n.attribute("d"))
            .map(|d| kurbo::BezPath::from_svg(d).unwrap().elements().len())
            .sum::<usize>()
    };
    assert!(vertices(&coarse) > 10, "nontrivial polar hole clip");
    assert!(
        vertices(&refined) > vertices(&coarse) * 5,
        "export must use the refined clipping recipe"
    );
}
