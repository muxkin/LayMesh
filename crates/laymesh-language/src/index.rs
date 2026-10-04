use regex::Regex;
use serde_json::{Value as J, json};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug)]
pub struct Token {
    pub text: String,
    pub from: usize,
    pub to: usize,
    pub kind: &'static str,
}
pub fn tokens(s: &str) -> Vec<Token> {
    let mut out = vec![];
    let mut i = 0;
    while i < s.len() {
        let from = i;
        let c = s[i..].chars().next().unwrap();
        if c.is_whitespace() {
            i += c.len_utf8();
            continue;
        }
        let kind;
        if c == '#' {
            i = s[i..].find('\n').map(|n| i + n).unwrap_or(s.len());
            kind = "comment"
        } else if s[i..].starts_with("/*") {
            i = s[i + 2..].find("*/").map(|n| i + n + 4).unwrap_or(s.len());
            kind = "comment"
        } else if let Some(end) = laymesh_core::parser::string_end(s, i) {
            i = end;
            kind = "string"
        } else if c == '\'' || c == '"' {
            i = s.len();
            kind = "string"
        } else if c.is_ascii_alphabetic() || c == '_' {
            i += 1;
            while i < s.len()
                && (s.as_bytes()[i].is_ascii_alphanumeric() || s.as_bytes()[i] == b'_')
            {
                i += 1
            }
            kind = "name"
        } else if c.is_ascii_digit() {
            i += 1;
            while i < s.len() && (s.as_bytes()[i].is_ascii_digit() || s.as_bytes()[i] == b'.') {
                i += 1
            }
            kind = "number"
        } else {
            i += c.len_utf8();
            kind = "punctuation"
        }
        out.push(Token {
            text: s[from..i].into(),
            from,
            to: i,
            kind,
        });
    }
    out
}

#[cfg(test)]
mod scope_tests {
    use super::*;
    #[test]
    fn nested_control_blocks_leave_function_locals_inside_the_body() {
        let source = "value=1\nif true { value=2 secret=3 }\nfunction f(value) {\n local=value\n for value in [1,2] { loop=value }\n while false { hidden=1 }\n return value\n}\nlast=value";
        let idx = index("test.lay", source, true);
        let local = idx.symbols.iter().find(|s| s.name == "local").unwrap();
        assert!(local.scope_to < source.len(), "{:#?}", idx.symbols);
    }
}
#[derive(Clone, Default, Debug)]
pub struct Doc {
    pub body: String,
    pub parameters: BTreeMap<String, J>,
    pub returns: Option<J>,
    pub typ: String,
    pub examples: Vec<String>,
    pub see: Vec<String>,
    pub variants: BTreeMap<String, Doc>,
}
impl Doc {
    pub fn select(&self, en: bool) -> Self {
        let first = self.variants.get(if en { "en" } else { "zh-CN" });
        let mut order: Vec<&Doc> = first
            .into_iter()
            .chain(std::iter::once(self))
            .chain(self.variants.get("en"))
            .chain(self.variants.values())
            .collect();
        order.dedup_by(|a, b| std::ptr::eq(*a, *b));
        let text = |f: fn(&Doc) -> &String| {
            order
                .iter()
                .map(|d| f(d))
                .find(|s| !s.trim().is_empty())
                .cloned()
                .unwrap_or_default()
        };
        let mut out = Doc {
            body: text(|d| &d.body),
            typ: text(|d| &d.typ),
            ..Doc::default()
        };
        for d in order.iter().rev() {
            for (k, v) in &d.parameters {
                let cur = out.parameters.entry(k.clone()).or_insert(json!({}));
                if v["description"]
                    .as_str()
                    .is_some_and(|s| !s.trim().is_empty())
                {
                    cur["description"] = v["description"].clone()
                }
                if !v["type"].is_null() {
                    cur["type"] = v["type"].clone()
                }
            }
            if let Some(ret) = &d.returns {
                if out.returns.is_none() {
                    out.returns = Some(ret.clone())
                } else {
                    let cur = out.returns.as_mut().unwrap();
                    if ret["description"].as_str().is_some_and(|s| !s.is_empty()) {
                        cur["description"] = ret["description"].clone()
                    }
                    if !ret["type"].is_null() {
                        cur["type"] = ret["type"].clone()
                    }
                }
            }
        }
        for (k, v) in &self.parameters {
            if !v["type"].is_null() {
                out.parameters.entry(k.clone()).or_insert(json!({}))["type"] = v["type"].clone()
            }
        }
        if !self.typ.is_empty() {
            out.typ = self.typ.clone()
        }
        if let (Some(root), Some(ret)) = (&self.returns, &mut out.returns) {
            if !root["type"].is_null() {
                ret["type"] = root["type"].clone()
            }
        }
        out.examples = order
            .iter()
            .find(|d| !d.examples.is_empty())
            .map(|d| d.examples.clone())
            .unwrap_or_default();
        out.see = order
            .iter()
            .find(|d| !d.see.is_empty())
            .map(|d| d.see.clone())
            .unwrap_or_default();
        out
    }
    pub fn markdown(&self, en: bool) -> String {
        let d = self.select(en);
        let mut chunks = vec![d.body];
        if let Some(r) = d.returns {
            chunks.push(format!(
                "**{}** {}\n\n{}",
                if en { "Returns" } else { "返回" },
                r["type"].as_str().unwrap_or(""),
                r["description"].as_str().unwrap_or("")
            ))
        }
        for ex in d.examples {
            chunks.push(format!(
                "**{}**\n\n```lay\n{ex}\n```",
                if en { "Example" } else { "示例" }
            ))
        }
        if !d.see.is_empty() {
            chunks.push(format!(
                "**{}**\n\n{}",
                if en { "See also" } else { "另见" },
                d.see
                    .iter()
                    .map(|s| format!("- {s}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            ))
        }
        chunks
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}
#[derive(Clone, Debug)]
pub struct Symbol {
    pub name: String,
    pub kind: String,
    pub uri: String,
    pub from: usize,
    pub to: usize,
    pub scope_from: usize,
    pub scope_to: usize,
    pub exported: bool,
    pub parameters: Vec<J>,
    pub doc: Doc,
    pub typ: String,
    pub alias: String,
    pub value: Option<laymesh_core::parser::Expr>,
}
#[derive(Clone, Debug)]
pub struct Import {
    pub name: String,
    pub alias: String,
    pub source: String,
    pub scope_from: usize,
    pub scope_to: usize,
    pub from: usize,
    pub to: usize,
    pub alias_from: usize,
    pub alias_to: usize,
    pub explicit_alias: bool,
}
#[derive(Clone, Default)]
pub struct Index {
    pub symbols: Vec<Symbol>,
    pub imports: Vec<Import>,
    pub issues: Vec<J>,
}
pub fn locale(s: &str) -> String {
    let s = s.replace('_', "-").to_lowercase();
    if s.starts_with("zh") {
        "zh-CN".into()
    } else if s.starts_with("en") {
        "en".into()
    } else {
        s
    }
}
fn doc_before(s: &str, start: usize) -> Vec<Token> {
    let mut lines = vec![];
    let mut end = start;
    for t in tokens(&s[..start]).into_iter().rev() {
        let between = &s[t.to..end];
        if t.kind != "comment"
            || !t.text.starts_with("##")
            || between.chars().filter(|&c| c == '\n').count() != 1
            || !between.trim().is_empty()
        {
            break;
        }
        end = t.from;
        lines.push(t)
    }
    lines.reverse();
    lines
}
fn documentation(lines: Vec<Token>, parameters: &[J], en: bool, issues: &mut Vec<J>) -> Doc {
    let mut root = Doc::default();
    let mut doc = Doc::default();
    let mut tag = String::new();
    let mut seen = BTreeSet::new();
    let mut types = BTreeMap::new();
    let mut append = ("body".to_string(), String::new());
    let re = Regex::new(r"^(?:\{([^}]+)\}\s*)?([A-Za-z_]\w*)\s*(?:-\s*)?(.*)$").unwrap();
    let value_re = Regex::new(r"^(?:\{([^}]+)\}\s*)?(.*)$").unwrap();
    let mut warn = |t: &Token, zh: String, english: String| {
        issues.push(json!({"from":t.from,"to":t.to,"severity":"warning","code":"W_DOC","message":if en{english}else{zh}}))
    };
    for t in lines {
        let text = t.text[2..].strip_prefix(' ').unwrap_or(&t.text[2..]);
        if let Some(at) = text.strip_prefix('@') {
            let (name, rest) = at.split_once(char::is_whitespace).unwrap_or((at, ""));
            let rest = rest.trim();
            if name == "lang" {
                if tag.is_empty() {
                    root = doc
                } else if tag != "!" {
                    root.variants.insert(tag.clone(), doc);
                }
                let next = locale(rest);
                if !Regex::new(r"^[a-zA-Z]{2,8}(?:-[a-zA-Z0-9]+)*$")
                    .unwrap()
                    .is_match(rest)
                {
                    warn(
                        &t,
                        "@lang 需要有效语言标签".into(),
                        "@lang requires a valid language tag".into(),
                    );
                    tag = "!".into()
                } else if root.variants.contains_key(&next) {
                    warn(
                        &t,
                        format!("重复的 @lang {rest}"),
                        format!("Duplicate @lang {rest}"),
                    );
                    tag = "!".into()
                } else {
                    tag = next
                }
                doc = Doc::default();
                seen.clear();
                append = ("body".into(), String::new());
                continue;
            }
            match name {
                "param" => {
                    if let Some(c) = re.captures(rest) {
                        let key = c[2].to_string();
                        let typ = c.get(1).map(|m| m.as_str());
                        let id = format!("param:{key}");
                        if !seen.insert(id.clone()) {
                            warn(
                                &t,
                                format!("重复的 @param {key}"),
                                format!("Duplicate @param {key}"),
                            )
                        }
                        if !parameters.iter().any(|p| p["name"] == key) {
                            warn(
                                &t,
                                format!("文档参数 {key} 不在函数声明中"),
                                format!("Documented parameter {key} is not in the declaration"),
                            )
                        }
                        if let Some(typ) = typ {
                            if let Some(old) = types.get(&id) {
                                if old != typ {
                                    warn(
                                        &t,
                                        format!("{key} 的文档类型冲突"),
                                        format!("Conflicting documented types for {key}"),
                                    )
                                }
                            } else {
                                types.insert(id, typ.to_string());
                            }
                        }
                        doc.parameters
                            .insert(key.clone(), json!({"type":typ,"description":&c[3]}));
                        append = ("param".into(), key);
                    } else {
                        warn(
                            &t,
                            "@param 需要参数名".into(),
                            "@param requires a parameter name".into(),
                        );
                        append = ("none".into(), String::new())
                    }
                }
                "returns" | "type" => {
                    if !seen.insert(name.into()) {
                        warn(&t, format!("重复的 @{name}"), format!("Duplicate @{name}"))
                    }
                    if name == "returns" {
                        let c = value_re.captures(rest).unwrap();
                        doc.returns = Some(
                            json!({"type":c.get(1).map(|m|m.as_str()),"description":c[2].trim_start_matches('-').trim()}),
                        );
                        if let Some(v) = c.get(1) {
                            types.entry("returns".into()).or_insert(v.as_str().into());
                        }
                        append = ("returns".into(), String::new())
                    } else {
                        doc.typ = rest.trim_matches(['{', '}']).into();
                        types.entry("type".into()).or_insert(doc.typ.clone());
                        append = ("none".into(), String::new());
                    }
                }
                "example" => {
                    doc.examples.push(rest.into());
                    append = ("example".into(), String::new())
                }
                "see" => {
                    doc.see.push(rest.into());
                    append = ("see".into(), String::new())
                }
                _ => {
                    warn(
                        &t,
                        format!("不支持的文档标签 @{name}"),
                        format!("Unsupported documentation tag @{name}"),
                    );
                    append = ("none".into(), String::new())
                }
            }
        } else {
            let append_text = |s: &mut String| {
                if !s.is_empty() {
                    s.push('\n')
                }
                s.push_str(text)
            };
            match append.0.as_str() {
                "body" => append_text(&mut doc.body),
                "param" => {
                    let v = doc.parameters.get_mut(&append.1).unwrap();
                    let mut d = v["description"].as_str().unwrap_or("").to_string();
                    append_text(&mut d);
                    v["description"] = json!(d)
                }
                "returns" => {
                    let v = doc.returns.as_mut().unwrap();
                    let mut d = v["description"].as_str().unwrap_or("").to_string();
                    append_text(&mut d);
                    v["description"] = json!(d)
                }
                "example" => append_text(doc.examples.last_mut().unwrap()),
                "see" => append_text(doc.see.last_mut().unwrap()),
                _ => {}
            }
        }
    }
    if tag.is_empty() {
        root = doc
    } else if tag != "!" {
        root.variants.insert(tag, doc);
    }
    for (k, v) in types {
        if k == "type" {
            root.typ = v
        } else if k == "returns" {
            root.returns.get_or_insert(json!({}))["type"] = json!(v)
        } else {
            root.parameters.entry(k[6..].into()).or_insert(json!({}))["type"] = json!(v)
        }
    }
    root
}
pub fn index(uri: &str, source: &str, en: bool) -> Index {
    let ts: Vec<Token> = tokens(source)
        .into_iter()
        .filter(|t| t.kind != "comment")
        .collect();
    let mut pairs = BTreeMap::new();
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
                pairs.insert(stack[at], i);
                stack.truncate(at)
            }
        }
    }
    let mut out = Index::default();
    // Complete ASTs identify assignments even when several statements share a line.
    // The token index remains available while the user is typing incomplete code.
    let mut values = BTreeMap::new();
    fn bindings(
        stmts: &[laymesh_core::parser::Stmt],
        ts: &[Token],
        values: &mut BTreeMap<usize, laymesh_core::parser::Expr>,
    ) {
        use laymesh_core::parser::StmtKind;
        for stmt in stmts {
            match &stmt.kind {
                StmtKind::Bind(name, value, _) => {
                    if let Some(t) = ts
                        .iter()
                        .find(|t| t.from >= stmt.loc.offset && t.text == *name)
                    {
                        values.insert(t.from, value.clone());
                    }
                }
                StmtKind::Function(_, _, body, _)
                | StmtKind::For(_, _, body)
                | StmtKind::While(_, body) => bindings(body, ts, values),
                StmtKind::If(branches, other) => {
                    for (_, body) in branches {
                        bindings(body, ts, values);
                    }
                    bindings(other, ts, values);
                }
                _ => {}
            }
        }
    }
    if let Ok(stmts) = laymesh_core::parser::parse(source, uri) {
        bindings(&stmts, &ts, &mut values);
    }
    let mut scopes = vec![(0, source.len())];
    let mut i = 0;
    while i < ts.len() {
        let t = &ts[i];
        while scopes.len() > 1 && t.from > scopes.last().unwrap().1 {
            scopes.pop();
        }
        let (scope_from, scope_to) = *scopes.last().unwrap();
        let exported = i > 0 && ts[i - 1].text == "export";
        if t.text == "function" && ts.get(i + 2).is_some_and(|t| t.text == "(") {
            let name = &ts[i + 1];
            let end = pairs.get(&(i + 2)).copied().unwrap_or(ts.len());
            let mut params = vec![];
            let mut j = i + 3;
            while j < end {
                if ts[j].kind != "name" {
                    j += 1;
                    continue;
                }
                let p = &ts[j];
                j += 1;
                let mut default = None;
                if j < end && ts[j].text == "=" {
                    j += 1;
                    let start = ts.get(j).map(|t| t.from).unwrap_or(source.len());
                    let mut last = start;
                    while j < end && ts[j].text != "," {
                        if let Some(&close) = pairs.get(&j) {
                            last = ts[close].to;
                            j = close + 1
                        } else {
                            last = ts[j].to;
                            j += 1
                        }
                    }
                    default = Some(source[start..last].trim().to_string());
                }
                params.push(json!({"name":p.text,"from":p.from,"to":p.to,"default":default}));
                if j < end && ts[j].text == "," {
                    j += 1
                }
            }
            let decl_start = if exported { ts[i - 1].from } else { t.from };
            let doc = documentation(doc_before(source, decl_start), &params, en, &mut out.issues);
            out.symbols.push(Symbol {
                name: name.text.clone(),
                kind: "function".into(),
                uri: uri.into(),
                from: name.from,
                to: name.to,
                scope_from,
                scope_to,
                exported,
                parameters: params.clone(),
                doc: doc.clone(),
                typ: String::new(),
                alias: String::new(),
                value: None,
            });
            let body = end + 1;
            if ts.get(body).is_some_and(|t| t.text == "{") {
                let body_end = pairs.get(&body).map(|&k| ts[k].to).unwrap_or(source.len());
                scopes.push((ts[body].from, body_end));
                for p in params {
                    let mut pd = Doc::default();
                    let selected = doc.parameters.get(p["name"].as_str().unwrap());
                    if let Some(v) = selected {
                        pd.body = v["description"].as_str().unwrap_or("").into();
                        pd.typ = v["type"].as_str().unwrap_or("").into()
                    }
                    for (tag, d) in &doc.variants {
                        let mut pdv = Doc::default();
                        if let Some(v) = d.parameters.get(p["name"].as_str().unwrap()) {
                            pdv.body = v["description"].as_str().unwrap_or("").into();
                            pdv.typ = v["type"].as_str().unwrap_or("").into()
                        }
                        pd.variants.insert(tag.clone(), pdv);
                    }
                    out.symbols.push(Symbol {
                        name: p["name"].as_str().unwrap().into(),
                        kind: "parameter".into(),
                        uri: uri.into(),
                        from: p["from"].as_u64().unwrap() as usize,
                        to: p["to"].as_u64().unwrap() as usize,
                        scope_from: ts[body].from,
                        scope_to: body_end,
                        exported: false,
                        parameters: vec![],
                        doc: pd,
                        typ: String::new(),
                        alias: String::new(),
                        value: p["default"]
                            .as_str()
                            .and_then(|s| laymesh_core::parser::expression(s, uri).ok()),
                    });
                }
                i = body + 1;
                continue;
            }
            i = end.min(ts.len());
        } else if t.text == "for" {
            let header_end = (i + 1..ts.len()).find(|&j| ts[j].text == "in");
            if let Some(header_end) = header_end {
                let body = (header_end + 1..ts.len()).find(|&j| {
                    ts[j].text == "{"
                        && laymesh_core::parser::expression(
                            &source[ts[header_end].to..ts[j].from],
                            uri,
                        )
                        .is_ok()
                });
                if let Some(body) = body {
                    let body_end = pairs.get(&body).map(|&k| ts[k].to).unwrap_or(source.len());
                    for name in ts[i + 1..header_end]
                        .iter()
                        .filter(|t| t.kind == "name" && t.text != "_")
                    {
                        out.symbols.push(Symbol {
                            name: name.text.clone(),
                            kind: "variable".into(),
                            uri: uri.into(),
                            from: name.from,
                            to: name.to,
                            scope_from: ts[body].from,
                            scope_to: body_end,
                            exported: false,
                            parameters: vec![],
                            doc: Doc::default(),
                            typ: String::new(),
                            alias: String::new(),
                            value: None,
                        });
                    }
                    scopes.push((ts[body].from, body_end));
                    i = body + 1;
                    continue;
                }
            }
        } else if matches!(t.text.as_str(), "if" | "while" | "else") {
            let body = if t.text == "else" && ts.get(i + 1).is_some_and(|t| t.text == "{") {
                Some(i + 1)
            } else if t.text == "else" {
                None
            } else {
                (i + 1..ts.len()).find(|&j| {
                    ts[j].text == "{"
                        && laymesh_core::parser::expression(&source[t.to..ts[j].from], uri).is_ok()
                })
            };
            if let Some(body) = body {
                let end = pairs.get(&body).map(|&k| ts[k].to).unwrap_or(source.len());
                scopes.push((ts[body].from, end));
                i = body + 1;
                continue;
            }
        } else if t.text == "import" && ts.get(i + 1).is_some_and(|t| t.text == "{") {
            let end = pairs.get(&(i + 1)).copied().unwrap_or(i + 1);
            if let Some(path) = ts.get(end + 2).filter(|t| t.kind == "string") {
                let source = match laymesh_core::parser::expression(&path.text, uri)
                    .ok()
                    .map(|e| e.kind)
                {
                    Some(laymesh_core::parser::ExprKind::String(path, _, _)) => path,
                    _ => path.text.trim_matches(['\'', '"']).to_string(),
                };
                let mut j = i + 2;
                while j < end {
                    let from = ts[j].from;
                    let to = ts[j].to;
                    let name = ts[j].text.clone();
                    let mut alias = name.clone();
                    let explicit_alias = ts.get(j + 1).is_some_and(|t| t.text == "as");
                    if explicit_alias {
                        alias = ts[j + 2].text.clone();
                        j += 2
                    }
                    out.imports.push(Import {
                        name,
                        alias,
                        source: source.clone(),
                        scope_from,
                        scope_to,
                        from,
                        to,
                        alias_from: ts[j].from,
                        alias_to: ts[j].to,
                        explicit_alias,
                    });
                    j += 1;
                    if j < end && ts[j].text == "," {
                        j += 1
                    }
                }
                i = end + 3;
                continue;
            }
        } else if t.kind == "name"
            && ts.get(i + 1).is_some_and(|t| t.text == "=")
            && (i == 0
                || values.contains_key(&t.from)
                || ts[i - 1].text == "export"
                || ts[i - 1].text == "{"
                || ts[i - 1].text == "}"
                || source[ts[i - 1].to..t.from].contains('\n'))
        {
            let value = ts.get(i + 2);
            let call = ts.get(i + 3).is_some_and(|t| t.text == "(");
            let add = ts.get(i + 3).is_some_and(|t| t.text == ".")
                && ts.get(i + 4).is_some_and(|t| t.text == "add");
            let typ = if add {
                "instance".into()
            } else if value.is_some_and(|v| v.text == "{") {
                "dict".into()
            } else if call {
                value.map(|v| v.text.clone()).unwrap_or_default()
            } else {
                String::new()
            };
            let alias = if !call
                && !add
                && !ts
                    .get(i + 3)
                    .is_some_and(|t| matches!(t.text.as_str(), "." | "["))
                && value.is_some_and(|v| v.kind == "name")
            {
                value.unwrap().text.clone()
            } else {
                String::new()
            };
            let doc = documentation(
                doc_before(source, if exported { ts[i - 1].from } else { t.from }),
                &[],
                en,
                &mut out.issues,
            );
            out.symbols.push(Symbol {
                name: t.text.clone(),
                kind: "variable".into(),
                uri: uri.into(),
                from: t.from,
                to: t.to,
                scope_from,
                scope_to,
                exported,
                parameters: vec![],
                doc,
                typ,
                alias,
                value: values.get(&t.from).cloned().or_else(|| {
                    let start = i + 2;
                    let mut end = start;
                    while end < ts.len() && ts[end].text != "}" {
                        if end > start && source[ts[end - 1].to..ts[end].from].contains('\n') {
                            break;
                        }
                        if let Some(&close) = pairs.get(&end) {
                            end = close + 1;
                        } else {
                            end += 1;
                        }
                    }
                    (end > start)
                        .then(|| {
                            laymesh_core::parser::expression(
                                &source[ts[start].from..ts[end - 1].to],
                                uri,
                            )
                            .ok()
                        })
                        .flatten()
                }),
            });
        }
        i += 1;
    }
    out
}
