//! Static resource references for workspace file operations. No source execution.
use crate::{LanguageService, index::tokens};
use laymesh_core::parser::{self, Expr, ExprKind, Stmt, StmtKind};
use serde_json::{Value as J, json};
use std::collections::{BTreeMap, BTreeSet};

pub fn references(service: &LanguageService, uri: &str) -> J {
    let source = service.documents.get(uri).map(String::as_str).unwrap_or("");
    let mut scan = Scan {
        uri,
        source,
        out: vec![],
        bindings: BTreeMap::new(),
        seen: BTreeSet::new(),
    };
    if uri.ends_with(".css") || uri.ends_with(".lcss") {
        scan.css(source, 0);
    } else if uri.ends_with(".lay") || uri.ends_with(".laydoc") {
        match parser::parse(source,uri) {
            Ok(stmts)=>{scan.bindings(&stmts);scan.statements(&stmts);},
            Err(e)=>scan.out.push(json!({"uri":uri,"from":e.loc.offset,"to":e.loc.offset,"dynamic":true,"message":"文件有语法错误，无法完整检查路径引用"})),
        }
    }
    json!(scan.out)
}
struct Scan<'a> {
    uri: &'a str,
    source: &'a str,
    out: Vec<J>,
    bindings: BTreeMap<String, Option<Expr>>,
    seen: BTreeSet<usize>,
}
impl Scan<'_> {
    fn add(&mut self, path: &str, from: usize, to: usize, encoding: &str) {
        if path.starts_with('@') || path.contains(":") || path.starts_with('#') {
            return;
        }
        let (resource, suffix) = path
            .split_once('#')
            .map_or((path, String::new()), |(a, b)| (a, format!("#{b}")));
        if self.seen.insert(from) {
            self.out.push(json!({"uri":self.uri,"from":from,"to":to,"path":path,"target":crate::resolve(self.uri,resource),"suffix":suffix,"encoding":encoding}));
        }
    }
    fn dynamic(&mut self, e: &Expr) {
        if self.seen.insert(e.loc.offset) {
            self.out.push(json!({"uri":self.uri,"from":e.loc.offset,"to":e.loc.offset,"dynamic":true,"message":"动态路径需要手动检查"}));
        }
    }
    fn bindings(&mut self, stmts: &[Stmt]) {
        for s in stmts {
            match &s.kind {
                StmtKind::Bind(name, e, _) => {
                    if self.bindings.contains_key(name) {
                        self.bindings.insert(name.clone(), None);
                    } else {
                        self.bindings.insert(name.clone(), Some(e.clone()));
                    }
                }
                StmtKind::If(branches, otherwise) => {
                    for (_, ss) in branches {
                        self.conditional_bindings(ss)
                    }
                    self.conditional_bindings(otherwise)
                }
                StmtKind::While(_, ss) | StmtKind::For(_, _, ss) => self.conditional_bindings(ss),
                // Scope-dependent bindings are deliberately not resolved by name globally.
                _ => {}
            }
        }
    }
    fn conditional_bindings(&mut self, ss: &[Stmt]) {
        for s in ss {
            match &s.kind {
                StmtKind::Bind(name, _, _) => {
                    self.bindings.insert(name.clone(), None);
                }
                StmtKind::If(branches, otherwise) => {
                    for (_, ss) in branches {
                        self.conditional_bindings(ss)
                    }
                    self.conditional_bindings(otherwise)
                }
                StmtKind::While(_, ss) | StmtKind::For(_, _, ss) => self.conditional_bindings(ss),
                _ => {}
            }
        }
    }
    fn font_path(&mut self, e: &Expr, chain: &mut BTreeSet<String>) {
        match &e.kind {
            ExprKind::String(path, _, false) => {
                let path = path.split('#').next().unwrap_or(path).to_ascii_lowercase();
                if [".ttf", ".otf", ".ttc", ".otc"]
                    .iter()
                    .any(|ext| path.ends_with(ext))
                {
                    self.path(e, &mut BTreeSet::new());
                }
            }
            ExprKind::List(es) => {
                for e in es {
                    self.font_path(e, chain)
                }
            }
            ExprKind::Ref(parts) if parts.len() == 1 && chain.insert(parts[0].clone()) => {
                if let Some(Some(value)) = self.bindings.get(&parts[0]).cloned() {
                    self.font_path(&value, chain)
                } else {
                    self.dynamic(e)
                }
                chain.remove(&parts[0]);
            }
            _ => self.dynamic(e),
        }
    }
    fn path(&mut self, e: &Expr, chain: &mut BTreeSet<String>) {
        match &e.kind {
            ExprKind::String(path, _, false) => {
                if let Some(t) = tokens(self.source)
                    .into_iter()
                    .find(|t| t.from == e.loc.offset && t.kind == "string")
                {
                    self.add(path, t.from, t.to, "string");
                }
            }
            ExprKind::List(es) => {
                for e in es {
                    self.path(e, chain)
                }
            }
            ExprKind::Dict(es) => {
                for (_, e) in es {
                    self.path(e, chain)
                }
            }
            ExprKind::Ref(parts) if parts.len() == 1 && chain.insert(parts[0].clone()) => {
                if let Some(Some(value)) = self.bindings.get(&parts[0]).cloned() {
                    self.path(&value, chain);
                } else {
                    self.dynamic(e)
                }
                chain.remove(&parts[0]);
            }
            _ => self.dynamic(e),
        }
    }
    fn expr(&mut self, e: &Expr) {
        match &e.kind {
            ExprKind::Call(parts, args) => {
                for (name, v) in args {
                    let builtin = parts.len() == 1
                        && laymesh_core::engine::API["api"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .any(|a| a["name"] == parts[0]);
                    if builtin && matches!(name.as_deref(), Some("src" | "stylesheet" | "textures"))
                    {
                        self.path(v, &mut BTreeSet::new());
                    } else if builtin && name.as_deref() == Some("font_family") {
                        self.font_path(v, &mut BTreeSet::new());
                    }
                    self.expr(v);
                }
            }
            ExprKind::Method(receiver, _, args) => {
                self.expr(receiver);
                for (_, v) in args {
                    self.expr(v)
                }
            }
            ExprKind::List(es) => {
                for e in es {
                    self.expr(e)
                }
            }
            ExprKind::Dict(es) => {
                for (_, v) in es {
                    self.expr(v)
                }
            }
            ExprKind::Index(a, b) | ExprKind::Binary(_, a, b) => {
                self.expr(a);
                self.expr(b)
            }
            ExprKind::Member(e, _) | ExprKind::Unary(_, e) => self.expr(e),
            _ => {}
        }
    }
    fn statements(&mut self, ss: &[Stmt]) {
        for s in ss {
            match &s.kind {
                StmtKind::Bind(_, e, _) | StmtKind::Expr(e) | StmtKind::Return(e) => self.expr(e),
                StmtKind::SetIndex(a, b) => {
                    self.expr(a);
                    self.expr(b)
                }
                StmtKind::Import(_, path) => {
                    if let Some(t) = tokens(self.source)
                        .into_iter()
                        .find(|t| t.from >= s.loc.offset && t.kind == "string")
                    {
                        self.add(path, t.from, t.to, "string")
                    }
                }
                StmtKind::Style(_) => {} // Inline CSS ranges below use the existing stylesheet parser.
                StmtKind::Function(_, params, ss, _) => {
                    let saved = self.bindings.clone();
                    for (name, _) in params {
                        self.bindings.insert(name.clone(), None);
                    }
                    self.bindings(ss);
                    self.statements(ss);
                    self.bindings = saved;
                }
                StmtKind::For(pattern, e, ss) => {
                    self.expr(e);
                    let saved = self.bindings.clone();
                    for name in pattern.names() {
                        self.bindings.insert(name, None);
                    }
                    self.bindings(ss);
                    self.statements(ss);
                    self.bindings = saved;
                }
                StmtKind::While(e, ss) => {
                    self.expr(e);
                    self.statements(ss)
                }
                StmtKind::If(branches, otherwise) => {
                    for (e, ss) in branches {
                        self.expr(e);
                        self.statements(ss)
                    }
                    self.statements(otherwise)
                }
                _ => {}
            }
        }
        for (from, to) in crate::stylesheets::inline_ranges(self.source) {
            self.css(&self.source[from..to], from);
        }
    }
    fn css(&mut self, source: &str, offset: usize) {
        let mut parser = cssparser::Parser::new(source);
        self.css_tokens(&mut parser, offset);
    }
    fn css_tokens(&mut self, p: &mut cssparser::Parser<'_>, offset: usize) {
        let mut import = false;
        let mut property = String::new();
        let mut value = false;
        loop {
            p.skip_whitespace();
            let from = p.position().byte_index();
            let Ok(token) = p.next().cloned() else { break };
            let to = p.position().byte_index();
            match token {
                cssparser::Token::Ident(name) if !value => property = name.to_ascii_lowercase(),
                cssparser::Token::Colon => value = true,
                cssparser::Token::Semicolon => {
                    value = false;
                    property.clear();
                    import = false;
                }
                cssparser::Token::AtKeyword(ref name) if name.eq_ignore_ascii_case("import") => {
                    import = true
                }
                cssparser::Token::QuotedString(path) if import => {
                    self.add(&path, offset + from, offset + to, "string");
                    import = false;
                }
                cssparser::Token::QuotedString(path) if value && property == "font-family" => {
                    let resource = path.split('#').next().unwrap_or(&path).to_ascii_lowercase();
                    if [".ttf", ".otf", ".ttc", ".otc"]
                        .iter()
                        .any(|ext| resource.ends_with(ext))
                    {
                        self.add(&path, offset + from, offset + to, "string");
                    }
                }
                cssparser::Token::UnquotedUrl(path) => {
                    self.add(&path, offset + from, offset + to, "css-url")
                }
                cssparser::Token::Function(ref name) if name.eq_ignore_ascii_case("url") => {
                    let path = p.parse_nested_block(|nested| {
                        nested
                            .expect_string_cloned()
                            .map_err(cssparser::ParseError::<()>::from)
                    });
                    if let Ok(path) = path {
                        self.add(
                            &path,
                            offset + from,
                            offset + p.position().byte_index(),
                            "css-url",
                        );
                    }
                }
                cssparser::Token::CurlyBracketBlock
                | cssparser::Token::ParenthesisBlock
                | cssparser::Token::SquareBracketBlock
                | cssparser::Token::Function(_) => {
                    let _: Result<(), cssparser::ParseError<()>> = p.parse_nested_block(|nested| {
                        self.css_tokens(nested, offset);
                        Ok(())
                    });
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn references_are_semantic_not_text_replacements() {
        let uri = "/project/main.laydoc";
        let source = "asset=\"assets/a.png\"\ndoc=document(transition=transition(name=\"luma\",textures={\"luma\":asset}))\ndoc.add(page(src=\"pages/a.lay\"))\nlabel=\"pages/a.lay\"\n# pages/a.lay";
        let mut s = LanguageService::default();
        s.documents.insert(uri.into(), source.into());
        let refs = references(&s, uri);
        let refs = refs.as_array().unwrap();
        assert_eq!(refs.len(), 2, "{refs:?}");
        assert_eq!(refs[0]["target"], "/project/assets/a.png");
        assert_eq!(refs[1]["target"], "/project/pages/a.lay");
    }
    #[test]
    fn css_urls_and_imports_preserve_non_resource_strings() {
        let uri = "/project/styles/main.lcss";
        let source = "@import \"other.css\"; canvas {-laymesh-transition-texture-luma:url('../assets/a.png');background:url(../assets/b.png);content:'../assets/c.png'}";
        let mut s = LanguageService::default();
        s.documents.insert(uri.into(), source.into());
        let refs = references(&s, uri);
        let refs = refs.as_array().unwrap();
        assert_eq!(refs.len(), 3, "{refs:?}");
        for r in refs {
            assert!(
                source[r["from"].as_u64().unwrap() as usize..r["to"].as_u64().unwrap() as usize]
                    .contains(r["path"].as_str().unwrap())
            );
        }
    }
    #[test]
    fn font_paths_and_conditional_paths_are_distinguished() {
        let uri = "/project/main.lay";
        let source = "fonts=[\"微软雅黑\",\"fonts/a.ttc#1\"]\np=canvas(font_family=fonts)\nasset=\"a.png\"\nif true { asset=\"b.png\" }\np.add(image(src=asset))";
        let mut s = LanguageService::default();
        s.documents.insert(uri.into(), source.into());
        let refs = references(&s, uri);
        let refs = refs.as_array().unwrap();
        assert_eq!(refs.len(), 2, "{refs:?}");
        assert_eq!(refs[0]["target"], "/project/fonts/a.ttc");
        assert_eq!(refs[0]["suffix"], "#1");
        assert_eq!(refs[1]["dynamic"], true);
    }
}
