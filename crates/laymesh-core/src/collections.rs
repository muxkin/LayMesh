//! Value-based dictionaries, snapshot iteration, and palette method dispatch.
use crate::{
    Loc, Result,
    engine::{API, Engine},
    model::*,
    parser::{Expr, ExprKind, Pattern},
};
use indexmap::IndexMap;
use std::rc::Rc;
impl Engine {
    pub(crate) fn key(&self, v: &V, l: Loc) -> Result<String> {
        match v.value() {
            V::Text(s, _) => Ok(s.clone()),
            _ => Err(self.error(
                "E_TYPE",
                "字典键必须为字符串 / dictionary keys must be strings",
                l,
            )),
        }
    }
    pub(crate) fn sequence(&self, v: V, l: Loc) -> Result<Vec<V>> {
        let items = match v {
            V::List(v) => v,
            V::Dict(d) => d.keys().map(V::text).collect(),
            V::Geometry(q) => self.geometry_items(&q, l)?,
            _ => {
                return Err(self.error(
                    "E_TYPE",
                    "需要列表、字典或几何集合 / expected iterable",
                    l,
                ));
            }
        };
        if items.len() > 10_000 {
            return Err(self.error("E_LIMIT", "循环次数超过 10,000", l));
        }
        Ok(items)
    }
    pub(crate) fn bind_pattern(&self, p: &Pattern, value: V, l: Loc) -> Result<Vec<(String, V)>> {
        match p {
            Pattern::Name(n) => Ok(if n == "_" {
                vec![]
            } else {
                vec![(n.clone(), value)]
            }),
            Pattern::Tuple(ps) => {
                let V::List(vs) = value else {
                    return Err(self.error("E_TYPE", "循环解包需要列表或元组", l));
                };
                if ps.len() != vs.len() {
                    return Err(self.error("E_ARG", "循环解包数量不匹配", l));
                }
                let mut out = vec![];
                for (p, v) in ps.iter().zip(vs) {
                    out.extend(self.bind_pattern(p, v, l)?);
                }
                Ok(out)
            }
        }
    }
    pub(crate) fn set_index(
        &mut self,
        target: &Expr,
        value: &Expr,
        scope: &Scope,
        l: Loc,
    ) -> Result<()> {
        fn path<'a>(e: &'a Expr, keys: &mut Vec<&'a Expr>) -> Option<&'a str> {
            match &e.kind {
                ExprKind::Ref(ps) if ps.len() == 1 => Some(&ps[0]),
                ExprKind::Index(a, b) => {
                    let root = path(a, keys)?;
                    keys.push(b);
                    Some(root)
                }
                _ => None,
            }
        }
        let mut indices = vec![];
        let root = path(target, &mut indices)
            .ok_or_else(|| self.error("E_TYPE", "索引赋值需要字典变量", l))?;
        let mut keys = vec![];
        for e in indices {
            let v = self.eval(e, scope)?;
            keys.push(self.key(&v, e.loc)?);
        }
        let new = self.eval(value, scope)?;
        let mut old = Environment::get(scope, root)
            .ok_or_else(|| self.error("E_NAME", format!("未知名称 {root}"), l))?;
        fn replace(e: &Engine, v: &mut V, keys: &[String], new: V, l: Loc) -> Result<()> {
            let V::Dict(d) = v else {
                return Err(e.error("E_TYPE", "索引赋值需要字典", l));
            };
            if keys.len() == 1 {
                d.insert(keys[0].clone(), new);
            } else {
                let next = d
                    .get_mut(&keys[0])
                    .ok_or_else(|| e.error("E_INDEX", format!("不存在字典键 {}", keys[0]), l))?;
                replace(e, next, &keys[1..], new, l)?;
            }
            Ok(())
        }
        replace(self, &mut old, &keys, new, l)?;
        if !Environment::set(scope, root.into(), old) {
            return Err(self.error("E_BINDING", "导入的名字不可赋值", l));
        }
        Ok(())
    }
    fn value_args(&self, name: &str, pos: &[V], mut a: Args, l: Loc) -> Result<Args> {
        self.validate_args(name, &mut a, pos, l)?;
        a.retain(|k, _| !k.starts_with("__"));
        let api = API["api"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["name"] == name)
            .unwrap();
        for (key, v) in api["positional"].as_array().into_iter().flatten().zip(pos) {
            let key = key.as_str().unwrap();
            if a.insert(key.into(), v.clone()).is_some() {
                return Err(self.error("E_ARG", format!("重复参数 {key}"), l));
            }
        }
        for key in api["required"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|k| k.as_str())
        {
            if !a.contains_key(key) {
                return Err(self.error("E_ARG", format!("缺少参数 {key}"), l));
            }
        }
        Ok(a)
    }
    fn color_count(&self, a: &Args, l: Loc) -> Result<Option<usize>> {
        match a.get("n") {
            None | Some(V::Null) => Ok(None),
            Some(v) => {
                let n = self.scalar(v, l)?;
                if n.fract() != 0. || !(0. ..=10_000.).contains(&n) {
                    return Err(self.error("E_ARG", "n 需要 0–10,000 的整数", l));
                }
                Ok(Some(n as usize))
            }
        }
    }
    pub(crate) fn value_method(
        &self,
        receiver: V,
        name: &str,
        pos: Vec<V>,
        a: Args,
        l: Loc,
    ) -> Result<V> {
        match receiver {
            V::Dict(mut d) => {
                let a = self.value_args(&format!("dict.{name}"), &pos, a, l)?;
                Ok(match name {
                    "keys" => V::List(d.keys().map(V::text).collect()),
                    "values" => V::List(d.values().cloned().collect()),
                    "items" => V::List(
                        d.iter()
                            .map(|(k, v)| V::List(vec![V::text(k), v.clone()]))
                            .collect(),
                    ),
                    "get" => d
                        .get(&self.key(&a["key"], l)?)
                        .cloned()
                        .unwrap_or_else(|| a.get("default").cloned().unwrap_or(V::Null)),
                    "update" => {
                        let V::Dict(other) = &a["other"] else {
                            return Err(self.error("E_TYPE", "update 需要字典", l));
                        };
                        for (k, v) in other {
                            d.insert(k.clone(), v.clone());
                        }
                        V::Dict(d)
                    }
                    _ => return Err(self.error("E_CALL", "未知字典方法", l)),
                })
            }
            V::Cmap(cm) => {
                let a = self.value_args(&format!("cmap.{name}"), &pos, a, l)?;
                match name {
                    "sample" => cm
                        .sample(self.scalar(&a["t"], l)?)
                        .map(|c| V::Color(Rc::new(c))),
                    "colors" => cm
                        .colors(self.color_count(&a, l)?)
                        .map(|v| V::List(v.into_iter().map(|c| V::Color(Rc::new(c))).collect())),
                    "reversed" => Ok(V::Cmap(cm.reversed())),
                    _ => Err("未知 cmap 方法"),
                }
                .map_err(|m| self.error("E_COLOR", m, l))
            }
            _ => self.geometry_method(receiver, name, pos, a, l),
        }
    }
    pub(crate) fn collection_builtin(
        &mut self,
        name: &str,
        pos: &[V],
        a: Args,
        l: Loc,
    ) -> Option<Result<V>> {
        if !["dict", "enumerate", "zip", "cmap", "cmap_names", "palette"].contains(&name) {
            return None;
        }
        Some((|| {
            if name == "zip" {
                if a.keys().any(|k| !k.starts_with("__")) {
                    return Err(self.error("E_ARG", "zip 只接受位置参数", l));
                }
                let seq = pos
                    .iter()
                    .map(|v| self.sequence(v.clone(), l))
                    .collect::<Result<Vec<_>>>()?;
                let n = seq.iter().map(Vec::len).min().unwrap_or(0);
                return Ok(V::List(
                    (0..n)
                        .map(|i| V::List(seq.iter().map(|s| s[i].clone()).collect()))
                        .collect(),
                ));
            }
            let a = self.value_args(name, pos, a, l)?;
            Ok(match name {
                "dict" => {
                    if let Some(src) = a.get("src") {
                        let path = resolve(&self.file, &self.key(src, l)?);
                        let bytes = self
                            .host
                            .read(&path, &self.file, l)
                            .map_err(|e| self.error("E_DATA", e.message, l))?;
                        crate::data::load("dict", &path, &bytes, &self.file, l)?
                    } else {
                        V::Dict(IndexMap::new())
                    }
                }
                "enumerate" => {
                    let start = a
                        .get("start")
                        .map(|v| self.scalar(v, l))
                        .transpose()?
                        .unwrap_or(0.);
                    let values = self.sequence(a["seq"].clone(), l)?;
                    let end = start + values.len().saturating_sub(1) as f64;
                    if start.fract() != 0.
                        || start.abs() > 9_007_199_254_740_991.
                        || end.abs() > 9_007_199_254_740_991.
                    {
                        return Err(self.error("E_ARG", "enumerate 编号需要可精确表示的整数", l));
                    }
                    V::List(
                        values
                            .into_iter()
                            .enumerate()
                            .map(|(i, v)| V::List(vec![V::num(start + i as f64), v]))
                            .collect(),
                    )
                }
                "cmap_names" => {
                    let cat = a
                        .get("category")
                        .filter(|v| !matches!(v, V::Null))
                        .map(|v| self.key(v, l))
                        .transpose()?;
                    if a.get("reversed").is_some_and(|v| !matches!(v, V::Bool(_))) {
                        return Err(self.error("E_TYPE", "reversed 需要布尔值", l));
                    }
                    V::List(
                        crate::colormap::names(cat.as_deref(), yes(&a, "reversed", false))
                            .map_err(|m| self.error("E_COLOR", m, l))?
                            .into_iter()
                            .map(V::text)
                            .collect(),
                    )
                }
                "cmap" | "palette" => {
                    let preset_name = a
                        .get("name")
                        .map(|v| self.key(v, l))
                        .transpose()?
                        .unwrap_or_else(|| {
                            if name == "palette" {
                                "tab10".into()
                            } else {
                                "viridis".into()
                            }
                        });
                    let cm = crate::colormap::Colormap::get(&preset_name).ok_or_else(|| {
                        self.error("E_COLOR", format!("未知配色 {preset_name}"), l)
                    })?;
                    if name == "palette" {
                        V::List(
                            cm.colors(self.color_count(&a, l)?)
                                .map_err(|m| self.error("E_COLOR", m, l))?
                                .into_iter()
                                .map(|c| V::Color(Rc::new(c)))
                                .collect(),
                        )
                    } else {
                        V::Cmap(cm)
                    }
                }
                _ => unreachable!(),
            })
        })())
    }
}
