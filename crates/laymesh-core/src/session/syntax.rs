use super::*;
use serde_json::Value as J;
use std::collections::BTreeSet;
#[derive(Default)]
pub(crate) struct SyntaxCache {
    files: BTreeMap<String, (String, Rc<Vec<Stmt>>)>,
    next: u64,
    functions: HashMap<u64, (Rc<Vec<(String, Option<Expr>)>>, Rc<Vec<Stmt>>)>,
    origins: HashMap<u64, CallOrigin>,
    pub parsed_bytes: usize,
    pub reused_statements: usize,
}
#[derive(Clone)]
pub(crate) struct CallOrigin {
    pub revision: u64,
    pub file: String,
    pub loc: Loc,
    pub arguments: BTreeMap<String, Loc>,
}
fn shape<T: Serialize>(node: &T) -> [u8; 32] {
    fn strip(v: &mut J) {
        match v {
            J::Object(m) => {
                m.remove("loc");
                for v in m.values_mut() {
                    strip(v);
                }
            }
            J::Array(a) => {
                for v in a {
                    strip(v);
                }
            }
            _ => {}
        }
    }
    let mut v = serde_json::to_value(node).unwrap();
    strip(&mut v);
    // Retain fixed-size fingerprints, not a second serialized copy of each
    // function and every nested statement in a large generated module.
    use sha2::{Digest, Sha256};
    struct HashWriter(Sha256);
    impl std::io::Write for HashWriter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.update(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut writer = HashWriter(Sha256::new());
    serde_json::to_writer(&mut writer, &v).unwrap();
    writer.0.finalize().into()
}
fn label(s: &Stmt) -> String {
    match &s.kind {
        StmtKind::Bind(n, ..) => format!("bind:{n}"),
        StmtKind::Function(n, ..) => format!("function:{n}"),
        StmtKind::Import(_, p) => format!("import:{p}"),
        StmtKind::For(p, ..) => format!("for:{}", serde_json::to_string(p).unwrap()),
        other => format!("{:?}", std::mem::discriminant(other)),
    }
}
impl SyntaxCache {
    fn remove_functions(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            match &stmt.kind {
                StmtKind::Function(_, _, body, _) => {
                    self.functions.remove(&stmt.identity);
                    self.remove_functions(body);
                }
                StmtKind::If(branches, other) => {
                    for (_, body) in branches {
                        self.remove_functions(body);
                    }
                    self.remove_functions(other);
                }
                StmtKind::For(_, _, body) | StmtKind::While(_, body) => self.remove_functions(body),
                _ => {}
            }
        }
    }
    pub fn remove(&mut self, file: &str) {
        if let Some((_, stmts)) = self.files.remove(file) {
            self.remove_functions(&stmts);
        }
        self.origins.retain(|_, o| o.file != file);
    }
    pub fn function(&self, id: u64) -> Option<(Rc<Vec<(String, Option<Expr>)>>, Rc<Vec<Stmt>>)> {
        self.functions.get(&id).cloned()
    }
    pub fn origin(&self, site: u64) -> Option<&CallOrigin> {
        self.origins.get(&site)
    }
    pub fn is_current(&self, file: &str, source: &str) -> bool {
        self.files.get(file).is_some_and(|(s, _)| s == source)
    }
    fn id(&mut self) -> u64 {
        self.next += 1;
        self.next
    }
    pub fn parse(&mut self, source: &str, file: &str) -> Result<Rc<Vec<Stmt>>> {
        if let Some((s, stmts)) = self.files.get(file) {
            if s == source {
                return Ok(stmts.clone());
            }
        }
        self.parsed_bytes = 0;
        self.reused_statements = 0;
        let previous = self.files.get(file).cloned();
        let next = if let Some((before, old)) = previous {
            if let Some(next) = self.region(&before, source, file, &old) {
                next?
            } else {
                self.parsed_bytes = source.len();
                let mut next = parser::parse(source, file)?;
                self.statements(&old, &mut next);
                next
            }
        } else {
            self.parsed_bytes = source.len();
            let mut next = parser::parse(source, file)?;
            self.statements(&[], &mut next);
            next
        };
        let mut next_origins = HashMap::new();
        collect_origins(&next, file, &mut next_origins);
        self.origins
            .retain(|site, origin| origin.file != file || next_origins.contains_key(site));
        for (site, mut origin) in next_origins {
            origin.revision = if let Some(old) = self.origins.get(&site).filter(|old| {
                old.file == origin.file
                    && old.loc == origin.loc
                    && old.arguments == origin.arguments
            }) {
                old.revision
            } else {
                self.id()
            };
            self.origins.insert(site, origin);
        }
        fn functions(
            stmts: &[Stmt],
            out: &mut HashMap<u64, (Rc<Vec<(String, Option<Expr>)>>, Rc<Vec<Stmt>>)>,
        ) {
            for stmt in stmts {
                match &stmt.kind {
                    StmtKind::Function(_, params, body, _) => {
                        out.insert(
                            stmt.identity,
                            (Rc::new(params.clone()), Rc::new(body.clone())),
                        );
                        functions(body, out);
                    }
                    StmtKind::If(branches, other) => {
                        for (_, body) in branches {
                            functions(body, out);
                        }
                        functions(other, out);
                    }
                    StmtKind::For(_, _, body) | StmtKind::While(_, body) => functions(body, out),
                    _ => {}
                }
            }
        }
        if let Some((_, old)) = self.files.get(file).cloned() {
            self.remove_functions(&old);
        }
        functions(&next, &mut self.functions);
        let next = Rc::new(next);
        self.files
            .insert(file.into(), (source.into(), next.clone()));
        Ok(next)
    }
    fn region(
        &mut self,
        before: &str,
        after: &str,
        file: &str,
        old: &[Stmt],
    ) -> Option<Result<Vec<Stmt>>> {
        if old.is_empty() {
            return None;
        }
        let mut prefix = before
            .bytes()
            .zip(after.bytes())
            .take_while(|(a, b)| a == b)
            .count();
        while !before.is_char_boundary(prefix) || !after.is_char_boundary(prefix) {
            prefix -= 1;
        }
        let mut suffix = before[prefix..]
            .bytes()
            .rev()
            .zip(after[prefix..].bytes().rev())
            .take_while(|(a, b)| a == b)
            .count();
        while !before.is_char_boundary(before.len() - suffix)
            || !after.is_char_boundary(after.len() - suffix)
        {
            suffix -= 1;
        }
        // Include neighbours: punctuation or an `else` can join a previously
        // separate statement. Ambiguous boundaries fall back to the parser.
        let first = old
            .partition_point(|s| s.loc.offset <= prefix)
            .saturating_sub(2);
        let mut last =
            (old.partition_point(|s| s.loc.offset < before.len() - suffix) + 1).min(old.len());
        while last < old.len()
            && !before[..old[last].loc.offset]
                .rsplit('\n')
                .next()
                .unwrap_or("")
                .trim()
                .is_empty()
        {
            last += 1;
        }
        let start = if first == 0 { 0 } else { old[first].loc.offset };
        let end = old.get(last).map_or(before.len(), |s| s.loc.offset);
        if end < before.len() - suffix {
            return None;
        }
        let next_end = (end as isize + after.len() as isize - before.len() as isize) as usize;
        if next_end < start || !after.is_char_boundary(next_end) {
            return None;
        }
        let origin = location(after, start);
        let mut middle = match parser::parse_region(&after[start..next_end], file, origin) {
            Ok(v) => v,
            Err(_) => return None,
        };
        self.parsed_bytes = next_end - start;
        self.reused_statements = first + old.len() - last;
        self.statements(&old[first..last], &mut middle);
        let mut next = old[..first].to_vec();
        next.extend(middle);
        let old_origin = location(before, end);
        let new_origin = location(after, next_end);
        let mut tail = old[last..].to_vec();
        for s in &mut tail {
            visit_stmt(s, &mut |loc| {
                loc.offset = (loc.offset as isize + new_origin.offset as isize
                    - old_origin.offset as isize) as usize;
                if loc.line == old_origin.line {
                    loc.column = (loc.column as isize + new_origin.column as isize
                        - old_origin.column as isize) as usize;
                }
                loc.line = (loc.line as isize + new_origin.line as isize - old_origin.line as isize)
                    as usize;
            });
        }
        next.extend(tail);
        Some(Ok(next))
    }
    fn statements(&mut self, old: &[Stmt], new: &mut [Stmt]) {
        let fingerprints: Vec<_> = old.iter().map(shape).collect();
        // On a cold parse there are no old identities to match. Hashing whole
        // functions and every nested literal only duplicates parsing work.
        let next_fingerprints: Vec<_> = if old.is_empty() {
            vec![]
        } else {
            new.iter().map(shape).collect()
        };
        let mut used = BTreeSet::new();
        let mut matches = vec![None; new.len()];
        // Reserve unchanged nodes before matching edited ones. Otherwise an
        // inserted expression can steal the identity of its unchanged sibling.
        for (i, fingerprint) in next_fingerprints.iter().enumerate() {
            if let Some(index) = fingerprints
                .iter()
                .enumerate()
                .find_map(|(j, old)| (!used.contains(&j) && old == fingerprint).then_some(j))
            {
                used.insert(index);
                matches[i] = Some(index);
            }
        }
        for (i, n) in new.iter().enumerate() {
            if matches[i].is_some() {
                continue;
            }
            let candidates: Vec<_> = old
                .iter()
                .enumerate()
                .filter_map(|(j, o)| (!used.contains(&j) && label(o) == label(n)).then_some(j))
                .collect();
            if let [index] = candidates.as_slice() {
                used.insert(*index);
                matches[i] = Some(*index);
            }
        }
        for (i, n) in new.iter_mut().enumerate() {
            let previous = matches[i].map(|j| &old[j]);
            n.revision = previous
                .filter(|o| shape(*o) == next_fingerprints[i])
                .map(|o| o.revision)
                .unwrap_or_else(|| self.id());
            n.identity = previous.map(|o| o.identity).unwrap_or_else(|| self.id());
            let o = previous.map(|o| &o.kind);
            match &mut n.kind {
                StmtKind::Bind(_, e, _) | StmtKind::Return(e) | StmtKind::Expr(e) => {
                    let old = match o {
                        Some(StmtKind::Bind(_, e, _) | StmtKind::Return(e) | StmtKind::Expr(e)) => {
                            Some(e)
                        }
                        _ => None,
                    };
                    self.expression(old, e);
                }
                StmtKind::SetIndex(a, b) => {
                    let pair = match o {
                        Some(StmtKind::SetIndex(a, b)) => (Some(a), Some(b)),
                        _ => (None, None),
                    };
                    self.expression(pair.0, a);
                    self.expression(pair.1, b);
                }
                StmtKind::Function(_, params, body, _) => {
                    let (op, ob) = match o {
                        Some(StmtKind::Function(_, p, b, _)) => (p.as_slice(), b.as_slice()),
                        _ => (&[][..], &[][..]),
                    };
                    for (name, e) in params {
                        if let Some(e) = e {
                            self.expression(
                                op.iter()
                                    .find(|(n, _)| n == name)
                                    .and_then(|(_, e)| e.as_ref()),
                                e,
                            );
                        }
                    }
                    self.statements(ob, body);
                }
                StmtKind::For(_, seq, body) | StmtKind::While(seq, body) => {
                    let (oe, ob) = match o {
                        Some(StmtKind::For(_, e, b) | StmtKind::While(e, b)) => {
                            (Some(e), b.as_slice())
                        }
                        _ => (None, &[][..]),
                    };
                    self.expression(oe, seq);
                    self.statements(ob, body);
                }
                StmtKind::If(branches, other) => {
                    let (obs, oo) = match o {
                        Some(StmtKind::If(b, o)) => (b.as_slice(), o.as_slice()),
                        _ => (&[][..], &[][..]),
                    };
                    for (i, (e, b)) in branches.iter_mut().enumerate() {
                        self.expression(obs.get(i).map(|(e, _)| e), e);
                        self.statements(obs.get(i).map(|(_, b)| b.as_slice()).unwrap_or(&[]), b);
                    }
                    self.statements(oo, other);
                }
                _ => {}
            }
        }
    }
    fn expression(&mut self, old: Option<&Expr>, new: &mut Expr) {
        // Content changes get a new query identity; unchanged descendants keep
        // theirs even when their enclosing call/statement has changed.
        new.site = old.map(|o| o.site).unwrap_or_else(|| self.id());
        new.identity = old
            .filter(|o| shape(*o) == shape(new))
            .map(|o| o.identity)
            .unwrap_or_else(|| self.id());
        let o = old.map(|o| &o.kind);
        match &mut new.kind {
            ExprKind::Call(_, args) | ExprKind::Method(_, _, args) => {
                let oa = match o {
                    Some(ExprKind::Call(_, a) | ExprKind::Method(_, _, a)) => a.as_slice(),
                    _ => &[],
                };
                for (i, (n, e)) in args.iter_mut().enumerate() {
                    let oe = if n.is_some() {
                        oa.iter().find(|(on, _)| on == n).map(|(_, e)| e)
                    } else {
                        oa.get(i).map(|(_, e)| e)
                    };
                    self.expression(oe, e);
                }
            }
            ExprKind::List(es) => {
                let os = match o {
                    Some(ExprKind::List(es)) => es.as_slice(),
                    _ => &[],
                };
                if os.is_empty() {
                    for e in es {
                        self.expression(None, e);
                    }
                    return;
                }
                let fingerprints: Vec<_> = os.iter().map(shape).collect();
                let mut used = BTreeSet::new();
                let matches: Vec<_> = es
                    .iter()
                    .map(|e| {
                        let fingerprint = shape(e);
                        let found = fingerprints.iter().enumerate().find_map(|(i, f)| {
                            (!used.contains(&i) && *f == fingerprint).then_some(i)
                        });
                        if let Some(i) = found {
                            used.insert(i);
                        }
                        found
                    })
                    .collect();
                for (i, e) in es.iter_mut().enumerate() {
                    let matched =
                        matches[i].or_else(|| (i < os.len() && !used.contains(&i)).then_some(i));
                    if let Some(i) = matched {
                        used.insert(i);
                    }
                    self.expression(matched.map(|i| &os[i]), e);
                }
            }
            ExprKind::Dict(es) => {
                let os = match o {
                    Some(ExprKind::Dict(es)) => es.as_slice(),
                    _ => &[],
                };
                for (i, (a, b)) in es.iter_mut().enumerate() {
                    self.expression(os.get(i).map(|(a, _)| a), a);
                    self.expression(os.get(i).map(|(_, b)| b), b);
                }
            }
            ExprKind::Binary(_, a, b) | ExprKind::Index(a, b) => {
                let os = match o {
                    Some(ExprKind::Binary(_, a, b) | ExprKind::Index(a, b)) => {
                        (Some(a.as_ref()), Some(b.as_ref()))
                    }
                    _ => (None, None),
                };
                self.expression(os.0, a);
                self.expression(os.1, b);
            }
            ExprKind::Unary(_, a) | ExprKind::Member(a, _) => {
                let oa = match o {
                    Some(ExprKind::Unary(_, a) | ExprKind::Member(a, _)) => Some(a.as_ref()),
                    _ => None,
                };
                self.expression(oa, a);
            }
            _ => {}
        }
        if let ExprKind::Method(receiver, _, _) = &mut new.kind {
            let old = match o {
                Some(ExprKind::Method(r, _, _)) => Some(r.as_ref()),
                _ => None,
            };
            self.expression(old, receiver);
        }
    }
}

fn location(source: &str, offset: usize) -> Loc {
    let before = &source[..offset];
    Loc {
        offset,
        line: before.bytes().filter(|b| *b == b'\n').count() + 1,
        column: before
            .rsplit('\n')
            .next()
            .unwrap_or("")
            .encode_utf16()
            .count()
            + 1,
    }
}
fn visit_stmt(s: &mut Stmt, f: &mut impl FnMut(&mut Loc)) {
    f(&mut s.loc);
    match &mut s.kind {
        StmtKind::Bind(_, e, _) | StmtKind::Return(e) | StmtKind::Expr(e) => visit_expr(e, f),
        StmtKind::SetIndex(a, b) => {
            visit_expr(a, f);
            visit_expr(b, f);
        }
        StmtKind::Function(_, p, b, _) => {
            for (_, e) in p {
                if let Some(e) = e {
                    visit_expr(e, f);
                }
            }
            for s in b {
                visit_stmt(s, f);
            }
        }
        StmtKind::For(_, e, b) | StmtKind::While(e, b) => {
            visit_expr(e, f);
            for s in b {
                visit_stmt(s, f);
            }
        }
        StmtKind::If(bs, other) => {
            for (e, b) in bs {
                visit_expr(e, f);
                for s in b {
                    visit_stmt(s, f);
                }
            }
            for s in other {
                visit_stmt(s, f);
            }
        }
        _ => {}
    }
}
fn visit_expr(e: &mut Expr, f: &mut impl FnMut(&mut Loc)) {
    f(&mut e.loc);
    match &mut e.kind {
        ExprKind::Call(_, a) => {
            for (_, e) in a {
                visit_expr(e, f);
            }
        }
        ExprKind::Method(r, _, a) => {
            visit_expr(r, f);
            for (_, e) in a {
                visit_expr(e, f);
            }
        }
        ExprKind::List(a) => {
            for e in a {
                visit_expr(e, f);
            }
        }
        ExprKind::Dict(a) => {
            for (a, b) in a {
                visit_expr(a, f);
                visit_expr(b, f);
            }
        }
        ExprKind::Binary(_, a, b) | ExprKind::Index(a, b) => {
            visit_expr(a, f);
            visit_expr(b, f);
        }
        ExprKind::Unary(_, a) | ExprKind::Member(a, _) => visit_expr(a, f),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn compare(cache: &mut SyntaxCache, source: &str) {
        let incremental = cache.parse(source, "/main.lay");
        let full = parser::parse(source, "/main.lay");
        match (incremental, full) {
            (Ok(a), Ok(b)) => assert_eq!(
                serde_json::to_value(a.as_ref()).unwrap(),
                serde_json::to_value(b).unwrap(),
                "{source}"
            ),
            (Err(a), Err(b)) => assert_eq!(a, b, "{source}"),
            (a, b) => panic!("different parse results: {a:?} / {b:?}\n{source}"),
        }
    }
    #[test]
    fn regional_parse_preserves_unaffected_identity_and_updates_locations() {
        let source = "page=canvas(size=(20,20))\na=1\nb=2\nfunction f(x) { return x+a }\nc=f(b)\nd=(1,2,3)\ne=\"中文😀\"\nf=8\ng=9";
        let mut cache = SyntaxCache::default();
        let before = cache.parse(source, "/main.lay").unwrap();
        let after = cache
            .parse(&source.replace("a=1", "# new line\na=3"), "/main.lay")
            .unwrap();
        assert_eq!(
            before.last().unwrap().identity,
            after.last().unwrap().identity
        );
        assert!(cache.parsed_bytes < source.len() / 2);
        assert!(cache.reused_statements > 3);
        compare(&mut cache, &source.replace("a=1", "# new line\na=3"));
    }
    #[test]
    fn differential_edit_sequences_cover_boundaries_comments_unicode_and_errors() {
        let sources = [
            "a=1\nb=2\nc=3\nd=4\ne=5\nf=6\ng=7",
            "a=1 b=2 c=3 d=4 e=5 f=6 g=7",
            "a=\"中文😀\"\nfunction f(x) { if x>2 { return x } else { return 0 } }\nb=f(2)\nc=(1,2,3)\nd=4\ne=5",
            "a=1\n# comment\nb=2\n'''note\nmultiple lines'''\nc=3\nd=4\ne=5\nf=6",
        ];
        for source in sources {
            let mut cache = SyntaxCache::default();
            compare(&mut cache, source);
            for index in 0..=source.len() {
                if !source.is_char_boundary(index) {
                    continue;
                }
                for inserted in ["#", "\n", " ", "}", "{", "'", "😀", "else", "+", "\"\"\""] {
                    let next = format!("{}{}{}", &source[..index], inserted, &source[index..]);
                    compare(&mut cache, &next);
                    compare(&mut cache, source);
                }
                if index < source.len() {
                    let end = index + source[index..].chars().next().unwrap().len_utf8();
                    compare(
                        &mut cache,
                        &format!("{}{}", &source[..index], &source[end..]),
                    );
                    compare(&mut cache, source);
                }
            }
        }
    }
}

fn collect_origins(statements: &[Stmt], file: &str, origins: &mut HashMap<u64, CallOrigin>) {
    fn expression(e: &Expr, file: &str, origins: &mut HashMap<u64, CallOrigin>) {
        match &e.kind {
            ExprKind::Call(_, args) | ExprKind::Method(_, _, args) => {
                let mut position = 0;
                let mut arguments = BTreeMap::new();
                for (name, e) in args {
                    let mut loc = e.loc;
                    if let ExprKind::String(_, raw, formatted) = e.kind {
                        let prefix = 1 + usize::from(raw) + usize::from(formatted);
                        loc.column += prefix;
                        loc.offset += prefix;
                    }
                    let name = name.clone().unwrap_or_else(|| {
                        let name = format!("_{position}");
                        position += 1;
                        name
                    });
                    arguments.insert(name, loc);
                    expression(e, file, origins);
                }
                origins.insert(
                    e.site,
                    CallOrigin {
                        revision: 0,
                        file: file.into(),
                        loc: e.loc,
                        arguments,
                    },
                );
                if let ExprKind::Method(r, _, _) = &e.kind {
                    expression(r, file, origins);
                }
            }
            ExprKind::List(a) => {
                for e in a {
                    expression(e, file, origins);
                }
            }
            ExprKind::Dict(a) => {
                for (a, b) in a {
                    expression(a, file, origins);
                    expression(b, file, origins);
                }
            }
            ExprKind::Index(a, b) | ExprKind::Binary(_, a, b) => {
                expression(a, file, origins);
                expression(b, file, origins);
            }
            ExprKind::Unary(_, a) | ExprKind::Member(a, _) => expression(a, file, origins),
            _ => {}
        }
    }
    for statement in statements {
        match &statement.kind {
            StmtKind::Bind(_, e, _) | StmtKind::Return(e) | StmtKind::Expr(e) => {
                expression(e, file, origins)
            }
            StmtKind::SetIndex(a, b) => {
                expression(a, file, origins);
                expression(b, file, origins);
            }
            StmtKind::Function(_, p, b, _) => {
                for (_, e) in p {
                    if let Some(e) = e {
                        expression(e, file, origins);
                    }
                }
                collect_origins(b, file, origins);
            }
            StmtKind::For(_, e, b) | StmtKind::While(e, b) => {
                expression(e, file, origins);
                collect_origins(b, file, origins);
            }
            StmtKind::If(bs, other) => {
                for (e, b) in bs {
                    expression(e, file, origins);
                    collect_origins(b, file, origins);
                }
                collect_origins(other, file, origins);
            }
            _ => {}
        }
    }
}
