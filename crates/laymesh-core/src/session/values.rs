//! Cached definitions contain immutable copies, never an engine's mutable
//! object handles. Source origins are rebased without rebuilding geometry.
use super::*;
use crate::model::{Args, Object, V};

// Metadata maps have ordinary value semantics but contain no mutable handles
// or source origins that need rebasing. Copy-on-write preserves that contract.
pub(crate) fn immutable_data(v: &V) -> bool {
    match v {
        V::Null | V::Bool(_) | V::Number(..) | V::Text(..) | V::Color(_) => true,
        V::Map(a) => a.values().all(immutable_data),
        V::List(a) => a.iter().all(immutable_data),
        V::Dict(a) => a.values().all(immutable_data),
        _ => false,
    }
}

pub(crate) fn immutable_kind(kind: &str) -> bool {
    !matches!(
        kind,
        "canvas" | "document" | "page" | "group" | "plot" | "instance" | "step" | "plotLayer"
    )
}
fn immutable(o: &Object) -> bool {
    immutable_kind(&o.kind)
        && o.nodes.is_empty()
        && o.layers.is_empty()
        && o.node.is_none()
        && o.syntax_site != 0
}
pub(crate) fn current(value: &V, syntax: &SyntaxCache) -> bool {
    match value {
        V::Object(o) => {
            let o = o.borrow();
            o.origin_revision != 0
                && syntax
                    .origin(o.syntax_site)
                    .is_some_and(|s| s.revision == o.origin_revision)
                && o.origin_dependencies.iter().all(|(site, revision)| {
                    syntax
                        .origin(*site)
                        .is_some_and(|s| s.revision == *revision)
                })
        }
        V::List(a) => a.iter().all(|v| current(v, syntax)),
        V::Map(a) => a.values().all(|v| current(v, syntax)),
        V::Dict(a) => a.values().all(|v| current(v, syntax)),
        _ => true,
    }
}
fn origin_dependencies(args: &Args) -> crate::model::Shared<Vec<(u64, u64)>> {
    fn collect(v: &V, origins: &mut Vec<(u64, u64)>) {
        match v {
            V::Object(o) => {
                let o = o.borrow();
                origins.push((o.syntax_site, o.origin_revision));
                origins.extend(o.origin_dependencies.iter().copied());
            }
            V::List(a) => {
                for v in a {
                    collect(v, origins)
                }
            }
            V::Map(a) => {
                for v in a.values() {
                    collect(v, origins)
                }
            }
            V::Dict(a) => {
                for v in a.values() {
                    collect(v, origins)
                }
            }
            _ => {}
        }
    }
    let mut origins = vec![];
    for v in args.values() {
        collect(v, &mut origins);
    }
    origins.sort_unstable();
    origins.dedup();
    origins.into()
}
pub(crate) fn freeze(value: &V, syntax: &SyntaxCache) -> Option<V> {
    fn copy(v: &V, syntax: &SyntaxCache, seen: &mut HashMap<usize, V>, depth: usize) -> Option<V> {
        if depth > 128 {
            return None;
        }
        Some(match v {
            V::Null | V::Bool(_) | V::Number(..) | V::Text(..) | V::Color(_) => v.clone(),
            V::List(a) => V::List(
                a.iter()
                    .map(|v| copy(v, syntax, seen, depth + 1))
                    .collect::<Option<_>>()?,
            ),
            V::Map(_) if immutable_data(v) => v.clone(),
            V::Map(a) => V::Map(
                a.iter()
                    .map(|(k, v)| Some((k.clone(), copy(v, syntax, seen, depth + 1)?)))
                    .collect::<Option<_>>()?,
            ),
            V::Dict(a) => V::Dict(
                a.iter()
                    .map(|(k, v)| Some((k.clone(), copy(v, syntax, seen, depth + 1)?)))
                    .collect::<Option<_>>()?,
            ),
            V::Object(object) => {
                let o = object.borrow();
                if !immutable(&o) {
                    return None;
                }
                if current(v, syntax) {
                    return Some(v.clone());
                }
                let identity = Rc::as_ptr(object) as usize;
                if let Some(v) = seen.get(&identity) {
                    return Some(v.clone());
                }
                let mut object = o.clone();
                object.origin_revision = syntax.origin(o.syntax_site)?.revision;
                object.args = o
                    .args
                    .iter()
                    .map(|(k, v)| Some((k.clone(), copy(v, syntax, seen, depth + 1)?)))
                    .collect::<Option<_>>()?;
                object.origin_dependencies = origin_dependencies(&object.args);
                let v = V::Object(Rc::new(RefCell::new(object)));
                seen.insert(identity, v.clone());
                v
            }
            _ => return None,
        })
    }
    copy(value, syntax, &mut HashMap::new(), 0)
}
pub(crate) fn available(value: &V, syntax: &SyntaxCache) -> bool {
    match value {
        V::Object(o) => {
            let o = o.borrow();
            syntax.origin(o.syntax_site).is_some() && o.args.values().all(|v| available(v, syntax))
        }
        V::List(a) => a.iter().all(|v| available(v, syntax)),
        V::Map(a) => a.values().all(|v| available(v, syntax)),
        V::Dict(a) => a.values().all(|v| available(v, syntax)),
        _ => true,
    }
}
pub(crate) fn thaw(value: &V, syntax: &SyntaxCache) -> V {
    fn restore(v: &V, syntax: &SyntaxCache, seen: &mut HashMap<usize, V>) -> V {
        match v {
            V::Object(object) => {
                if current(v, syntax) {
                    return v.clone();
                }
                let identity = Rc::as_ptr(object) as usize;
                let o = object.borrow();
                if let Some(v) = seen.get(&identity) {
                    return v.clone();
                }
                let origin = syntax.origin(o.syntax_site).unwrap();
                let mut args: Args = o
                    .args
                    .iter()
                    .map(|(k, v)| (k.clone(), restore(v, syntax, seen)))
                    .collect();
                if args.contains_key("__call_origin") {
                    args.insert(
                        "__call_origin".into(),
                        V::from_json(&serde_json::json!({"file":origin.file,"loc":origin.loc})),
                    );
                }
                if args.contains_key("__arg_locations") {
                    args.insert(
                        "__arg_locations".into(),
                        V::Map(
                            origin
                                .arguments
                                .iter()
                                .map(|(name, loc)| {
                                    (
                                        name.clone(),
                                        V::from_json(
                                            &serde_json::json!({"file":origin.file,"loc":loc}),
                                        ),
                                    )
                                })
                                .collect(),
                        ),
                    );
                }
                let mut object = o.clone();
                object.file = origin.file.clone();
                object.loc = origin.loc;
                object.args = args.into();
                object.origin_revision = origin.revision;
                object.origin_dependencies = origin_dependencies(&object.args);
                let result = V::Object(Rc::new(RefCell::new(object)));
                seen.insert(identity, result.clone());
                result
            }
            V::List(a) => V::List(a.iter().map(|v| restore(v, syntax, seen)).collect()),
            V::Map(_) if immutable_data(v) => v.clone(),
            V::Map(a) => V::Map(
                a.iter()
                    .map(|(k, v)| (k.clone(), restore(v, syntax, seen)))
                    .collect(),
            ),
            V::Dict(a) => V::Dict(
                a.iter()
                    .map(|(k, v)| (k.clone(), restore(v, syntax, seen)))
                    .collect(),
            ),
            _ => v.clone(),
        }
    }
    restore(value, syntax, &mut HashMap::new())
}
pub(crate) fn signature(object: &Object) -> Option<serde_json::Value> {
    if !immutable(object) {
        return None;
    }
    Some(serde_json::json!([
        "definition",
        object.kind,
        object.file,
        object.loc,
        object
            .args
            .iter()
            .map(|(k, v)| Some((k, super::queries::signature(v)?)))
            .collect::<Option<BTreeMap<_, _>>>()?
    ]))
}
