//! Property and child-contribution patches, negotiated by incremental-session-v1.
use super::*;
use serde_json::{Value as J, to_value};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PropertyUpdate {
    pub path: Vec<usize>,
    pub values: BTreeMap<String, J>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChildrenUpdate {
    pub parent: Vec<usize>,
    pub start: usize,
    pub delete: usize,
    pub insert: Vec<ChildSource>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ChildSource {
    Existing(usize),
    New(DrawNode),
}

fn identity(node: &DrawNode) -> &str {
    node.hint["_identity"].as_str().unwrap_or(&node.name)
}
fn same_identity(a: &DrawNode, b: &DrawNode) -> bool {
    let key = identity(a);
    if key.is_empty() {
        a == b
    } else {
        key == identity(b)
    }
}
fn diff_children(
    old: &[DrawNode],
    new: &[&DrawNode],
    parent: &mut Vec<usize>,
    update: &mut DisplayUpdate,
) {
    if old.len() == new.len() && old.iter().zip(new).all(|(a, b)| identity(a) == identity(b)) {
        for (i, (a, b)) in old.iter().zip(new).enumerate() {
            parent.push(i);
            diff_node(a, b, parent, update);
            parent.pop();
        }
        return;
    }
    let mut prefix = 0;
    while prefix < old.len().min(new.len()) && same_identity(&old[prefix], &new[prefix]) {
        parent.push(prefix);
        diff_node(&old[prefix], &new[prefix], parent, update);
        parent.pop();
        prefix += 1;
    }
    let mut suffix = 0;
    while suffix < old.len().min(new.len()) - prefix
        && same_identity(&old[old.len() - 1 - suffix], &new[new.len() - 1 - suffix])
    {
        let i = old.len() - 1 - suffix;
        parent.push(i);
        diff_node(&old[i], &new[new.len() - 1 - suffix], parent, update);
        parent.pop();
        suffix += 1;
    }
    let mut names = BTreeMap::<&str, Vec<usize>>::new();
    for (i, node) in old.iter().enumerate().take(old.len() - suffix).skip(prefix) {
        if !identity(node).is_empty() {
            names.entry(identity(node)).or_default().push(i);
        }
    }
    let mut used = std::collections::BTreeSet::new();
    let mut insert = vec![];
    for node in &new[prefix..new.len() - suffix] {
        let index = names
            .get(identity(node))
            .filter(|v| v.len() == 1)
            .and_then(|v| used.insert(v[0]).then_some(v[0]));
        if let Some(index) = index {
            parent.push(index);
            diff_node(&old[index], node, parent, update);
            parent.pop();
            insert.push(ChildSource::Existing(index));
        } else {
            insert.push(ChildSource::New((*node).clone()));
        }
    }
    update.structure.push(ChildrenUpdate {
        parent: parent.clone(),
        start: prefix,
        delete: old.len() - suffix - prefix,
        insert,
    });
}
fn diff_node(a: &DrawNode, b: &DrawNode, path: &mut Vec<usize>, update: &mut DisplayUpdate) {
    if std::ptr::eq(a, b) || a == b {
        return;
    }
    if std::mem::discriminant(&a.kind) != std::mem::discriminant(&b.kind) {
        update.nodes.push(NodeUpdate {
            path: path.clone(),
            node: b.clone(),
        });
        return;
    }
    let mut values = BTreeMap::new();
    macro_rules! field {
        ($name:expr,$a:expr,$b:expr) => {
            if $a != $b {
                values.insert($name.into(), to_value(&$b).unwrap());
            }
        };
    }
    field!("name", a.name, b.name);
    field!("transform", a.transform, b.transform);
    field!("opacity", a.opacity, b.opacity);
    field!("effects", a.effects, b.effects);
    field!("semantic", a.semantic, b.semantic);
    field!("hint", a.hint, b.hint);
    match (&a.kind, &b.kind) {
        (
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
        ) => {
            field!("kind.clip", ap, bp);
            field!("kind.mask", am, bm);
            diff_children(ac, &bc.iter().collect::<Vec<_>>(), path, update);
        }
        (
            DrawKind::Path {
                path: ap,
                fill: af,
                stroke: as_,
                even_odd: ae,
            },
            DrawKind::Path {
                path: bp,
                fill: bf,
                stroke: bs,
                even_odd: be,
            },
        ) => {
            field!("kind.path", ap, bp);
            field!("kind.fill", af, bf);
            field!("kind.stroke", as_, bs);
            field!("kind.even_odd", ae, be);
        }
        (
            DrawKind::Image {
                resource: ar,
                rect: ab,
                nearest: an,
            },
            DrawKind::Image {
                resource: br,
                rect: bb,
                nearest: bn,
            },
        ) => {
            field!("kind.resource", ar, br);
            field!("kind.rect", ab, bb);
            field!("kind.nearest", an, bn);
        }
        (
            DrawKind::Glyphs {
                resource: ar,
                text: at,
                size: as_,
                glyphs: ag,
                outlines: ao,
                color: ac,
                family: af,
                weight: aw,
                italic: ai,
                width: ax,
                ascent: aa,
                descent: ad,
            },
            DrawKind::Glyphs {
                resource: br,
                text: bt,
                size: bs,
                glyphs: bg,
                outlines: bo,
                color: bc,
                family: bf,
                weight: bw,
                italic: bi,
                width: bx,
                ascent: ba,
                descent: bd,
            },
        ) => {
            field!("kind.resource", ar, br);
            field!("kind.text", at, bt);
            field!("kind.size", as_, bs);
            field!("kind.glyphs", ag, bg);
            field!("kind.outlines", ao, bo);
            field!("kind.color", ac, bc);
            field!("kind.family", af, bf);
            field!("kind.weight", aw, bw);
            field!("kind.italic", ai, bi);
            field!("kind.width", ax, bx);
            field!("kind.ascent", aa, ba);
            field!("kind.descent", ad, bd);
        }
        _ => unreachable!(),
    }
    if !values.is_empty() {
        update.properties.push(PropertyUpdate {
            path: path.clone(),
            values,
        });
    }
}
fn children<'a>(list: &'a DisplayList, parent: &[usize]) -> Result<&'a [DrawNode]> {
    if parent.is_empty() {
        return Ok(&list.nodes);
    }
    match &node_at(&list.nodes, parent)?.kind {
        DrawKind::Group { children, .. } => Ok(children),
        _ => Err(error("结构更新目标不是组")),
    }
}
fn fields(node: &mut DrawNode, values: &BTreeMap<String, J>, apply: bool) -> Result<()> {
    macro_rules! set {($target:expr,$value:expr)=>{{let v=serde_json::from_value($value.clone()).map_err(|_|error("无效绘制属性"))?;if apply{$target=v;}}};}
    for (name, value) in values {
        match name.as_str() {
            "name" => set!(node.name, value),
            "transform" => set!(node.transform, value),
            "opacity" => set!(node.opacity, value),
            "effects" => set!(node.effects, value),
            "semantic" => set!(node.semantic, value),
            "hint" => set!(node.hint, value),
            name => match &mut node.kind {
                DrawKind::Group { clip, mask, .. } => match name {
                    "kind.clip" => set!(*clip, value),
                    "kind.mask" => set!(*mask, value),
                    _ => return Err(error("未知组属性")),
                },
                DrawKind::Path {
                    path,
                    fill,
                    stroke,
                    even_odd,
                } => match name {
                    "kind.path" => set!(*path, value),
                    "kind.fill" => set!(*fill, value),
                    "kind.stroke" => set!(*stroke, value),
                    "kind.even_odd" => set!(*even_odd, value),
                    _ => return Err(error("未知路径属性")),
                },
                DrawKind::Image {
                    resource,
                    rect,
                    nearest,
                } => match name {
                    "kind.resource" => set!(*resource, value),
                    "kind.rect" => set!(*rect, value),
                    "kind.nearest" => set!(*nearest, value),
                    _ => return Err(error("未知图片属性")),
                },
                DrawKind::Glyphs {
                    resource,
                    text,
                    size,
                    glyphs,
                    outlines,
                    color,
                    family,
                    weight,
                    italic,
                    width,
                    ascent,
                    descent,
                } => match name {
                    "kind.resource" => set!(*resource, value),
                    "kind.text" => set!(*text, value),
                    "kind.size" => set!(*size, value),
                    "kind.glyphs" => set!(*glyphs, value),
                    "kind.outlines" => set!(*outlines, value),
                    "kind.color" => set!(*color, value),
                    "kind.family" => set!(*family, value),
                    "kind.weight" => set!(*weight, value),
                    "kind.italic" => set!(*italic, value),
                    "kind.width" => set!(*width, value),
                    "kind.ascent" => set!(*ascent, value),
                    "kind.descent" => set!(*descent, value),
                    _ => return Err(error("未知文字属性")),
                },
            },
        }
    }
    Ok(())
}
fn shell(node: &DrawNode) -> DrawNode {
    // Validation never clones the existing geometry, glyphs or descendants.
    let kind = match &node.kind {
        DrawKind::Group { .. } => DrawKind::Group {
            children: vec![],
            clip: None,
            mask: None,
        },
        DrawKind::Path { .. } => DrawKind::Path {
            path: kurbo::BezPath::new(),
            fill: Paint::None,
            stroke: None,
            even_odd: false,
        },
        DrawKind::Image { .. } => DrawKind::Image {
            resource: String::new(),
            rect: kurbo::Rect::ZERO,
            nearest: false,
        },
        DrawKind::Glyphs { .. } => DrawKind::Glyphs {
            resource: String::new(),
            text: String::new(),
            size: 0.,
            glyphs: vec![],
            outlines: vec![],
            color: [0.; 4],
            family: String::new(),
            weight: 400,
            italic: false,
            width: 0.,
            ascent: 0.,
            descent: 0.,
        },
    };
    DrawNode::new(kind)
}
impl DisplayUpdate {
    pub fn between_incremental(old: &DisplayList, new: &DisplayList) -> Self {
        Self::between_retained(old, new, &new.nodes.iter().collect::<Vec<_>>())
    }
    /// `header.nodes` is ignored; nodes may borrow unchanged old contributions.
    pub fn between_retained(old: &DisplayList, new: &DisplayList, nodes: &[&DrawNode]) -> Self {
        let mut update = Self {
            media: new.media.clone(),
            animation: new.animation.clone(),
            page_transition: new.page_transition.clone(),
            schema_version: new.schema_version,
            width: new.width,
            height: new.height,
            nodes: vec![],
            roots: None,
            resources: (old.resources != new.resources).then(|| new.resources.clone()),
            properties: vec![],
            structure: vec![],
        };
        diff_children(&old.nodes, nodes, &mut vec![], &mut update);
        update
    }
    pub(super) fn validate_semantic(&self, list: &DisplayList) -> Result<()> {
        let mut paths = std::collections::BTreeSet::new();
        for property in &self.properties {
            if !paths.insert(property.path.clone())
                || self
                    .nodes
                    .iter()
                    .any(|n| property.path.starts_with(&n.path))
            {
                return Err(error("绘制属性更新路径重复或重叠"));
            }
            let node = node_at(&list.nodes, &property.path)?;
            fields(&mut shell(node), &property.values, false)?;
        }
        let mut parents = std::collections::BTreeSet::new();
        for splice in &self.structure {
            if !parents.insert(splice.parent.clone())
                || self
                    .nodes
                    .iter()
                    .any(|n| splice.parent.starts_with(&n.path))
            {
                return Err(error("绘制结构更新路径重复或重叠"));
            }
            let nodes = children(list, &splice.parent)?;
            let end = splice
                .start
                .checked_add(splice.delete)
                .filter(|end| *end <= nodes.len())
                .ok_or_else(|| error("结构更新范围越界"))?;
            let mut used = std::collections::BTreeSet::new();
            for source in &splice.insert {
                if let ChildSource::Existing(index) = source {
                    if *index < splice.start || *index >= end || !used.insert(*index) {
                        return Err(error("结构更新对象引用无效或重复"));
                    }
                }
            }
        }
        Ok(())
    }
    pub(super) fn apply_semantic(
        properties: Vec<PropertyUpdate>,
        mut structure: Vec<ChildrenUpdate>,
        list: &mut DisplayList,
    ) {
        for property in properties {
            fields(
                node_at_mut(&mut list.nodes, &property.path),
                &property.values,
                true,
            )
            .unwrap();
        }
        structure.sort_by_key(|s| std::cmp::Reverse(s.parent.len()));
        for splice in structure {
            let nodes = if splice.parent.is_empty() {
                &mut list.nodes
            } else {
                let DrawKind::Group { children, .. } =
                    &mut node_at_mut(&mut list.nodes, &splice.parent).kind
                else {
                    unreachable!()
                };
                children
            };
            let mut removed: Vec<_> = nodes
                .drain(splice.start..splice.start + splice.delete)
                .map(Some)
                .collect();
            let insert = splice.insert.into_iter().map(|source| match source {
                ChildSource::New(n) => n,
                ChildSource::Existing(index) => removed[index - splice.start].take().unwrap(),
            });
            nodes.splice(splice.start..splice.start, insert);
        }
    }
    pub fn changes_index(&self) -> bool {
        self.roots.is_some()
            || !self.nodes.is_empty()
            || !self.structure.is_empty()
            || self
                .properties
                .iter()
                .any(|p| p.values.contains_key("name"))
    }
}
