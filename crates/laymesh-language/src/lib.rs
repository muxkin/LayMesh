//! Static, non-evaluating language services shared by native LSP and WebAssembly.
mod bindings;
mod colors;
mod connections;
mod formatting;
mod index;
pub mod inspect;
pub mod lsp;
use index::{Symbol, Token, index, tokens};
use laymesh_core::{
    engine::{API, predefined_value, predefined_variables},
    parser::{self, Expr, ExprKind, Stmt, StmtKind},
};
use regex::Regex;
use serde_json::{Value as J, json};
use std::collections::{BTreeMap, BTreeSet};
use std::{
    cell::RefCell,
    hash::{Hash, Hasher},
};
#[derive(Default)]
pub struct LanguageService {
    pub documents: BTreeMap<String, String>,
    pub locale: String,
    indexes: RefCell<BTreeMap<String, (u64, bool, index::Index)>>,
    bindings: RefCell<Option<(u64, bindings::Graph)>>,
}
#[derive(Default)]
struct Context {
    name: String,
    receiver: String,
    parameter: String,
    used: Vec<String>,
    positional: usize,
    prefix: String,
}
pub(crate) fn resolve(uri: &str, relative: &str) -> String {
    if let Some(path) = uri.strip_prefix("file://") {
        // URI paths are independent of the host filesystem's separators.
        let (authority, path) = if path.starts_with('/') {
            ("", path)
        } else {
            path.split_once('/').unwrap_or((path, ""))
        };
        let relative = if cfg!(windows) {
            relative.replace('\\', "/")
        } else {
            relative.to_owned()
        };
        let mut encoded = String::new();
        for byte in relative.bytes() {
            if byte.is_ascii_alphanumeric() || b"-._~/:".contains(&byte) {
                encoded.push(byte as char);
            } else {
                encoded.push_str(&format!("%{byte:02X}"));
            }
        }
        let drive_path = relative.as_bytes().get(1) == Some(&b':')
            && relative
                .as_bytes()
                .first()
                .is_some_and(u8::is_ascii_alphabetic);
        let joined = if drive_path {
            format!("/{encoded}")
        } else if encoded.starts_with('/') {
            encoded
        } else {
            format!(
                "{}/{encoded}",
                path.rsplit_once('/')
                    .map(|(parent, _)| parent)
                    .unwrap_or("")
            )
        };
        let mut parts: Vec<&str> = Vec::new();
        for part in joined.split('/') {
            match part {
                "" | "." => {}
                ".." => {
                    if !(parts.len() == 1 && parts[0].ends_with(':')) {
                        parts.pop();
                    }
                }
                _ => parts.push(part),
            }
        }
        format!("file://{authority}/{}", parts.join("/"))
    } else {
        laymesh_core::model::resolve(uri, relative)
    }
}
fn strs(j: &J) -> Vec<String> {
    j.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}
fn entries() -> &'static Vec<J> {
    API["api"].as_array().unwrap()
}
fn entry(name: &str) -> Option<J> {
    entries().iter().find(|a| a["name"] == name).cloned()
}
fn desc<'a>(p: &'a J, en: bool) -> &'a str {
    p[if en { "descriptionEn" } else { "description" }]
        .as_str()
        .unwrap_or("")
}
fn link(name: &str, en: bool) -> String {
    format!(
        "[{}](https://muxkin.github.io/LayMesh/{}docs/topics/interface-reference{}.html#{})",
        if en { "Documentation" } else { "文档" },
        if en { "en/" } else { "" },
        if en { "" } else { ".zh-CN" },
        name.replace('.', "-")
    )
}
fn parameter_doc(p: &J, en: bool, owner: &str) -> String {
    let mut out = vec![
        format!(
            "**{}** — `{}`",
            p["name"].as_str().unwrap_or(""),
            parameter_type(p)
        ),
        desc(p, en).into(),
    ];
    for value in strs(&p["values"]) {
        let explanation = p["valueDescriptions"][&value][if en { "en" } else { "zh" }]
            .as_str()
            .unwrap_or("");
        out.push(format!("`{value}`: {explanation}"));
    }
    if let Some(parent) = p["defaultFrom"].as_str() {
        out.push(format!(
            "{} `{parent}`",
            if en {
                "Default inherits"
            } else {
                "默认继承"
            }
        ));
    }
    if p["unit"] != "none" {
        out.push(format!(
            "{}: {}",
            if en { "Unit" } else { "单位" },
            if p["unit"] == "geometry" {
                "figure unit (default mm)"
            } else {
                p["unit"].as_str().unwrap_or("")
            }
        ))
    }
    if let Some(d) = p["default"].as_str() {
        out.push(format!("{}: `{d}`", if en { "Default" } else { "默认" }))
    }
    if let Some(e) = p["example"].as_str() {
        out.push(format!("```lay\n{e}\n```"))
    }
    if !owner.is_empty() {
        out.push(link(owner, en));
    }
    out.into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}
fn parameter_type(p: &J) -> String {
    let values = strs(&p["values"]);
    if values.is_empty() {
        return p["type"].as_str().unwrap_or("value").into();
    }
    let mut typ = values
        .iter()
        .map(|v| format!("\"{v}\""))
        .collect::<Vec<_>>()
        .join(" | ");
    if p["name"] == "anchor" && values.contains(&"top_left".into()) {
        typ.push_str(" | self selector");
    }
    if p["type"].as_str().is_some_and(|t| t.contains("boolean")) {
        typ.push_str(" | boolean");
    }
    if p["name"] == "position" {
        typ.push_str(" | (length, length)");
    }
    typ
}
fn variable_doc(name: &str, value: &str) -> String {
    format!("```lay\n{name}: string = {}\n```", json!(value))
}
fn builtin(name: &str, en: bool) -> Option<J> {
    let e = entry(name)?;
    let mut names = strs(&e["positional"]);
    for p in e["parameters"].as_array().unwrap() {
        let n = p["name"].as_str().unwrap().to_string();
        if !names.contains(&n) {
            names.push(n)
        }
    }
    let ps:Vec<J>=names.iter().map(|n|{let p=e["parameters"].as_array().unwrap().iter().find(|p|p["name"]==*n).cloned().or_else(||entries().iter().flat_map(|a|a["parameters"].as_array().unwrap()).find(|p|p["name"]==*n).cloned()).unwrap_or(json!({"name":n,"type":"material","description":"需要放置的素材。","descriptionEn":"Material to place."}));let positional=strs(&e["positional"]).contains(n);let named=e["parameters"].as_array().unwrap().iter().any(|p|p["name"]==*n);json!({"name":n,"type":parameter_type(&p),"description":desc(&p,en),"unit":p["unit"],"example":p["example"],"documentation":parameter_doc(&p,en,name),"default":p["default"],"required":strs(&e["required"]).contains(n),"kind":if positional{if named{"either"}else{"positional"}}else{"named"},"label":[0,0]})}).collect();
    Some(
        json!({"name":name,"summary":desc(&e,en),"returns":e["returns"][if en{"en"}else{"zh"}],"documentation":format!("{}\n\n{}",desc(&e,en),link(name,en)),"parameters":ps}),
    )
}
fn authored(s: &Symbol, name: &str, en: bool) -> J {
    let d = s.doc.select(en);
    let ps:Vec<J>=s.parameters.iter().map(|p|{let n=p["name"].as_str().unwrap();let info=d.parameters.get(n).cloned().unwrap_or(json!({}));let default=p["default"].clone();json!({"name":n,"type":info["type"].as_str().unwrap_or("value"),"description":info["description"].as_str().unwrap_or(""),"default":default,"required":default.is_null(),"kind":"either","documentation":format!("**{n}**\n\n{}{}",info["description"].as_str().unwrap_or(""),default.as_str().map(|s|format!("\n\n{}: `{s}`",if en{"Default"}else{"默认"})).unwrap_or_default()),"label":[0,0]})}).collect();
    json!({"name":name,"summary":d.body.split("\n\n").next().unwrap_or(""),"documentation":s.doc.markdown(en),"parameters":ps})
}
fn word(source: &str, offset: usize) -> (usize, usize, String) {
    let mut a = offset.min(source.len());
    while a > 0
        && source.as_bytes()[a - 1].is_ascii()
        && (source.as_bytes()[a - 1].is_ascii_alphanumeric()
            || b"_-".contains(&source.as_bytes()[a - 1]))
    {
        a -= 1
    }
    let mut b = offset.min(source.len());
    while b < source.len()
        && (source.as_bytes()[b].is_ascii_alphanumeric() || b"_-".contains(&source.as_bytes()[b]))
    {
        b += 1
    }
    (a, b, source[a..b].into())
}
fn receiver_before(source: &str, offset: usize) -> Option<String> {
    let ts: Vec<Token> = tokens(&source[..offset.min(source.len())])
        .into_iter()
        .filter(|t| t.kind != "comment")
        .collect();
    if ts.last()?.text != "." {
        return None;
    }
    let end = ts.last()?.from;
    let mut depth = 0;
    let mut start = end;
    for t in ts[..ts.len() - 1].iter().rev() {
        if depth == 0 && start < end && source[t.to..start].contains('\n') {
            break;
        }
        match t.text.as_str() {
            ")" | "]" | "}" => depth += 1,
            "(" | "[" | "{" if depth > 0 => depth -= 1,
            "(" | "[" | "{" => break,
            _ if depth == 0 && t.text != "." && t.kind != "name" => break,
            _ => {}
        }
        if depth == 0 && start < end && source[t.to..start].contains('\n') {
            break;
        }
        start = t.from;
    }
    (start < end).then(|| source[start..end].trim().to_string())
}
fn expression(source: &str) -> Option<Expr> {
    let stmts = parser::parse(&format!("__probe={source}"), "<completion>").ok()?;
    match stmts.into_iter().next()?.kind {
        StmtKind::Bind(_, e, _) => Some(e),
        _ => None,
    }
}
fn type_info(typ: &str) -> &'static J {
    let typ = if typ == "table" { "dict" } else { typ };
    if API["valueTypes"].get(typ).is_some() {
        &API["valueTypes"][typ]
    } else {
        &API["geometryTypes"][typ]
    }
}
fn geometry_member_type(typ: &str, name: &str) -> Option<String> {
    type_info(typ)["members"][name].as_str().map(str::to_string)
}
fn geometry_surface(name: &str) -> Option<String> {
    entries()
        .iter()
        .find(|e| {
            e["geometryMethod"] == true
                && e["name"].as_str().unwrap().rsplit('.').next() == Some(name)
        })
        .and_then(|e| e["name"].as_str().map(str::to_string))
}
fn expression_type(e: &Expr, lookup: &impl Fn(&str) -> Option<String>) -> Option<String> {
    match &e.kind {
        ExprKind::Number(_, _) => Some("number".into()),
        ExprKind::String(_, _, _) => Some("string".into()),
        ExprKind::Bool(_) => Some("bool".into()),
        ExprKind::Unary(op, value) => {
            if op == "not" {
                Some("bool".into())
            } else {
                expression_type(value, lookup)
            }
        }
        ExprKind::Binary(op, a, b) => {
            if ["==", "!=", "<", ">", "<=", ">=", "and", "or"].contains(&op.as_str()) {
                Some("bool".into())
            } else {
                let a = expression_type(a, lookup)?;
                let b = expression_type(b, lookup)?;
                (a == b).then_some(a)
            }
        }
        ExprKind::Dict(_) => Some("dict".into()),
        ExprKind::List(_) => Some("list".into()),
        ExprKind::Ref(ps) => {
            let mut typ = if ps[0] == "self" {
                "instance".into()
            } else {
                lookup(&ps[0])?
            };
            for member in &ps[1..] {
                typ = geometry_member_type(&typ, member)?;
            }
            Some(typ)
        }
        ExprKind::Member(receiver, name) => {
            geometry_member_type(&expression_type(receiver, lookup)?, name)
        }
        ExprKind::Index(receiver, _) => type_info(&expression_type(receiver, lookup)?)["index"]
            .as_str()
            .map(str::to_string),
        ExprKind::Method(receiver, name, _) => {
            type_info(&expression_type(receiver, lookup)?)["methods"][name]
                .as_str()
                .map(str::to_string)
        }
        ExprKind::Call(ps, _) => {
            if ps.len() == 1 {
                return Some(
                    entry(&ps[0])
                        .and_then(|a| a["returnType"].as_str().map(str::to_string))
                        .unwrap_or_else(|| ps[0].clone()),
                );
            }
            let mut typ = if ps[0] == "self" {
                "instance".into()
            } else {
                lookup(&ps[0])?
            };
            for member in &ps[1..ps.len() - 1] {
                typ = geometry_member_type(&typ, member)?;
            }
            let method = ps.last()?;
            if method == "add" || method == "fuse" {
                return Some("instance".into());
            }
            type_info(&typ)["methods"][method]
                .as_str()
                .map(str::to_string)
                .or_else(|| matches!(method.as_str(), "data" | "axis").then(|| "anchor".into()))
        }
    }
}
fn context(source: &str, offset: usize) -> Option<Context> {
    let ts: Vec<Token> = tokens(&source[..offset.min(source.len())])
        .into_iter()
        .filter(|t| t.kind != "comment")
        .collect();
    let mut stack = vec![];
    for (i, t) in ts.iter().enumerate() {
        if ["(", "[", "{"].contains(&t.text.as_str()) {
            stack.push(i)
        } else if [")", "]", "}"].contains(&t.text.as_str()) {
            let open = match t.text.as_str() {
                ")" => "(",
                "]" => "[",
                _ => "{",
            };
            if let Some(at) = stack.iter().rposition(|&j| ts[j].text == open) {
                stack.truncate(at)
            }
        }
    }
    let &at = stack.iter().rev().find(|&&i| {
        ts[i].text == "("
            && i > 0
            && ts[i - 1].kind == "name"
            && (i < 2 || ts[i - 2].text != "function")
    })?;
    let mut c = Context {
        name: ts[at - 1].text.clone(),
        ..Context::default()
    };
    if at >= 3 && ts[at - 2].text == "." {
        c.receiver = receiver_before(source, ts[at - 2].to).unwrap_or_default()
    }
    let mut depth = 0;
    let mut start = at + 1;
    for i in at + 1..ts.len() {
        let t = &ts[i];
        if ["(", "[", "{"].contains(&t.text.as_str()) {
            depth += 1
        } else if [")", "]", "}"].contains(&t.text.as_str()) {
            depth -= 1
        } else if depth == 0 {
            if t.text == "," {
                if c.parameter.is_empty() && i > start {
                    c.positional += 1
                }
                c.parameter.clear();
                start = i + 1;
            }
            if i == start && t.kind == "name" && ts.get(i + 1).is_some_and(|t| t.text == "=") {
                c.parameter = t.text.clone();
                c.used.push(t.text.clone());
            }
        }
    }
    if ts.len() == start + 1 && ts[start].kind == "name" {
        c.prefix = ts[start].text.clone()
    }
    Some(c)
}
impl LanguageService {
    pub fn new(locale: &str) -> Self {
        Self {
            documents: BTreeMap::new(),
            locale: locale.into(),
            ..Self::default()
        }
    }
    pub fn update(&mut self, uri: &str, text: &str) {
        self.documents.insert(uri.into(), text.into());
    }
    pub fn remove(&mut self, uri: &str) {
        self.documents.remove(uri);
    }
    fn en(&self) -> bool {
        !self.locale.to_lowercase().starts_with("zh")
    }
    fn source(&self, uri: &str) -> &str {
        self.documents.get(uri).map(String::as_str).unwrap_or("")
    }
    fn document_index(&self, uri: &str) -> index::Index {
        let source = self.source(uri);
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        source.hash(&mut hasher);
        let hash = hasher.finish();
        let en = self.en();
        let mut cached = self.indexes.borrow_mut();
        if !cached
            .get(uri)
            .is_some_and(|(h, e, _)| *h == hash && *e == en)
        {
            cached.insert(uri.into(), (hash, en, index(uri, source, en)));
        }
        cached[uri].2.clone()
    }
    fn binding_graph(&self) -> bindings::Graph {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.documents.hash(&mut hasher);
        self.locale.hash(&mut hasher);
        let hash = hasher.finish();
        if self
            .bindings
            .borrow()
            .as_ref()
            .is_none_or(|(old, _)| *old != hash)
        {
            let graph = bindings::Graph::build(self);
            *self.bindings.borrow_mut() = Some((hash, graph));
        }
        self.bindings.borrow().as_ref().unwrap().1.clone()
    }
    fn visible(&self, uri: &str, offset: usize) -> Vec<Symbol> {
        let mut symbols = self.document_index(uri).symbols;
        symbols.retain(|s| {
            s.scope_from <= offset
                && s.scope_to >= offset
                && (s.from <= offset || s.kind == "function")
        });
        symbols.sort_by_key(|s| (usize::MAX - (s.scope_to - s.scope_from), s.from));
        let mut out = BTreeMap::new();
        for s in symbols {
            out.insert(s.name.clone(), s);
        }
        out.into_values().collect()
    }
    fn resolve(
        &self,
        uri: &str,
        name: &str,
        offset: usize,
        seen: &mut BTreeSet<String>,
    ) -> Option<Symbol> {
        if !seen.insert(format!("{uri}:{name}:{offset}")) {
            return None;
        }
        if let Some(s) = self
            .visible(uri, offset)
            .into_iter()
            .find(|s| s.name == name)
        {
            if !s.alias.is_empty() && s.doc.body.is_empty() && s.doc.variants.is_empty() {
                if let Some(target) = self.resolve(uri, &s.alias, s.from.saturating_sub(1), seen) {
                    if target.kind == "function" {
                        return Some(target);
                    }
                }
            }
            return Some(s);
        }
        for imp in self.document_index(uri).imports {
            if imp.alias == name
                && imp.scope_from <= offset
                && imp.scope_to >= offset
                && imp.alias_from <= offset
            {
                let target = resolve(uri, &imp.source);
                let s = self
                    .document_index(&target)
                    .symbols
                    .into_iter()
                    .find(|s| s.exported && s.scope_from == 0 && s.name == imp.name)?;
                return if !s.alias.is_empty() {
                    self.resolve(&target, &s.alias, s.from.saturating_sub(1), seen)
                        .filter(|target| target.kind == "function")
                        .or(Some(s))
                } else {
                    Some(s)
                };
            }
        }
        None
    }
    fn named(&self, uri: &str, offset: usize) -> Vec<(String, Symbol)> {
        let mut out: BTreeMap<String, Symbol> = self
            .visible(uri, offset)
            .into_iter()
            .map(|s| (s.name.clone(), s))
            .collect();
        for imp in self.document_index(uri).imports {
            if !out.contains_key(&imp.alias) {
                if let Some(s) = self.resolve(uri, &imp.alias, offset, &mut BTreeSet::new()) {
                    out.insert(imp.alias, s);
                }
            }
        }
        out.into_iter().collect()
    }
    fn type_of(&self, uri: &str, name: &str, offset: usize) -> Option<String> {
        self.type_of_inner(uri, name, offset, &mut BTreeSet::new())
    }
    fn type_of_inner(
        &self,
        uri: &str,
        name: &str,
        offset: usize,
        active: &mut BTreeSet<String>,
    ) -> Option<String> {
        if !active.insert(format!("{uri}:{name}:{offset}")) {
            return None;
        }
        let symbol = self.resolve(uri, name, offset, &mut BTreeSet::new())?;
        let declared = symbol.doc.select(self.en()).typ;
        if !declared.is_empty() {
            return Some(declared);
        }
        if let Some(factory) =
            self.resolve(&symbol.uri, &symbol.typ, symbol.from, &mut BTreeSet::new())
        {
            if let Some(returns) = factory.doc.select(self.en()).returns {
                if let Some(typ) = returns["type"].as_str() {
                    return Some(typ.into());
                }
            }
        }
        if let Some(value) = &symbol.value {
            if let Some(inferred) = expression_type(value, &|other| {
                self.type_of_inner(
                    &symbol.uri,
                    other,
                    symbol.from.saturating_sub(1),
                    &mut active.clone(),
                )
            }) {
                return Some(inferred);
            }
        }
        if symbol.typ.is_empty() {
            let source = self.source(&symbol.uri);
            let tail = &source[symbol.to..];
            if let Some(rhs) = tail.trim_start().strip_prefix('=') {
                if let Some(expr) = expression(rhs.split('\n').next().unwrap_or("")) {
                    let inferred = expression_type(&expr, &|other| {
                        if other == name {
                            None
                        } else {
                            self.type_of_inner(&symbol.uri, other, symbol.from, &mut active.clone())
                        }
                    });
                    if inferred.is_some() {
                        return inferred;
                    }
                }
            }
        }
        Some(symbol.typ)
    }
    fn receiver_type(&self, uri: &str, receiver: &str, offset: usize) -> Option<String> {
        expression_type(&expression(receiver)?, &|name| {
            self.type_of(uri, name, offset)
        })
    }
    fn classes(&self) -> BTreeSet<String> {
        let pattern = Regex::new(r"\.([A-Za-z_][\w-]*)\s*(?:[.{,:>])").unwrap();
        self.documents
            .values()
            .flat_map(|source| pattern.captures_iter(source).map(|c| c[1].to_string()))
            .collect()
    }
    fn callable(&self, uri: &str, offset: usize, c: &Context) -> Option<J> {
        if c.receiver.is_empty() {
            if let Some(s) = self.resolve(uri, &c.name, offset, &mut BTreeSet::new()) {
                return if s.kind == "function" {
                    Some(authored(&s, &c.name, self.en()))
                } else {
                    None
                };
            }
        }
        let mut name = c.name.clone();
        if !c.receiver.is_empty() {
            if let Some(typ) = self.receiver_type(uri, &c.receiver, offset) {
                if let Some(prefix) = type_info(&typ)["methodPrefix"].as_str() {
                    if type_info(&typ)["methods"][&name].is_string() {
                        return builtin(&format!("{prefix}.{name}"), self.en());
                    }
                }
                if ["plot", "polar_plot", "radar_plot"].contains(&typ.as_str()) {
                    name = format!("plot.{name}")
                } else if typ == "instance" && ["data", "axis"].contains(&name.as_str()) {
                    name = format!("instance.{name}")
                }
            }
        }
        builtin(&name, self.en())
    }
    pub fn signature(&self, uri: &str, offset: usize) -> J {
        let Some(c) = context(self.source(uri), offset) else {
            return J::Null;
        };
        let Some(mut entry) = self.callable(uri, offset, &c) else {
            return J::Null;
        };
        let mut label = format!("{}(", entry["name"].as_str().unwrap());
        let ps = entry["parameters"].as_array_mut().unwrap();
        for (i, p) in ps.iter_mut().enumerate() {
            if i > 0 {
                label.push_str(", ")
            }
            let from = label.encode_utf16().count();
            label.push_str(p["name"].as_str().unwrap());
            if let Some(d) = p["default"].as_str() {
                label.push('=');
                label.push_str(d)
            }
            p["label"] = json!([from, label.encode_utf16().count()]);
        }
        label.push(')');
        let active = if !c.parameter.is_empty() {
            ps.iter().position(|p| p["name"] == c.parameter)
        } else {
            let positional: Vec<usize> = ps
                .iter()
                .enumerate()
                .filter(|(_, p)| p["kind"] != "named")
                .map(|(i, _)| i)
                .collect();
            let candidate = positional.get(c.positional).copied().filter(|&i| {
                !c.used
                    .contains(&ps[i]["name"].as_str().unwrap().to_string())
            });
            if candidate.is_some() && c.prefix.is_empty() {
                candidate
            } else {
                let consumed: BTreeSet<usize> =
                    positional.iter().take(c.positional).copied().collect();
                let eligible: Vec<usize> = ps
                    .iter()
                    .enumerate()
                    .filter(|(i, p)| {
                        p["kind"] != "positional"
                            && !consumed.contains(i)
                            && !c.used.contains(&p["name"].as_str().unwrap().to_string())
                            && (c.prefix.is_empty()
                                || p["name"].as_str().unwrap().starts_with(&c.prefix))
                    })
                    .map(|(i, _)| i)
                    .collect();
                if c.prefix.is_empty() || eligible.len() == 1 {
                    eligible.first().copied()
                } else {
                    None
                }
            }
        };
        entry["label"] = json!(label);
        if let Some(i) = active {
            entry["activeParameter"] = json!(i);
        }
        entry
    }
    pub fn hover(&self, uri: &str, offset: usize) -> J {
        let source = self.source(uri);
        let (from, to, w) = word(source, offset);
        if w.is_empty() {
            return J::Null;
        }
        let en = self.en();
        let canonical = w.replace('-', "_");
        if source[to..].trim_start().starts_with('=') {
            if let Some(c) = context(source, from) {
                if let Some(e) = self.callable(uri, offset, &c) {
                    if let Some(p) = e["parameters"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|p| p["name"] == canonical)
                    {
                        return json!({"from":from,"to":to,"contents":p["documentation"]});
                    }
                }
            }
        }
        if uri.ends_with(".lcss") || css_region(source, offset).is_some() {
            if let Some(p) = entries()
                .iter()
                .flat_map(|e| e["parameters"].as_array().unwrap())
                .find(|p| p["name"] == canonical)
            {
                return json!({"from":from,"to":to,"contents":parameter_doc(p,en,"")});
            }
        }
        let graph = self.binding_graph();
        let binding = graph.at(uri, offset);
        if let Some(s) = binding.and_then(|o| {
            self.resolve(uri, &w, offset, &mut BTreeSet::new())
                .or_else(|| {
                    graph
                        .indexes
                        .get(&o.key.uri)?
                        .symbols
                        .iter()
                        .find(|s| s.from == o.key.from)
                        .cloned()
                })
        }) {
            let contents = if s.kind == "function" {
                let e = authored(&s, &w, en);
                format!(
                    "```lay\n{}({})\n```\n\n{}",
                    w,
                    s.parameters
                        .iter()
                        .map(|p| p["name"].as_str().unwrap())
                        .collect::<Vec<_>>()
                        .join(", "),
                    e["documentation"].as_str().unwrap()
                )
            } else {
                let typ = self
                    .type_of(uri, &w, offset)
                    .filter(|t| !t.is_empty())
                    .or_else(|| (!s.doc.select(en).typ.is_empty()).then(|| s.doc.select(en).typ))
                    .unwrap_or_else(|| "value".into());
                format!("```lay\n{w}: {typ}\n```\n\n{}", s.doc.markdown(en))
            };
            return json!({"from":from,"to":to,"contents":contents});
        }
        if !source[to..].trim_start().starts_with('(')
            && receiver_before(source, from).is_none()
            && !tokens(source)
                .iter()
                .any(|t| matches!(t.kind, "string" | "comment") && t.from <= from && t.to >= to)
        {
            if let Some(value) = predefined_value(&w) {
                return json!({"from":from,"to":to,"contents":variable_doc(&w,value)});
            }
        }
        let receiver = receiver_before(source, from).unwrap_or_default();
        let c = Context {
            name: w,
            receiver,
            ..Context::default()
        };
        if let Some(e) = self.callable(uri, offset, &c) {
            return json!({"from":from,"to":to,"contents":format!("```lay\n{}(...)\n```\n\n{}",e["name"].as_str().unwrap(),e["documentation"].as_str().unwrap())});
        }
        if let Some(typ) = self
            .receiver_type(uri, &c.receiver, offset)
            .and_then(|typ| geometry_member_type(&typ, &c.name))
        {
            return json!({"from":from,"to":to,"contents":format!("**{}** → `{typ}`\n\n{}",c.name,desc(&type_info(&typ),en))});
        }
        J::Null
    }
    pub fn definition(&self, uri: &str, offset: usize) -> J {
        let source = self.source(uri);
        let (_, _, w) = word(source, offset);
        let graph = self.binding_graph();
        if let Some(o) = graph.at(uri, offset) {
            if let Some(s) = self
                .resolve(uri, &w, offset, &mut BTreeSet::new())
                .filter(|s| s.kind == "function")
            {
                return json!({"uri":s.uri,"from":s.from,"to":s.to});
            }
            if let Some(definition) = graph.definition(&o.key) {
                return definition;
            }
        }
        if let Some(t) = tokens(source)
            .iter()
            .find(|t| t.from <= offset && t.to >= offset && t.kind == "string")
        {
            let p = t.text.trim_matches(['\'', '"']);
            let target = resolve(uri, p);
            if self.documents.contains_key(&target) {
                return json!({"uri":target,"from":0,"to":0});
            }
        }
        if !w.is_empty() {
            let pattern = format!(
                r"(?:\.|--){}(?:\s|\{{|:)",
                regex::escape(w.trim_start_matches('-'))
            );
            let re = Regex::new(&pattern).unwrap();
            for (file, text) in &self.documents {
                if let Some(m) = re.find(text) {
                    return json!({"uri":file,"from":m.start(),"to":m.end()-1});
                }
            }
        }
        J::Null
    }
    pub fn completions(&self, uri: &str, offset: usize) -> J {
        let source = self.source(uri);
        let offset = offset.min(source.len());
        let before = &source[..offset];
        let line = before.rsplit('\n').next().unwrap();
        let (from, to, _) = word(source, offset);
        let en = self.en();
        let option = |label: &str, kind: &str, info: String, apply: String, detail: &str| json!({"label":label,"type":kind,"info":info,"apply":apply,"detail":detail,"from":from,"to":to});
        if line.trim_start().starts_with("##") {
            let mut out = vec![];
            let decl = index(uri, source, en)
                .symbols
                .into_iter()
                .find(|s| s.kind == "function" && s.from > offset);
            if line.contains("@lang ") {
                for tag in ["zh-CN", "en"] {
                    out.push(option(
                        tag,
                        "text",
                        String::new(),
                        tag.into(),
                        "Documentation",
                    ))
                }
            } else if line.contains("@param ") {
                if let Some(s) = decl {
                    let section = before.rsplit("## @lang").next().unwrap_or(before);
                    for p in s.parameters {
                        let n = p["name"].as_str().unwrap();
                        if !Regex::new(&format!(
                            r"@param\s+(?:\{{[^}}]*\}}\s*)?{}\b",
                            regex::escape(n)
                        ))
                        .unwrap()
                        .is_match(section)
                        {
                            out.push(option(
                                n,
                                "text",
                                String::new(),
                                format!("{n} - "),
                                "Documentation",
                            ))
                        }
                    }
                }
            } else if line.trim() == "##" {
                let params = decl.map(|s| s.parameters).unwrap_or_default();
                let build = |english| {
                    let mut a = vec![if english {
                        "Describe this declaration.".into()
                    } else {
                        "说明此声明的用途。".into()
                    }];
                    for p in &params {
                        a.push(format!(
                            "@param {} - {}",
                            p["name"].as_str().unwrap(),
                            if english {
                                "Description."
                            } else {
                                "参数说明。"
                            }
                        ))
                    }
                    a.push(format!(
                        "@returns {}",
                        if english {
                            "Result."
                        } else {
                            "返回结果。"
                        }
                    ));
                    a
                };
                let template = build(en).join("\n## ");
                let mut bilingual = vec!["@lang zh-CN".into()];
                bilingual.extend(build(false));
                bilingual.push("@lang en".into());
                bilingual.extend(build(true));
                for (label, apply) in [
                    (
                        if en {
                            "Documentation template"
                        } else {
                            "文档模板"
                        },
                        template,
                    ),
                    (
                        if en {
                            "Bilingual documentation template"
                        } else {
                            "双语文档模板"
                        },
                        bilingual.join("\n## "),
                    ),
                ] {
                    let mut o = option(label, "text", String::new(), apply, "Documentation");
                    o["from"] = json!(offset);
                    out.push(o)
                }
            } else {
                for tag in ["lang", "param", "returns", "type", "example", "see"] {
                    let mut o = option(
                        &format!("@{tag}"),
                        "text",
                        String::new(),
                        format!("@{tag} "),
                        "Documentation",
                    );
                    if from > 0 && source.as_bytes()[from - 1] == b'@' {
                        o["from"] = json!(from - 1)
                    }
                    out.push(o)
                }
            }
            return json!(out);
        }
        if tokens(source)
            .iter()
            .any(|t| t.kind == "comment" && t.from <= offset && t.to >= offset)
        {
            return json!([]);
        }
        if let Some(c) = context(source, offset) {
            let preset_name = c.receiver.is_empty()
                && ["cmap", "palette"].contains(&c.name.as_str())
                && (c.parameter == "name" || c.parameter.is_empty() && c.positional == 0)
                && self
                    .resolve(uri, &c.name, offset, &mut BTreeSet::new())
                    .is_none();
            if preset_name || c.parameter == "cmap" && self.callable(uri, offset, &c).is_some() {
                let quoted = source[..from].trim_end().ends_with(['\'', '"']);
                return json!(
                    laymesh_core::colormap::names(None, true)
                        .unwrap()
                        .into_iter()
                        .map(|name| {
                            option(
                                &name,
                                "value",
                                format!("Matplotlib {}", laymesh_core::colormap::VERSION),
                                if quoted {
                                    name.clone()
                                } else {
                                    json!(name).to_string()
                                },
                                "cmap",
                            )
                        })
                        .collect::<Vec<_>>()
                );
            }
        }
        let css = if uri.ends_with(".lcss") {
            Some(0)
        } else {
            css_region(source, offset)
        };
        if let Some(start) = css {
            let local = &source[start..offset];
            if Regex::new(r"::[\w-]*$").unwrap().is_match(local) {
                return json!(
                    [
                        "area",
                        "axis",
                        "axis-label",
                        "tick-label",
                        "legend",
                        "colorbar",
                        "marker",
                        "grid"
                    ]
                    .iter()
                    .map(|name| option(
                        name,
                        "keyword",
                        String::new(),
                        name.to_string(),
                        "LCSS part"
                    ))
                    .collect::<Vec<_>>()
                );
            }
            if Regex::new(r"var\(\s*--[\w-]*$").unwrap().is_match(local) {
                let pattern = Regex::new(r"(--[A-Za-z_][\w-]*)\s*:").unwrap();
                let names: BTreeSet<String> = self
                    .documents
                    .values()
                    .flat_map(|text| pattern.captures_iter(text).map(|c| c[1].to_string()))
                    .collect();
                return json!(
                    names
                        .iter()
                        .map(|name| option(
                            name,
                            "variable",
                            String::new(),
                            name.clone(),
                            "LCSS variable"
                        ))
                        .collect::<Vec<_>>()
                );
            }
            if Regex::new(r"\.[\w-]*$").unwrap().is_match(local) {
                let names = self.classes();
                return json!(
                    names
                        .iter()
                        .map(|name| option(
                            name,
                            "class",
                            String::new(),
                            name.clone(),
                            "LCSS class"
                        ))
                        .collect::<Vec<_>>()
                );
            }
            if local.rfind('{') > local.rfind('}') {
                if let Some(cap) = Regex::new(r"([\w-]+)\s*:\s*([^;{}]*)$")
                    .unwrap()
                    .captures(local)
                {
                    let key = cap[1].replace('-', "_");
                    let p = entries()
                        .iter()
                        .flat_map(|a| a["parameters"].as_array().unwrap())
                        .find(|p| p["name"] == key);
                    let mut values = p.map(|p| strs(&p["values"])).unwrap_or_default();
                    values.push("var()".into());
                    if key.contains("fill") || key == "background" {
                        values.extend(
                            [
                                "none",
                                "linear-gradient()",
                                "radial-gradient()",
                                "hatch()",
                                "url()",
                            ]
                            .map(str::to_string),
                        )
                    }
                    return json!(
                        values
                            .iter()
                            .map(|v| option(v, "keyword", String::new(), v.clone(), ""))
                            .collect::<Vec<_>>()
                    );
                }
                let mut names = BTreeSet::new();
                let mut options = vec![];
                for p in entries()
                    .iter()
                    .flat_map(|a| a["parameters"].as_array().unwrap())
                {
                    let name = p["name"].as_str().unwrap().replace('_', "-");
                    if names.insert(name.clone()) {
                        options.push(option(
                            &name,
                            "property",
                            parameter_doc(p, en, ""),
                            format!("{name}: "),
                            "",
                        ))
                    }
                }
                return json!(options);
            }
            return json!(
                entries()
                    .iter()
                    .filter(|a| !a["name"].as_str().unwrap().contains('.'))
                    .map(|a| option(
                        a["name"].as_str().unwrap(),
                        "keyword",
                        desc(a, en).into(),
                        a["name"].as_str().unwrap().into(),
                        ""
                    ))
                    .collect::<Vec<_>>()
            );
        }
        if Regex::new(r#"\bclass\s*=\s*["'][^"']*$"#)
            .unwrap()
            .is_match(before)
        {
            return json!(
                self.classes()
                    .iter()
                    .map(|name| option(name, "class", String::new(), name.clone(), "LCSS class"))
                    .collect::<Vec<_>>()
            );
        }
        if let Some(receiver) = receiver_before(source, from) {
            let typ = self
                .receiver_type(uri, &receiver, offset)
                .unwrap_or_default();
            let names = if ["plot", "polar_plot", "radar_plot"].contains(&typ.as_str()) {
                entries()
                    .iter()
                    .filter_map(|e| {
                        e["name"]
                            .as_str()
                            .unwrap()
                            .strip_prefix("plot.")
                            .map(str::to_string)
                    })
                    .collect()
            } else if let Some(members) = type_info(&typ)["members"].as_object() {
                members
                    .keys()
                    .chain(type_info(&typ)["methods"].as_object().unwrap().keys())
                    .cloned()
                    .chain((typ == "instance").then_some("data".into()))
                    .chain((typ == "instance").then_some("axis".into()))
                    .collect()
            } else {
                [
                    "add",
                    "fuse",
                    "width",
                    "height",
                    "top_left",
                    "top_center",
                    "top_right",
                    "middle_left",
                    "center",
                    "middle_right",
                    "bottom_left",
                    "bottom_center",
                    "bottom_right",
                    "plot_center",
                    "data",
                    "axis",
                ]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
            };
            return json!(
                names
                    .iter()
                    .map(|n| {
                        let c = Context {
                            name: n.clone(),
                            receiver: receiver.clone(),
                            ..Context::default()
                        };
                        let e = self.callable(uri, offset, &c);
                        let selected = geometry_member_type(&typ, n);
                        let detail = selected
                            .as_deref()
                            .or_else(|| e.as_ref().and_then(|e| e["returns"].as_str()))
                            .unwrap_or("");
                        option(
                            n,
                            if e.is_some() { "function" } else { "property" },
                            e.as_ref()
                                .and_then(|e| e["documentation"].as_str())
                                .unwrap_or("")
                                .into(),
                            if e.is_some() {
                                format!("{n}(")
                            } else {
                                n.clone()
                            },
                            detail,
                        )
                    })
                    .collect::<Vec<_>>()
            );
        }
        let named = self.named(uri, offset);
        let mut out: Vec<J> = named
            .iter()
            .map(|(name, s)| {
                option(
                    name,
                    if s.kind == "function" {
                        "function"
                    } else {
                        "variable"
                    },
                    s.doc.markdown(en),
                    if s.kind == "function" {
                        format!("{name}(")
                    } else {
                        name.clone()
                    },
                    &self
                        .type_of(uri, name, offset)
                        .unwrap_or_else(|| s.typ.clone()),
                )
            })
            .collect();
        if let Some(c) = context(source, offset) {
            if let Some(e) = self.callable(uri, offset, &c) {
                let positional = e["parameters"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|p| p["kind"] != "named")
                    .nth(c.positional)
                    .and_then(|p| p["name"].as_str())
                    .unwrap_or("");
                let selected = if c.parameter.is_empty() {
                    positional
                } else {
                    &c.parameter
                };
                if let Some(api) = entry(e["name"].as_str().unwrap()) {
                    if let Some(p) = api["parameters"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|p| p["name"] == selected)
                    {
                        if p["values"].is_array() {
                            let quoted = tokens(source)
                                .into_iter()
                                .find(|t| t.kind == "string" && t.from < offset && offset <= t.to);
                            let mut choices = vec![];
                            for value in strs(&p["values"]) {
                                if let Some(var) =
                                    predefined_variables().iter().find(|v| v["value"] == value)
                                {
                                    let name = var["name"].as_str().unwrap();
                                    let mut o = option(
                                        if quoted.is_some() { &value } else { name },
                                        "variable",
                                        variable_doc(name, &value),
                                        if quoted.is_some() {
                                            value.clone()
                                        } else {
                                            name.into()
                                        },
                                        "string",
                                    );
                                    if let Some(t) = &quoted {
                                        let quote = t.text.chars().next().unwrap();
                                        o["from"] = json!(t.from + 1);
                                        o["to"] =
                                            json!(if t.text.len() > 1 && t.text.ends_with(quote) {
                                                t.to - 1
                                            } else {
                                                t.to
                                            });
                                    }
                                    if quoted.is_some() || !named.iter().any(|(n, _)| n == name) {
                                        choices.push(o);
                                    }
                                }
                            }
                            if quoted.is_some() {
                                return json!(choices);
                            }
                            if selected == "anchor"
                                && p["values"]
                                    .as_array()
                                    .unwrap()
                                    .iter()
                                    .any(|v| v == "top_left")
                            {
                                choices.push(option(
                                    "self",
                                    "variable",
                                    parameter_doc(p, en, "add"),
                                    "self.".into(),
                                    "instance selector",
                                ));
                            }
                            choices.append(&mut out);
                            out = choices;
                            if !c.parameter.is_empty() {
                                return json!(out);
                            }
                        }
                    }
                }
                if c.parameter.is_empty() {
                    let consumed: Vec<_> = e["parameters"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .filter(|p| p["kind"] != "named")
                        .take(c.positional)
                        .map(|p| p["name"].as_str().unwrap().to_string())
                        .collect();
                    for p in e["parameters"].as_array().unwrap() {
                        let n = p["name"].as_str().unwrap();
                        if p["kind"] != "positional"
                            && !c.used.contains(&n.to_string())
                            && !consumed.contains(&n.to_string())
                        {
                            out.push(option(
                                n,
                                "property",
                                p["documentation"].as_str().unwrap_or("").into(),
                                format!("{n}="),
                                p["type"].as_str().unwrap_or(""),
                            ))
                        }
                    }
                    return json!(out);
                }
                if let Some(api) = entry(e["name"].as_str().unwrap()) {
                    if let Some(p) = api["parameters"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|p| p["name"] == c.parameter)
                    {
                        if p["unit"].as_str().is_some_and(|unit| unit != "none") {
                            out.extend(["mm", "cm", "in", "inch", "pt", "px", "auto"].iter().map(
                                |v| option(v, "keyword", parameter_doc(p, en, ""), (*v).into(), ""),
                            ));
                            return json!(out);
                        }
                    }
                }
            }
        }
        let shadowed: BTreeSet<&str> = named.iter().map(|(n, _)| n.as_str()).collect();
        if !tokens(source)
            .iter()
            .any(|t| t.kind == "string" && t.from < offset && offset <= t.to)
        {
            for var in predefined_variables() {
                let n = var["name"].as_str().unwrap();
                if !shadowed.contains(n) {
                    out.push(option(
                        n,
                        "variable",
                        variable_doc(n, var["value"].as_str().unwrap()),
                        n.into(),
                        "string",
                    ));
                }
            }
        } else {
            return json!([]);
        }
        for a in entries() {
            let n = a["name"].as_str().unwrap();
            if !n.contains('.') && !shadowed.contains(n) {
                out.push(option(
                    n,
                    "function",
                    builtin(n, en).unwrap()["documentation"]
                        .as_str()
                        .unwrap()
                        .into(),
                    format!("{n}("),
                    "",
                ))
            }
        }
        json!(out)
    }
    pub fn diagnostics(&self, uri: &str) -> J {
        diagnostics(self, uri)
    }
    pub fn references(&self, uri: &str, offset: usize, include_declaration: bool) -> J {
        let graph = self.binding_graph();
        json!(
            graph
                .at(uri, offset)
                .map(|o| graph
                    .related_references(&o.key, include_declaration)
                    .into_iter()
                    .map(|o| o.json())
                    .collect::<Vec<_>>())
                .unwrap_or_default()
        )
    }
    pub fn highlights(&self, uri: &str, offset: usize) -> J {
        json!(
            self.references(uri, offset, true)
                .as_array()
                .unwrap()
                .iter()
                .filter(|o| o["uri"] == uri)
                .cloned()
                .collect::<Vec<_>>()
        )
    }
    pub fn prepare_rename(&self, uri: &str, offset: usize) -> J {
        let graph = self.binding_graph();
        let Some(o) = graph.at(uri, offset).filter(|o| graph.can_rename(&o.key)) else {
            return J::Null;
        };
        json!({"from":o.from,"to":o.to,"placeholder":&self.source(uri)[o.from..o.to]})
    }
    pub fn rename(&self, uri: &str, offset: usize, new_name: &str) -> Result<J, String> {
        self.binding_graph()
            .rename(uri, offset, new_name)
            .map(|edits| json!(edits))
    }
    pub fn query(&self, method: &str, uri: &str, offset: usize) -> J {
        match method {
            "completions" => self.completions(uri, offset),
            "hover" => self.hover(uri, offset),
            "signature" => self.signature(uri, offset),
            "definition" => self.definition(uri, offset),
            "references" => self.references(uri, offset, true),
            "highlights" => self.highlights(uri, offset),
            "prepareRename" => self.prepare_rename(uri, offset),
            "diagnostics" => self.diagnostics(uri),
            "colors" => self.document_colors(uri),
            _ => J::Null,
        }
    }
}
pub fn css_region(source: &str, offset: usize) -> Option<usize> {
    let ts = tokens(source);
    for i in 0..ts.len().saturating_sub(1) {
        if ts[i].text == "style" && ts[i + 1].text == "{" {
            let from = ts[i + 1].to;
            let mut depth = 1;
            let mut end = source.len();
            for t in &ts[i + 2..] {
                if t.text == "{" {
                    depth += 1
                } else if t.text == "}" {
                    depth -= 1;
                    if depth == 0 {
                        end = t.from;
                        break;
                    }
                }
            }
            if offset >= from && offset <= end {
                return Some(from);
            }
        }
    }
    None
}
pub fn byte_offset(source: &str, utf16: usize) -> usize {
    let mut n = 0;
    for (i, c) in source.char_indices() {
        if n >= utf16 {
            return i;
        }
        n += c.len_utf16()
    }
    source.len()
}
fn boundary(source: &str, byte: usize) -> usize {
    let mut byte = byte.min(source.len());
    while !source.is_char_boundary(byte) {
        byte -= 1;
    }
    byte
}
pub fn utf16_offset(source: &str, byte: usize) -> usize {
    source[..boundary(source, byte)].encode_utf16().count()
}
pub fn position(source: &str, byte: usize) -> J {
    let before = &source[..boundary(source, byte)];
    json!({"line":before.bytes().filter(|&b|b==b'\n').count(),"character":before.rsplit('\n').next().unwrap().encode_utf16().count()})
}
pub fn offset(source: &str, pos: &J) -> usize {
    let line = pos["line"].as_u64().unwrap_or(0) as usize;
    let col = pos["character"].as_u64().unwrap_or(0) as usize;
    let start = source
        .split_inclusive('\n')
        .take(line)
        .map(str::len)
        .sum::<usize>()
        .min(source.len());
    start + byte_offset(source[start..].split('\n').next().unwrap(), col)
}
fn diagnostics(service: &LanguageService, uri: &str) -> J {
    let source = service.source(uri);
    let en = service.en();
    let mut out = if uri.ends_with(".lcss") {
        vec![]
    } else {
        index(uri, source, en).issues
    };
    let emit = |out: &mut Vec<J>,
                from: usize,
                to: usize,
                code: &str,
                message: String,
                replacement: Option<&str>| {
        let from = boundary(source, from);
        let mut to = to.min(source.len());
        while !source.is_char_boundary(to) {
            to += 1;
        }
        let mut d = json!({"from":from,"to":to,"severity":"error","code":code,"message":message});
        if let Some(r) = replacement {
            d["replacement"] = json!(r)
        }
        out.push(d)
    };
    if uri.ends_with(".lcss") {
        if let Err(error) = laymesh_core::style::validate_stylesheet(
            source,
            uri,
            laymesh_core::Loc {
                line: 1,
                column: 1,
                offset: 0,
            },
        ) {
            emit(
                &mut out,
                error.loc.offset,
                error.loc.offset + 1,
                &error.code,
                error.message,
                None,
            );
        }
        return json!(out);
    }
    let stmts = match parser::parse(source, uri) {
        Ok(v) => v,
        Err(e) => {
            emit(
                &mut out,
                e.loc.offset,
                e.loc.offset + 1,
                &e.code,
                e.message,
                None,
            );
            return json!(out);
        }
    };
    let mut declared: BTreeSet<String> = entries()
        .iter()
        .map(|a| a["name"].as_str().unwrap().into())
        .chain(
            predefined_variables()
                .iter()
                .map(|v| v["name"].as_str().unwrap().to_string()),
        )
        .chain(["null", "self"].map(str::to_string))
        .collect();
    fn declare(stmts: &[Stmt], names: &mut BTreeSet<String>) {
        for stmt in stmts {
            match &stmt.kind {
                StmtKind::Bind(n, _, _) => {
                    names.insert(n.clone());
                }
                StmtKind::Function(n, ps, b, _) => {
                    names.insert(n.clone());
                    for (p, _) in ps {
                        names.insert(p.clone());
                    }
                    declare(b, names)
                }
                StmtKind::Import(ns, _) => {
                    for (_, a) in ns {
                        names.insert(a.clone());
                    }
                }
                StmtKind::For(n, _, b) => {
                    names.extend(n.names());
                    declare(b, names)
                }
                StmtKind::While(_, b) => declare(b, names),
                StmtKind::If(bs, other) => {
                    for (_, b) in bs {
                        declare(b, names)
                    }
                    declare(other, names)
                }
                _ => {}
            }
        }
    }
    declare(&stmts, &mut declared);
    let inferred: BTreeMap<String, String> = index(uri, source, en)
        .symbols
        .into_iter()
        .map(|s| (s.name, s.typ))
        .collect();
    fn walk(
        e: &Expr,
        out: &mut Vec<J>,
        names: &BTreeSet<String>,
        inferred: &BTreeMap<String, String>,
        source: &str,
        en: bool,
    ) {
        let mut report =
            |from: usize, to: usize, code: &str, message: String, replacement: Option<&str>| {
                let from = boundary(source, from);
                let mut to = to.min(source.len());
                while !source.is_char_boundary(to) {
                    to += 1;
                }
                let mut d =
                    json!({"from":from,"to":to,"severity":"error","code":code,"message":message});
                if let Some(r) = replacement {
                    d["replacement"] = json!(r)
                }
                out.push(d)
            };
        match &e.kind {
            ExprKind::Ref(ps) => {
                if !names.contains(&ps[0]) {
                    report(
                        e.loc.offset,
                        e.loc.offset + ps[0].len(),
                        "E_NAME",
                        format!(
                            "{} {}",
                            if en {
                                "Unknown name"
                            } else {
                                "未定义名称"
                            },
                            ps[0]
                        ),
                        None,
                    )
                }
            }
            ExprKind::Call(ps, args) => {
                if ps == &["arrow"] && !names.contains("arrow") {
                    let edit = laymesh_core::migration::arrow_edits(source)
                        .into_iter()
                        .find(|v| v.0 == e.loc.offset);
                    let to = edit.as_ref().map(|v| v.1).unwrap_or(e.loc.offset + 5);
                    report(
                        e.loc.offset,
                        to,
                        "E_API_MIGRATION",
                        if en {
                            "arrow has been removed; use line(..., end_head=head(...))"
                        } else {
                            "arrow 已移除，使用 line(..., end_head=head(...))"
                        }
                        .into(),
                        edit.as_ref().map(|v| v.2.as_str()),
                    );
                } else if !names.contains(&ps[0]) {
                    report(
                        e.loc.offset,
                        e.loc.offset + ps[0].len(),
                        "E_NAME",
                        format!(
                            "{} {}",
                            if en {
                                "Unknown callable"
                            } else {
                                "未定义调用"
                            },
                            ps[0]
                        ),
                        None,
                    )
                }
                let last = ps.last().unwrap();
                let typ = inferred.get(&ps[0]).map(String::as_str).unwrap_or("");
                let surface = if ps.len() > 1 && ["plot", "polar_plot", "radar_plot"].contains(&typ)
                {
                    format!("plot.{last}")
                } else if ps.len() > 1
                    && typ == "instance"
                    && ["data", "axis"].contains(&last.as_str())
                {
                    format!("instance.{last}")
                } else if ps.len() > 1 && ["dict", "table", "cmap"].contains(&typ) {
                    format!("{}.{last}", if typ == "table" { "dict" } else { typ })
                } else if ps.len() > 1 {
                    geometry_surface(last).unwrap_or_else(|| last.clone())
                } else {
                    last.clone()
                };
                if let Some(api) = entry(&surface) {
                    for (n, v) in args {
                        if let Some(n) = n {
                            let before = &source[e.loc.offset..v.loc.offset.min(source.len())];
                            let re = Regex::new(&format!(r"\b{}\s*=", regex::escape(n))).unwrap();
                            let from = re
                                .find_iter(before)
                                .last()
                                .map(|m| e.loc.offset + m.start())
                                .unwrap_or(v.loc.offset);
                            let rename = match n.as_str() {
                                "font" => Some("font_family"),
                                "weight" => Some("font_weight"),
                                "italic" => Some("font_style"),
                                "stroke" => Some("border_color"),
                                "stroke_width" => Some("border_width"),
                                "classes" => Some("class"),
                                "width" | "height" => Some("size"),
                                _ => None,
                            };
                            if let Some(rename) = rename {
                                report(
                                    from,
                                    from + n.len(),
                                    "E_API_MIGRATION",
                                    format!("{n} → {rename}"),
                                    Some(rename),
                                );
                                continue;
                            }
                            let parameter = api["parameters"]
                                .as_array()
                                .unwrap()
                                .iter()
                                .find(|p| p["name"] == *n);
                            if parameter.is_none() {
                                report(
                                    from,
                                    from + n.len(),
                                    "E_ARG",
                                    format!(
                                        "{surface}: {} {n}",
                                        if en {
                                            "unknown parameter"
                                        } else {
                                            "不支持参数"
                                        }
                                    ),
                                    None,
                                );
                                continue;
                            }
                            let p = parameter.unwrap();
                            fn angle_in_length(v: &Expr) -> bool {
                                match &v.kind {
                                    ExprKind::Number(_, unit) => unit == "deg" || unit == "rad",
                                    ExprKind::List(values) => values.iter().any(angle_in_length),
                                    _ => false,
                                }
                            }
                            if (p["unit"] == "geometry" || p["unit"] == "pt") && angle_in_length(v)
                            {
                                report(
                                    v.loc.offset,
                                    v.loc.offset + 1,
                                    "E_UNIT",
                                    if en {
                                        "Expected a length, received an angle"
                                    } else {
                                        "需要长度，不能使用角度"
                                    }
                                    .into(),
                                    None,
                                )
                            }
                            fn literal_number(v: &Expr) -> Option<f64> {
                                match &v.kind {
                                    ExprKind::Number(value, _) => Some(*value),
                                    ExprKind::Unary(op, value) if op == "-" => {
                                        literal_number(value).map(|v| -v)
                                    }
                                    ExprKind::Unary(op, value) if op == "+" => {
                                        literal_number(value)
                                    }
                                    _ => None,
                                }
                            }
                            if (["blur", "spread", "depth", "text_stroke_width"]
                                .contains(&n.as_str())
                                && literal_number(v).is_some_and(|v| v < 0.))
                                || (n == "wavelength" && literal_number(v).is_some_and(|v| v <= 0.))
                            {
                                report(
                                    v.loc.offset,
                                    v.loc.offset + 1,
                                    "E_EFFECT",
                                    if n == "wavelength" {
                                        "wavelength must be positive"
                                    } else {
                                        "Effect length must be nonnegative"
                                    }
                                    .into(),
                                    None,
                                );
                            }
                            if n == "effects"
                                && matches!(
                                    v.kind,
                                    ExprKind::Call(..)
                                        | ExprKind::Number(..)
                                        | ExprKind::String(..)
                                        | ExprKind::Bool(_)
                                )
                            {
                                report(
                                    v.loc.offset,
                                    v.loc.offset + 1,
                                    "E_EFFECT",
                                    "effects requires a list of shadow(...) or glow(...)".into(),
                                    None,
                                );
                            }
                            if n == "font_family"
                                && (matches!(v.kind, ExprKind::Number(..) | ExprKind::Bool(_))
                                    || matches!(&v.kind, ExprKind::List(values) if values.iter().any(|v| matches!(v.kind, ExprKind::Number(..) | ExprKind::Bool(_)))))
                            {
                                report(
                                    v.loc.offset,
                                    v.loc.offset + 1,
                                    "E_TYPE",
                                    if en {
                                        "Expected a font name or ordered string list"
                                    } else {
                                        "font_family 需要字体名称或有序字符串列表"
                                    }
                                    .into(),
                                    None,
                                )
                            }
                            if matches!(v.kind, ExprKind::Bool(_))
                                && (p["unit"] == "geometry" || p["unit"] == "pt")
                            {
                                report(
                                    v.loc.offset,
                                    v.loc.offset + 1,
                                    "E_TYPE",
                                    if en {
                                        "Expected a physical length"
                                    } else {
                                        "需要物理长度"
                                    }
                                    .into(),
                                    None,
                                );
                            }
                            if n == "size" && matches!(&v.kind,ExprKind::List(xs)if xs.len()!=2) {
                                report(
                                    v.loc.offset,
                                    v.loc.offset + 1,
                                    "E_TYPE",
                                    "size requires two dimensions".into(),
                                    None,
                                )
                            }
                            if n == "opacity"
                                && matches!(&v.kind,ExprKind::Number(n,u)if !u.is_empty()||*n<0.||*n>1.)
                            {
                                report(
                                    v.loc.offset,
                                    v.loc.offset + 1,
                                    "E_VALUE",
                                    "opacity: 0–1".into(),
                                    None,
                                )
                            }
                            if let ExprKind::Ref(parts) = &v.kind {
                                if parts.len() == 1 && !inferred.contains_key(&parts[0]) {
                                    if let Some(value) = predefined_value(&parts[0]) {
                                        if p["values"].is_array()
                                            && !strs(&p["values"]).contains(&value.to_string())
                                        {
                                            report(
                                                v.loc.offset,
                                                v.loc.offset + parts[0].len(),
                                                "E_VALUE",
                                                format!("{n}: {}", strs(&p["values"]).join(" / ")),
                                                None,
                                            );
                                        }
                                    }
                                }
                            }
                            if let ExprKind::String(s, _, _) = &v.kind {
                                if matches!(p["type"].as_str(), Some("color" | "paint"))
                                    && laymesh_core::color::Color::parse(s).is_err()
                                {
                                    report(v.loc.offset,v.loc.offset+1,"E_COLOR",if en {"Invalid color; use HEX, RGB, HSV or OKLCH with valid channels"} else {"无效颜色；使用通道合法的 HEX、RGB、HSV 或 OKLCH"}.into(),None);
                                }
                                if p["values"].is_array()
                                    && !strs(&p["values"]).contains(s)
                                    && n != "scale"
                                    && n != "style"
                                {
                                    report(
                                        v.loc.offset,
                                        v.loc.offset + 1,
                                        "E_VALUE",
                                        format!("{n}: {}", strs(&p["values"]).join(" / ")),
                                        None,
                                    )
                                }
                            }
                        }
                    }
                }
                fn number(e: &Expr) -> bool {
                    match &e.kind {
                        ExprKind::Number(..) => true,
                        ExprKind::Unary(op, v) if op == "-" || op == "+" => number(v),
                        _ => false,
                    }
                }
                if ps.len() == 1
                    && ["rgb", "hsv", "oklch"].contains(&last.as_str())
                    && !inferred.contains_key(last)
                    && args.len() >= 3
                    && args.iter().all(|(_, v)| number(v))
                    && laymesh_core::color::constant_expression(e).is_none()
                {
                    report(e.loc.offset,e.loc.offset+last.len(),"E_COLOR",if en {"Invalid color channels: RGB 0–255, HSV S/V and OKLCH L 0–1, C ≥ 0, alpha 0–1; angle units only on hue"} else {"颜色通道无效：RGB 0–255，HSV S/V 和 OKLCH L 0–1，C ≥ 0，alpha 0–1；仅色相接受角度单位"}.into(),None);
                }
                if surface == "line" {
                    let keys = args
                        .iter()
                        .filter_map(|(k, _)| k.as_deref())
                        .collect::<Vec<_>>();
                    if (keys.contains(&"dx") && !keys.contains(&"dy"))
                        || (keys.contains(&"dy") && !keys.contains(&"dx"))
                        || (keys.contains(&"angle") && !keys.contains(&"length"))
                    {
                        report(
                            e.loc.offset,
                            e.loc.offset + 4,
                            "E_ARG",
                            if en {
                                "Geometry must specify both dx/dy, or length with optional angle"
                            } else {
                                "几何定义需要完整的 dx/dy，或 length 及可选 angle"
                            }
                            .into(),
                            None,
                        );
                    }
                    if (keys.contains(&"length") || keys.contains(&"angle"))
                        && (keys.contains(&"dx") || keys.contains(&"dy"))
                    {
                        report(
                            e.loc.offset,
                            e.loc.offset + 4,
                            "E_ARG",
                            if en {
                                "length/angle and dx/dy are mutually exclusive"
                            } else {
                                "length/angle 与 dx/dy 互斥"
                            }
                            .into(),
                            None,
                        );
                    }
                    if !keys.contains(&"angle")
                        && args.iter().any(|(k, v)| {
                            k.as_deref() == Some("length")
                                && matches!(&v.kind,ExprKind::Number(n,_)if *n==0.)
                        })
                    {
                        report(
                            e.loc.offset,
                            e.loc.offset + 4,
                            "E_ANCHOR_DIRECTION",
                            if en {
                                "Zero-length line requires an explicit angle"
                            } else {
                                "零长度线需要显式 angle"
                            }
                            .into(),
                            None,
                        );
                    }
                }
                for (_, v) in args {
                    walk(v, out, names, inferred, source, en)
                }
            }
            ExprKind::Dict(es) => {
                for (key, value) in es {
                    walk(key, out, names, inferred, source, en);
                    walk(value, out, names, inferred, source, en);
                }
            }
            ExprKind::List(vs) => {
                for v in vs {
                    walk(v, out, names, inferred, source, en)
                }
            }
            ExprKind::Member(receiver, _) => walk(receiver, out, names, inferred, source, en),
            ExprKind::Method(receiver, name, args) => {
                walk(receiver, out, names, inferred, source, en);
                let call = Expr {
                    loc: e.loc,
                    kind: ExprKind::Call(vec!["self".into(), name.clone()], args.clone()),
                };
                walk(&call, out, names, inferred, source, en);
            }
            ExprKind::Index(a, b) | ExprKind::Binary(_, a, b) => {
                walk(a, out, names, inferred, source, en);
                walk(b, out, names, inferred, source, en)
            }
            ExprKind::Unary(_, v) => walk(v, out, names, inferred, source, en),
            _ => {}
        }
    }
    fn statements(
        stmts: &[Stmt],
        out: &mut Vec<J>,
        names: &BTreeSet<String>,
        inferred: &BTreeMap<String, String>,
        source: &str,
        en: bool,
    ) {
        for s in stmts {
            match &s.kind {
                StmtKind::Bind(_, e, _) | StmtKind::Expr(e) | StmtKind::Return(e) => {
                    walk(e, out, names, inferred, source, en)
                }
                StmtKind::SetIndex(target, value) => {
                    walk(target, out, names, inferred, source, en);
                    walk(value, out, names, inferred, source, en);
                }
                StmtKind::Function(_, ps, b, _) => {
                    for (_, e) in ps {
                        if let Some(e) = e {
                            walk(e, out, names, inferred, source, en)
                        }
                    }
                    statements(b, out, names, inferred, source, en)
                }
                StmtKind::If(bs, other) => {
                    for (e, b) in bs {
                        walk(e, out, names, inferred, source, en);
                        statements(b, out, names, inferred, source, en)
                    }
                    statements(other, out, names, inferred, source, en)
                }
                StmtKind::For(_, e, b) | StmtKind::While(e, b) => {
                    walk(e, out, names, inferred, source, en);
                    statements(b, out, names, inferred, source, en)
                }
                StmtKind::Style(css) => {
                    if let Err(error) = laymesh_core::style::validate_stylesheet(css, "", s.loc) {
                        out.push(json!({"from":error.loc.offset.min(source.len()),"to":(error.loc.offset+1).min(source.len()),"severity":"error","code":error.code,"message":error.message}));
                    }
                }
                _ => {}
            }
        }
    }
    statements(&stmts, &mut out, &declared, &inferred, source, en);
    out.extend(connections::diagnostics(service, uri, &stmts));
    json!(out)
}
