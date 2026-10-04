//! Restricted LayMesh syntax. No host language execution or dynamic imports.
use crate::{Diagnostic, Loc, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Expr {
    pub kind: ExprKind,
    pub loc: Loc,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ExprKind {
    Number(f64, String),
    String(String, bool, bool),
    Bool(bool),
    Ref(Vec<String>),
    List(Vec<Expr>),
    Dict(Vec<(Expr, Expr)>),
    Call(Vec<String>, Vec<(Option<String>, Expr)>),
    Index(Box<Expr>, Box<Expr>),
    Member(Box<Expr>, String),
    Method(Box<Expr>, String, Vec<(Option<String>, Expr)>),
    Unary(String, Box<Expr>),
    Binary(String, Box<Expr>, Box<Expr>),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Stmt {
    pub kind: StmtKind,
    pub loc: Loc,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum StmtKind {
    Bind(String, Expr, bool),
    SetIndex(Expr, Expr),
    Expr(Expr),
    Style(String),
    Import(Vec<(String, String)>, String),
    Function(String, Vec<(String, Option<Expr>)>, Vec<Stmt>, bool),
    If(Vec<(Expr, Vec<Stmt>)>, Vec<Stmt>),
    For(Pattern, Expr, Vec<Stmt>),
    While(Expr, Vec<Stmt>),
    Return(Expr),
    Break,
    Continue,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Pattern {
    Name(String),
    Tuple(Vec<Pattern>),
}
impl Pattern {
    pub fn names(&self) -> Vec<String> {
        match self {
            Self::Name(n) => vec![n.clone()],
            Self::Tuple(ps) => ps.iter().flat_map(Self::names).collect(),
        }
    }
}
#[derive(Clone, Debug)]
pub struct Token {
    pub text: String,
    pub loc: Loc,
    pub end: usize,
    pub string: bool,
}
fn loc(s: &str, p: usize) -> Loc {
    let pre = &s[..p];
    Loc {
        line: pre.bytes().filter(|b| *b == b'\n').count() + 1,
        column: pre.rsplit('\n').next().unwrap_or("").encode_utf16().count() + 1,
        offset: p,
    }
}
pub fn string_end(s: &str, start: usize) -> Option<usize> {
    let b = s.as_bytes();
    let mut q = start;
    while q < b.len() && q - start < 2 && matches!(b[q], b'r' | b'f' | b'R' | b'F') {
        q += 1;
    }
    if q >= b.len() || !matches!(b[q], b'\'' | b'"') {
        return None;
    }
    let delim = if s[q..].starts_with(&String::from_utf8(vec![b[q]; 3]).ok()?) {
        3
    } else {
        1
    };
    let quote = &s[q..q + delim];
    let formatted = s[start..q].to_lowercase().contains('f');
    let mut i = q + delim;
    let mut depth = 0;
    while i < b.len() {
        if b[i] == b'\\' {
            i = (i + 2).min(b.len());
            continue;
        }
        if formatted && depth > 0 {
            if let Some(end) = string_end(s, i) {
                i = end;
                continue;
            }
            if b[i] == b'{' {
                depth += 1;
            }
            if b[i] == b'}' {
                depth -= 1;
            }
            i += 1;
            continue;
        }
        if formatted && b[i] == b'{' {
            if b.get(i + 1) == Some(&b'{') {
                i += 2;
                continue;
            }
            depth = 1;
            i += 1;
            continue;
        }
        if s.is_char_boundary(i) && s[i..].starts_with(quote) {
            return Some(i + delim);
        }
        if delim == 1 && b[i] == b'\n' {
            return None;
        }
        i += 1;
    }
    None
}
pub fn tokenize(s: &str, file: &str) -> Result<Vec<Token>> {
    let b = s.as_bytes();
    let mut i = 0;
    let mut ts = vec![];
    while i < b.len() {
        let start = i;
        if b[i].is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if b[i] == b'#' {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if let Some(end) = string_end(s, i) {
            i = end;
            ts.push(Token {
                text: s[start..i].into(),
                loc: loc(s, start),
                end: i,
                string: true,
            });
            continue;
        }
        if b[i].is_ascii_alphabetic() || b[i] == b'_' {
            i += 1;
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                i += 1;
            }
            if &s[start..i] == "style" {
                let mut k = i;
                while k < b.len() && b[k].is_ascii_whitespace() {
                    k += 1;
                }
                if b.get(k) == Some(&b'{') {
                    let from = k + 1;
                    let mut depth = 1;
                    k += 1;
                    while k < b.len() && depth > 0 {
                        if let Some(end) = string_end(s, k) {
                            k = end;
                            continue;
                        }
                        if b.get(k..k + 2) == Some(b"/*") {
                            k = s[k + 2..].find("*/").map(|n| k + n + 4).unwrap_or(b.len());
                            continue;
                        }
                        if b[k] == b'{' {
                            depth += 1;
                        }
                        if b[k] == b'}' {
                            depth -= 1;
                        }
                        k += 1;
                    }
                    if depth != 0 {
                        return Err(Diagnostic::new(
                            "E_SYNTAX",
                            "Unclosed style block",
                            file,
                            loc(s, start),
                        ));
                    }
                    ts.push(Token {
                        text: format!("@style{}", &s[from..k - 1]),
                        loc: loc(s, from),
                        end: k,
                        string: false,
                    });
                    i = k;
                    continue;
                }
            }
        } else if b[i].is_ascii_digit()
            || (b[i] == b'.' && b.get(i + 1).is_some_and(u8::is_ascii_digit))
        {
            i += 1;
            while i < b.len() && (b[i].is_ascii_digit() || b[i] == b'.') {
                i += 1;
            }
            if i < b.len() && matches!(b[i], b'e' | b'E') {
                i += 1;
                if i < b.len() && matches!(b[i], b'+' | b'-') {
                    i += 1;
                }
                while i < b.len() && b[i].is_ascii_digit() {
                    i += 1;
                }
            }
        } else if b"=<>!".contains(&b[i]) && b.get(i + 1) == Some(&b'=') {
            i += 2;
        } else if b"=<>+-*/.,:(){}[]".contains(&b[i]) {
            i += 1;
        } else {
            return Err(Diagnostic::new(
                "E_SYNTAX",
                format!("Unexpected character {}", s[i..].chars().next().unwrap()),
                file,
                loc(s, i),
            ));
        }
        ts.push(Token {
            text: s[start..i].into(),
            loc: loc(s, start),
            end: i,
            string: false,
        });
    }
    ts.push(Token {
        text: String::new(),
        loc: loc(s, s.len()),
        end: s.len(),
        string: false,
    });
    Ok(ts)
}
pub fn decode(s: &str, file: &str, at: Loc) -> Result<(String, bool, bool)> {
    let prefix = s.find(['\'', '"']).unwrap();
    let raw = s[..prefix].to_lowercase().contains('r');
    let formatted = s[..prefix].to_lowercase().contains('f');
    let quote = s.as_bytes()[prefix];
    let width = if s[prefix..].starts_with(&String::from_utf8(vec![quote; 3]).unwrap()) {
        3
    } else {
        1
    };
    let body = &s[prefix + width..s.len() - width];
    if raw {
        return Ok((body.into(), true, formatted));
    }
    let mut result = String::new();
    let mut chars = body.chars().peekable();
    let mut math = false;
    while let Some(c) = chars.next() {
        if c == '$' {
            if chars.peek() == Some(&'$') {
                result.push('$');
                chars.next();
            }
            math = !math;
            result.push('$');
        } else if c == '\\' {
            let n = chars.next().unwrap_or('\0');
            if math {
                result.push(c);
                result.push(n);
                continue;
            }
            match n {
                'n' => result.push('\n'),
                'r' => result.push('\r'),
                't' => result.push('\t'),
                '\\' | '\'' | '"' => result.push(n),
                '$' => result.push_str("\\$"),
                '\n' => {}
                'u' => {
                    let h: String = chars.by_ref().take(4).collect();
                    result.push(
                        u32::from_str_radix(&h, 16)
                            .ok()
                            .and_then(char::from_u32)
                            .ok_or_else(|| {
                                Diagnostic::new("E_SYNTAX", "Invalid Unicode escape", file, at)
                            })?,
                    );
                }
                _ => {
                    return Err(Diagnostic::new(
                        "E_SYNTAX",
                        format!("Invalid escape \\{n}; use a raw string"),
                        file,
                        at,
                    ));
                }
            }
        } else {
            result.push(c);
        }
    }
    Ok((result, false, formatted))
}
struct Parser<'a> {
    ts: Vec<Token>,
    i: usize,
    file: &'a str,
    depth: usize,
}
impl<'a> Parser<'a> {
    fn peek(&self) -> &str {
        &self.ts[self.i].text
    }
    fn take(&mut self) -> Token {
        let t = self.ts[self.i].clone();
        if !t.text.is_empty() {
            self.i += 1;
        }
        t
    }
    fn eat(&mut self, s: &str) -> bool {
        if self.peek() == s {
            self.take();
            true
        } else {
            false
        }
    }
    fn err(&self, m: impl Into<String>) -> Diagnostic {
        Diagnostic::new("E_SYNTAX", m, self.file, self.ts[self.i].loc)
    }
    fn need(&mut self, s: &str) -> Result<()> {
        if self.eat(s) {
            Ok(())
        } else {
            Err(self.err(format!("Expected {s}, found {}", self.peek())))
        }
    }
    fn ident(&mut self) -> Result<String> {
        if self
            .peek()
            .as_bytes()
            .first()
            .is_some_and(|c| c.is_ascii_alphabetic() || *c == b'_')
        {
            Ok(self.take().text)
        } else {
            Err(self.err("Expected identifier"))
        }
    }
    fn block(&mut self) -> Result<Vec<Stmt>> {
        self.need("{")?;
        let r = self.stmts(true)?;
        self.need("}")?;
        Ok(r)
    }
    fn pattern(&mut self, depth: usize) -> Result<Pattern> {
        if depth > 256 {
            return Err(self.err("Pattern nesting exceeds 256"));
        }
        if self.eat("(") {
            let mut ps = vec![self.pattern(depth + 1)?];
            while self.eat(",") && self.peek() != ")" {
                ps.push(self.pattern(depth + 1)?);
            }
            self.need(")")?;
            Ok(Pattern::Tuple(ps))
        } else {
            Ok(Pattern::Name(self.ident()?))
        }
    }
    fn stmts(&mut self, block: bool) -> Result<Vec<Stmt>> {
        let mut r = vec![];
        while !self.peek().is_empty() && !(block && self.peek() == "}") {
            r.push(self.stmt()?);
        }
        Ok(r)
    }
    fn stmt(&mut self) -> Result<Stmt> {
        let at = self.ts[self.i].loc;
        let exported = self.eat("export");
        let kind = match self.peek() {
            "import" => {
                self.take();
                self.need("{")?;
                let mut names = vec![];
                loop {
                    let n = self.ident()?;
                    let a = if self.eat("as") {
                        self.ident()?
                    } else {
                        n.clone()
                    };
                    names.push((n, a));
                    if !self.eat(",") {
                        break;
                    }
                }
                self.need("}")?;
                self.need("from")?;
                let t = self.take();
                if !t.string {
                    return Err(self.err("Expected module path"));
                }
                StmtKind::Import(names, decode(&t.text, self.file, t.loc)?.0)
            }
            "function" => {
                self.take();
                let n = self.ident()?;
                self.need("(")?;
                let mut ps = vec![];
                while self.peek() != ")" {
                    let p = self.ident()?;
                    let d = if self.eat("=") {
                        Some(self.expr(0)?)
                    } else {
                        None
                    };
                    ps.push((p, d));
                    if !self.eat(",") {
                        break;
                    }
                }
                self.need(")")?;
                StmtKind::Function(n, ps, self.block()?, exported)
            }
            "if" => {
                self.take();
                let c = self.expr(0)?;
                let mut bs = vec![(c, self.block()?)];
                let mut other = vec![];
                while self.eat("else") {
                    if self.eat("if") {
                        let c = self.expr(0)?;
                        bs.push((c, self.block()?));
                    } else {
                        other = self.block()?;
                        break;
                    }
                }
                StmtKind::If(bs, other)
            }
            "for" => {
                self.take();
                let mut n = self.pattern(0)?;
                if self.eat(",") {
                    let mut ps = vec![n, self.pattern(0)?];
                    while self.eat(",") {
                        ps.push(self.pattern(0)?);
                    }
                    n = Pattern::Tuple(ps);
                }
                let mut names = std::collections::BTreeSet::new();
                if n.names()
                    .iter()
                    .filter(|n| n.as_str() != "_")
                    .any(|n| !names.insert(n.clone()))
                {
                    return Err(self.err("Duplicate loop binding"));
                }
                self.need("in")?;
                let e = self.expr(0)?;
                StmtKind::For(n, e, self.block()?)
            }
            "while" => {
                self.take();
                let e = self.expr(0)?;
                StmtKind::While(e, self.block()?)
            }
            "return" => {
                self.take();
                StmtKind::Return(self.expr(0)?)
            }
            "break" => {
                self.take();
                StmtKind::Break
            }
            "continue" => {
                self.take();
                StmtKind::Continue
            }
            _ if self.peek().starts_with("@style") => StmtKind::Style(self.take().text[6..].into()),
            _ if self.ts.get(self.i + 1).is_some_and(|t| t.text == "=") => {
                let n = self.ident()?;
                self.take();
                StmtKind::Bind(n, self.expr(0)?, exported)
            }
            _ => {
                let e = self.expr(0)?;
                if self.eat("=") {
                    if exported || !matches!(e.kind, ExprKind::Index(..)) {
                        return Err(self.err("Assignment needs a dictionary index"));
                    }
                    StmtKind::SetIndex(e, self.expr(0)?)
                } else {
                    StmtKind::Expr(e)
                }
            }
        };
        Ok(Stmt { kind, loc: at })
    }
    fn expr(&mut self, min: u8) -> Result<Expr> {
        self.depth += 1;
        if self.depth > 256 {
            return Err(self.err("Expression nesting exceeds 256"));
        }
        let t = self.take();
        let at = t.loc;
        let kind = if t.string {
            let (s, r, f) = decode(&t.text, self.file, at)?;
            ExprKind::String(s, r, f)
        } else {
            match t.text.as_str() {
                "-" | "not" => ExprKind::Unary(t.text, Box::new(self.expr(6)?)),
                "true" => ExprKind::Bool(true),
                "false" => ExprKind::Bool(false),
                "{" => {
                    let mut es = vec![];
                    while self.peek() != "}" {
                        let key = self.expr(0)?;
                        self.need(":")?;
                        es.push((key, self.expr(0)?));
                        if !self.eat(",") {
                            break;
                        }
                    }
                    self.need("}")?;
                    ExprKind::Dict(es)
                }
                "[" | "(" => {
                    let end = if t.text == "[" { "]" } else { ")" };
                    let mut es = vec![];
                    let mut tuple = t.text == "[";
                    while self.peek() != end {
                        es.push(self.expr(0)?);
                        if !self.eat(",") {
                            break;
                        }
                        tuple = true;
                    }
                    self.need(end)?;
                    if !tuple && es.len() == 1 {
                        es.remove(0).kind
                    } else {
                        ExprKind::List(es)
                    }
                }
                _ if t.text.parse::<f64>().is_ok() => {
                    let mut unit = String::new();
                    if self.ts[self.i].loc.line == at.line
                        && self.peek().chars().all(|c| c.is_ascii_alphabetic())
                        && !self.peek().is_empty()
                        && !matches!(
                            self.peek(),
                            "and" | "or" | "not" | "else" | "return" | "break" | "continue"
                        )
                        && (self.peek() != "in"
                            || self.ts[self.i].loc.offset == t.end
                            || self.ts.get(self.i + 1).is_none_or(|next| {
                                next.text.is_empty()
                                    || next.loc.line > at.line
                                    || [
                                        ",", ")", "]", "}", "+", "-", "*", "/", "==", "!=", "<",
                                        ">", "<=", ">=",
                                    ]
                                    .contains(&next.text.as_str())
                            }))
                        && !self.ts.get(self.i + 1).is_some_and(|t| t.text == "=")
                    {
                        unit = self.take().text;
                    }
                    ExprKind::Number(t.text.parse().unwrap(), unit)
                }
                _ if t
                    .text
                    .as_bytes()
                    .first()
                    .is_some_and(|b| b.is_ascii_alphabetic() || *b == b'_') =>
                {
                    let mut parts = vec![t.text];
                    while self.eat(".") {
                        parts.push(self.ident()?);
                    }
                    if self.eat("(") {
                        let mut args = vec![];
                        while self.peek() != ")" {
                            let name = if self.ts.get(self.i + 1).is_some_and(|t| t.text == "=") {
                                let n = self.ident()?;
                                self.take();
                                Some(n)
                            } else {
                                None
                            };
                            args.push((name, self.expr(0)?));
                            if !self.eat(",") {
                                break;
                            }
                        }
                        self.need(")")?;
                        ExprKind::Call(parts, args)
                    } else {
                        ExprKind::Ref(parts)
                    }
                }
                _ => {
                    return Err(Diagnostic::new(
                        "E_SYNTAX",
                        format!("Expected expression, found {}", t.text),
                        self.file,
                        at,
                    ));
                }
            }
        };
        let mut left = Expr { kind, loc: at };
        loop {
            if self.eat("[") {
                let index = self.expr(0)?;
                self.need("]")?;
                left = Expr {
                    kind: ExprKind::Index(Box::new(left), Box::new(index)),
                    loc: at,
                };
            } else if self.eat(".") {
                let name = self.ident()?;
                if self.eat("(") {
                    let mut args = vec![];
                    while self.peek() != ")" {
                        let name = if self.ts.get(self.i + 1).is_some_and(|t| t.text == "=") {
                            let n = self.ident()?;
                            self.take();
                            Some(n)
                        } else {
                            None
                        };
                        args.push((name, self.expr(0)?));
                        if !self.eat(",") {
                            break;
                        }
                    }
                    self.need(")")?;
                    left = Expr {
                        kind: ExprKind::Method(Box::new(left), name, args),
                        loc: at,
                    };
                } else {
                    left = Expr {
                        kind: ExprKind::Member(Box::new(left), name),
                        loc: at,
                    };
                }
            } else {
                break;
            }
        }
        loop {
            let p = match self.peek() {
                "or" => 1,
                "and" => 2,
                "==" | "!=" | "<" | ">" | "<=" | ">=" | "in" => 3,
                "not" if self.ts.get(self.i + 1).is_some_and(|t| t.text == "in") => 3,
                "+" | "-" => 4,
                "*" | "/" => 5,
                _ => 0,
            };
            if p == 0 || p < min {
                break;
            }
            let mut op = self.take();
            if op.text == "not" {
                self.need("in")?;
                op.text = "not in".into();
            }
            let right = self.expr(p + 1)?;
            left = Expr {
                kind: ExprKind::Binary(op.text, Box::new(left), Box::new(right)),
                loc: op.loc,
            };
        }
        self.depth -= 1;
        Ok(left)
    }
}
pub fn parse(s: &str, file: &str) -> Result<Vec<Stmt>> {
    Parser {
        ts: tokenize(s, file)?,
        i: 0,
        file,
        depth: 0,
    }
    .stmts(false)
}
pub fn expression(s: &str, file: &str) -> Result<Expr> {
    let mut p = Parser {
        ts: tokenize(s, file)?,
        i: 0,
        file,
        depth: 0,
    };
    let e = p.expr(0)?;
    if !p.peek().is_empty() {
        return Err(p.err("Unexpected trailing expression"));
    }
    Ok(e)
}
