use crate::{
    Diagnostic, Loc, Result,
    parser::{Expr, Stmt},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value as Json, json};
use std::{
    cell::RefCell,
    collections::BTreeMap,
    path::{Component, Path, PathBuf},
    rc::Rc,
};

pub const PT: f64 = 25.4 / 72.0;
pub type Args = BTreeMap<String, V>;
pub type Scope = Rc<RefCell<Environment>>;
#[derive(Default, Debug)]
pub struct Environment {
    pub values: BTreeMap<String, V>,
    pub readonly: Vec<String>,
    pub parent: Option<Scope>,
}
impl Environment {
    pub fn root() -> Scope {
        Rc::new(RefCell::new(Self::default()))
    }
    pub fn child(parent: &Scope) -> Scope {
        Rc::new(RefCell::new(Self {
            parent: Some(parent.clone()),
            ..Self::default()
        }))
    }
    pub fn get(s: &Scope, key: &str) -> Option<V> {
        let x = s.borrow();
        x.values
            .get(key)
            .cloned()
            .or_else(|| x.parent.as_ref().and_then(|p| Self::get(p, key)))
    }
    pub fn set(s: &Scope, key: String, value: V) -> bool {
        if s.borrow().values.contains_key(&key) {
            if s.borrow().readonly.contains(&key) {
                return false;
            }
            s.borrow_mut().values.insert(key, value);
            return true;
        }
        let parent = s.borrow().parent.clone();
        if let Some(p) = parent {
            if Self::get(&p, &key).is_some() {
                return Self::set(&p, key, value);
            }
        }
        s.borrow_mut().values.insert(key, value);
        true
    }
}
#[derive(Clone, Debug)]
pub struct AnchorReference {
    pub instance: usize,
    pub name: String,
}
#[derive(Clone, Debug)]
pub enum V {
    Null,
    Number(f64, String),
    Bool(bool),
    Text(String, bool),
    Color(Rc<crate::color::Color>),
    List(Vec<V>),
    /// User dictionaries have insertion order and ordinary value semantics.
    Dict(indexmap::IndexMap<String, V>),
    Cmap(crate::colormap::Colormap),
    Map(Args),
    Object(Rc<RefCell<Object>>),
    Geometry(Rc<crate::geometry_query::Query>),
    /// A measured scalar/vector carries its selector for dependency replay.
    Measured(Rc<crate::geometry_query::Query>, Box<V>),
    Function {
        params: Vec<(String, Option<Expr>)>,
        body: Vec<Stmt>,
        scope: Scope,
        file: String,
    },
    Anchor {
        owner: usize,
        x: f64,
        y: f64,
        /// A named anchor retains identity even when another instance happens
        /// to have the same coordinates. Replaying a group resolves it again.
        reference: Option<AnchorReference>,
    },
}
#[derive(Clone, Debug)]
pub struct Object {
    pub kind: String,
    pub args: Args,
    pub file: String,
    pub loc: Loc,
    pub id: usize,
    pub nodes: Vec<Json>,
    pub layers: Vec<(String, Args)>,
    pub sealed: bool,
    pub width: f64,
    pub height: f64,
    pub node: Option<Json>,
    pub parent: usize,
}
impl V {
    pub fn value(&self) -> &V {
        match self {
            Self::Measured(_, v) => v.value(),
            _ => self,
        }
    }

    pub fn text(s: impl Into<String>) -> Self {
        Self::Text(s.into(), false)
    }
    pub fn num(n: f64) -> Self {
        Self::Number(n, String::new())
    }
    pub fn mm(n: f64) -> Self {
        Self::Number(n, "mm".into())
    }
    pub fn as_str(&self) -> String {
        match self.value() {
            Self::Text(s, _) => s.clone(),
            Self::Color(c) => c.css(),
            Self::Number(v, _) => v.to_string(),
            Self::Bool(b) => b.to_string(),
            Self::Null => "null".into(),
            _ => String::new(),
        }
    }
    pub fn number(&self) -> Option<f64> {
        if let Self::Number(n, _) = self.value() {
            Some(*n)
        } else {
            None
        }
    }
    pub fn list(&self) -> Vec<V> {
        if let Self::List(v) = self.value() {
            v.clone()
        } else {
            vec![]
        }
    }
    pub fn object(&self) -> Option<Rc<RefCell<Object>>> {
        if let Self::Object(o) = self {
            Some(o.clone())
        } else {
            None
        }
    }
    pub fn json(&self) -> Json {
        match self.value() {
            Self::Null => Json::Null,
            Self::Number(n, _) => {
                if n.fract() == 0. && n.abs() <= 9_007_199_254_740_991. {
                    json!(*n as i64)
                } else {
                    json!(n)
                }
            }
            Self::Bool(b) => json!(b),
            Self::Color(c) => json!(c.css()),
            Self::Text(s, raw) => {
                if *raw {
                    json!({"value":s,"raw":true})
                } else {
                    json!(s)
                }
            }
            Self::List(v) => json!(v.iter().map(Self::json).collect::<Vec<_>>()),
            Self::Dict(m) => Json::Object(m.iter().map(|(k, v)| (k.clone(), v.json())).collect()),
            Self::Cmap(c) => json!(c.name()),
            Self::Map(m) => json!(
                m.iter()
                    .map(|(k, v)| (k, v.json()))
                    .collect::<BTreeMap<_, _>>()
            ),
            Self::Object(o) => {
                let o = o.borrow();
                let mut j = args_json(&o.args);
                j["kind"] = json!(o.kind);
                j["file"] = json!(o.file);
                j["loc"] = json!(o.loc);
                j
            }
            _ => Json::Null,
        }
    }
    pub fn from_json(v: &Json) -> Self {
        match v {
            Json::Null => Self::Null,
            Json::Bool(b) => Self::Bool(*b),
            Json::Number(n) => Self::num(n.as_f64().unwrap_or(f64::NAN)),
            Json::String(s) => Self::text(s),
            Json::Array(a) => Self::List(a.iter().map(Self::from_json).collect()),
            Json::Object(m) => Self::Map(
                m.iter()
                    .map(|(k, v)| (k.clone(), Self::from_json(v)))
                    .collect(),
            ),
        }
    }
}
pub fn args_json(args: &Args) -> Json {
    json!(
        args.iter()
            .map(|(k, v)| (k, v.json()))
            .collect::<BTreeMap<_, _>>()
    )
}
pub fn string(a: &Args, k: &str, default: &str) -> String {
    let s = a.get(k).map(V::as_str).unwrap_or_else(|| default.into());
    if k.ends_with("color") || matches!(k, "fill" | "background") {
        crate::color::css(&s)
    } else {
        s
    }
}
pub fn num(a: &Args, k: &str, default: f64) -> f64 {
    a.get(k).and_then(V::number).unwrap_or(default)
}
pub fn yes(a: &Args, k: &str, default: bool) -> bool {
    match a.get(k) {
        Some(V::Bool(v)) => *v,
        _ => default,
    }
}
pub fn array(a: &Args, k: &str) -> Vec<V> {
    a.get(k).map(V::list).unwrap_or_default()
}
pub fn nums(a: &Args, k: &str) -> Vec<f64> {
    array(a, k)
        .iter()
        .map(|v| v.number().unwrap_or(f64::NAN))
        .collect()
}
pub fn value_length(v: &V, unit: &str, dpi: f64) -> Option<f64> {
    if let V::Number(n, u) = v.value() {
        let u = if u.is_empty() { unit } else { u };
        Some(
            *n * match u {
                "mm" => 1.0,
                "cm" => 10.0,
                "in" | "inch" => 25.4,
                "pt" => PT,
                "px" => 25.4 / dpi,
                _ => return None,
            },
        )
    } else {
        None
    }
}
pub fn length(a: &Args, k: &str, default: f64, unit: &str, dpi: f64) -> f64 {
    a.get(k)
        .and_then(|v| value_length(v, unit, dpi))
        .unwrap_or(default)
}
pub fn pair(a: &Args, k: &str, default: [f64; 2], unit: &str, dpi: f64) -> [f64; 2] {
    let v = array(a, k);
    if v.len() == 2 {
        [
            value_length(&v[0], unit, dpi).unwrap_or(default[0]),
            value_length(&v[1], unit, dpi).unwrap_or(default[1]),
        ]
    } else {
        default
    }
}
pub fn normalize_path(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            _ => out.push(c.as_os_str()),
        }
    }
    out
}
pub fn resolve(file: &str, relative: &str) -> String {
    normalize_path(
        &Path::new(file)
            .parent()
            .unwrap_or(Path::new("."))
            .join(relative),
    )
    .to_string_lossy()
    .into_owned()
}

#[derive(Clone, Default)]
pub struct Host {
    pub files: BTreeMap<String, Vec<u8>>,
    pub native: bool,
}
impl Host {
    pub fn read(&self, path: &str, file: &str, loc: Loc) -> Result<Vec<u8>> {
        self.files
            .get(path)
            .cloned()
            .or_else(|| {
                if self.native {
                    std::fs::read(path).ok()
                } else {
                    None
                }
            })
            .ok_or_else(|| Diagnostic::new("E_ASSET", format!("无法读取资源：{path}"), file, loc))
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FontAsset {
    pub data: Vec<u8>,
    pub index: u32,
    pub family: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scene {
    pub schema_version: u32,
    pub width: f64,
    pub height: f64,
    pub background: Json,
    pub layout_dpi: f64,
    pub export_dpi: f64,
    pub nodes: Vec<Json>,
    pub warnings: Vec<Diagnostic>,
    #[serde(default)]
    pub fonts: BTreeMap<String, FontAsset>,
}
pub fn base(kind: &str, w: f64, h: f64) -> Json {
    json!({"kind":kind,"id":"","x":0.0,"y":0.0,"width":w,"height":h,"rotation":0.0,"opacity":1.0})
}
pub fn jnum(n: &Json, k: &str, d: f64) -> f64 {
    n.get(k).and_then(Json::as_f64).unwrap_or(d)
}
pub fn jstr<'a>(n: &'a Json, k: &str, d: &'a str) -> &'a str {
    n.get(k).and_then(Json::as_str).unwrap_or(d)
}
pub fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
pub fn anchor(name: &str) -> Option<[f64; 2]> {
    Some(match name {
        "top_left" => [0., 0.],
        "top_center" => [0.5, 0.],
        "top_right" => [1., 0.],
        "middle_left" => [0., 0.5],
        "center" => [0.5, 0.5],
        "middle_right" => [1., 0.5],
        "bottom_left" => [0., 1.],
        "bottom_center" => [0.5, 1.],
        "bottom_right" => [1., 1.],
        _ => return None,
    })
}
pub fn node_anchor(n: &Json, name: &str) -> [f64; 2] {
    let f = anchor(name).unwrap_or([0., 0.]);
    let (w, h, r) = (
        jnum(n, "width", 0.),
        jnum(n, "height", 0.),
        jnum(n, "rotation", 0.).to_radians(),
    );
    let (bw, bh) = (
        w * r.cos().abs() + h * r.sin().abs(),
        h * r.cos().abs() + w * r.sin().abs(),
    );
    [
        jnum(n, "x", 0.) + (w - bw) / 2. + f[0] * bw,
        jnum(n, "y", 0.) + (h - bh) / 2. + f[1] * bh,
    ]
}
