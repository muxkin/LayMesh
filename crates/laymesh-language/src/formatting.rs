//! Lossless, static formatting. Source spelling is retained; reparsing is the
//! final guard against whitespace changing the meaning of LayMesh units.
use crate::{LanguageService, offset, position};
use laymesh_core::{
    Loc,
    parser::{self, Stmt, StmtKind},
};
use serde_json::{Value as J, json};
use std::collections::BTreeSet;

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Token,
    String,
    Comment,
    Style,
}
#[derive(Clone)]
struct Token {
    text: String,
    from: usize,
    to: usize,
    kind: Kind,
}
#[derive(Clone)]
enum Node {
    Token(Token),
    Group {
        open: Token,
        close: Token,
        children: Vec<Node>,
    },
}
impl Node {
    fn from(&self) -> usize {
        match self {
            Self::Token(t) => t.from,
            Self::Group { open, .. } => open.from,
        }
    }
    fn to(&self) -> usize {
        match self {
            Self::Token(t) => t.to,
            Self::Group { close, .. } => close.to,
        }
    }
    fn first(&self) -> &Token {
        match self {
            Self::Token(t) => t,
            Self::Group { open, .. } => open,
        }
    }
    fn last(&self) -> &Token {
        match self {
            Self::Token(t) => t,
            Self::Group { close, .. } => close,
        }
    }
}
enum Doc {
    Text(String),
    Line(bool),
    Soft,
    Join(Vec<Doc>),
    Nest(Box<Doc>),
    Group(Box<Doc>),
}
fn text(s: impl Into<String>) -> Doc {
    Doc::Text(s.into())
}
fn join(ds: Vec<Doc>) -> Doc {
    Doc::Join(ds)
}
fn line() -> Doc {
    Doc::Line(false)
}
fn hardline() -> Doc {
    Doc::Line(true)
}
fn flat_width(d: &Doc) -> Option<usize> {
    match d {
        Doc::Text(s) if !s.contains(['\n', '\r']) => Some(s.chars().count()),
        Doc::Text(_) | Doc::Line(true) => None,
        Doc::Line(false) => Some(1),
        Doc::Soft => Some(0),
        Doc::Join(ds) => ds
            .iter()
            .try_fold(0usize, |n, d| n.checked_add(flat_width(d)?)),
        Doc::Nest(d) | Doc::Group(d) => flat_width(d),
    }
}
struct Options {
    unit: String,
    tab: usize,
    width: usize,
    eol: &'static str,
}
fn render(d: &Doc, options: &Options, depth: usize) -> String {
    let mut out = String::new();
    let mut pending_indent = Some(depth);
    let mut col = depth * options.tab;
    let mut stack = vec![(depth, false, d)];
    while let Some((indent, flat, d)) = stack.pop() {
        match d {
            Doc::Text(s) => {
                if !s.is_empty() {
                    if let Some(n) = pending_indent.take() {
                        out.push_str(&options.unit.repeat(n));
                    }
                }
                out.push_str(s);
                col = if let Some(last) = s.rsplit('\n').next().filter(|_| s.contains('\n')) {
                    last.chars().count()
                } else {
                    col + s.chars().count()
                };
            }
            Doc::Line(hard) => {
                if flat && !hard {
                    if let Some(n) = pending_indent.take() {
                        out.push_str(&options.unit.repeat(n));
                    }
                    out.push(' ');
                    col += 1;
                } else {
                    // Only generated indentation is pending here; strings and
                    // comments, including their internal whitespace, are opaque.
                    out.push_str(options.eol);
                    pending_indent = Some(indent);
                    col = indent * options.tab;
                }
            }
            Doc::Soft => {
                if !flat {
                    out.push_str(options.eol);
                    pending_indent = Some(indent);
                    col = indent * options.tab;
                }
            }
            Doc::Join(ds) => {
                for child in ds.iter().rev() {
                    stack.push((indent, flat, child));
                }
            }
            Doc::Nest(child) => stack.push((indent + 1, flat, child)),
            Doc::Group(child) => stack.push((
                indent,
                flat || flat_width(child).is_some_and(|n| col + n <= options.width),
                child,
            )),
        }
    }
    out
}

fn lay_tokens(source: &str, uri: &str) -> Option<Vec<Token>> {
    let ts = parser::tokenize(source, uri).ok()?;
    let mut out = Vec::new();
    let mut end = 0;
    for t in ts.iter().filter(|t| !t.text.is_empty()) {
        let style = t.text.starts_with("@style") && !t.string;
        let from = if style {
            source[..t.loc.offset].rfind("style")?
        } else {
            t.loc.offset
        };
        let mut at = end;
        while at < from {
            if source.as_bytes()[at] == b'#' {
                let to = source[at..from].find('\n').map_or(from, |n| at + n);
                let to = if source.as_bytes().get(to.wrapping_sub(1)) == Some(&b'\r') {
                    to - 1
                } else {
                    to
                };
                out.push(Token {
                    text: source[at..to].into(),
                    from: at,
                    to,
                    kind: Kind::Comment,
                });
                at = to;
            } else {
                at += 1;
            }
        }
        out.push(Token {
            text: source[from..t.end].into(),
            from,
            to: t.end,
            kind: if style {
                Kind::Style
            } else if t.string {
                Kind::String
            } else {
                Kind::Token
            },
        });
        end = t.end;
    }
    let mut at = end;
    while at < source.len() {
        if source.as_bytes()[at] == b'#' {
            let to = source[at..].find('\n').map_or(source.len(), |n| at + n);
            let to = if source.as_bytes().get(to.wrapping_sub(1)) == Some(&b'\r') {
                to - 1
            } else {
                to
            };
            out.push(Token {
                text: source[at..to].into(),
                from: at,
                to,
                kind: Kind::Comment,
            });
            at = to;
        } else {
            at += 1;
        }
    }
    Some(out)
}

fn css_tokens(source: &str, base: usize) -> Option<Vec<Token>> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < source.len() {
        let c = source[at..].chars().next()?;
        if c.is_whitespace() {
            at += c.len_utf8();
            continue;
        }
        let from = at;
        let kind;
        if source[at..].starts_with("/*") {
            at += source[at + 2..].find("*/")? + 4;
            kind = Kind::Comment;
        } else if matches!(c, '\'' | '"') {
            at += 1;
            let mut closed = false;
            while at < source.len() {
                let ch = source[at..].chars().next()?;
                at += ch.len_utf8();
                if ch == '\\' {
                    at += source[at..].chars().next()?.len_utf8();
                } else if ch == c {
                    closed = true;
                    break;
                }
            }
            if !closed {
                return None;
            }
            kind = Kind::String;
        } else if "{}()[]:;,".contains(c) {
            at += 1;
            kind = Kind::Token;
        } else {
            at += c.len_utf8();
            while at < source.len() {
                let ch = source[at..].chars().next()?;
                if ch.is_whitespace()
                    || "{}()[]:;,\"'".contains(ch)
                    || source[at..].starts_with("/*")
                {
                    break;
                }
                if ch == '\\' {
                    at += 1;
                    at += source[at..].chars().next()?.len_utf8();
                } else {
                    at += ch.len_utf8();
                }
            }
            kind = Kind::Token;
        }
        out.push(Token {
            text: source[from..at].into(),
            from: base + from,
            to: base + at,
            kind,
        });
    }
    Some(out)
}
fn nodes(ts: &[Token], at: &mut usize, end: Option<&str>, depth: usize) -> Option<Vec<Node>> {
    if depth > 256 {
        return None;
    }
    let mut out = Vec::new();
    while let Some(t) = ts.get(*at) {
        if t.kind == Kind::Token && matches!(t.text.as_str(), ")" | "]" | "}") {
            return (end == Some(t.text.as_str())).then_some(out);
        }
        *at += 1;
        let close = if t.kind == Kind::Token {
            match t.text.as_str() {
                "(" => Some(")"),
                "[" => Some("]"),
                "{" => Some("}"),
                _ => None,
            }
        } else {
            None
        };
        if let Some(close) = close {
            let children = nodes(ts, at, Some(close), depth + 1)?;
            let closing = ts.get(*at)?.clone();
            *at += 1;
            out.push(Node::Group {
                open: t.clone(),
                close: closing,
                children,
            });
        } else {
            out.push(Node::Token(t.clone()));
        }
    }
    end.is_none().then_some(out)
}

#[derive(Clone)]
struct Scope {
    from: usize,
    to: usize,
    depth: usize,
    spans: Vec<(usize, usize)>,
    css: bool,
    declaration: bool,
}
struct Layout<'a> {
    source: &'a str,
    starts: BTreeSet<usize>,
    blocks: BTreeSet<usize>,
    scopes: Vec<Scope>,
}
fn actual_start(stmt: &Stmt, ts: &[Token]) -> Option<usize> {
    if matches!(stmt.kind, StmtKind::Style(_)) {
        ts.iter()
            .find(|t| t.kind == Kind::Style && t.from <= stmt.loc.offset && t.to >= stmt.loc.offset)
            .map(|t| t.from)
    } else {
        Some(stmt.loc.offset)
    }
}
fn match_close(ts: &[Token], start: usize) -> Option<usize> {
    let mut depth = 0;
    for (i, t) in ts.iter().enumerate().skip(start) {
        if t.kind != Kind::Token {
            continue;
        }
        if matches!(t.text.as_str(), "(" | "[" | "{") {
            depth += 1;
        }
        if matches!(t.text.as_str(), ")" | "]" | "}") {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
    }
    None
}
fn body(ts: &[Token], from: usize, source: &str, uri: &str) -> Option<usize> {
    let mut at = from;
    while at < ts.len() {
        if ts[at].text == "{"
            && parser::expression(&source[ts[from].from..ts[at].from], uri).is_ok()
        {
            return Some(at);
        }
        if matches!(ts[at].text.as_str(), "(" | "[" | "{") {
            at = match_close(ts, at)? + 1;
        } else {
            at += 1;
        }
    }
    None
}
impl Layout<'_> {
    fn statements(
        &mut self,
        stmts: &[Stmt],
        ts: &[Token],
        from: usize,
        to: usize,
        depth: usize,
        uri: &str,
    ) -> Option<()> {
        let mut spans = Vec::new();
        for (n, stmt) in stmts.iter().enumerate() {
            let start = actual_start(stmt, ts)?;
            self.starts.insert(start);
            let limit = stmts
                .get(n + 1)
                .and_then(|s| actual_start(s, ts))
                .unwrap_or(to);
            let end = ts
                .get(ts.partition_point(|t| t.to <= limit).checked_sub(1)?)?
                .to;
            spans.push((start, end));
            let mut at = ts.binary_search_by_key(&start, |t| t.from).ok()?;
            if ts[at].text == "export" {
                at += 1;
            }
            let source = self.source;
            let mut add_body = |open: usize, children: &[Stmt]| -> Option<usize> {
                let close = match_close(ts, open)?;
                self.blocks.insert(ts[open].from);
                self.statements(children, ts, ts[open].to, ts[close].from, depth + 1, uri)?;
                Some(close + 1)
            };
            match &stmt.kind {
                StmtKind::Function(_, _, children, _) => {
                    let paren = at + 2;
                    let open = match_close(ts, paren)? + 1;
                    add_body(open, children)?;
                }
                StmtKind::If(branches, other) => {
                    for (_, children) in branches {
                        let open = body(ts, at + 1, source, uri)?;
                        at = add_body(open, children)?;
                        if ts.get(at).is_some_and(|t| t.text == "else") {
                            at += 1;
                        }
                    }
                    if ts.get(at).is_some_and(|t| t.text == "{") {
                        add_body(at, other)?;
                    }
                }
                StmtKind::For(_, _, children) => {
                    let expr = ts
                        .iter()
                        .enumerate()
                        .skip(at + 1)
                        .find(|(_, t)| t.text == "in")?
                        .0
                        + 1;
                    let open = body(ts, expr, source, uri)?;
                    add_body(open, children)?;
                }
                StmtKind::While(_, children) => {
                    let open = body(ts, at + 1, source, uri)?;
                    add_body(open, children)?;
                }
                _ => {}
            }
        }
        self.scopes.push(Scope {
            from,
            to,
            depth,
            spans,
            css: false,
            declaration: false,
        });
        Some(())
    }
    fn blank(&self, from: usize, to: usize) -> bool {
        self.source[from..to]
            .bytes()
            .filter(|b| *b == b'\n')
            .count()
            > 1
    }
    fn inline(&self, from: usize, to: usize) -> bool {
        !self.source[from..to].contains('\n')
    }
    fn separator(
        &self,
        prev: &Node,
        next: &Node,
        scope: bool,
        css: bool,
        declaration: bool,
        unary: bool,
    ) -> Doc {
        let p = prev.last();
        let n = next.first();
        if n.kind == Kind::Comment {
            return if self.inline(p.to, n.from) {
                text("  ")
            } else if self.blank(p.to, n.from) {
                join(vec![hardline(), hardline()])
            } else {
                hardline()
            };
        }
        if p.kind == Kind::Comment
            || (scope
                && ((!css && self.starts.contains(&next.from()))
                    || (css && (p.text == ";" || p.text == "}"))))
        {
            return if self.blank(p.to, n.from) {
                join(vec![hardline(), hardline()])
            } else {
                hardline()
            };
        }
        if css {
            if declaration && n.text == ":" {
                return text("");
            }
            if declaration && p.text == ":" {
                return text(" ");
            }
            if n.text == ";" || n.text == "," {
                return text("");
            }
            if p.text == "," {
                return if scope { text(" ") } else { line() };
            }
            if n.text == "{" {
                return text(" ");
            }
            return text(if self.source[p.to..n.from].is_empty() {
                ""
            } else {
                " "
            });
        }
        if n.text == "," || n.text == ":" || n.text == "." || p.text == "." {
            return text("");
        }
        if p.text == "," {
            return if scope { text(" ") } else { line() };
        }
        if p.text == ":" {
            return text(" ");
        }
        if unary && p.text == "-" {
            return text("");
        }
        if n.text == "(" || n.text == "[" {
            if (p.kind == Kind::Token
                && p.text
                    .as_bytes()
                    .first()
                    .is_some_and(|b| b.is_ascii_alphabetic() || *b == b'_')
                && !matches!(
                    p.text.as_str(),
                    "return" | "for" | "if" | "while" | "in" | "and" | "or" | "not"
                ))
                || matches!(p.text.as_str(), ")" | "]" | "}")
            {
                return text("");
            }
        }
        text(" ")
    }
    fn sequence(&self, ns: &[Node], scope: bool, css: bool, declaration: bool) -> Doc {
        let mut ds = Vec::new();
        for (i, n) in ns.iter().enumerate() {
            if i > 0 {
                let unary = ns[i - 1].first().text == "-"
                    && (i == 1
                        || matches!(
                            ns[i - 2].last().text.as_str(),
                            "=" | ","
                                | ":"
                                | "+"
                                | "-"
                                | "*"
                                | "/"
                                | "=="
                                | "!="
                                | "<"
                                | ">"
                                | "<="
                                | ">="
                                | "return"
                                | "in"
                                | "and"
                                | "or"
                                | "not"
                        ));
                ds.push(self.separator(&ns[i - 1], n, scope, css, declaration, unary));
            }
            ds.push(self.node(n, css));
        }
        join(ds)
    }
    fn node(&self, n: &Node, css: bool) -> Doc {
        match n {
            Node::Token(t) if t.kind == Kind::Style => {
                let open = t.text.find('{').unwrap();
                let css_from = t.from + open + 1;
                let ts = css_tokens(&t.text[open + 1..t.text.len() - 1], css_from).unwrap();
                let children = nodes(&ts, &mut 0, None, 0).unwrap();
                if children.is_empty() {
                    return text("style {}");
                }
                join(vec![
                    text("style {"),
                    Doc::Nest(Box::new(join(vec![
                        hardline(),
                        self.sequence(&children, true, true, false),
                    ]))),
                    hardline(),
                    text("}"),
                ])
            }
            Node::Token(t) => text(&t.text),
            Node::Group {
                open,
                close,
                children,
            } => {
                if children.is_empty() {
                    return text(format!("{}{}", open.text, close.text));
                }
                let block = self.blocks.contains(&open.from) || (css && open.text == "{");
                if block {
                    join(vec![
                        text(&open.text),
                        Doc::Nest(Box::new(join(vec![
                            hardline(),
                            self.sequence(children, true, css, css),
                        ]))),
                        hardline(),
                        text(&close.text),
                    ])
                } else {
                    // Boundary soft breaks have zero width when flat. They are
                    // represented as groups separately from comma soft breaks.
                    Doc::Group(Box::new(join(vec![
                        text(&open.text),
                        Doc::Nest(Box::new(join(vec![
                            Doc::Soft,
                            self.sequence(children, false, css, false),
                        ]))),
                        Doc::Soft,
                        text(&close.text),
                    ])))
                }
            }
        }
    }
}

fn css_signature(source: &str) -> Option<J> {
    let ts = css_tokens(source, 0)?;
    let mut previous: Option<&Token> = None;
    let mut sig = Vec::new();
    for t in &ts {
        let significant_gap = previous.is_some_and(|p| {
            p.kind != Kind::Comment
                && t.kind != Kind::Comment
                && !matches!(p.text.as_str(), "{" | "}" | ";" | ":" | "," | "(" | "[")
                && !matches!(t.text.as_str(), "{" | "}" | ";" | ":" | "," | ")" | "]")
                && !source[p.to..t.from].is_empty()
        });
        sig.push(json!([t.text, significant_gap]));
        previous = Some(t);
    }
    Some(json!(sig))
}
fn canonical_ast(stmts: &[Stmt]) -> Option<J> {
    fn normalize(j: &mut J) -> Option<()> {
        match j {
            J::Object(map) => {
                map.remove("loc");
                if let Some(style) = map.get_mut("Style") {
                    *style = css_signature(style.as_str()?)?;
                }
                for value in map.values_mut() {
                    normalize(value)?;
                }
            }
            J::Array(values) => {
                for value in values {
                    normalize(value)?;
                }
            }
            _ => {}
        }
        Some(())
    }
    let mut j = json!(stmts);
    normalize(&mut j)?;
    Some(j)
}
fn validate_css_tokens(ts: &[Token], source: &str, uri: &str, css: bool) -> Option<()> {
    let origin = Loc {
        line: 1,
        column: 1,
        offset: 0,
    };
    if css {
        laymesh_core::style::validate_stylesheet(source, uri, origin).ok()?;
    }
    for t in ts.iter().filter(|t| t.kind == Kind::Style) {
        let open = t.text.find('{')?;
        let raw = &t.text[open + 1..t.text.len() - 1];
        laymesh_core::style::validate_stylesheet(raw, uri, origin).ok()?;
        let css = css_tokens(raw, t.from + open + 1)?;
        nodes(&css, &mut 0, None, 0)?;
    }
    Some(())
}
fn css_scopes(
    ns: &[Node],
    from: usize,
    to: usize,
    depth: usize,
    declaration: bool,
    scopes: &mut Vec<Scope>,
) {
    let mut spans = Vec::new();
    let mut start = None;
    for n in ns {
        if n.first().kind == Kind::Comment {
            continue;
        }
        start.get_or_insert(n.from());
        if let Node::Group {
            open,
            close,
            children,
        } = n
        {
            if open.text == "{" {
                css_scopes(children, open.to, close.from, depth + 1, true, scopes);
                spans.push((start.take().unwrap(), n.to()));
            }
        } else if n.last().text == ";" {
            spans.push((start.take().unwrap(), n.to()));
        }
    }
    if let Some(start) = start {
        spans.push((start, ns.last().unwrap().to()));
    }
    scopes.push(Scope {
        from,
        to,
        depth,
        spans,
        css: true,
        declaration,
    });
}
fn find_scope_nodes(ns: &[Node], from: usize, to: usize) -> Option<Vec<Node>> {
    let selected: Vec<_> = ns
        .iter()
        .filter(|n| n.from() >= from && n.to() <= to)
        .cloned()
        .collect();
    if selected.iter().any(|n| n.from() == from) {
        return Some(selected);
    }
    for n in ns {
        match n {
            Node::Group { children, .. } if n.from() <= from && n.to() >= to => {
                if let Some(found) = find_scope_nodes(children, from, to) {
                    return Some(found);
                }
            }
            Node::Token(t) if t.kind == Kind::Style && t.from <= from && t.to >= to => {
                let open = t.text.find('{')?;
                let ts = css_tokens(&t.text[open + 1..t.text.len() - 1], t.from + open + 1)?;
                let children = nodes(&ts, &mut 0, None, 0)?;
                if let Some(found) = find_scope_nodes(&children, from, to) {
                    return Some(found);
                }
            }
            _ => {}
        }
    }
    None
}

impl LanguageService {
    /// Standard LSP edits against the in-memory document, without evaluating it
    /// or loading its imports. An uncertain transformation produces no edits.
    pub fn formatting(&self, uri: &str, options: &J, selected: Option<&J>) -> J {
        self.formatting_inner(uri, options, selected)
            .unwrap_or_else(|| json!([]))
    }
    fn formatting_inner(&self, uri: &str, options: &J, selected: Option<&J>) -> Option<J> {
        let source = self.documents.get(uri)?;
        let css = uri.ends_with(".lcss");
        let ast = if css {
            Vec::new()
        } else {
            parser::parse(source, uri).ok()?
        };
        let ts = if css {
            css_tokens(source, 0)?
        } else {
            lay_tokens(source, uri)?
        };
        validate_css_tokens(&ts, source, uri, css)?;
        let mut protected_ranges: Vec<_> = ts
            .iter()
            .filter(|t| matches!(t.kind, Kind::String | Kind::Comment))
            .map(|t| (t.from, t.to))
            .collect();
        let tree = nodes(&ts, &mut 0, None, 0)?;
        let semantic_tokens: Vec<_> = ts
            .iter()
            .filter(|t| t.kind != Kind::Comment)
            .cloned()
            .collect();
        let mut layout = Layout {
            source,
            starts: BTreeSet::new(),
            blocks: BTreeSet::new(),
            scopes: Vec::new(),
        };
        if css {
            css_scopes(&tree, 0, source.len(), 0, false, &mut layout.scopes);
        } else {
            layout.statements(&ast, &semantic_tokens, 0, source.len(), 0, uri)?;
            for t in ts.iter().filter(|t| t.kind == Kind::Style) {
                let open = t.text.find('{')?;
                let css_from = t.from + open + 1;
                let css_ts = css_tokens(&t.text[open + 1..t.text.len() - 1], css_from)?;
                protected_ranges.extend(
                    css_ts
                        .iter()
                        .filter(|t| matches!(t.kind, Kind::String | Kind::Comment))
                        .map(|t| (t.from, t.to)),
                );
                let children = nodes(&css_ts, &mut 0, None, 0)?;
                let depth = layout
                    .scopes
                    .iter()
                    .filter(|s| s.from <= t.from && s.to >= t.to)
                    .map(|s| s.depth)
                    .max()
                    .unwrap_or(0);
                css_scopes(
                    &children,
                    css_from,
                    t.to - 1,
                    depth + 1,
                    false,
                    &mut layout.scopes,
                );
            }
        }
        let tab = options["tabSize"].as_u64().unwrap_or(4).clamp(1, 32) as usize;
        let config = Options {
            unit: if options["insertSpaces"].as_bool().unwrap_or(true) {
                " ".repeat(tab)
            } else {
                "\t".into()
            },
            tab,
            width: options["lineWidth"].as_u64().unwrap_or(100).clamp(20, 1000) as usize,
            eol: if source.contains("\r\n") {
                "\r\n"
            } else {
                "\n"
            },
        };
        let (mut from, mut to, depth, selected_nodes, selection_css, declaration) =
            if let Some(r) = selected {
                let a = offset(source, &r["start"]);
                let b = offset(source, &r["end"]);
                if a > b {
                    return None;
                }
                if protected_ranges
                    .iter()
                    .any(|(from, to)| *from <= a && *to >= b)
                {
                    return None;
                }
                let mut scopes: Vec<_> = layout
                    .scopes
                    .iter()
                    .filter(|s| s.from <= a && s.to >= b)
                    .collect();
                scopes.sort_by_key(|s| s.to - s.from);
                let mut found = None;
                for scope in scopes {
                    let spans: Vec<_> = scope
                        .spans
                        .iter()
                        .filter(|(start, end)| {
                            let line_start = source[..*start].rfind('\n').map_or(0, |n| n + 1);
                            let effective_start = if source[line_start..*start].trim().is_empty() {
                                line_start
                            } else {
                                *start
                            };
                            if a == b {
                                effective_start <= a && *end >= b
                            } else {
                                effective_start < b && *end > a
                            }
                        })
                        .collect();
                    let Some(first) = spans.first() else {
                        continue;
                    };
                    let end = spans.last()?.1;
                    let start = first.0;
                    let line_start = source[..start].rfind('\n').map_or(0, |n| n + 1);
                    if !source[line_start..start].trim().is_empty() {
                        continue;
                    }
                    let ns = find_scope_nodes(&tree, start, end)?;
                    found = Some((
                        line_start,
                        end,
                        scope.depth,
                        ns,
                        scope.css,
                        scope.declaration,
                    ));
                    break;
                }
                found?
            } else {
                (0, source.len(), 0, tree.clone(), css, false)
            };
        let mut formatted = render(
            &layout.sequence(&selected_nodes, true, selection_css, declaration),
            &config,
            depth,
        );
        if selected.is_none() {
            let original_final = source.ends_with('\n');
            if original_final || options["insertFinalNewline"].as_bool().unwrap_or(false) {
                formatted.push_str(config.eol);
            }
            if !options["trimFinalNewlines"].as_bool().unwrap_or(true) {
                let endings = source
                    .bytes()
                    .rev()
                    .take_while(|b| matches!(b, b'\n' | b'\r'))
                    .filter(|b| *b == b'\n')
                    .count();
                for _ in 1..endings {
                    formatted.push_str(config.eol);
                }
            }
        }
        let mut candidate = source.clone();
        candidate.replace_range(from..to, &formatted);
        let new_ts = if css {
            css_tokens(&candidate, 0)?
        } else {
            lay_tokens(&candidate, uri)?
        };
        validate_css_tokens(&new_ts, &candidate, uri, css)?;
        if css {
            if css_signature(source)? != css_signature(&candidate)? {
                return None;
            }
        } else {
            if canonical_ast(&ast)? != canonical_ast(&parser::parse(&candidate, uri).ok()?)? {
                return None;
            }
            let spelling = |ts: &[Token]| {
                ts.iter()
                    .filter(|t| t.kind != Kind::Style)
                    .map(|t| (t.text.clone(), t.kind as u8))
                    .collect::<Vec<_>>()
            };
            if spelling(&ts) != spelling(&new_ts) {
                return None;
            }
        }
        // A single minimal replacement retains cursor anchors outside the
        // changed text and never splits UTF-8 characters or CRLF pairs.
        let old = &source[from..to];
        let prefix = old
            .chars()
            .zip(formatted.chars())
            .take_while(|(a, b)| a == b)
            .map(|(c, _)| c.len_utf8())
            .sum::<usize>();
        if prefix == old.len() && prefix == formatted.len() {
            return Some(json!([]));
        }
        let mut prefix = prefix;
        if prefix > 0 && old.as_bytes()[prefix - 1] == b'\r' {
            prefix -= 1;
        }
        let suffix = old[prefix..]
            .chars()
            .rev()
            .zip(formatted[prefix..].chars().rev())
            .take_while(|(a, b)| a == b)
            .map(|(c, _)| c.len_utf8())
            .sum::<usize>();
        let mut suffix = suffix;
        if suffix > 0
            && suffix < old.len()
            && old.as_bytes().get(old.len() - suffix) == Some(&b'\n')
            && old.as_bytes().get(old.len() - suffix - 1) == Some(&b'\r')
        {
            suffix -= 1;
        }
        from += prefix;
        to -= suffix;
        Some(
            json!([{"range":{"start":position(source, from),"end":position(source, to)},"newText":&formatted[prefix..formatted.len() - suffix]}]),
        )
    }
}
