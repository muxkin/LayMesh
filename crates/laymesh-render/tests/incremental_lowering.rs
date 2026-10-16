use laymesh_core::{model::Host, session::CompileSession};
use laymesh_render::display_list::{outline_nodes, to_display_list, to_display_list_reusing};
use std::sync::Arc;

#[test]
fn retained_lowering_matches_full_after_text_font_geometry_and_structure_edits() {
    let original = r##"p=canvas(size=(80,50),background="#eeeeee")
a=p.add(text("Title",font_family=["/font.ttf"],font_size=4))
b=p.add(path(commands=[move_to(0,0),cubic_to(5,10,15,-2,25,5)],fill="#336699"),offset=(5,15))"##;
    for outlined in [false, true] {
        let mut session = CompileSession::new(false);
        let mut host = Host::default();
        host.shared_files.insert(
            "/font.ttf".into(),
            Arc::new(include_bytes!("../../../tests/fonts/DejaVuSans.ttf").to_vec()),
        );
        let mut baseline = None;
        for (i, source) in [
            original.into(),
            original.replace("#eeeeee", "#ffffff"),
            original.replace("Title", "Changed title"),
            original.replace("(5,15)", "(9,18)"),
            original.replace("5,10,15,-2,25,5", "5,12,15,-2,25,5"),
            original.replace("a=p.add", "p.add(rect(size=(2,2)))\na=p.add"),
            original
                .lines()
                .filter(|line| !line.starts_with("a="))
                .collect::<Vec<_>>()
                .join("\n"),
            original.into(),
        ]
        .into_iter()
        .enumerate()
        {
            session.sync_host(&host, i as u64 + 1).unwrap();
            let scene = session.evaluate(&source, "/main.lay", None).unwrap();
            let mut expected = to_display_list(&scene).unwrap();
            if outlined {
                outline_nodes(&mut expected.nodes);
            }
            let actual = if let Some((previous, list)) = &baseline {
                let patch = laymesh_render::display_list::display_update_reusing(
                    &scene, previous, list, outlined,
                )
                .unwrap();
                let mut patched = list.clone();
                patch.apply(&mut patched).unwrap();
                assert_eq!(
                    patched, expected,
                    "borrowed patch, outlined={outlined}, revision={i}"
                );
                to_display_list_reusing(&scene, previous, list, outlined).unwrap()
            } else {
                expected.clone()
            };
            assert_eq!(actual, expected, "outlined={outlined}, revision={i}");
            baseline = Some((scene.as_ref().clone(), actual));
            if i == 6 {
                host.shared_files.insert(
                    "/font.ttf".into(),
                    Arc::new(
                        include_bytes!("../../../tests/fonts/text/Latin-Regular.ttf").to_vec(),
                    ),
                );
            }
        }
    }
}

#[test]
#[cfg(feature = "native")]
fn native_retained_patches_preserve_external_fonts_and_image_derivatives() {
    use laymesh_render::{config::PreviewSettings, preview_assets};
    let directory =
        std::env::temp_dir().join(format!("laymesh-retained-resources-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let mut host = Host::default();
    host.shared_files.insert(
        "/font.ttf".into(),
        Arc::new(include_bytes!("../../../tests/fonts/DejaVuSans.ttf").to_vec()),
    );
    let mut session = CompileSession::new(false);
    session.sync_host(&host, 1).unwrap();
    let original=session.evaluate("p=canvas(size=(80,50))\np.add(text(\"Title\",font_family=[\"/font.ttf\"],font_size=4))","/main.lay",None).unwrap();
    let mut baseline = None;
    let settings = PreviewSettings::default();
    for revision in 0..5 {
        let mut scene = original.as_ref().clone();
        scene.background = serde_json::json!(if revision % 2 == 0 {
            "#eeeeee"
        } else {
            "#ffffff"
        });
        if revision < 4 {
            let color = if revision < 2 {
                [255, 0, 0, 255]
            } else {
                [0, 0, 255, 100]
            };
            let image = image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
                16,
                8,
                image::Rgba(color),
            ));
            let key = laymesh_core::asset_cache::insert(image);
            scene.nodes.push(Arc::new(serde_json::json!({"kind":"image","id":"photo","_identity":"photo","mime":"image/png","rasterKey":key,"width":25.4,"height":12.7,"intrinsicWidth":16,"intrinsicHeight":8,"fit":"contain","x":revision,"y":10})));
        }
        let (expected, _) = preview_assets::display_frame(&scene, &directory, &settings).unwrap();
        if let Some((previous_scene, previous)) = &baseline {
            let (update, resources) = preview_assets::display_update_frame(
                &scene,
                previous_scene,
                previous,
                &directory,
                &settings,
            )
            .unwrap();
            for resource in resources {
                assert!(std::path::Path::new(resource["path"].as_str().unwrap()).is_file());
            }
            let mut actual = previous.clone();
            update.apply(&mut actual).unwrap();
            assert_eq!(actual, expected, "revision {revision}");
        }
        baseline = Some((scene, expected));
    }
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
#[cfg(feature = "native")]
fn group_translation_and_sibling_insertion_keep_paths_and_animation_pixels() {
    use laymesh_render::{animation::AnimationRenderer, display_list::display_update_reusing};
    let source = r##"style {@keyframes f {from{opacity:0}to{opacity:1}}}
p=canvas(size=(80,50),background="#ffffff")
g=group()
for i in range(24) {g.add(path(commands=[move_to(0,0),cubic_to(2.33,5.14,7.76,-1.11,10.8,4.3)],fill="none",border_color="#224477",border_width=0.1),offset=(0,i*0.2))}
a=p.add(g,size=(23,13),offset=(5.125,4.375))
s=p.step()
s.animate(a,name="f",duration=1s,fill="both")"##;
    let mut session = CompileSession::new(false);
    let mut baseline = None;
    for (i, text) in [
        source.to_owned(),
        source.replace("5.125,4.375", "8.125,6.375"),
        source
            .replace("a=p.add", "p.add(rect(size=(2,2)))\na=p.add")
            .replace("5.125,4.375", "9.125,7.375"),
        source.to_owned(),
    ]
    .iter()
    .enumerate()
    {
        session
            .apply_changes(
                i as u64 + 1,
                [("/main.lay".into(), Some(text.as_bytes().to_vec()))],
            )
            .unwrap();
        let scene = session.evaluate(text, "/main.lay", None).unwrap();
        let expected = to_display_list(&scene).unwrap();
        let actual = if let Some((old, previous)) = &baseline {
            let update = display_update_reusing(&scene, old, previous, false).unwrap();
            assert!(
                update
                    .properties
                    .iter()
                    .all(|p| !p.values.contains_key("kind.path")),
                "translation rebuilt geometry"
            );
            assert!(
                serde_json::to_vec(&update).unwrap().len() < 10000,
                "translation transmitted a full group"
            );
            let mut actual = previous.clone();
            update.apply(&mut actual).unwrap();
            let rect = kurbo::Rect::new(0., 0., 80., 50.);
            let mut a = AnimationRenderer::new(actual.clone()).unwrap();
            let mut b = AnimationRenderer::new(expected.clone()).unwrap();
            for time in [None, Some(0.), Some(0.5), Some(1.)] {
                assert_eq!(
                    a.render(rect, 640, 400, time, false).unwrap().rgba,
                    b.render(rect, 640, 400, time, false).unwrap().rgba,
                    "revision {i}, time {time:?}"
                );
            }
            actual
        } else {
            expected
        };
        baseline = Some((scene.as_ref().clone(), actual));
    }
}

#[test]
fn changing_draw_animation_bindings_invalidates_dependent_lowering() {
    let source = r##"style {@keyframes f {from{opacity:0}to{opacity:1}}}
p=canvas(size=(80,50))
a=p.add(path(commands=[move_to(0,0),cubic_to(2,5,7,-1,10,4)],fill="none",border_color="#224477",border_width=0.1))
s=p.step()
s.animate(a,name="f",duration=1s,fill="both")"##;
    let mut session = CompileSession::new(false);
    let mut baseline = None;
    for (i, text) in [
        source.to_owned(),
        source
            .replace("opacity:0", "-laymesh-draw:0")
            .replace("opacity:1", "-laymesh-draw:1"),
        source.to_owned(),
    ]
    .iter()
    .enumerate()
    {
        session
            .apply_changes(
                i as u64 + 1,
                [("/main.lay".into(), Some(text.as_bytes().to_vec()))],
            )
            .unwrap();
        let scene = session.evaluate(text, "/main.lay", None).unwrap();
        let expected = to_display_list(&scene).unwrap();
        if let Some((old, previous)) = &baseline {
            let update =
                laymesh_render::display_list::display_update_reusing(&scene, old, previous, false)
                    .unwrap();
            let mut actual = previous.clone();
            update.apply(&mut actual).unwrap();
            assert_eq!(actual, expected, "animation binding {i}");
        }
        baseline = Some((scene.as_ref().clone(), expected));
    }
}

#[test]
fn translated_shapes_rebase_synthetic_path_animation_metadata() {
    use laymesh_render::display_list::{DrawKind, DrawNode, display_update_reusing};
    fn hints(a: &DrawNode, b: &DrawNode) {
        assert_eq!(a.hint, b.hint, "stale animation metadata for {}", a.name);
        if let (DrawKind::Group { children: a, .. }, DrawKind::Group { children: b, .. }) =
            (&a.kind, &b.kind)
        {
            assert_eq!(a.len(), b.len());
            for (a, b) in a.iter().zip(b) {
                hints(a, b);
            }
        }
    }
    let source = r##"style {@keyframes f {from{opacity:0}to{opacity:1}}}
p=canvas(size=(80,50))
a=p.add(ellipse(size=(3.2,3.2),fill="#089bad"),offset=(5.4,7.4))
s=p.step()
s.animate(a,name="f",duration=1s,fill="both")"##;
    let mut session = CompileSession::new(false);
    let mut baseline = None;
    for (i, text) in [
        source.to_owned(),
        source.replace("5.4,7.4", "6.4,7.4"),
        source.to_owned(),
    ]
    .iter()
    .enumerate()
    {
        session
            .apply_changes(
                i as u64 + 1,
                [("/main.lay".into(), Some(text.as_bytes().to_vec()))],
            )
            .unwrap();
        let scene = session.evaluate(text, "/main.lay", None).unwrap();
        let expected = to_display_list(&scene).unwrap();
        let actual = if let Some((old, previous)) = &baseline {
            let update = display_update_reusing(&scene, old, previous, false).unwrap();
            assert!(
                update
                    .properties
                    .iter()
                    .all(|p| !p.values.contains_key("kind.path"))
            );
            let mut actual = previous.clone();
            update.apply(&mut actual).unwrap();
            for (a, b) in actual.nodes.iter().zip(&expected.nodes) {
                hints(a, b);
            }
            actual
        } else {
            expected
        };
        baseline = Some((scene.as_ref().clone(), actual));
    }
}
