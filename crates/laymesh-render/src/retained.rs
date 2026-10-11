//! Immutable layout and small per-frame updates shared by Skia and CanvasKit.
use crate::{
    animation::{affine_matrix, back_facing, reveal, trim_paths},
    display_list::*,
    display_update::DisplayUpdate,
    error,
};
use kurbo::Affine;
use laymesh_core::{
    Result,
    animation::values::{multiply, translate},
};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeFrame {
    pub matrix: [f64; 9],
    pub opacity: f64,
    /// Only geometry-changing extensions replace commands, never CSS transforms.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<DrawNode>,
}
pub struct RetainedScene {
    list: DisplayList,
    targets: BTreeMap<String, Vec<usize>>,
}
impl RetainedScene {
    fn index(nodes: &[DrawNode], parent: &[usize], out: &mut BTreeMap<String, Vec<usize>>) {
        for (i, node) in nodes.iter().enumerate() {
            let mut path = parent.to_vec();
            path.push(i);
            if !node.name.is_empty() {
                out.insert(node.name.clone(), path.clone());
            }
            if let DrawKind::Group { children, .. } = &node.kind {
                Self::index(children, &path, out);
            }
        }
    }
    pub fn new(list: DisplayList) -> Self {
        let mut targets = BTreeMap::new();
        Self::index(&list.nodes, &[], &mut targets);
        Self { list, targets }
    }
    pub fn base(&self) -> &DisplayList {
        &self.list
    }
    pub fn update(&mut self, update: DisplayUpdate) -> Result<()> {
        update.apply(&mut self.list)?;
        self.targets.clear();
        Self::index(&self.list.nodes, &[], &mut self.targets);
        Ok(())
    }
    pub fn sample(
        &self,
        time: f64,
        reduced: bool,
        print: bool,
    ) -> Result<BTreeMap<String, NodeFrame>> {
        if !time.is_finite() || time < 0. {
            return Err(error("Invalid animation time"));
        }
        let mut output = BTreeMap::new();
        let Some(animation) = &self.list.animation else {
            return Ok(output);
        };
        for (name, style) in animation.timeline_with_media(reduced, print).sample(time) {
            let Some(path) = self.targets.get(&name) else {
                continue;
            };
            let mut nodes = &self.list.nodes;
            let mut original = &nodes[path[0]];
            for (depth, &index) in path.iter().enumerate() {
                original = &nodes[index];
                if depth + 1 < path.len() {
                    if let DrawKind::Group { children, .. } = &original.kind {
                        nodes = children;
                    }
                }
            }
            let mut content = None;
            if style.draw.is_some() || style.reveal.is_some() {
                let mut node = original.clone();
                if let Some(p) = style.draw {
                    trim_paths(&mut node, p)?;
                }
                if let Some(p) = style.reveal {
                    reveal(&mut node, p);
                }
                content = Some(node);
            }
            let node = content.as_ref().unwrap_or(original);
            let mut combined = affine_matrix(node.transform);
            if let Some(m) = style.transform {
                let o = style.origin.unwrap_or([0.; 3]);
                let m = multiply(
                    multiply(translate(o[0], o[1], o[2]), m),
                    translate(-o[0], -o[1], -o[2]),
                );
                let layout = node.hint["animationLayout"]
                    .as_array()
                    .filter(|v| v.len() == 6)
                    .map(|v| Affine::new(std::array::from_fn(|i| v[i].as_f64().unwrap_or(0.))))
                    .unwrap_or(node.transform);
                combined = multiply(
                    multiply(affine_matrix(layout), m),
                    affine_matrix(layout.inverse() * node.transform),
                );
            }
            let hidden = style.visible == Some(false)
                || (style.backface == Some(false) && back_facing(combined));
            output.insert(
                path.iter()
                    .map(usize::to_string)
                    .collect::<Vec<_>>()
                    .join("/"),
                NodeFrame {
                    matrix: [
                        combined[0],
                        combined[1],
                        combined[3],
                        combined[4],
                        combined[5],
                        combined[7],
                        combined[12],
                        combined[13],
                        combined[15],
                    ],
                    opacity: if hidden {
                        0.
                    } else {
                        style.opacity.unwrap_or(node.opacity)
                    },
                    content,
                },
            );
        }
        Ok(output)
    }
}
