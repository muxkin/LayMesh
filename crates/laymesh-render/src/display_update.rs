//! Lossless, explicitly negotiated updates to an already installed display list.
use crate::{display_list::*, error};
use laymesh_core::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayUpdate {
    #[serde(default)]
    pub animation: Option<laymesh_core::animation::Animation>,
    pub schema_version: u32,
    pub width: f64,
    pub height: f64,
    /// A changed node or group replaces exactly one existing subtree.
    pub nodes: Vec<NodeUpdate>,
    /// Changing root count/order uses an explicit complete root replacement.
    pub roots: Option<Vec<DrawNode>>,
    pub resources: Option<BTreeMap<String, Resource>>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeUpdate {
    pub path: Vec<usize>,
    pub node: DrawNode,
}
impl DisplayUpdate {
    pub fn between(old: &DisplayList, new: &DisplayList) -> Self {
        let mut update = Self {
            animation: new.animation.clone(),
            schema_version: new.schema_version,
            width: new.width,
            height: new.height,
            nodes: Vec::new(),
            roots: None,
            resources: (old.resources != new.resources).then(|| new.resources.clone()),
        };
        if old.nodes.len() != new.nodes.len() {
            update.roots = Some(new.nodes.clone());
        } else {
            for (i, (a, b)) in old.nodes.iter().zip(&new.nodes).enumerate() {
                diff(a, b, &mut vec![i], &mut update.nodes);
            }
        }
        update
    }
    /// Validate everything before mutating a retained drawing. Overlapping or
    /// repeated paths would make application order ambiguous and are rejected.
    pub fn validate(&self, list: &DisplayList) -> Result<()> {
        if self.schema_version != DISPLAY_LIST_VERSION
            || !self.width.is_finite()
            || !self.height.is_finite()
            || self.width <= 0.
            || self.height <= 0.
            || (self.roots.is_some() && !self.nodes.is_empty())
        {
            return Err(error("无效绘制增量"));
        }
        let mut paths: Vec<_> = self.nodes.iter().map(|n| n.path.as_slice()).collect();
        paths.sort_unstable();
        for pair in paths.windows(2) {
            if pair[1].starts_with(pair[0]) {
                return Err(error("绘制增量路径重复或重叠"));
            }
        }
        for path in paths {
            node_at(&list.nodes, path)?;
        }
        Ok(())
    }
    pub fn apply(self, list: &mut DisplayList) -> Result<()> {
        self.validate(list)?;
        list.animation = self.animation;
        list.width = self.width;
        list.height = self.height;
        if let Some(roots) = self.roots {
            list.nodes = roots;
        }
        for change in self.nodes {
            *node_at_mut(&mut list.nodes, &change.path) = change.node;
        }
        if let Some(resources) = self.resources {
            list.resources = resources;
        }
        Ok(())
    }
}
fn diff(a: &DrawNode, b: &DrawNode, path: &mut Vec<usize>, out: &mut Vec<NodeUpdate>) {
    if a == b {
        return;
    }
    if a.name == b.name
        && a.transform == b.transform
        && a.opacity == b.opacity
        && a.effects == b.effects
        && a.semantic == b.semantic
        && a.hint == b.hint
        && let (
            DrawKind::Group {
                children: ac,
                clip: ap,
                mask: am,
            },
            DrawKind::Group {
                children: bc,
                clip: bp,
                mask: bm,
            },
        ) = (&a.kind, &b.kind)
        && ac.len() == bc.len()
        && ap == bp
        && am == bm
    {
        for (i, (a, b)) in ac.iter().zip(bc).enumerate() {
            path.push(i);
            diff(a, b, path, out);
            path.pop();
        }
        return;
    }
    out.push(NodeUpdate {
        path: path.clone(),
        node: b.clone(),
    });
}
pub(crate) fn node_at<'a>(nodes: &'a [DrawNode], path: &[usize]) -> Result<&'a DrawNode> {
    let Some((&index, rest)) = path.split_first() else {
        return Err(error("绘制增量路径为空"));
    };
    let node = nodes.get(index).ok_or_else(|| error("绘制增量路径越界"))?;
    if rest.is_empty() {
        return Ok(node);
    }
    if let DrawKind::Group { children, .. } = &node.kind {
        node_at(children, rest)
    } else {
        Err(error("绘制增量路径不是组"))
    }
}
fn node_at_mut<'a>(nodes: &'a mut [DrawNode], path: &[usize]) -> &'a mut DrawNode {
    let (index, rest) = path.split_first().unwrap();
    let node = &mut nodes[*index];
    if rest.is_empty() {
        return node;
    }
    let DrawKind::Group { children, .. } = &mut node.kind else {
        unreachable!()
    };
    node_at_mut(children, rest)
}
