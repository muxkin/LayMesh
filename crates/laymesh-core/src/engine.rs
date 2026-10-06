use crate::{
    Diagnostic, Loc, Result,
    model::*,
    parser::{self, Expr, ExprKind, Stmt, StmtKind},
};
use serde_json::{Value as Json, json};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    path::Path,
    rc::Rc,
    sync::LazyLock,
};

pub static API: LazyLock<Json> =
    LazyLock::new(|| serde_json::from_str(include_str!("../api.json")).unwrap());
/// Builtin string variables are fallback bindings, so user scopes retain priority.
pub fn predefined_variables() -> &'static Vec<Json> {
    API["predefinedVariables"].as_array().unwrap()
}
pub fn predefined_value(name: &str) -> Option<&'static str> {
    predefined_variables().iter().find(|v| v["name"] == name)?["value"].as_str()
}
pub struct Engine {
    pub host: Host,
    pub warnings: Vec<Diagnostic>,
    pub fonts: crate::text::FontSystem,
    pub(crate) art_cache: BTreeMap<String, Json>,
    pub unit: String,
    pub dpi: f64,
    pub canvas: Option<Rc<RefCell<Object>>>,
    pub modules: BTreeMap<String, Scope>,
    pub loading: BTreeSet<String>,
    pub objects: BTreeMap<usize, Rc<RefCell<Object>>>,
    pub styles: Vec<crate::style::StyleRule>,
    pub file: String,
    pub steps: usize,
    pub calls: usize,
    pub serial: usize,
    unnamed_instances: usize,
    instance_names: BTreeMap<String, usize>,
    // Successful image reads are a snapshot for one preview compilation only.
    preview_images: BTreeMap<String, Json>,
    cyclic_scopes: Vec<std::rc::Weak<RefCell<Environment>>>,
}
impl Drop for Engine {
    fn drop(&mut self) {
        // User functions close over their defining environment, and layer
        // handles point back to plots. Break these deliberate Rc cycles at
        // compilation end so repeated WASM editor requests remain bounded.
        for scope in &self.cyclic_scopes {
            if let Some(scope) = scope.upgrade() {
                scope.borrow_mut().values.clear();
            }
        }
        for object in self.objects.values() {
            let mut o = object.borrow_mut();
            o.args.clear();
            o.layers.clear();
        }
    }
}
enum Flow {
    Normal,
    Return(V),
    Break,
    Continue,
}
impl Engine {
    pub(crate) fn load_image(&mut self, path: &str, file: &str, loc: Loc) -> Result<Json> {
        let preview = crate::asset_cache::is_preview();
        if preview {
            if let Some(asset) = self.preview_images.get(path) {
                return Ok(asset.clone());
            }
        }
        let bytes = self.host.read(path, file, loc)?;
        let asset = crate::assets::load(&bytes, path, file, loc)?;
        if preview {
            self.preview_images.insert(path.into(), asset.clone());
        }
        Ok(asset)
    }

    pub fn new(host: Host) -> Self {
        let native = host.native;
        let mut fonts = crate::text::FontSystem::new(native);
        for (name, data) in &host.files {
            if Path::new(name)
                .extension()
                .and_then(|s| s.to_str())
                .is_some_and(|s| {
                    matches!(
                        s.to_ascii_lowercase().as_str(),
                        "ttf" | "otf" | "ttc" | "otc"
                    )
                })
            {
                fonts.register_font(name, data.clone());
            }
        }
        Self {
            host,
            warnings: vec![],
            fonts,
            art_cache: BTreeMap::new(),
            unit: "mm".into(),
            dpi: 96.,
            canvas: None,
            modules: BTreeMap::new(),
            loading: BTreeSet::new(),
            objects: BTreeMap::new(),
            styles: vec![],
            file: String::new(),
            steps: 0,
            calls: 0,
            serial: 0,
            unnamed_instances: 0,
            instance_names: BTreeMap::new(),
            preview_images: BTreeMap::new(),
            cyclic_scopes: vec![],
        }
    }
    pub fn error(&self, c: &str, m: impl Into<String>, l: Loc) -> Diagnostic {
        Diagnostic::new(c, m, &self.file, l)
    }
    pub fn warn(&mut self, c: &str, m: impl Into<String>, l: Loc) {
        let d = self.error(c, m, l);
        if !self.warnings.iter().any(|w| {
            w.file == d.file && w.loc == d.loc && w.code == d.code && w.message == d.message
        }) {
            self.warnings.push(d);
        }
    }
    pub fn len(&self, v: &V, l: Loc) -> Result<f64> {
        value_length(v, &self.unit, self.dpi)
            .filter(|n| n.is_finite())
            .ok_or_else(|| self.error("E_UNIT", "需要有限的物理长度", l))
    }
    pub fn scalar(&self, v: &V, l: Loc) -> Result<f64> {
        match v.value() {
            V::Number(n, u) if u.is_empty() && n.is_finite() => Ok(*n),
            _ => Err(self.error("E_UNIT", "需要有限的无单位数值", l)),
        }
    }
    pub fn object(&mut self, kind: &str, args: Args, loc: Loc) -> V {
        self.serial += 1;
        {
            let object = Rc::new(RefCell::new(Object {
                kind: kind.into(),
                args,
                file: self.file.clone(),
                loc,
                id: self.serial,
                nodes: vec![],
                layers: vec![],
                sealed: false,
                width: 0.,
                height: 0.,
                node: None,
                parent: 0,
            }));
            self.objects.insert(self.serial, object.clone());
            V::Object(object)
        }
    }
    pub fn compile(&mut self, source: &str, file: &str) -> Result<Scene> {
        self.preview_images.clear();
        self.file = file.into();
        let stmts = parser::parse(source, file)?;
        let scope = Environment::root();
        if !matches!(self.run(&stmts, &scope)?, Flow::Normal) {
            return Err(self.error(
                "E_STATEMENT",
                "入口顶层不能使用 return、break 或 continue",
                Loc {
                    line: 1,
                    column: 1,
                    offset: 0,
                },
            ));
        }
        let c = self.canvas.clone().ok_or_else(|| {
            self.error(
                "E_CANVAS",
                "入口文件需要一个 canvas",
                Loc {
                    line: 1,
                    column: 1,
                    offset: 0,
                },
            )
        })?;
        let c = c.borrow();
        for node in &c.nodes {
            if let Some(b) = crate::art::decorated_bounds(node) {
                if b.x0 < -0.01 || b.y0 < -0.01 || b.x1 > c.width + 0.01 || b.y1 > c.height + 0.01 {
                    self.warn(
                        "W_EFFECT_OVERFLOW",
                        "Decorated content or effects extend beyond the canvas",
                        Loc::default(),
                    );
                }
            }
        }
        Ok(Scene {
            schema_version: 8,
            width: c.width,
            height: c.height,
            background: c
                .args
                .get("background")
                .map(V::json)
                .unwrap_or(json!("none")),
            layout_dpi: self.dpi,
            canvas_unit: self.unit.clone(),
            export_dpi: 1200.,
            nodes: c.nodes.clone(),
            warnings: self.warnings.clone(),
            fonts: self.fonts.assets.clone(),
        })
    }
    fn module(&mut self, path: &str, l: Loc) -> Result<Scope> {
        if self.loading.contains(path) {
            return Err(self.error("E_IMPORT", format!("循环导入：{path}"), l));
        }
        if let Some(s) = self.modules.get(path) {
            return Ok(s.clone());
        }
        self.loading.insert(path.into());
        let input = self.host.read(path, &self.file, l)?;
        let source =
            String::from_utf8(input).map_err(|_| self.error("E_IMPORT", "模块不是 UTF-8", l))?;
        let stmts = parser::parse(&source, path)?;
        let scope = Environment::root();
        let old = std::mem::replace(&mut self.file, path.into());
        let result = self.run(&stmts, &scope);
        self.file = old;
        self.loading.remove(path);
        result?;
        let exports = Environment::root();
        for stmt in stmts {
            let n = match stmt.kind {
                StmtKind::Bind(n, _, true) | StmtKind::Function(n, _, _, true) => Some(n),
                _ => None,
            };
            if let Some(n) = n {
                if let Some(v) = Environment::get(&scope, &n) {
                    exports.borrow_mut().values.insert(n, v);
                }
            }
        }
        self.modules.insert(path.into(), exports.clone());
        Ok(exports)
    }
    fn run(&mut self, stmts: &[Stmt], scope: &Scope) -> Result<Flow> {
        self.declare_functions(stmts, scope);
        for stmt in stmts {
            self.steps += 1;
            if self.steps > 1_000_000 {
                return Err(self.error("E_LIMIT", "执行步骤超过 1,000,000", stmt.loc));
            }
            let flow = self.run_statement(stmt, scope)?;
            if !matches!(flow, Flow::Normal) {
                return Ok(flow);
            }
        }
        Ok(Flow::Normal)
    }
    #[inline(never)]
    fn run_statement(&mut self, stmt: &Stmt, scope: &Scope) -> Result<Flow> {
        // Keep recursive returns outside the large statement dispatch frame.
        match &stmt.kind {
            StmtKind::Return(e) => Ok(Flow::Return(self.eval(e, scope)?)),
            StmtKind::Bind(n, e, _) => {
                self.run_binding(n, e, scope, stmt.loc)?;
                Ok(Flow::Normal)
            }
            StmtKind::If(bs, other) => self.run_if(bs, other, scope),
            _ => self.run_other_statement(stmt, scope),
        }
    }
    #[inline(never)]
    fn run_binding(
        &mut self,
        name: &str,
        expression: &Expr,
        scope: &Scope,
        loc: Loc,
    ) -> Result<()> {
        let value = self.eval(expression, scope)?;
        self.bind_value(name, value, scope, loc)
    }
    #[inline(never)]
    fn declare_functions(&mut self, stmts: &[Stmt], scope: &Scope) {
        for stmt in stmts {
            if let StmtKind::Function(n, ps, body, _) = &stmt.kind {
                self.cyclic_scopes.push(Rc::downgrade(scope));
                scope.borrow_mut().values.insert(
                    n.clone(),
                    V::Function {
                        params: ps.clone(),
                        body: body.clone(),
                        scope: scope.clone(),
                        file: self.file.clone(),
                    },
                );
            }
        }
    }
    #[inline(never)]
    fn run_if(
        &mut self,
        branches: &[(Expr, Vec<Stmt>)],
        other: &[Stmt],
        scope: &Scope,
    ) -> Result<Flow> {
        let mut branch = other;
        for (condition, body) in branches {
            if self.truth(condition, scope)? {
                branch = body;
                break;
            }
        }
        self.run(branch, &Environment::child(scope))
    }
    #[inline(never)]
    fn run_other_statement(&mut self, stmt: &Stmt, scope: &Scope) -> Result<Flow> {
        match &stmt.kind {
            StmtKind::Bind(..) | StmtKind::Return(..) | StmtKind::If(..) => unreachable!(),
            StmtKind::SetIndex(target, value) => self.set_index(target, value, scope, stmt.loc)?,
            StmtKind::Expr(e) => {
                if !matches!(&e.kind,ExprKind::Call(parts,_) if parts.len()==2 && (matches!(parts[1].as_str(),"add"|"fuse") || Environment::get(scope,&parts[0]).and_then(|v|v.object()).is_some_and(|o|o.borrow().kind=="plot")))
                {
                    return Err(self.error(
                        "E_STATEMENT",
                        "独立语句只能调用容器的 add/fuse",
                        stmt.loc,
                    ));
                }
                let value = self.eval(e, scope)?;
                if let Some(instance) = value.object().filter(|o| o.borrow().kind == "instance") {
                    self.unnamed_instances += 1;
                    let name = json!(format!("@{}", self.unnamed_instances));
                    let mut instance = instance.borrow_mut();
                    let parent = instance.parent;
                    if let Some(node) = instance.node.as_mut() {
                        let old = node["id"].clone();
                        node["id"] = name.clone();
                        if let Some(owner) = self.objects.get(&parent) {
                            for node in &mut owner.borrow_mut().nodes {
                                if node["id"] == old {
                                    node["id"] = name.clone();
                                }
                            }
                        }
                    }
                }
            }
            StmtKind::Function(..) => {}
            StmtKind::Break => return Ok(Flow::Break),
            StmtKind::Continue => return Ok(Flow::Continue),
            StmtKind::Import(ns, path) => {
                let path = resolve(&self.file, path);
                let m = self.module(&path, stmt.loc)?;
                for (n, a) in ns {
                    let v = Environment::get(&m, n).ok_or_else(|| {
                        self.error("E_IMPORT", format!("模块没有导出 {n}"), stmt.loc)
                    })?;
                    scope.borrow_mut().values.insert(a.clone(), v);
                    scope.borrow_mut().readonly.push(a.clone());
                }
            }
            StmtKind::Style(css) => {
                self.stylesheet_scoped(css, &self.file.clone(), stmt.loc, Some(&self.file.clone()))?
            }
            StmtKind::For(n, seq, body) => {
                let v = self.eval(seq, scope)?;
                let vs = self.sequence(v, stmt.loc)?;
                if vs.len() > 10_000 {
                    return Err(self.error("E_LIMIT", "循环次数超过 10,000", stmt.loc));
                }
                for v in vs {
                    let s = Environment::child(scope);
                    let bindings = self.bind_pattern(n, v, stmt.loc)?;
                    s.borrow_mut().values.extend(bindings);
                    match self.run(body, &s)? {
                        Flow::Break => break,
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                        _ => {}
                    }
                }
            }
            StmtKind::While(c, body) => {
                let mut count = 0;
                while self.truth(c, scope)? {
                    count += 1;
                    if count > 10_000 {
                        return Err(self.error("E_LIMIT", "循环次数超过 10,000", stmt.loc));
                    }
                    match self.run(body, &Environment::child(scope))? {
                        Flow::Break => break,
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                        _ => {}
                    }
                }
            }
        }
        Ok(Flow::Normal)
    }
    #[inline(never)]
    fn bind_value(&mut self, n: &str, v: V, scope: &Scope, l: Loc) -> Result<()> {
        if let Some(o) = v.object() {
            let mut o = o.borrow_mut();
            let parent = o.parent;
            if let Some(node) = o.node.as_mut() {
                let count = self.instance_names.entry(n.to_string()).or_default();
                *count += 1;
                let name = if *count == 1 {
                    n.to_string()
                } else {
                    format!("{n}#{count}")
                };
                let old = node["id"].clone();
                node["id"] = json!(name);
                if let Some(p) = self.objects.get(&parent) {
                    for node in &mut p.borrow_mut().nodes {
                        if node["id"] == old {
                            node["id"] = json!(name);
                        }
                    }
                }
            }
        }
        if !Environment::set(scope, n.to_string(), v) {
            return Err(self.error("E_BINDING", "导入的名字不可赋值", l));
        }
        Ok(())
    }
    fn truth(&mut self, e: &Expr, s: &Scope) -> Result<bool> {
        match self.eval(e, s)? {
            V::Bool(b) => Ok(b),
            _ => Err(self.error("E_TYPE", "条件必须为布尔值", e.loc)),
        }
    }
    pub fn eval(&mut self, e: &Expr, s: &Scope) -> Result<V> {
        // Debug builds also need to honor the 64-call limit on small native stacks.
        // Drop the larger expression dispatch frame before entering user code.
        match &e.kind {
            ExprKind::Call(parts, args) => {
                let (pos, named) = self.eval_call_arguments(args, s, e.loc)?;
                self.call(parts, pos, named, s, e.loc)
            }
            ExprKind::Binary(op, a, b) => self.eval_binary(op, a, b, s, e.loc),
            _ => self.eval_other(e, s),
        }
    }
    #[inline(never)]
    fn eval_binary(&mut self, op: &str, a: &Expr, b: &Expr, s: &Scope, l: Loc) -> Result<V> {
        let left = self.eval(a, s)?;
        if op == "and" && matches!(left, V::Bool(false)) {
            Ok(V::Bool(false))
        } else if op == "or" && matches!(left, V::Bool(true)) {
            Ok(V::Bool(true))
        } else {
            let right = self.eval(b, s)?;
            self.binary(op, left, right, l)
        }
    }
    #[inline(never)]
    fn eval_other(&mut self, e: &Expr, s: &Scope) -> Result<V> {
        let l = e.loc;
        Ok(match &e.kind {
            ExprKind::Number(n, u) => {
                if !n.is_finite() {
                    return Err(self.error("E_UNIT", "数值不是有限值", l));
                }
                if u == "px" && self.canvas.is_none() {
                    return Err(self.error("E_UNIT", "px 需要先定义画布的 layout_dpi", l));
                }
                if !matches!(
                    u.as_str(),
                    "" | "mm" | "cm" | "in" | "inch" | "pt" | "px" | "deg" | "rad"
                ) {
                    return Err(self.error("E_UNIT", format!("未知单位 {u}"), l));
                }
                if u == "rad" {
                    V::Number(n.to_degrees(), "deg".into())
                } else {
                    V::Number(*n, u.clone())
                }
            }
            ExprKind::Bool(b) => V::Bool(*b),
            ExprKind::String(text, raw, fmt) => V::Text(
                if *fmt {
                    self.interpolate(text, s, l)?
                } else {
                    text.clone()
                },
                *raw,
            ),
            ExprKind::List(es) => {
                V::List(es.iter().map(|e| self.eval(e, s)).collect::<Result<_>>()?)
            }
            ExprKind::Dict(es) => {
                let mut d = indexmap::IndexMap::new();
                for (key, value) in es {
                    let k = self.eval(key, s)?;
                    let k = self.key(&k, key.loc)?;
                    d.insert(k, self.eval(value, s)?);
                }
                V::Dict(d)
            }
            ExprKind::Ref(parts) => {
                let mut v = if let Some(v) = Environment::get(s, &parts[0]) {
                    v
                } else if parts[0] == "self" {
                    crate::geometry_query::root(None)
                } else if parts[0] == "null" {
                    V::Null
                } else if let Some(value) = predefined_value(&parts[0]) {
                    V::text(value)
                } else {
                    return Err(self.error("E_NAME", format!("未知名称 {}", parts[0]), l));
                };
                for p in &parts[1..] {
                    v = self.member(v, p, l)?;
                }
                v
            }
            ExprKind::Index(a, b) => {
                let a = self.eval(a, s)?;
                let b = self.eval(b, s)?;
                match a {
                    V::Measured(q, _) | V::Geometry(q) => {
                        self.geometry_step(&q, crate::geometry_query::Step::Index(b), l)?
                    }
                    V::List(vs) => {
                        let i = self.scalar(&b, l)?;
                        if i.fract() != 0. || i < 0. || i as usize >= vs.len() {
                            return Err(self.error("E_INDEX", "索引超出范围", l));
                        }
                        vs[i as usize].clone()
                    }
                    V::Dict(m) => {
                        let key = self.key(&b, l)?;
                        m.get(&key).cloned().ok_or_else(|| {
                            self.error("E_INDEX", format!("不存在数据列或字典键 {key}"), l)
                        })?
                    }
                    V::Map(m) => m.get(&b.as_str()).cloned().ok_or_else(|| {
                        self.error("E_DATA", format!("不存在数据列 {}", b.as_str()), l)
                    })?,
                    _ => return Err(self.error("E_INDEX", "对象不支持索引", l)),
                }
            }
            ExprKind::Unary(op, a) => {
                let v = self.eval(a, s)?;
                match (op.as_str(), v.value().clone()) {
                    ("-", V::Number(n, u)) => V::Number(-n, u),
                    ("not", V::Bool(b)) => V::Bool(!b),
                    _ => return Err(self.error("E_TYPE", "无效的一元运算", l)),
                }
            }
            ExprKind::Member(receiver, name) => {
                let receiver = self.eval(receiver, s)?;
                self.member(receiver, name, l)?
            }
            ExprKind::Method(receiver, name, args) => {
                let receiver = self.eval(receiver, s)?;
                let (pos, named) = self.eval_arguments(args, s, l)?;
                self.value_method(receiver, name, pos, named, l)?
            }
            ExprKind::Call(..) | ExprKind::Binary(..) => unreachable!(),
        })
    }
    // Drop argument provenance temporaries before entering a user function.
    // This keeps the 64-call budget safe on ordinary native/WASM stacks.
    #[inline(never)]
    fn eval_call_arguments(
        &mut self,
        args: &[(Option<String>, Expr)],
        s: &Scope,
        l: Loc,
    ) -> Result<(Vec<V>, Args)> {
        let mut pos = vec![];
        let mut named = Args::new();
        let mut origins = Args::new();
        let mut saw_named = false;
        for (n, e) in args {
            let v = self.eval(e, s)?;
            let mut loc = e.loc;
            if let ExprKind::String(_, raw, formatted) = &e.kind {
                let prefix = 1 + usize::from(*raw) + usize::from(*formatted);
                loc.column += prefix;
                loc.offset += prefix;
            }
            origins.insert(
                n.clone().unwrap_or_else(|| format!("_{}", pos.len())),
                V::from_json(&json!({"file":self.file,"loc":loc})),
            );
            if let Some(n) = n {
                if named.insert(n.clone(), v).is_some() {
                    return Err(self.error("E_ARG", format!("重复参数 {n}"), l));
                }
                saw_named = true;
            } else {
                if saw_named {
                    return Err(self.error("E_ARG", "位置参数必须位于命名参数之前", l));
                }
                pos.push(v);
            }
        }
        if !origins.is_empty() {
            named.insert("__arg_locations".into(), V::Map(origins));
        }
        named.insert(
            "__call_origin".into(),
            V::from_json(&json!({"file":self.file,"loc":l})),
        );
        Ok((pos, named))
    }
    fn binary(&self, op: &str, a: V, b: V, l: Loc) -> Result<V> {
        let a = a.value().clone();
        let b = b.value().clone();
        if op == "+" {
            if matches!(&a, V::Text(..)) && matches!(&b, V::Text(..)) {
                return Ok(V::text(a.as_str() + &b.as_str()));
            }
        }
        if matches!(op, "and" | "or") {
            if let (V::Bool(a), V::Bool(b)) = (a, b) {
                return Ok(V::Bool(if op == "and" { a && b } else { a || b }));
            }
            return Err(self.error("E_TYPE", "逻辑运算需要布尔值", l));
        }
        if op == "in" || op == "not in" {
            let V::Dict(d) = &b else {
                return Err(self.error("E_TYPE", "in 需要字典", l));
            };
            let contains = d.contains_key(&self.key(&a, l)?);
            return Ok(V::Bool(if op == "in" { contains } else { !contains }));
        }
        if matches!(op, "==" | "!=") && !matches!((&a, &b), (V::Number(..), V::Number(..))) {
            let eq = match (&a, &b) {
                (V::Text(a, _), V::Text(b, _)) => a == b,
                _ => a.json() == b.json(),
            };
            return Ok(V::Bool(if op == "==" { eq } else { !eq }));
        }
        let (V::Number(mut x, mut u), V::Number(mut y, v)) = (a, b) else {
            return Err(self.error("E_TYPE", "数值运算需要数字", l));
        };
        let dim = |u: &str| {
            if u.is_empty() {
                0
            } else if matches!(u, "deg" | "rad") {
                2
            } else {
                1
            }
        };
        let (da, db) = (dim(&u), dim(&v));
        if da == 1 {
            x = value_length(&V::Number(x, u.clone()), "mm", self.dpi).unwrap();
            u = "mm".into();
        } else if u == "rad" {
            x = x.to_degrees();
            u = "deg".into();
        }
        if db == 1 {
            y = value_length(&V::Number(y, v.clone()), "mm", self.dpi).unwrap();
        } else if v == "rad" {
            y = y.to_degrees();
        }
        if matches!(op, "+" | "-" | "==" | "!=" | "<" | ">" | "<=" | ">=") && da != db {
            return Err(self.error("E_UNIT", "运算维度不匹配", l));
        }
        let n = match op {
            "+" => x + y,
            "-" => x - y,
            "*" => {
                if da != 0 && db != 0 {
                    return Err(self.error("E_UNIT", "不支持复合单位", l));
                }
                if da == 0 {
                    u = if db == 1 { "mm".into() } else { v };
                }
                x * y
            }
            "/" => {
                if y == 0. {
                    return Err(self.error("E_UNIT", "除数不能为零", l));
                }
                if da == db {
                    u.clear();
                } else if db != 0 {
                    return Err(self.error("E_UNIT", "不支持倒数单位", l));
                }
                x / y
            }
            "==" => return Ok(V::Bool(x == y)),
            "!=" => return Ok(V::Bool(x != y)),
            "<" => return Ok(V::Bool(x < y)),
            ">" => return Ok(V::Bool(x > y)),
            "<=" => return Ok(V::Bool(x <= y)),
            ">=" => return Ok(V::Bool(x >= y)),
            _ => return Err(self.error("E_TYPE", "未知运算符", l)),
        };
        if !n.is_finite() {
            return Err(self.error("E_UNIT", "运算结果不是有限值", l));
        }
        Ok(V::Number(n, u))
    }
    pub(crate) fn member(&self, v: V, p: &str, l: Loc) -> Result<V> {
        if let V::Cmap(cm) = &v {
            return match p {
                "name" => Ok(V::text(cm.name())),
                "category" => Ok(V::text(cm.category())),
                _ => Err(self.error("E_NAME", format!("未知 cmap 属性 {p}"), l)),
            };
        }
        if let V::Geometry(q) = &v {
            return self.geometry_step(q, crate::geometry_query::Step::Member(p.into()), l);
        }
        if let Some(o) = v.object() {
            let o = o.borrow();
            if matches!(p, "bounds" | "path" | "ink" | "plot_area" | "axes") {
                if !matches!(o.kind.as_str(), "instance" | "canvas") {
                    return Err(self.error("E_TYPE", "几何查询需要已放置实例", l));
                }
                let q = crate::geometry_query::Query {
                    instance: Some(o.id),
                    steps: vec![],
                };
                return self.geometry_step(&q, crate::geometry_query::Step::Member(p.into()), l);
            }
        }
        let o = v
            .object()
            .ok_or_else(|| self.error("E_NAME", "对象没有属性", l))?;
        let o = o.borrow();
        if yes(&o.args, "__consumed", false) {
            return Err(self.error("E_FUSE", "已融合的实例不能再次引用", l));
        }
        if matches!(p, "width" | "height") {
            if !matches!(o.kind.as_str(), "instance" | "canvas") {
                return Err(self.error("E_TYPE", "素材定义没有可读取的实例尺寸", l));
            }
            return Ok(V::mm(if p == "width" { o.width } else { o.height }));
        }
        if matches!(p, "start" | "end") {
            let n = o
                .node
                .as_ref()
                .ok_or_else(|| self.error("E_LAYOUT", "端点需要线段或箭头实例", l))?;
            let q = n["endpoints"][if p == "start" { 0 } else { 1 }]
                .as_array()
                .ok_or_else(|| self.error("E_LAYOUT", "端点需要线段或箭头实例", l))?;
            let xy = crate::geometry::node_transform(n)
                * kurbo::Point::new(q[0].as_f64().unwrap(), q[1].as_f64().unwrap());
            return Ok(V::Anchor {
                owner: o.parent,
                x: xy.x,
                y: xy.y,
                reference: Some(AnchorReference {
                    instance: o.id,
                    name: p.into(),
                }),
            });
        }
        if let Some(f) = anchor(p) {
            if !matches!(o.kind.as_str(), "instance" | "canvas" | "group") {
                return Err(self.error("E_TYPE", "素材定义没有页面锚点", l));
            }
            if o.kind == "group" && p != "top_left" {
                return Err(self.error("E_LAYOUT", "组内定位只能以组的 top_left 原点为目标", l));
            }
            let xy = if let Some(n) = &o.node {
                node_anchor(n, p)
            } else {
                [f[0] * o.width, f[1] * o.height]
            };
            return Ok(V::Anchor {
                owner: if o.kind == "instance" { o.parent } else { o.id },
                x: xy[0],
                y: xy[1],
                reference: (o.kind == "instance").then(|| AnchorReference {
                    instance: o.id,
                    name: p.into(),
                }),
            });
        }
        if p.starts_with("plot_") {
            if let Some(n) = &o.node {
                let b = &n["plotBounds"];
                if let Some(f) = anchor(&p[5..]) {
                    if !b.is_object() {
                        return Err(self.error("E_LAYOUT", "plot 锚点需要原生图表实例", l));
                    }
                    let xy = crate::plot::local_to_parent(
                        n,
                        [
                            jnum(b, "x", 0.) + f[0] * jnum(b, "width", 0.),
                            jnum(b, "y", 0.) + f[1] * jnum(b, "height", 0.),
                        ],
                    );
                    return Ok(V::Anchor {
                        owner: o.parent,
                        x: xy[0],
                        y: xy[1],
                        reference: Some(AnchorReference {
                            instance: o.id,
                            name: p.into(),
                        }),
                    });
                }
            }
        }
        Err(self.error("E_NAME", format!("未知属性 {p}"), l))
    }
    fn call(&mut self, parts: &[String], pos: Vec<V>, a: Args, s: &Scope, l: Loc) -> Result<V> {
        if parts.len() == 1 {
            if let Some(V::Function {
                params,
                body,
                scope,
                file,
            }) = Environment::get(s, &parts[0])
            {
                if self.calls >= 64 {
                    return Err(self.error("E_LIMIT", "函数调用深度超过 64", l));
                }
                let local = self.bind_call_arguments(&params, pos, a, &scope, l)?;
                let old = std::mem::replace(&mut self.file, file);
                self.calls += 1;
                let result = self.run(&body, &local);
                self.calls -= 1;
                self.file = old;
                return Ok(match result? {
                    Flow::Return(v) => v,
                    _ => V::Null,
                });
            }
        }
        self.call_builtin(parts, pos, a, s, l)
    }
    #[inline(never)]
    fn bind_call_arguments(
        &mut self,
        params: &[(String, Option<Expr>)],
        pos: Vec<V>,
        mut a: Args,
        scope: &Scope,
        l: Loc,
    ) -> Result<Scope> {
        a.remove("__arg_locations");
        a.remove("__call_origin");
        if pos.len() > params.len() {
            return Err(self.error("E_ARG", "位置参数过多", l));
        }
        let local = Environment::child(scope);
        for (i, (n, d)) in params.iter().enumerate() {
            if i < pos.len() && a.contains_key(n) {
                return Err(self.error("E_ARG", format!("重复参数 {n}"), l));
            }
            let v = if let Some(v) = pos.get(i) {
                v.clone()
            } else if let Some(v) = a.remove(n) {
                v
            } else if let Some(d) = d {
                self.eval(d, scope)?
            } else {
                return Err(self.error("E_ARG", format!("缺少参数 {n}"), l));
            };
            local.borrow_mut().values.insert(n.clone(), v);
        }
        if !a.is_empty() {
            return Err(self.error("E_ARG", "函数收到未知参数", l));
        }
        Ok(local)
    }
    #[inline(never)]
    fn call_builtin(
        &mut self,
        parts: &[String],
        pos: Vec<V>,
        mut a: Args,
        s: &Scope,
        l: Loc,
    ) -> Result<V> {
        if parts.len() == 1 && parts[0] == "ray" {
            a.retain(|k, _| !k.starts_with("__"));
            if !pos.is_empty()
                || a.len() != 2
                || !a.contains_key("origin")
                || !a.contains_key("direction")
            {
                return Err(self.error("E_ARG", "ray 需要 origin 与 direction", l));
            }
            let direction = a["direction"].list();
            if direction.len() != 2
                || self
                    .scalar(&direction[0], l)?
                    .hypot(self.scalar(&direction[1], l)?)
                    == 0.
            {
                return Err(self.error("E_ARG", "direction 需要非零无单位向量", l));
            }
            return Ok(V::Map(a));
        }
        if parts.len() > 1 {
            if let Some(receiver @ (V::Dict(_) | V::Cmap(_))) = Environment::get(s, &parts[0]) {
                if parts.len() != 2 {
                    return Err(self.error("E_CALL", "字典方法通过索引选择嵌套值", l));
                }
                return self.value_method(receiver, parts.last().unwrap(), pos, a, l);
            }
            if parts.len() > 2 {
                let mut receiver = Environment::get(s, &parts[0])
                    .or_else(|| (parts[0] == "self").then(|| crate::geometry_query::root(None)))
                    .ok_or_else(|| self.error("E_NAME", "未知查询对象", l))?;
                for member in &parts[1..parts.len() - 1] {
                    receiver = self.member(receiver, member, l)?;
                }
                return self.geometry_method(receiver, parts.last().unwrap(), pos, a, l);
            }
            if let Some(receiver @ V::Geometry(_)) = Environment::get(s, &parts[0])
                .or_else(|| (parts[0] == "self").then(|| crate::geometry_query::root(None)))
            {
                return self.geometry_method(receiver, parts.last().unwrap(), pos, a, l);
            }
            let owner = Environment::get(s, &parts[0])
                .and_then(|v| v.object())
                .ok_or_else(|| {
                    self.error("E_CALL", format!("不允许调用 {}", parts.join(".")), l)
                })?;
            let method = parts.last().unwrap().as_str();
            match method {
                "add" => return self.add(owner, pos, a, l),
                "fuse" => return self.fuse(owner, pos, a, l),
                "data" | "axis" => return self.data_anchor(owner, method, pos, a, l),
                _ => {
                    let kind = owner.borrow().kind.clone();
                    if kind != "plot" {
                        return Err(self.error("E_NAME", format!("对象没有方法 {method}"), l));
                    }
                    if owner.borrow().sealed {
                        return Err(self.error("E_PLOT", "已经放置的图表不能再修改", l));
                    }
                    self.validate_args(&format!("plot.{method}"), &mut a, &pos, l)?;
                    crate::plot::validate_layer(self, &owner.borrow(), method, &a, l)?;
                    let mut handle = a.clone();
                    handle.insert("_type".into(), V::text(method));
                    handle.insert("_index".into(), V::num(owner.borrow().layers.len() as f64));
                    handle.insert("_owner".into(), V::Object(owner.clone()));
                    let item = self.object("plotLayer", handle, l);
                    if method == "add_axis" {
                        owner.borrow_mut().layers.push(("add_axis".into(), a));
                    } else {
                        let mut a = a;
                        for (i, v) in pos.into_iter().enumerate() {
                            a.insert(format!("_{i}"), v);
                        }
                        owner.borrow_mut().layers.push((method.into(), a));
                    }
                    return Ok(item);
                }
            }
        }
        let name = parts[0].as_str();
        if let Some(result) = self.collection_builtin(name, &pos, a.clone(), l) {
            return result;
        }
        if name == "arrow" {
            return Err(self.error(
                "E_API_MIGRATION",
                "arrow 已移除；使用 line(..., end_head=head(...))",
                l,
            ));
        }
        if name == "outline" {
            return Err(self.error(
                "E_API_MIGRATION",
                "outline 已移除，使用 border_* 或 line_*",
                l,
            ));
        }
        match name {
            "range" => {
                let vs = pos
                    .iter()
                    .map(|v| self.scalar(v, l))
                    .collect::<Result<Vec<_>>>()?;
                let (start, end, step) = match vs.as_slice() {
                    [end] => (0., *end, 1.),
                    [start, end] => (*start, *end, 1.),
                    [start, end, step] => (*start, *end, *step),
                    _ => return Err(self.error("E_ARG", "range 需要 1–3 个参数", l)),
                };
                if step == 0. || vs.iter().any(|x| x.fract() != 0.) {
                    return Err(self.error("E_ARG", "range 需要整数和非零步长", l));
                }
                let count = ((end - start) / step).ceil().max(0.) as usize;
                if count > 10_000 {
                    return Err(self.error("E_LIMIT", "range 元素超过 10,000", l));
                }
                return Ok(V::List(
                    (0..count)
                        .map(|i| V::num(start + i as f64 * step))
                        .collect(),
                ));
            }
            "len" => {
                if pos.len() != 1 {
                    return Err(self.error("E_ARG", "len 需要一个参数", l));
                }
                return Ok(V::num(match &pos[0] {
                    V::List(a) => a.len(),
                    V::Dict(a) => a.len(),
                    V::Text(s, _) => s.chars().count(),
                    V::Geometry(q) => self.geometry_items(q, l)?.len(),
                    _ => return Err(self.error("E_TYPE", "len 需要列表或字符串", l)),
                } as f64));
            }
            "append" => {
                if pos.len() != 2 {
                    return Err(self.error("E_ARG", "append 需要两个参数", l));
                }
                let V::List(mut v) = pos[0].clone() else {
                    return Err(self.error("E_TYPE", "append 需要列表", l));
                };
                v.push(pos[1].clone());
                return Ok(V::List(v));
            }
            "str" => {
                if pos.len() != 1 {
                    return Err(self.error("E_ARG", "str 需要一个参数", l));
                }
                return match pos[0].value() {
                    V::Number(n, u) => Ok(V::text(if u.is_empty() {
                        n.to_string()
                    } else if matches!(u.as_str(), "deg" | "rad") {
                        format!("{} deg", if u == "rad" { n.to_degrees() } else { *n })
                    } else {
                        format!("{} mm", self.len(&pos[0], l)?)
                    })),
                    V::Text(..) | V::Bool(..) => Ok(V::text(pos[0].as_str())),
                    _ => Err(self.error("E_TYPE", "str 不支持该类型", l)),
                };
            }
            "abs" => {
                if let Some(V::Number(n, u)) = pos.first().map(V::value) {
                    return Ok(V::Number(n.abs(), u.clone()));
                }
                return Err(self.error("E_TYPE", "abs 需要数字", l));
            }
            "min" | "max" => {
                let mut best = pos
                    .first()
                    .cloned()
                    .ok_or_else(|| self.error("E_ARG", "需要数值", l))?;
                for v in &pos[1..] {
                    if matches!(
                        self.binary(
                            if name == "min" { "<" } else { ">" },
                            v.clone(),
                            best.clone(),
                            l
                        )?,
                        V::Bool(true)
                    ) {
                        best = v.clone();
                    }
                }
                return Ok(best);
            }
            _ => {}
        }
        self.validate_args(name, &mut a, &pos, l)?;
        let positional = API["api"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["name"] == name)
            .and_then(|v| v["positional"].as_array())
            .cloned()
            .unwrap_or_default();
        for (i, v) in pos.into_iter().enumerate() {
            let key = positional.get(i).and_then(Json::as_str).unwrap_or("");
            if a.insert(key.into(), v).is_some() {
                return Err(self.error("E_ARG", format!("重复参数 {key}"), l));
            }
        }
        self.validate_definition(name, &a, l)?;
        crate::plot::validate_definition(self, name, &a, l)?;
        if matches!(name, "rgb" | "hsv" | "oklch") {
            let names = match name {
                "rgb" => ["r", "g", "b"],
                "hsv" => ["h", "s", "v"],
                _ => ["l", "c", "h"],
            };
            let mut channels = [0.; 3];
            for (i, key) in names.iter().enumerate() {
                let value = &a[*key];
                channels[i] = if (name == "hsv" && i == 0) || (name == "oklch" && i == 2) {
                    match value.value() {
                        V::Number(n, u) if u.is_empty() || u == "deg" => *n,
                        V::Number(n, u) if u == "rad" => n.to_degrees(),
                        _ => return Err(self.error("E_UNIT", "色相需要角度", l)),
                    }
                } else {
                    self.scalar(value, l)?
                };
            }
            let alpha = a
                .get("alpha")
                .map(|v| self.scalar(v, l))
                .transpose()?
                .unwrap_or(1.);
            return crate::color::Color::new(name, channels, alpha)
                .map(|c| V::Color(Rc::new(c)))
                .map_err(|m| self.error("E_COLOR", m, l));
        }
        if name == "canvas" {
            if self.canvas.is_some() {
                return Err(self.error("E_CANVAS", "只能创建一个画布", l));
            }
            self.unit = string(&a, "unit", "mm");
            self.dpi = num(&a, "layout_dpi", 96.);
            if !matches!(
                self.unit.as_str(),
                "mm" | "cm" | "in" | "inch" | "pt" | "px"
            ) || self.dpi <= 0.
            {
                return Err(self.error("E_UNIT", "无效画布单位或 DPI", l));
            }
            if let Some(v) = a.get("stylesheet") {
                let files = if let V::List(v) = v {
                    v.clone()
                } else {
                    vec![v.clone()]
                };
                for f in files {
                    let path = resolve(&self.file, &f.as_str());
                    let bytes = self.host.read(&path, &self.file, l)?;
                    self.stylesheet(
                        &String::from_utf8_lossy(&bytes),
                        &path,
                        Loc {
                            line: 1,
                            column: 1,
                            offset: 0,
                        },
                    )?;
                }
            }
            let size = pair(&a, "size", [0., 0.], &self.unit, self.dpi);
            if size.iter().any(|v| *v <= 0.) {
                return Err(self.error("E_LAYOUT", "画布尺寸必须为正", l));
            }
            let v = self.object(name, a, l);
            let o = v.object().unwrap();
            o.borrow_mut().width = size[0];
            o.borrow_mut().height = size[1];
            self.canvas = Some(o);
            return Ok(v);
        }
        if matches!(name, "table" | "array") {
            let path = resolve(&self.file, &string(&a, "src", ""));
            let bytes = self
                .host
                .read(&path, &self.file, l)
                .map_err(|e| self.error("E_DATA", e.message, l))?;
            return crate::data::load(name, &path, &bytes, &self.file, l);
        }
        if name == "image_fill" {
            let path = resolve(&self.file, &string(&a, "src", ""));
            let asset = self.load_image(&path, &self.file.clone(), l)?;
            for key in [
                "data",
                "mime",
                "width",
                "height",
                "rasterKey",
                "sourceJpegHash",
            ] {
                a.insert(key.into(), V::from_json(&asset[key]));
            }
        }
        if name == "hatch" {
            let spacing = length(&a, "spacing", 2., &self.unit, self.dpi);
            let width = length(&a, "line_width", 0.5 * 25.4 / 72., "pt", self.dpi);
            if spacing <= 0. || width <= 0. {
                return Err(self.error("E_PAINT", "纹理间距和线宽必须为正", l));
            }
            a.insert("spacing".into(), V::mm(spacing));
            a.insert("line_width".into(), V::mm(width));
        }
        Ok(self.object(name, a, l))
    }
    pub fn validate_args(&self, name: &str, a: &mut Args, pos: &[V], l: Loc) -> Result<()> {
        let api = API["api"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["name"] == name)
            .ok_or_else(|| self.error("E_CALL", format!("未知构造器 {name}"), l))?;
        let allowed = api["parameters"].as_array().unwrap();
        for (k, v) in a.iter_mut() {
            if k.starts_with("__") {
                continue;
            }
            let rename = match k.as_str() {
                "font" => Some("font_family"),
                "stroke" => Some("border_color"),
                "stroke_width" => Some("border_width"),
                "weight" => Some("font_weight"),
                "italic" => Some("font_style"),
                "width" | "height" => Some("size"),
                "classes" => Some("class"),
                "outline" => Some("border_* 或 line_*"),
                _ => None,
            };
            if let Some(new) = rename {
                return Err(self.error("E_API_MIGRATION", format!("{k} 已改为 {new}"), l));
            }
            let param = allowed
                .iter()
                .find(|p| p["name"] == k.as_str())
                .ok_or_else(|| self.error("E_ARG", format!("{name} 不支持参数 {k}"), l))?;
            if param["unit"] == "pt" {
                fn cv(v: &mut V) {
                    match v {
                        V::Number(n, u) if u.is_empty() => {
                            *n *= PT;
                            *u = "mm".into();
                        }
                        V::List(vs) => vs.iter_mut().for_each(cv),
                        _ => {}
                    }
                }
                cv(v);
            }
        }
        let count = if name == "fuse" {
            2
        } else {
            api["positional"].as_array().map(Vec::len).unwrap_or(0)
        };
        if pos.len() > count && !(name == "plot.colorbar" && pos.len() == 1) {
            return Err(self.error("E_ARG", format!("{name} 位置参数过多"), l));
        }
        Ok(())
    }
    fn interpolate(&mut self, text: &str, s: &Scope, l: Loc) -> Result<String> {
        let mut out = String::new();
        let mut i = 0;
        while i < text.len() {
            if text[i..].starts_with("{{") || text[i..].starts_with("}}") {
                out.push(text.as_bytes()[i] as char);
                i += 2;
                continue;
            }
            if text.as_bytes()[i] == b'}' {
                return Err(self.error("E_FORMAT", "字面花括号使用 }}", l));
            }
            if text.as_bytes()[i] != b'{' {
                let c = text[i..].chars().next().unwrap();
                out.push(c);
                i += c.len_utf8();
                continue;
            }
            let start = i + 1;
            i = start;
            let mut depth = 0usize;
            let mut end_expr = None;
            let mut conversion = None;
            let mut fmt = "";
            while i < text.len() {
                if let Some(end) = parser::string_end(text, i) {
                    i = end;
                    continue;
                }
                let c = text.as_bytes()[i];
                if depth == 0
                    && (c == b':' || (c == b'!' && text.as_bytes().get(i + 1) != Some(&b'=')))
                {
                    end_expr = Some(i);
                    if c == b'!' {
                        i += 1;
                        conversion = text.as_bytes().get(i).map(|c| *c as char);
                        if !matches!(conversion, Some('s' | 'r')) {
                            return Err(self.error("E_FORMAT", "转换仅支持 !s 或 !r", l));
                        }
                        i += 1;
                    }
                    if text.as_bytes().get(i) == Some(&b':') {
                        i += 1;
                        let from = i;
                        while i < text.len() && text.as_bytes()[i] != b'}' {
                            i += 1;
                        }
                        fmt = &text[from..i];
                    }
                    break;
                }
                if c == b'}' && depth == 0 {
                    break;
                }
                if matches!(c, b'(' | b'[' | b'{') {
                    depth += 1;
                }
                if matches!(c, b')' | b']' | b'}') {
                    depth = depth.saturating_sub(1);
                }
                i += 1;
            }
            if text.as_bytes().get(i) != Some(&b'}') {
                return Err(self.error("E_FORMAT", "插值缺少闭合花括号", l));
            }
            let expr = &text[start..end_expr.unwrap_or(i)];
            let expr = parser::expression(expr, &self.file)
                .map_err(|e| self.error("E_FORMAT", e.message, l))?;
            let v = self
                .eval(&expr, s)
                .map_err(|e| self.error("E_FORMAT", e.message, l))?;
            out.push_str(
                &crate::formatting::value(&v, fmt, conversion, self.dpi)
                    .map_err(|m| self.error("E_FORMAT", m, l))?,
            );
            i += 1;
        }
        Ok(out)
    }
}
pub fn compile_file(file: &str) -> Result<Scene> {
    let p = std::fs::canonicalize(file).map_err(|e| {
        Diagnostic::new(
            "E_ASSET",
            e.to_string(),
            file,
            Loc {
                line: 1,
                column: 1,
                offset: 0,
            },
        )
    })?;
    let f = p.to_string_lossy();
    let source = std::fs::read_to_string(&p)
        .map_err(|e| Diagnostic::new("E_ASSET", e.to_string(), file, Loc::default()))?;
    Engine::new(Host {
        native: true,
        ..Host::default()
    })
    .compile(&source, &f)
}
pub fn compile_source(source: &str, file: &str, host: Host) -> Result<Scene> {
    Engine::new(host).compile(source, file)
}

#[cfg(test)]
mod preview_image_tests {
    use super::*;

    #[test]
    fn preview_image_snapshot_is_reused_only_within_one_compilation() {
        struct Reset;
        impl Drop for Reset {
            fn drop(&mut self) {
                crate::asset_cache::preview_mode(false);
            }
        }
        crate::asset_cache::preview_mode(true);
        let _reset = Reset;
        let png = |color| {
            let image = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
                2,
                2,
                image::Rgb(color),
            ));
            let mut out = std::io::Cursor::new(Vec::new());
            image.write_to(&mut out, image::ImageFormat::Png).unwrap();
            out.into_inner()
        };
        let dependencies = Rc::new(RefCell::new(BTreeSet::new()));
        let mut host = Host {
            dependencies: Some(dependencies.clone()),
            ..Host::default()
        };
        host.files.insert("/image.png".into(), png([255, 0, 0]));
        let mut engine = Engine::new(host);
        let first = engine
            .load_image("/image.png", "/main.lay", Loc::default())
            .unwrap();
        assert!(dependencies.borrow().contains("/image.png"));
        engine.host.files.clear();
        let mut reused = engine
            .load_image("/image.png", "/main.lay", Loc::default())
            .unwrap();
        assert_eq!(first, reused);
        reused["width"] = json!(999);
        assert_eq!(
            engine
                .load_image("/image.png", "/main.lay", Loc::default())
                .unwrap()["width"],
            2
        );

        let source = "page=canvas(size=(20,20))\npage.add(image(src=\"image.png\"),size=(10,10))";
        // A new compile must observe deletion, and then recover with new pixels.
        let error = engine.compile(source, "/main.lay").unwrap_err();
        assert_eq!(error.code, "E_ASSET");
        engine
            .host
            .files
            .insert("/image.png".into(), png([0, 0, 255]));
        let scene = compile_source(source, "/main.lay", engine.host.clone()).unwrap();
        assert_ne!(scene.nodes[0]["rasterKey"], first["rasterKey"]);
    }
}
