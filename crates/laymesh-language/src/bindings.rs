//! Binding identity is independent of value aliases and authored documentation.
//! Only syntax is inspected; neither indexing nor refactoring evaluates user code.
use crate::{
    LanguageService,
    index::{Import, Index, Symbol, Token, tokens},
};
use laymesh_core::parser::{self, Expr, ExprKind, Stmt, StmtKind};
use serde_json::{Value as J, json};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Key {
    pub uri: String,
    pub from: usize,
}
#[derive(Clone, Debug)]
pub(crate) struct Occurrence {
    pub key: Key,
    pub uri: String,
    pub from: usize,
    pub to: usize,
    pub declaration: bool,
    pub write: bool,
    pub import_source: bool,
}
impl Occurrence {
    pub fn json(&self) -> J {
        json!({"uri":self.uri,"from":self.from,"to":self.to,"kind":if self.write {3} else {2}})
    }
}
#[derive(Clone, Default)]
pub(crate) struct Graph {
    pub indexes: BTreeMap<String, Index>,
    pub occurrences: Vec<Occurrence>,
    valid: BTreeSet<String>,
    aliases: BTreeMap<Key, Key>,
}
impl Graph {
    pub fn build(service: &LanguageService) -> Self {
        let mut graph = Self::default();
        for uri in service.documents.keys().filter(|uri| uri.ends_with(".lay")) {
            graph
                .indexes
                .insert(uri.clone(), service.document_index(uri));
        }
        for (uri, index) in graph.indexes.clone() {
            for symbol in &index.symbols {
                if let Some(key) = graph.identity(symbol, &mut BTreeSet::new()) {
                    let declaration = key.uri == uri && key.from == symbol.from;
                    graph.push(key, &uri, symbol.from, symbol.to, declaration, true, false);
                }
            }
            for import in &index.imports {
                if let Some(key) = graph.export_key(
                    &crate::resolve(&uri, &import.source),
                    &import.name,
                    &mut BTreeSet::new(),
                ) {
                    if import.explicit_alias {
                        graph.aliases.insert(
                            Key {
                                uri: uri.clone(),
                                from: import.alias_from,
                            },
                            key.clone(),
                        );
                    }
                    graph.push(
                        key,
                        &uri,
                        import.from,
                        import.to,
                        false,
                        false,
                        import.explicit_alias,
                    );
                }
                if import.explicit_alias {
                    graph.push(
                        Key {
                            uri: uri.clone(),
                            from: import.alias_from,
                        },
                        &uri,
                        import.alias_from,
                        import.alias_to,
                        true,
                        true,
                        false,
                    );
                }
            }
            let source = service.source(&uri);
            let ts = tokens(source);
            if let Ok(stmts) = parser::parse(source, &uri) {
                graph.valid.insert(uri.clone());
                graph.statements(service, &uri, source, &ts, &stmts);
            } else {
                // Useful references in unfinished documents; edits require a complete AST.
                for (i, t) in ts.iter().enumerate().filter(|(_, t)| t.kind == "name") {
                    if graph
                        .occurrences
                        .iter()
                        .any(|o| o.uri == uri && o.from == t.from)
                        || i > 0 && ts[i - 1].text == "."
                        || ts.get(i + 1).is_some_and(|t| t.text == "=")
                        || crate::css_region(source, t.from).is_some()
                    {
                        continue;
                    }
                    if let Some(key) = graph.lookup(&uri, &t.text, t.from, &mut BTreeSet::new()) {
                        graph.push(key, &uri, t.from, t.to, false, false, false);
                    }
                }
            }
        }
        graph
            .occurrences
            .sort_by_key(|o| (o.uri.clone(), o.from, o.to));
        graph
            .occurrences
            .dedup_by(|a, b| a.uri == b.uri && a.from == b.from && a.to == b.to && a.key == b.key);
        graph
    }
    fn push(
        &mut self,
        key: Key,
        uri: &str,
        from: usize,
        to: usize,
        declaration: bool,
        write: bool,
        import_source: bool,
    ) {
        self.occurrences.push(Occurrence {
            key,
            uri: uri.into(),
            from,
            to,
            declaration,
            write,
            import_source,
        });
    }
    pub fn at(&self, uri: &str, at: usize) -> Option<&Occurrence> {
        self.occurrences
            .iter()
            .find(|o| o.uri == uri && o.from <= at && at < o.to)
            .or_else(|| self.occurrences.iter().find(|o| o.uri == uri && o.to == at))
    }
    pub fn references(&self, key: &Key, include_declaration: bool) -> Vec<&Occurrence> {
        self.occurrences
            .iter()
            .filter(|o| &o.key == key && (include_declaration || !o.declaration))
            .collect()
    }
    pub fn related_references(&self, key: &Key, include_declaration: bool) -> Vec<&Occurrence> {
        let mut related = BTreeSet::from([key.clone()]);
        loop {
            let added: Vec<_> = self
                .aliases
                .iter()
                .filter(|(alias, target)| related.contains(*target) && !related.contains(*alias))
                .map(|(alias, _)| alias.clone())
                .collect();
            if added.is_empty() {
                break;
            }
            related.extend(added);
        }
        self.occurrences
            .iter()
            .filter(|o| related.contains(&o.key) && (include_declaration || !o.declaration))
            .collect()
    }
    fn imported(&self, uri: &str, name: &str, at: usize) -> Option<&Import> {
        self.indexes
            .get(uri)?
            .imports
            .iter()
            .filter(|i| {
                i.alias == name && i.scope_from <= at && at <= i.scope_to && i.alias_from <= at
            })
            .min_by_key(|i| i.scope_to - i.scope_from)
    }
    pub fn symbol(&self, uri: &str, name: &str, at: usize) -> Option<&Symbol> {
        self.indexes
            .get(uri)?
            .symbols
            .iter()
            .filter(|s| {
                s.name == name
                    && s.scope_from <= at
                    && at <= s.scope_to
                    && (s.from <= at || s.kind == "function")
            })
            .min_by_key(|s| (s.scope_to - s.scope_from, usize::MAX - s.from))
    }
    fn identity(&self, s: &Symbol, seen: &mut BTreeSet<Key>) -> Option<Key> {
        let key = Key {
            uri: s.uri.clone(),
            from: s.from,
        };
        if !seen.insert(key.clone()) {
            return None;
        }
        // Parameters and loop patterns deliberately introduce a new local binding.
        if s.kind == "variable" && s.from >= s.scope_from && s.from > 0 {
            if let Some(previous) = self.symbol(&s.uri, &s.name, s.from - 1) {
                return self.identity(previous, seen);
            }
            if let Some(import) = self.imported(&s.uri, &s.name, s.from - 1) {
                return self.import_key(&s.uri, import, &mut BTreeSet::new());
            }
        }
        Some(key)
    }
    fn import_key(&self, uri: &str, import: &Import, seen: &mut BTreeSet<String>) -> Option<Key> {
        if import.explicit_alias {
            Some(Key {
                uri: uri.into(),
                from: import.alias_from,
            })
        } else {
            self.export_key(&crate::resolve(uri, &import.source), &import.name, seen)
        }
    }
    fn export_key(&self, uri: &str, name: &str, seen: &mut BTreeSet<String>) -> Option<Key> {
        if !seen.insert(format!("{uri}:{name}")) {
            return None;
        }
        let s = self
            .indexes
            .get(uri)?
            .symbols
            .iter()
            .find(|s| s.name == name && s.exported && s.scope_from == 0)?;
        self.identity(s, &mut BTreeSet::new())
    }
    fn lookup(&self, uri: &str, name: &str, at: usize, seen: &mut BTreeSet<String>) -> Option<Key> {
        let symbol = self.symbol(uri, name, at);
        let import = self.imported(uri, name, at);
        if let Some(import) = import {
            if symbol
                .is_none_or(|s| import.scope_to - import.scope_from < s.scope_to - s.scope_from)
            {
                return self.import_key(uri, import, seen);
            }
        }
        if let Some(s) = symbol {
            self.identity(s, &mut BTreeSet::new())
        } else {
            self.import_key(uri, import?, seen)
        }
    }
    pub fn definition(&self, key: &Key) -> Option<J> {
        let index = self.indexes.get(&key.uri)?;
        if let Some(s) = index.symbols.iter().find(|s| s.from == key.from) {
            return Some(json!({"uri":key.uri,"from":s.from,"to":s.to}));
        }
        index
            .imports
            .iter()
            .find(|i| i.alias_from == key.from)
            .map(|i| json!({"uri":key.uri,"from":i.alias_from,"to":i.alias_to}))
    }
    pub fn can_rename(&self, key: &Key) -> bool {
        self.valid.contains(&key.uri)
            && self
                .references(key, true)
                .iter()
                .all(|o| self.valid.contains(&o.uri))
    }
    pub fn rename(&self, uri: &str, at: usize, new_name: &str) -> Result<Vec<J>, String> {
        let occurrence = self
            .at(uri, at)
            .ok_or("No variable binding at this position")?;
        let key = &occurrence.key;
        if !self.can_rename(key) {
            return Err("Complete the affected documents before renaming".into());
        }
        if !new_name
            .as_bytes()
            .first()
            .is_some_and(|c| c.is_ascii_alphabetic() || *c == b'_')
            || !new_name
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'_')
            || [
                "function", "return", "import", "export", "from", "as", "if", "else", "for", "in",
                "while", "break", "continue", "true", "false", "null", "and", "or", "not", "style",
                "self", "auto",
            ]
            .contains(&new_name)
        {
            return Err("The new name must be a non-keyword LayMesh identifier".into());
        }
        let refs = self.references(key, true);
        for o in &refs {
            if !o.import_source {
                if let Some(other) = self.lookup(&o.uri, new_name, o.from, &mut BTreeSet::new()) {
                    if &other != key {
                        return Err(format!(
                            "Renaming would capture or conflict with '{new_name}'"
                        ));
                    }
                }
            }
        }
        let declaration = self.definition(key).ok_or("Missing declaration")?;
        let declaration_name = self.indexes[&key.uri]
            .symbols
            .iter()
            .find(|s| s.from == key.from)
            .map(|s| s.name.as_str())
            .or_else(|| {
                self.indexes[&key.uri]
                    .imports
                    .iter()
                    .find(|i| i.alias_from == key.from)
                    .map(|i| i.alias.as_str())
            })
            .unwrap_or("");
        if new_name != declaration_name
            && (laymesh_core::engine::predefined_value(new_name).is_some()
                || crate::entry(new_name).is_some())
        {
            return Err("Renaming would shadow a built-in name".into());
        }
        // Also catch declarations later in the same scope, before their first use.
        let owner = self.indexes[&key.uri]
            .symbols
            .iter()
            .find(|s| s.from == key.from);
        let owner_scope = owner
            .map(|s| (s.scope_from, s.scope_to))
            .or_else(|| {
                self.indexes[&key.uri]
                    .imports
                    .iter()
                    .find(|i| i.alias_from == key.from)
                    .map(|i| (i.scope_from, i.scope_to))
            })
            .unwrap();
        for s in &self.indexes[&key.uri].symbols {
            let same_scope = (s.scope_from, s.scope_to) == owner_scope;
            let captured_assignment = s.kind == "variable"
                && s.from >= s.scope_from
                && owner_scope.0 <= s.scope_from
                && s.scope_to <= owner_scope.1;
            if s.name == new_name
                && (same_scope || captured_assignment)
                && self.identity(s, &mut BTreeSet::new()).as_ref() != Some(key)
            {
                return Err(format!(
                    "A binding named '{new_name}' already exists in this scope"
                ));
            }
        }
        for o in &refs {
            // An unused unaliased import can still conflict with a later declaration.
            let index = &self.indexes[&o.uri];
            let imported = index
                .imports
                .iter()
                .find(|i| i.alias_from == o.from && (!i.explicit_alias || o.declaration));
            if let Some(imported) = imported {
                if index.symbols.iter().any(|s| {
                    s.name == new_name
                        && (s.scope_from, s.scope_to) == (imported.scope_from, imported.scope_to)
                }) || index.imports.iter().any(|i| {
                    i.alias == new_name
                        && (i.scope_from, i.scope_to) == (imported.scope_from, imported.scope_to)
                        && i.alias_from != imported.alias_from
                }) {
                    return Err(format!(
                        "An imported binding would conflict with '{new_name}'"
                    ));
                }
            }
        }
        // Keep the declaration lookup above part of validation, including import aliases.
        let _ = declaration;
        Ok(refs.iter().map(|o| o.json()).collect())
    }
    fn name(&mut self, uri: &str, name: &str, at: usize) {
        if let Some(key) = self.lookup(uri, name, at, &mut BTreeSet::new()) {
            self.push(key, uri, at, at + name.len(), false, false, false);
        }
    }
    fn statements(
        &mut self,
        service: &LanguageService,
        uri: &str,
        source: &str,
        ts: &[Token],
        stmts: &[Stmt],
    ) {
        for stmt in stmts {
            match &stmt.kind {
                StmtKind::Bind(_, e, _) | StmtKind::Expr(e) | StmtKind::Return(e) => {
                    self.expression(service, uri, source, ts, e, 0)
                }
                StmtKind::SetIndex(a, b) => {
                    self.expression(service, uri, source, ts, a, 0);
                    self.expression(service, uri, source, ts, b, 0);
                }
                StmtKind::Function(_, ps, body, _) => {
                    for (_, e) in ps {
                        if let Some(e) = e {
                            self.expression(service, uri, source, ts, e, 0);
                        }
                    }
                    self.statements(service, uri, source, ts, body);
                }
                StmtKind::If(branches, other) => {
                    for (e, body) in branches {
                        self.expression(service, uri, source, ts, e, 0);
                        self.statements(service, uri, source, ts, body);
                    }
                    self.statements(service, uri, source, ts, other);
                }
                StmtKind::For(_, e, body) | StmtKind::While(e, body) => {
                    self.expression(service, uri, source, ts, e, 0);
                    self.statements(service, uri, source, ts, body);
                }
                _ => {}
            }
        }
    }
    fn arguments(
        &mut self,
        service: &LanguageService,
        uri: &str,
        source: &str,
        ts: &[Token],
        args: &[(Option<String>, Expr)],
        base: usize,
        call: Option<&Symbol>,
        start: usize,
    ) {
        for (label, e) in args {
            if let (Some(label), Some(call)) = (label, call) {
                if let Some(parameter) = call.parameters.iter().find(|p| p["name"] == *label) {
                    let at = ts.iter().enumerate().rev().find(|(i, t)| {
                        t.from >= start
                            && t.to <= base + e.loc.offset
                            && t.text == *label
                            && ts.get(i + 1).is_some_and(|t| t.text == "=")
                    });
                    if let Some((_, t)) = at {
                        self.push(
                            Key {
                                uri: call.uri.clone(),
                                from: parameter["from"].as_u64().unwrap() as usize,
                            },
                            uri,
                            t.from,
                            t.to,
                            false,
                            false,
                            true,
                        );
                    }
                }
            }
            self.expression(service, uri, source, ts, e, base);
        }
    }
    fn expression(
        &mut self,
        service: &LanguageService,
        uri: &str,
        source: &str,
        ts: &[Token],
        e: &Expr,
        base: usize,
    ) {
        let at = base + e.loc.offset;
        match &e.kind {
            ExprKind::Ref(parts) => self.name(uri, &parts[0], at),
            ExprKind::Call(parts, args) => {
                self.name(uri, &parts[0], at);
                let function = if parts.len() == 1 {
                    service
                        .resolve(uri, &parts[0], at, &mut BTreeSet::new())
                        .filter(|s| s.kind == "function")
                } else {
                    None
                };
                self.arguments(service, uri, source, ts, args, base, function.as_ref(), at);
            }
            ExprKind::Method(receiver, _, args) => {
                self.expression(service, uri, source, ts, receiver, base);
                self.arguments(service, uri, source, ts, args, base, None, at);
            }
            ExprKind::Member(receiver, _) | ExprKind::Unary(_, receiver) => {
                self.expression(service, uri, source, ts, receiver, base)
            }
            ExprKind::Index(a, b) | ExprKind::Binary(_, a, b) => {
                self.expression(service, uri, source, ts, a, base);
                self.expression(service, uri, source, ts, b, base);
            }
            ExprKind::List(es) => {
                for e in es {
                    self.expression(service, uri, source, ts, e, base);
                }
            }
            ExprKind::Dict(es) => {
                for (a, b) in es {
                    self.expression(service, uri, source, ts, a, base);
                    self.expression(service, uri, source, ts, b, base);
                }
            }
            ExprKind::String(_, _, true) => {
                if let Some(t) = ts.iter().find(|t| t.from == at && t.kind == "string") {
                    for (from, to) in interpolation_ranges(&t.text) {
                        let from = t.from + from;
                        let to = t.from + to;
                        if let Ok(expr) = parser::expression(&source[from..to], uri) {
                            let nested: Vec<_> = tokens(&source[from..to])
                                .into_iter()
                                .map(|mut t| {
                                    t.from += from;
                                    t.to += from;
                                    t
                                })
                                .collect();
                            self.expression(service, uri, source, &nested, &expr, from);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

/// Work on original source bytes, so escapes before an interpolation never shift edits.
pub(crate) fn interpolation_ranges(text: &str) -> Vec<(usize, usize)> {
    let Some(quote) = text.find(['\'', '"']) else {
        return vec![];
    };
    let width = if text[quote..].starts_with("\"\"\"") || text[quote..].starts_with("'''") {
        3
    } else {
        1
    };
    let end = text.len().saturating_sub(width);
    let mut i = quote + width;
    let mut ranges = vec![];
    while i < end {
        if text[i..].starts_with("{{") || text[i..].starts_with("}}") {
            i += 2;
            continue;
        }
        if text.as_bytes()[i] == b'\\' && !text[..quote].contains('r') {
            i += 1;
            if i < end {
                i += text[i..].chars().next().unwrap().len_utf8();
            }
            continue;
        }
        if text.as_bytes()[i] != b'{' {
            i += text[i..].chars().next().unwrap().len_utf8();
            continue;
        }
        let from = i + 1;
        i = from;
        let mut stack = vec![];
        let mut expression_end = None;
        while i < end {
            if let Some(next) = parser::string_end(text, i) {
                i = next;
                continue;
            }
            let c = text.as_bytes()[i];
            if stack.is_empty()
                && (c == b'}'
                    || c == b':'
                    || c == b'!' && text.as_bytes().get(i + 1) != Some(&b'='))
            {
                expression_end = Some(i);
                break;
            }
            if b"([{ ".contains(&c) && c != b' ' {
                stack.push(c);
            } else if b")] }".contains(&c) && c != b' ' {
                stack.pop();
            }
            i += text[i..].chars().next().unwrap().len_utf8();
        }
        if let Some(to) = expression_end {
            ranges.push((from, to));
        }
        while i < end && text.as_bytes()[i] != b'}' {
            i += text[i..].chars().next().unwrap().len_utf8();
        }
        if i < end {
            i += 1;
        }
    }
    ranges
}

#[cfg(test)]
mod tests {
    #[test]
    fn unfinished_unicode_escape_keeps_interpolation_offsets_on_char_boundaries() {
        let source = r#"f"\中 😀 {value}""#;
        let ranges = super::interpolation_ranges(source);
        assert_eq!(ranges.len(), 1);
        assert_eq!(&source[ranges[0].0..ranges[0].1], "value");
    }
}
