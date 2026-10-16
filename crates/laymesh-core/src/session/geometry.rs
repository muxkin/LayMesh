//! Geometry derivations exclude paint and placement. Keys retain units and the
//! actual command values; diagnostics are evaluated again after input changes.
use super::*;
use crate::model::{Args, Object, V};
use kurbo::BezPath;
use serde_json::{Value as J, json};
type Shape = (BezPath, Option<(f64, f64)>, J);
#[derive(Default)]
pub(crate) struct GeometryCache {
    entries: HashMap<String, (Shape, usize, u64)>,
    paths: HashMap<[u8; 32], BezPath>,
    bytes: usize,
    tick: u64,
}
impl GeometryCache {
    pub fn bytes(&self) -> usize {
        self.bytes
            + self
                .paths
                .values()
                .map(|p| p.elements().len() * std::mem::size_of::<kurbo::PathEl>())
                .sum::<usize>()
    }
    pub fn retain(&mut self, live: &std::collections::HashSet<super::lifetime::Key>) {
        self.paths
            .retain(|key, _| live.contains(&super::lifetime::Key::Path(*key)));
        self.entries
            .retain(|key, _| live.contains(&super::lifetime::Key::Geometry(key.clone())));
        self.bytes = self.entries.values().map(|(_, bytes, _)| *bytes).sum();
    }
}
pub(crate) fn memo_path(key: [u8; 32], compute: impl FnOnce() -> BezPath) -> BezPath {
    let Some(state) = active() else {
        return compute();
    };
    state
        .borrow_mut()
        .lifetime
        .touch(super::lifetime::Key::Path(key));
    if let Some(path) = state.borrow().geometry.paths.get(&key) {
        return path.clone();
    }
    let path = compute();
    state.borrow_mut().geometry.paths.insert(key, path.clone());
    path
}
pub(crate) fn value(v: &V, depth: usize) -> Option<J> {
    if depth > 64 {
        return None;
    }
    Some(match v {
        V::Object(object) => {
            let object = object.borrow();
            if !object.nodes.is_empty() || !object.layers.is_empty() {
                return None;
            }
            json!([
                "object",
                object.kind,
                object.file,
                object.loc,
                object
                    .args
                    .iter()
                    .map(|(k, v)| Some((k, value(v, depth + 1)?)))
                    .collect::<Option<BTreeMap<_, _>>>()?
            ])
        }
        V::List(a) => json!([
            "list",
            a.iter()
                .map(|v| value(v, depth + 1))
                .collect::<Option<Vec<_>>>()?
        ]),
        V::Dict(m) => json!([
            "dict",
            m.iter()
                .map(|(k, v)| Some((k, value(v, depth + 1)?)))
                .collect::<Option<Vec<_>>>()?
        ]),
        V::Map(m) => json!([
            "map",
            m.iter()
                .map(|(k, v)| Some((k, value(v, depth + 1)?)))
                .collect::<Option<Vec<_>>>()?
        ]),
        _ => super::queries::signature(v)?,
    })
}
fn origin_key(key: &str) -> bool {
    matches!(
        key,
        "__call_origin" | "__arg_locations" | "__placement_origin"
    )
}
pub(crate) fn semantic_value(v: &V, depth: usize) -> Option<J> {
    if depth > 64 {
        return None;
    }
    Some(match v {
        V::Object(object) => {
            let o = object.borrow();
            if !o.nodes.is_empty() || !o.layers.is_empty() {
                return None;
            }
            json!([
                "object",
                o.kind,
                semantic_value(&V::Map(o.args.clone()), depth + 1)?
            ])
        }
        V::Map(a) => json!([
            "map",
            a.iter()
                .filter(|(k, _)| !origin_key(k))
                .map(|(k, v)| Some((k, semantic_value(v, depth + 1)?)))
                .collect::<Option<Vec<_>>>()?
        ]),
        V::Dict(a) => json!([
            "dict",
            a.iter()
                .map(|(k, v)| Some((k, semantic_value(v, depth + 1)?)))
                .collect::<Option<Vec<_>>>()?
        ]),
        V::List(a) => json!([
            "list",
            a.iter()
                .map(|v| semantic_value(v, depth + 1))
                .collect::<Option<Vec<_>>>()?
        ]),
        _ => super::queries::signature(v)?,
    })
}
pub(crate) fn rebase_recipe(recipe: &mut J, args: &Args) {
    if let (Some(commands), Some(V::List(values))) = (
        recipe.get_mut("commands").and_then(J::as_array_mut),
        args.get("commands"),
    ) {
        for (command, value) in commands.iter_mut().zip(values) {
            if let V::Object(o) = value {
                let o = o.borrow();
                for key in ["__call_origin", "__arg_locations", "__placement_origin"] {
                    if let Some(value) = o.args.get(key) {
                        command[key] = value.json();
                    } else {
                        command.as_object_mut().unwrap().remove(key);
                    }
                }
            }
        }
    }
}
pub(crate) fn memo(
    engine: &Engine,
    object: &Object,
    args: &Args,
    size: [f64; 2],
    compute: impl FnOnce() -> Result<Shape>,
) -> Result<Shape> {
    let Some(state) = active() else {
        return compute();
    };
    let keys: &[&str] = match object.kind.as_str() {
        "rect" | "ellipse" => &["size", "border_radius"],
        "line" => &["dx", "dy", "length", "angle"],
        "path" => &["commands"],
        "polyline" | "polygon" => &["points"],
        "star" => &["points", "outer_radius", "inner_radius", "rotation"],
        "ring" => &["outer_radius", "inner_radius"],
        "arc" | "sector" => &["radius", "start", "end"],
        _ => return compute(),
    };
    let values = keys
        .iter()
        .map(|key| {
            Some((
                *key,
                args.get(*key)
                    .map(|v| semantic_value(v, 0))
                    .transpose_option()?,
            ))
        })
        .collect::<Option<BTreeMap<_, _>>>();
    let Some(values) = values else {
        return compute();
    };
    let geometry_size = matches!(object.kind.as_str(), "rect" | "ellipse").then_some(size);
    let key = json!([object.kind, engine.unit, engine.dpi, geometry_size, values]).to_string();
    {
        let mut state = state.borrow_mut();
        state
            .lifetime
            .touch(super::lifetime::Key::Geometry(key.clone()));
        state.geometry.tick += 1;
        let tick = state.geometry.tick;
        if let Some((result, _, used)) = state.geometry.entries.get_mut(&key) {
            *used = tick;
            let mut result = result.clone();
            rebase_recipe(&mut result.2, args);
            state.metrics.reused_geometry += 1;
            return Ok(result);
        }
        state.metrics.rebuilt_geometry += 1;
    }
    let result = compute()?;
    let bytes = key.len()
        + result.0.elements().len() * std::mem::size_of::<kurbo::PathEl>()
        + result.2.to_string().len()
        + 128;
    let mut state = state.borrow_mut();
    {
        let tick = state.geometry.tick;
        if let Some((_, old, _)) = state
            .geometry
            .entries
            .insert(key, (result.clone(), bytes, tick))
        {
            state.geometry.bytes -= old;
        }
        state.geometry.bytes += bytes;
    }
    Ok(result)
}
trait TransposeOption<T> {
    fn transpose_option(self) -> Option<Option<T>>;
}
impl<T> TransposeOption<T> for Option<Option<T>> {
    fn transpose_option(self) -> Option<Option<T>> {
        match self {
            None => Some(None),
            Some(v) => v.map(Some),
        }
    }
}
