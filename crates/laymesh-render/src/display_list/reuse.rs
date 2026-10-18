//! Retain local geometry when only its placement changes. Translation does not
//! change approximation tolerance, physical stroke width, or local clipping.
use super::*;
use crate::display_update::{DisplayUpdate, PropertyUpdate};
pub(super) struct Translation {
    old_root: usize,
    new_root: usize,
    change: PropertyUpdate,
}
pub(super) fn context_matches(old: &Scene, new: &Scene) -> bool {
    fn targets(scene: &Scene) -> std::collections::BTreeSet<&str> {
        let mut names = std::collections::BTreeSet::new();
        if let Some(a) = &scene.animation {
            for timeline in [&a.normal, &a.reduced, &a.print, &a.print_reduced] {
                for clip in timeline.steps.iter().flat_map(|s| &s.clips) {
                    if clip.tracks.iter().any(|t| t.property == "-laymesh-draw") {
                        names.insert(clip.target.as_str());
                    }
                }
            }
        }
        names
    }
    old.fonts == new.fonts
        && old.animation.is_some() == new.animation.is_some()
        && targets(old) == targets(new)
}
fn world_draw_geometry(node: &DrawNode) -> bool {
    node.hint.get("drawGeometry").is_some()
        || match &node.kind {
            DrawKind::Group { children, .. } => children.iter().any(world_draw_geometry),
            _ => false,
        }
}
fn visit(
    old: &Json,
    new: &Json,
    drawing: &DrawNode,
    path: &mut Vec<usize>,
    changes: &mut Vec<PropertyUpdate>,
) -> bool {
    let (Some(a), Some(b)) = (old.as_object(), new.as_object()) else {
        return false;
    };
    if a.len() != b.len()
        || a.iter()
            .any(|(k, v)| !matches!(k.as_str(), "x" | "y" | "children") && b.get(k) != Some(v))
    {
        return false;
    }
    if old["children"] != new["children"] {
        let (Some(a), Some(b), DrawKind::Group { children, .. }) = (
            old["children"].as_array(),
            new["children"].as_array(),
            &drawing.kind,
        ) else {
            return false;
        };
        let Some(DrawNode {
            kind: DrawKind::Group {
                children: drawings, ..
            },
            ..
        }) = children.first()
        else {
            return false;
        };
        if children.len() != 1 || a.len() != b.len() || a.len() != drawings.len() {
            return false;
        }
        path.push(0);
        for (i, ((a, b), d)) in a.iter().zip(b).zip(drawings).enumerate() {
            path.push(i);
            let valid = visit(a, b, d, path, changes);
            path.pop();
            if !valid {
                return false;
            }
        }
        path.pop();
    }
    let dx = jnum(new, "x", 0.) - jnum(old, "x", 0.);
    let dy = jnum(new, "y", 0.) - jnum(old, "y", 0.);
    if dx != 0. || dy != 0. {
        // Legacy draw-progress recipes retain page-space endpoint geometry.
        // Rebuild those recipes so animated paths retain exact physical speed.
        if world_draw_geometry(drawing) {
            return false;
        }
        let mut values = BTreeMap::from([(
            "transform".into(),
            json!(Affine::translate((dx, dy)) * drawing.transform),
        )]);
        if drawing.hint.get("animationLayout").is_some() {
            let mut hint = drawing.hint.clone();
            hint["animationLayout"] =
                json!(laymesh_core::geometry::node_transform(new).as_coeffs());
            // Lowering copies the shape's layout hint onto its anonymous
            // paint paths. Rebase those copies too; their geometry and local
            // transform stay unchanged. Real child objects have their own
            // logicalKind/layout and are handled by visit above.
            if let DrawKind::Group { children, .. } = &drawing.kind {
                for (i, child) in children.iter().enumerate() {
                    if matches!(child.kind, DrawKind::Path { .. })
                        && child.hint["logicalKind"].is_null()
                        && child.hint.get("animationLayout").is_some()
                    {
                        let mut child_hint = child.hint.clone();
                        child_hint["animationLayout"] = hint["animationLayout"].clone();
                        let mut child_path = path.clone();
                        child_path.push(i);
                        changes.push(PropertyUpdate {
                            path: child_path,
                            values: BTreeMap::from([("hint".into(), child_hint)]),
                        });
                    }
                }
            }
            values.insert("hint".into(), hint);
        }
        changes.push(PropertyUpdate {
            path: path.clone(),
            values,
        });
    }
    true
}
pub(super) fn translations(
    old: &Json,
    new: &Json,
    drawing: &DrawNode,
    old_root: usize,
    new_root: usize,
) -> Option<Vec<Translation>> {
    let mut changes = vec![];
    visit(old, new, drawing, &mut vec![], &mut changes).then(|| {
        changes
            .into_iter()
            .map(|change| Translation {
                old_root,
                new_root,
                change,
            })
            .collect()
    })
}
pub(super) fn patch(update: &mut DisplayUpdate, changes: Vec<Translation>, new_paths: bool) {
    for mut change in changes {
        change.change.path.insert(
            0,
            if new_paths {
                change.new_root
            } else {
                change.old_root
            },
        );
        update.properties.push(change.change);
    }
}
pub(super) fn snapshot(list: &mut DisplayList, changes: Vec<Translation>) -> Result<()> {
    if changes.is_empty() {
        return Ok(());
    }
    let mut update = DisplayUpdate {
        media: list.media.clone(),
        page_transition: list.page_transition.clone(),
        animation: list.animation.clone(),
        schema_version: list.schema_version,
        width: list.width,
        height: list.height,
        nodes: vec![],
        roots: None,
        resources: None,
        properties: vec![],
        structure: vec![],
    };
    patch(&mut update, changes, true);
    update.apply(list)
}
