use kurbo::{Affine, Rect, Shape};
use laymesh_render::{display_list::*, display_update::*, raster::PreparedRaster};
use std::collections::BTreeMap;

fn node(x: f64, color: [f64; 4]) -> DrawNode {
    let mut n = DrawNode::new(DrawKind::Path {
        path: kurbo::Circle::new((0., 0.), 1.347).to_path(0.0001),
        fill: Paint::Solid { rgba: color },
        stroke: None,
        even_odd: false,
    });
    n.transform = Affine::translate((x, 2.823));
    n
}
fn list() -> DisplayList {
    DisplayList {
        media: Default::default(),
        page_transition: None,
        animation: None,
        schema_version: DISPLAY_LIST_VERSION,
        width: 20.,
        height: 10.,
        nodes: vec![
            node(11.2, [0.1, 0.3, 0.7, 0.43]),
            node(4.5, [0.8, 0.1, 0.1, 1.]),
        ],
        resources: BTreeMap::new(),
    }
}
fn compare(prepared: &mut PreparedRaster, list: &DisplayList, rect: Rect, w: u32, h: u32) {
    let actual = prepared.render_retained_region(rect, w, h).unwrap();
    let expected = laymesh_render::raster::render_region(list, rect, w, h).unwrap();
    let diff = actual
        .rgba
        .iter()
        .zip(&expected.rgba)
        .map(|(a, b)| a.abs_diff(*b))
        .max()
        .unwrap();
    let at = actual
        .rgba
        .iter()
        .zip(&expected.rgba)
        .position(|(a, b)| a.abs_diff(*b) > 1);
    assert!(
        diff <= 1,
        "retained/full maximum channel difference {diff}, at {at:?}"
    );
}
#[test]
fn lossless_updates_keep_unchanged_paths_and_reject_ambiguous_or_invalid_updates() {
    let old = list();
    let mut next = old.clone();
    next.nodes[0] = DrawNode::group(vec![node(10.4, [0.2, 0.7, 0.2, 1.])]);
    let update = DisplayUpdate::between(&old, &next);
    assert_eq!(update.nodes.len(), 1);
    let mut actual = old.clone();
    update.clone().apply(&mut actual).unwrap();
    assert_eq!(actual, next);
    for paths in [
        vec![vec![]],
        vec![vec![2]],
        vec![vec![1, 0]],
        vec![vec![0], vec![0]],
        vec![vec![0], vec![0, 0]],
    ] {
        let mut bad = update.clone();
        bad.nodes = paths
            .into_iter()
            .map(|path| NodeUpdate {
                path,
                node: next.nodes[0].clone(),
            })
            .collect();
        assert!(bad.apply(&mut actual).is_err());
        assert_eq!(actual, next);
    }
    let mut nested = next.clone();
    let DrawKind::Group { children, .. } = &mut nested.nodes[0].kind else {
        unreachable!()
    };
    children[0].opacity = 0.7;
    let update = DisplayUpdate::between(&next, &nested);
    assert_eq!(update.nodes[0].path, vec![0, 0]);
    update.apply(&mut actual).unwrap();
    assert_eq!(actual, nested);
}
#[test]
fn retained_pixels_clear_alpha_and_follow_moved_removed_and_resized_objects() {
    let mut scene = list();
    let mut prepared = PreparedRaster::new(scene.clone()).unwrap();
    let rect = Rect::new(0., 0., 20., 10.);
    compare(&mut prepared, &scene, rect, 1800, 900);
    for i in 0..6 {
        let old = scene.clone();
        scene.nodes[0] = node(11.2 + (i as f64) * 0.11, [0.2, 0.7, 0.3, 0.23]);
        scene.nodes[0].opacity = if i == 5 { 0. } else { 0.8 };
        prepared
            .update(DisplayUpdate::between(&old, &scene))
            .unwrap();
        compare(&mut prepared, &scene, rect, 1800, 900);
    }
    let old = scene.clone();
    scene.nodes.remove(0);
    prepared
        .update(DisplayUpdate::between(&old, &scene))
        .unwrap();
    compare(&mut prepared, &scene, rect, 1800, 900);
    let old = scene.clone();
    scene.width = 21.;
    prepared
        .update(DisplayUpdate::between(&old, &scene))
        .unwrap();
    compare(
        &mut prepared,
        &scene,
        Rect::new(0.3, 0.2, 17.2, 9.8),
        1301,
        731,
    );
    prepared.set_cache_budget(0);
    compare(&mut prepared, &scene, rect, 1800, 900);
}
#[test]
fn clips_transforms_effects_and_two_accumulated_updates_remain_correct() {
    let mut scene = list();
    scene.nodes[0] = DrawNode::group(vec![scene.nodes[0].clone()]);
    scene.nodes[0].transform = Affine::rotate(0.013) * Affine::scale_non_uniform(1.02, 0.99);
    let DrawKind::Group { clip, .. } = &mut scene.nodes[0].kind else {
        unreachable!()
    };
    *clip = Some(Clip {
        path: Rect::new(9.9, 1.3, 13., 4.1).to_path(0.0001),
        even_odd: false,
    });
    let mut prepared = PreparedRaster::new(scene.clone()).unwrap();
    let rect = Rect::new(0., 0., 20., 10.);
    compare(&mut prepared, &scene, rect, 1800, 900);
    // Both the sharp frame and an overview survive a content edit.
    compare(&mut prepared, &scene, rect, 384, 192);
    for i in 0..2 {
        let old = scene.clone();
        let DrawKind::Group { children, .. } = &mut scene.nodes[0].kind else {
            unreachable!()
        };
        children[0].transform *= Affine::translate((0.17 + i as f64 * 0.1, 0.04));
        prepared
            .update(DisplayUpdate::between(&old, &scene))
            .unwrap();
    }
    compare(&mut prepared, &scene, rect, 1800, 900);
    compare(&mut prepared, &scene, rect, 384, 192);
    let old = scene.clone();
    scene.nodes[0].effects.push(Effect {
        reflection: None,
        soft_edge: None,
        inner: false,
        blur: 0.17,
        spread: 0.03,
        offset: [0.1, 0.2],
        rgba: [0., 0., 0., 0.6],
        source_path: None,
        knockout: false,
    });
    prepared
        .update(DisplayUpdate::between(&old, &scene))
        .unwrap();
    compare(&mut prepared, &scene, rect, 1200, 600);
    let old = scene.clone();
    scene.nodes[0].opacity = 0.31;
    prepared
        .update(DisplayUpdate::between(&old, &scene))
        .unwrap();
    compare(&mut prepared, &scene, rect, 1200, 600);
}

#[test]
fn semantic_properties_do_not_copy_group_geometry() {
    let mut old = list();
    old.nodes = vec![DrawNode::group(
        (0..1536)
            .map(|i| node(i as f64, [0.1, 0.2, 0.3, 1.]))
            .collect(),
    )];
    let mut new = old.clone();
    new.nodes[0].transform = Affine::translate((2., 3.));
    let update = DisplayUpdate::between_incremental(&old, &new);
    let bytes = serde_json::to_vec(&update).unwrap();
    assert!(bytes.len() < 400, "{} bytes", bytes.len());
    assert!(update.nodes.is_empty() && update.structure.is_empty());
    assert_eq!(update.properties[0].values.len(), 1);
    let mut actual = old.clone();
    update.apply(&mut actual).unwrap();
    assert_eq!(actual, new);
    let previous = new.clone();
    let DrawKind::Group { children, .. } = &mut new.nodes[0].kind else {
        unreachable!()
    };
    let DrawKind::Path { fill, .. } = &mut children[517].kind else {
        unreachable!()
    };
    *fill = Paint::Solid {
        rgba: [0.8, 0.2, 0.1, 0.6],
    };
    let update = DisplayUpdate::between_incremental(&previous, &new);
    assert!(serde_json::to_vec(&update).unwrap().len() < 400);
    assert_eq!(update.properties[0].path, vec![0, 517]);
    update.apply(&mut actual).unwrap();
    assert_eq!(actual, new);
}
#[test]
fn semantic_splices_retain_moved_children_and_apply_nested_changes_atomically() {
    let mut old = list();
    for (i, n) in old.nodes.iter_mut().enumerate() {
        n.name = format!("n{i}");
    }
    old.nodes[0] = DrawNode::group(old.nodes.clone());
    old.nodes[0].name = "group".into();
    let mut new = old.clone();
    let DrawKind::Group { children, .. } = &mut new.nodes[0].kind else {
        unreachable!()
    };
    children.swap(0, 1);
    children[0].opacity = 0.3;
    let mut added = node(7., [0., 0., 1., 1.]);
    added.name = "new".into();
    children.insert(1, added);
    new.nodes.swap(0, 1);
    new.nodes.insert(0, node(2., [0., 1., 0., 1.]));
    let update = DisplayUpdate::between_incremental(&old, &new);
    assert_eq!(update.structure.len(), 2);
    assert!(update.structure.iter().any(|s| {
        s.insert
            .iter()
            .any(|v| matches!(v, ChildSource::Existing(_)))
    }));
    let mut actual = old.clone();
    update.clone().apply(&mut actual).unwrap();
    assert_eq!(actual, new);
    let mut raster = PreparedRaster::new(old.clone()).unwrap();
    raster.update(update.clone()).unwrap();
    compare(&mut raster, &new, Rect::new(0., 0., 20., 10.), 600, 300);
    for case in 0..5 {
        let mut bad = update.clone();
        match case {
            0 => bad.properties.push(PropertyUpdate {
                path: vec![0],
                values: BTreeMap::from([("kind.children".into(), serde_json::json!([]))]),
            }),
            1 => bad.properties[0]
                .values
                .insert("opacity".into(), serde_json::json!("bad"))
                .map(|_| ())
                .unwrap_or(()),
            2 => bad.structure[0].delete = usize::MAX,
            3 => bad.structure[0].insert.push(ChildSource::Existing(999)),
            _ => bad.structure.push(bad.structure[0].clone()),
        }
        let mut target = old.clone();
        assert!(bad.apply(&mut target).is_err());
        assert_eq!(target, old);
    }
    let removed = list();
    let update = DisplayUpdate::between_incremental(&new, &removed);
    update.apply(&mut actual).unwrap();
    assert_eq!(actual, removed);
}

#[test]
fn anonymous_source_insertions_use_internal_identity_instead_of_shifted_names() {
    let source = "page=canvas(size=(80,50))\npage.add(path(commands=[move_to(0,0),cubic_to(1,2,3,4,5,6)]))\npage.add(ellipse(size=(5,5)))";
    let mut session = laymesh_core::session::CompileSession::new(false);
    session
        .apply_changes(1, [("/main.lay".into(), Some(source.as_bytes().to_vec()))])
        .unwrap();
    let a = session.evaluate_file("/main.lay", None).unwrap();
    let old = laymesh_render::to_display_list(&a).unwrap();
    let source = source.replace("page.add(path", "page.add(rect(size=(3,4)))\npage.add(path");
    session
        .apply_changes(2, [("/main.lay".into(), Some(source.into_bytes()))])
        .unwrap();
    let b = session.evaluate_file("/main.lay", None).unwrap();
    let new = laymesh_render::to_display_list(&b).unwrap();
    let update = DisplayUpdate::between_incremental(&old, &new);
    assert!(update.nodes.is_empty());
    assert_eq!(update.structure.iter().map(|s| s.delete).sum::<usize>(), 0);
    assert_eq!(
        update
            .structure
            .iter()
            .map(|s| s.insert.len())
            .sum::<usize>(),
        1
    );
    assert!(
        !update
            .properties
            .iter()
            .any(|p| p.values.contains_key("kind.path"))
    );
    let mut actual = old;
    update.apply(&mut actual).unwrap();
    assert_eq!(actual, new);
}
