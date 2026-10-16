use laymesh_core::{engine::compile_source, model::Host};
use laymesh_render::{retained::RetainedScene, to_display_list};

#[test]
fn planning_matches_frame_matrices_without_constructing_animated_content() {
    for shape in [
        r#"rect(size=(12,8))"#,
        r##"path(commands=[move_to(0,0),cubic_to(c1x=5,c1y=20,c2x=20,c2y=-5,x=30,y=10)],fill="none",border_color="#123456",end_head=head(size=(4mm,3mm)))"##,
    ] {
        for geometry in ["-laymesh-draw", "-laymesh-reveal"] {
            let source = format!(
                r#"
style {{ @keyframes mixed {{
 from {{ {geometry}:0; transform:perspective(100mm) rotateY(20deg) scale(0.5); transform-origin:20% 70%; opacity:0.3 }}
 to {{ {geometry}:1; transform:perspective(100mm) rotateY(60deg) scale(1.5); transform-origin:80% 30%; opacity:0.8 }}
}} }}
p=canvas(size=(80,50))
a=p.add({shape},offset=(15,12),rotation=17deg)
s=p.step()
s.animate(a,name="mixed",duration=2s,fill="both",easing="linear")
"#
            );
            let scene = compile_source(&source, "/planning.lay", Host::default()).unwrap();
            let retained = RetainedScene::new(to_display_list(&scene).unwrap());
            if geometry != "-laymesh-draw" || shape.starts_with("path") {
                assert!(
                    retained
                        .sample(0.9, false, false)
                        .unwrap()
                        .values()
                        .any(|f| f.content.is_some())
                );
            }
            for reduced in [false, true] {
                for print in [false, true] {
                    for time in [0., 0.2, 0.9, 1.8, 2., 3.] {
                        let frames = retained.sample(time, reduced, print).unwrap();
                        let transforms = retained.sample_transforms(time, reduced, print).unwrap();
                        assert_eq!(frames.len(), transforms.len());
                        for (id, actual) in frames {
                            let planned = &transforms[&id];
                            assert_eq!(actual.matrix, planned.matrix, "{geometry} {time} {id}");
                            assert_eq!(actual.opacity, planned.opacity);
                            assert!(planned.content.is_none());
                        }
                    }
                }
            }
            assert!(retained.sample_transforms(f64::NAN, false, false).is_err());
            assert!(retained.sample_transforms(-1., false, false).is_err());
        }
    }
}

#[test]
fn presentation_reveal_keeps_geometry_out_of_frame_messages() {
    let source = r#"
style { @keyframes move {from {transform:rotateZ(5deg);opacity:0.2}to {transform:rotateZ(25deg);opacity:1}} }
p=canvas(size=(100,80))
g=group()
for i in range(1500) { g.add(line(length=30mm),offset=(0mm,i*0.02mm)) }
a=p.add(g,offset=(15,12),rotation=17deg)
s=p.step()
s.animate(a,name="reveal",duration=1s,fill="both",easing="linear")
s.animate(a,name="move",duration=3s,fill="both",easing="linear")
"#;
    let scene = compile_source(source, "/compact.lay", Host::default()).unwrap();
    let retained = RetainedScene::new(to_display_list(&scene).unwrap());
    for reduced in [false, true] {
        for time in [0., 0.3, 0.9, 1., 2., 3., 0.3] {
            let expected = retained.sample(time, reduced, false).unwrap();
            let frames = retained.sample_presentation(time, reduced).unwrap();
            for (id, frame) in &frames {
                assert_eq!(frame.matrix, expected[id].matrix);
                assert_eq!(frame.opacity, expected[id].opacity);
                assert!(
                    frame.content.is_none(),
                    "Unchanged paths must never be cloned into a reveal frame"
                );
                let reveal = frame.reveal.as_ref().unwrap();
                let node = expected[id].content.as_ref().unwrap();
                let laymesh_render::display_list::DrawKind::Group { children, clip, .. } =
                    &node.kind
                else {
                    panic!()
                };
                assert_eq!(reveal.transform, children[0].transform.as_coeffs());
                assert_eq!(
                    kurbo::Shape::bounding_box(&clip.as_ref().unwrap().path),
                    kurbo::Rect::new(
                        reveal.clip[0],
                        reveal.clip[1],
                        reveal.clip[2],
                        reveal.clip[3]
                    )
                );
            }
            assert!(serde_json::to_string(&frames).unwrap().len() < 1000);
        }
    }
}
