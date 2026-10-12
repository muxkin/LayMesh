//! Static CSS dependency/symbol index. No Lay expressions are executed.
use crate::{LanguageService, context, css_region, index::tokens};
use cssparser::{Parser, Token};
use laymesh_core::{
    Loc,
    animation::css::{self, Declaration},
    parser::{self, Expr, ExprKind, StmtKind},
};
use serde_json::{Value as J, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, LazyLock},
};

pub const SCHEME: &str = "laymesh-library:/";
pub fn library_path(uri: &str) -> String {
    uri.strip_prefix(SCHEME)
        .map(|s| format!("@laymesh/{s}"))
        .unwrap_or_else(|| uri.into())
}
pub fn library_uri(path: &str) -> String {
    path.strip_prefix("@laymesh/")
        .map(|s| format!("{SCHEME}{s}"))
        .unwrap_or_else(|| path.into())
}
pub fn content(uri: &str) -> Option<&'static str> {
    laymesh_core::style::bundled_stylesheet(&library_path(uri))
}
pub fn target(uri: &str, path: &str) -> String {
    if path.starts_with("@laymesh/") {
        library_uri(path)
    } else {
        crate::resolve(uri, path)
    }
}
#[derive(Clone, Debug)]
pub struct Symbol {
    pub name: String,
    pub kind: &'static str,
    pub uri: String,
    pub from: usize,
    pub to: usize,
    pub value: String,
    pub selector: String,
    pub declarations: Vec<Declaration>,
}
impl Symbol {
    pub fn location(&self) -> J {
        json!({"uri":self.uri,"from":self.from,"to":self.to})
    }
    pub fn documentation(&self, en: bool) -> String {
        let path = library_path(&self.uri);
        let library = laymesh_core::style::bundled_stylesheets()
            .iter()
            .find(|l| l.path() == path);
        let origin = library
            .map(|l| {
                format!(
                    "{} {} · {}\n\n[{}]({})",
                    l.name,
                    l.version,
                    l.license,
                    if en {
                        "Upstream source"
                    } else {
                        "上游来源"
                    },
                    l.source
                )
            })
            .unwrap_or_else(|| self.uri.clone());
        let declarations = self
            .declarations
            .iter()
            .take(24)
            .map(|d| {
                format!(
                    "{}: {}{};",
                    d.property,
                    d.value,
                    if d.important { " !important" } else { "" }
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "**{}** · {}\n\n{}\n\n```css\n{}\n```",
            self.name,
            self.kind,
            origin,
            if self.kind == "class" {
                format!("{} {{\n{}\n}}", self.selector, declarations)
            } else {
                self.value.clone()
            }
        )
    }
}
#[derive(Clone, Debug, Default)]
pub struct Index {
    pub symbols: Vec<Symbol>,
    pub imports: Vec<(String, usize, usize)>,
}
fn origin(source: &str, from: usize) -> Loc {
    let prefix = &source[..from];
    Loc {
        offset: from,
        line: prefix.matches('\n').count() + 1,
        column: prefix
            .rsplit('\n')
            .next()
            .unwrap_or("")
            .encode_utf16()
            .count()
            + 1,
    }
}
fn selector_classes(p: &mut Parser<'_>, output: &mut Vec<(String, usize, usize)>, depth: usize) {
    if depth > 32 {
        return;
    }
    while let Ok(token) = p.next_including_whitespace_and_comments().cloned() {
        match token {
            Token::Delim('.') => {
                let from = p.position().byte_index();
                if let Ok(Token::Ident(name)) = p.next().cloned() {
                    output.push((name.to_string(), from, p.position().byte_index()));
                }
            }
            Token::Function(_) | Token::ParenthesisBlock => {
                let _: Result<(), cssparser::ParseError<()>> = p.parse_nested_block(|p| {
                    selector_classes(p, output, depth + 1);
                    Ok(())
                });
            }
            Token::SquareBracketBlock => {
                let _: Result<(),cssparser::ParseError<()>>=p.parse_nested_block(|p| {
                    if !matches!(p.next().cloned(),Ok(Token::Ident(key)) if key.eq_ignore_ascii_case("class")) {return Ok(());}
                    if !matches!(p.next().cloned(),Ok(Token::IncludeMatch | Token::Delim('='))) {return Ok(());}
                    match p.next().cloned() {
                        Ok(Token::Ident(value))=>{let to=p.position().byte_index();output.push((value.to_string(),to-value.len(),to));}
                        Ok(Token::QuotedString(value))=>{
                            let to=p.position().byte_index();let from=to-value.len()-1;let mut start=0;
                            for name in value.split_whitespace(){let at=value[start..].find(name).unwrap()+start;output.push((name.into(),from+at,from+at+name.len()));start=at+name.len();}
                        },_=>{}
                    }Ok(())
                });
            }
            _ => {}
        }
    }
}
/// Parse original CSS with the same tokenizer used by the compiler.
pub fn parse(css_source: &str, uri: &str, loc: Loc) -> Index {
    let mut out = Index::default();
    let Ok((_, sheet)) = css::extract(css_source, uri, loc, None) else {
        return out;
    };
    for (name, l) in sheet.imports {
        out.imports.push((name, l.offset, l.offset + 7));
    }
    let mut seen = BTreeSet::new();
    for rule in sheet.rules {
        let position = rule
            .declarations
            .first()
            .map(|d| d.loc.offset.saturating_sub(loc.offset))
            .unwrap_or(css_source.len())
            .min(css_source.len());
        let start = css_source[..position]
            .rfind(&rule.selector)
            .unwrap_or(position);
        let mut names = vec![];
        selector_classes(&mut Parser::new(&rule.selector), &mut names, 0);
        for (name, from, to) in names {
            if seen.insert(("class", name.to_string(), start + from)) {
                out.symbols.push(Symbol {
                    name: name.to_string(),
                    kind: "class",
                    uri: uri.into(),
                    from: loc.offset + start + from,
                    to: loc.offset + start + to,
                    value: String::new(),
                    selector: rule.selector.clone(),
                    declarations: rule.declarations.clone(),
                });
            }
        }
        for d in &rule.declarations {
            if d.property.starts_with("--")
                && seen.insert(("variable", d.property.clone(), d.loc.offset))
            {
                let from = d.loc.offset;
                let local = from.saturating_sub(loc.offset).min(css_source.len());
                let length = css_source[local..]
                    .split(':')
                    .next()
                    .unwrap_or(&d.property)
                    .trim_end()
                    .len();
                out.symbols.push(Symbol {
                    name: d.property.clone(),
                    kind: "variable",
                    uri: uri.into(),
                    from,
                    to: from + length,
                    value: d.value.clone(),
                    selector: rule.selector.clone(),
                    declarations: vec![d.clone()],
                });
            }
        }
    }
    for k in sheet.keyframes {
        let local = k
            .loc
            .offset
            .saturating_sub(loc.offset)
            .min(css_source.len());
        let from = css_source[local..]
            .find(&k.name)
            .map(|p| loc.offset + local + p)
            .unwrap_or(k.loc.offset);
        if seen.insert(("keyframe", k.name.clone(), from)) {
            out.symbols.push(Symbol {
                name: k.name.clone(),
                kind: "keyframe",
                uri: uri.into(),
                from,
                to: from + k.name.len(),
                value: {
                    let start = local;
                    let mut at = css_source[local..]
                        .find('{')
                        .map(|i| local + i)
                        .unwrap_or(css_source.len());
                    let mut depth = 0;
                    while at < css_source.len() {
                        if let Some(end) = parser::string_end(css_source, at) {
                            at = end;
                            continue;
                        }
                        match css_source.as_bytes()[at] {
                            b'{' => depth += 1,
                            b'}' => {
                                depth -= 1;
                                if depth == 0 {
                                    at += 1;
                                    break;
                                }
                            }
                            _ => {}
                        }
                        at += css_source[at..].chars().next().unwrap().len_utf8();
                    }
                    css_source[start..at].into()
                },
                selector: String::new(),
                declarations: k.frames.iter().flat_map(|(_, ds)| ds.clone()).collect(),
            });
        }
    }
    out
}
static BUNDLED: LazyLock<BTreeMap<String, Arc<Index>>> = LazyLock::new(|| {
    laymesh_core::style::bundled_stylesheets()
        .iter()
        .map(|l| {
            let uri = library_uri(&l.path());
            let index = parse(
                content(&uri).unwrap(),
                &uri,
                Loc {
                    line: 1,
                    column: 1,
                    offset: 0,
                },
            );
            (uri, Arc::new(index))
        })
        .collect()
});
fn constants(expr: &Expr, values: &BTreeMap<String, Vec<String>>) -> Option<Vec<String>> {
    match &expr.kind {
        ExprKind::String(s, _, false) => Some(vec![s.clone()]),
        ExprKind::Ref(names) if names.len() == 1 => values.get(&names[0]).cloned(),
        ExprKind::List(items) => items
            .iter()
            .map(|e| constants(e, values))
            .collect::<Option<Vec<_>>>()
            .map(|v| v.into_iter().flatten().collect()),
        ExprKind::Binary(op, a, b) if op == "+" => {
            let a = constants(a, values)?;
            let b = constants(b, values)?;
            if a.len() == 1 && b.len() == 1 {
                Some(vec![format!("{}{}", a[0], b[0])])
            } else {
                None
            }
        }
        _ => None,
    }
}
fn stylesheet_refs(
    expr: &Expr,
    values: &BTreeMap<String, Vec<String>>,
    source: &str,
    out: &mut Index,
) {
    if let ExprKind::List(items) = &expr.kind {
        for item in items {
            stylesheet_refs(item, values, source, out);
        }
        return;
    }
    if let Some(paths) = constants(expr, values) {
        let from = expr.loc.offset;
        let to = parser::string_end(source, from).unwrap_or_else(|| match &expr.kind {
            ExprKind::Ref(names) => from + names.join(".").len(),
            _ => from + 1,
        });
        for path in paths {
            out.imports.push((path, from, to));
        }
    }
}
fn calls(expr: &Expr, values: &BTreeMap<String, Vec<String>>, source: &str, out: &mut Index) {
    match &expr.kind {
        ExprKind::Call(_, args) | ExprKind::Method(_, _, args) => {
            for (name, e) in args {
                if name.as_deref() == Some("stylesheet") {
                    stylesheet_refs(e, values, source, out);
                }
                calls(e, values, source, out);
            }
        }
        ExprKind::List(es) => {
            for e in es {
                calls(e, values, source, out)
            }
        }
        _ => {}
    }
}
pub fn inline_ranges(source: &str) -> Vec<(usize, usize)> {
    if let Ok(ts) = parser::tokenize(source, "/index.lay") {
        return ts
            .into_iter()
            .filter(|t| t.text.starts_with("@style"))
            .map(|t| (t.loc.offset, t.end - 1))
            .collect();
    }
    let ts = tokens(source);
    let mut out = vec![];
    let mut previous_end = 0;
    for i in 0..ts.len().saturating_sub(1) {
        if ts[i].from < previous_end || ts[i].text != "style" || ts[i + 1].text != "{" {
            continue;
        }
        let from = ts[i + 1].to;
        let mut at = from;
        let mut depth = 1;
        while at < source.len() {
            if source[at..].starts_with("/*") {
                at = source[at + 2..]
                    .find("*/")
                    .map(|p| at + p + 4)
                    .unwrap_or(source.len());
                continue;
            }
            if let Some(end) = parser::string_end(source, at) {
                at = end;
                continue;
            }
            match source.as_bytes()[at] {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            at += source[at..].chars().next().unwrap().len_utf8();
        }
        out.push((from, at));
        previous_end = at;
    }
    out
}
pub fn document(source: &str, uri: &str) -> Index {
    if uri.ends_with(".css") || uri.ends_with(".lcss") || content(uri).is_some() {
        return parse(
            source,
            uri,
            Loc {
                line: 1,
                column: 1,
                offset: 0,
            },
        );
    }
    let mut out = Index::default();
    let ts = tokens(source);
    for (from, end) in inline_ranges(source) {
        let parsed = parse(&source[from..end], uri, origin(source, from));
        out.symbols.extend(parsed.symbols);
        out.imports.extend(parsed.imports);
    }
    if let Ok(stmts) = parser::parse(source, uri) {
        let mut values = BTreeMap::new();
        for stmt in stmts {
            match stmt.kind {
                StmtKind::Bind(name, e, _) => {
                    calls(&e, &values, source, &mut out);
                    if let Some(v) = constants(&e, &values) {
                        values.insert(name, v);
                    }
                }
                StmtKind::Expr(e) => calls(&e, &values, source, &mut out),
                _ => {}
            }
        }
    } else {
        // Recover completed imports while the surrounding Lay call is unfinished.
        for t in &ts {
            if t.kind != "string" || css_region(source, t.from).is_some() {
                continue;
            }
            if !context(source, t.from).is_some_and(|c| c.parameter == "stylesheet") {
                continue;
            }
            if let Ok(e) = parser::expression(&t.text, uri) {
                if let Some(paths) = constants(&e, &BTreeMap::new()) {
                    for path in paths {
                        out.imports.push((path, t.from, t.to));
                    }
                }
            }
        }
    }
    // Module imports load separate lexical scopes: retain them as dependencies,
    // but do not expose their local CSS classes as entry-page declarations.
    out.imports.sort();
    out.imports.dedup();
    out
}
impl LanguageService {
    pub fn stylesheet_source(&self, uri: &str) -> &str {
        content(uri).unwrap_or_else(|| self.source(uri))
    }
    pub fn stylesheet_content(&self, uri: &str) -> J {
        content(uri).map(|text|json!({"uri":library_uri(&library_path(uri)),"text":text,"languageId":"css","readOnly":true})).unwrap_or(J::Null)
    }
    fn css_index(&self, uri: &str) -> Arc<Index> {
        if let Some(index) = BUNDLED.get(&library_uri(&library_path(uri))) {
            return index.clone();
        }
        use std::hash::{Hash, Hasher};
        let source = self.source(uri);
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        source.hash(&mut hash);
        let fingerprint = hash.finish();
        if let Some((h, index)) = self.stylesheet_indexes.borrow().get(uri) {
            if *h == fingerprint {
                return index.clone();
            }
        }
        let index = Arc::new(document(source, uri));
        self.stylesheet_indexes
            .borrow_mut()
            .insert(uri.into(), (fingerprint, index.clone()));
        index
    }
    pub fn stylesheet_dependencies(&self, uri: &str) -> Vec<(String, usize, usize)> {
        self.css_index(uri)
            .imports
            .iter()
            .map(|(p, a, b)| (target(uri, p), *a, *b))
            .collect()
    }
    fn visit_stylesheets(
        &self,
        uri: &str,
        active: &mut BTreeSet<String>,
        seen: &mut BTreeSet<String>,
        symbols: &mut Vec<Symbol>,
        issues: &mut Vec<J>,
        root: &str,
        reference: Option<(usize, usize)>,
    ) {
        if !seen.insert(uri.into()) {
            return;
        }
        active.insert(uri.into());
        let index = self.css_index(uri);
        for (path, from, to) in &index.imports {
            let dest = target(uri, path);
            let (a, b) = reference.unwrap_or((*from, *to));
            if active.contains(&dest) {
                issues.push(json!({"from":a,"to":b,"severity":"error","code":"E_LCSS","message":format!("CSS import cycle / 样式导入循环：{dest}")}));
                continue;
            }
            if content(&dest).is_none() && !self.documents.contains_key(&dest) {
                issues.push(json!({"from":a,"to":b,"severity":"error","code":"E_ASSET","message":format!("Cannot read stylesheet / 无法读取样式：{path}")}));
                continue;
            }
            self.visit_stylesheets(&dest, active, seen, symbols, issues, root, Some((a, b)));
        }
        let _ = root;
        symbols.extend(index.symbols.clone());
        active.remove(uri);
    }
    pub fn stylesheet_symbols(&self, uri: &str) -> Vec<Symbol> {
        let mut out = vec![];
        self.visit_stylesheets(
            uri,
            &mut BTreeSet::new(),
            &mut BTreeSet::new(),
            &mut out,
            &mut vec![],
            uri,
            None,
        );
        out
    }
    pub fn stylesheet_issues(&self, uri: &str) -> Vec<J> {
        let mut out = vec![];
        self.visit_stylesheets(
            uri,
            &mut BTreeSet::new(),
            &mut BTreeSet::new(),
            &mut vec![],
            &mut out,
            uri,
            None,
        );
        out
    }
    pub fn stylesheet_paths(&self, uri: &str, from: usize, to: usize, quoted: bool) -> J {
        let mut paths = BTreeMap::new();
        for l in laymesh_core::style::bundled_stylesheets() {
            paths.insert(
                l.path(),
                format!("{} {} · {}", l.name, l.version, l.license),
            );
        }
        let base: Vec<_> = uri
            .rsplit_once('/')
            .map(|(p, _)| p.split('/').collect())
            .unwrap_or_default();
        for file in self.documents.keys().filter(|f| {
            (*f != uri)
                && (f.ends_with(".css") || f.ends_with(".lcss"))
                && !self.source(uri)[from..].starts_with("@laymesh/")
        }) {
            let parts: Vec<_> = file.split('/').collect();
            let common = base.iter().zip(&parts).take_while(|(a, b)| a == b).count();
            if common == 0 {
                continue;
            }
            let relative = std::iter::repeat_n("..", base.len() - common)
                .chain(parts[common..].iter().copied())
                .collect::<Vec<_>>()
                .join("/");
            let relative = if relative.starts_with("../") {
                relative
            } else {
                format!("./{relative}")
            };
            let mut decoded = Vec::new();
            let b = relative.as_bytes();
            let mut i = 0;
            while i < b.len() {
                if b[i] == b'%' && i + 2 < b.len() {
                    if let Ok(n) = u8::from_str_radix(&relative[i + 1..i + 3], 16) {
                        decoded.push(n);
                        i += 3;
                        continue;
                    }
                }
                decoded.push(b[i]);
                i += 1;
            }
            let relative = String::from_utf8_lossy(&decoded).to_string();
            paths.insert(relative.clone(), file.clone());
            for (i, c) in relative.char_indices() {
                if c == '/' && i > 1 {
                    paths
                        .entry(relative[..=i].into())
                        .or_insert_with(|| "Directory / 目录".into());
                }
            }
        }
        json!(paths.into_iter().map(|(path,detail)|{let info=laymesh_core::style::bundled_stylesheets().iter().find(|l|l.path()==path).map(|l|crate::stylesheet_doc(l,self.en())).unwrap_or_else(||detail.clone());json!({"label":path,"type":"text","detail":detail,"info":info,"apply":if quoted{path}else{json!(path).to_string()},"from":from,"to":to})}).collect::<Vec<_>>())
    }
    pub fn stylesheet_completions(&self, uri: &str, kind: &str, from: usize, to: usize) -> J {
        let mut names = BTreeMap::new();
        for s in self.stylesheet_symbols(uri) {
            if s.kind == kind {
                names.insert(s.name.clone(), s);
            }
        }
        json!(names.values().map(|s|json!({"label":s.name,"type":if kind=="keyframe"{"value"}else{kind},"detail":laymesh_core::style::bundled_stylesheets().iter().find(|l|l.path()==library_path(&s.uri)).map(|l|format!("{} {} · {}",l.name,l.version,l.path())).unwrap_or_else(||library_path(&s.uri)),"info":s.documentation(self.en()),"apply":s.name,"from":from,"to":to})).collect::<Vec<_>>())
    }
    pub fn stylesheet_at(
        &self,
        uri: &str,
        offset: usize,
    ) -> Option<(usize, usize, String, &'static str)> {
        let source = self.source(uri);
        let (from, to, w) = crate::word(source, offset);
        if w.is_empty() {
            return None;
        }
        let before = &source[..from];
        let kind = if w.starts_with("--") {
            "variable"
        } else if css_region(source, offset).is_some()
            || uri.ends_with(".lcss")
            || uri.ends_with(".css")
        {
            if before.trim_end().ends_with('.') {
                "class"
            } else if before.rfind("animation").is_some() {
                "keyframe"
            } else {
                return None;
            }
        } else {
            let token = tokens(source)
                .into_iter()
                .find(|t| t.kind == "string" && t.from <= offset && offset <= t.to)?;
            let c = context(source, token.from)?;
            if ["class", "classes"].contains(&c.parameter.as_str()) {
                "class"
            } else if c.parameter == "name" && c.name == "animate" {
                "keyframe"
            } else {
                return None;
            }
        };
        Some((from, to, w, kind))
    }
    pub fn stylesheet_definition(&self, uri: &str, offset: usize) -> J {
        if let Some((from, to, _)) = crate::stylesheet_range(
            self.source(uri),
            offset,
            uri.ends_with(".css")
                || uri.ends_with(".lcss")
                || css_region(self.source(uri), offset).is_some(),
        ) {
            let dest = target(uri, &self.source(uri)[from..to]);
            if content(&dest).is_some() || self.documents.contains_key(&dest) {
                return json!({"uri":dest,"from":0,"to":0});
            }
        }
        let Some((_, _, name, kind)) = self.stylesheet_at(uri, offset) else {
            return J::Null;
        };
        let mut seen = BTreeSet::new();
        let locations = self
            .stylesheet_symbols(uri)
            .iter()
            .filter(|s| s.kind == kind && s.name == name)
            .filter(|s| seen.insert((s.uri.clone(), s.from)))
            .map(Symbol::location)
            .collect::<Vec<_>>();
        match locations.len() {
            0 => J::Null,
            1 => locations[0].clone(),
            _ => json!(locations),
        }
    }
}
