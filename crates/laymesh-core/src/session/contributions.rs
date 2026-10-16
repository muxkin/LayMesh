//! Transaction-local object contributions. Cached graphs own immutable copies;
//! replay creates new mutable handles and rebases internal object references.
use super::*;
use crate::geometry_query::{Query, Step};
use crate::model::{Args, Object, V};

#[derive(Clone)]
pub(crate) struct Context {
    settings: String,
    unnamed: usize,
    names: BTreeMap<String, usize>,
}
fn context_args(engine: &Engine) -> serde_json::Value {
    fn strip(v: &mut serde_json::Value) {
        match v {
            serde_json::Value::Object(m) => {
                for key in ["__call_origin", "__arg_locations", "__placement_origin"] {
                    m.remove(key);
                }
                for value in m.values_mut() {
                    strip(value);
                }
            }
            serde_json::Value::Array(a) => {
                for v in a {
                    strip(v)
                }
            }
            _ => {}
        }
    }
    let mut value = engine
        .canvas
        .as_ref()
        .map(|c| crate::model::args_json(&engine.style_parent_inputs(&c.borrow().args)))
        .unwrap_or_default();
    strip(&mut value);
    value
}
impl Context {
    pub fn capture(engine: &Engine) -> Self {
        Self {
            settings: format!(
                "{:?}|{:?}|{}|{}|{:?}",
                engine.css_viewport,
                engine.css_entry_file,
                engine.unit,
                engine.dpi,
                context_args(engine)
            ),
            unnamed: engine.unnamed_instances,
            names: engine.instance_names.clone(),
        }
    }
    pub fn matches(&self, engine: &Engine) -> bool {
        self.settings == Self::capture(engine).settings
    }
}
#[derive(Clone)]
pub(crate) struct Contributions {
    objects: Vec<Rc<RefCell<Object>>>,
    origins: Vec<(u64, u64)>,
    start: usize,
    unnamed: usize,
    names: BTreeMap<String, usize>,
    pub context: Context,
    pub bytes: usize,
}
struct Copy<'a> {
    map: HashMap<usize, Rc<RefCell<Object>>>,
    start: usize,
    target: usize,
    syntax: &'a SyntaxCache,
    freeze: bool,
    names: BTreeMap<String, String>,
    current: RefCell<HashMap<usize, bool>>,
}
impl Copy<'_> {
    fn id(&self, id: usize) -> Option<usize> {
        if id == 0 {
            Some(0)
        } else if id > self.start {
            Some(self.target + id - self.start)
        } else {
            None
        }
    }
    fn query(&self, q: &Query) -> Option<Rc<Query>> {
        Some(Rc::new(Query {
            instance: q.instance.map(|i| self.id(i)).transpose_option()?,
            steps: q
                .steps
                .iter()
                .map(|step| {
                    Some(match step {
                        Step::Member(s) => Step::Member(s.clone()),
                        Step::Index(v) => Step::Index(self.value(v)?),
                        Step::Call(n, p, a) => Step::Call(
                            n.clone(),
                            p.iter().map(|v| self.value(v)).collect::<Option<_>>()?,
                            self.args(a)?,
                        ),
                    })
                })
                .collect::<Option<_>>()?,
        }))
    }
    fn args(&self, a: &Args) -> Option<Args> {
        a.iter()
            .map(|(k, v)| {
                Some((
                    k.clone(),
                    if matches!(k.as_str(), "__result_id" | "__left" | "__right") {
                        V::num(self.id(v.number()? as usize)? as f64)
                    } else {
                        self.value(v)?
                    },
                ))
            })
            .collect()
    }
    fn value(&self, v: &V) -> Option<V> {
        Some(match v {
            V::Object(o) => {
                let identity = Rc::as_ptr(o) as usize;
                if let Some(copy) = self.map.get(&identity) {
                    V::Object(copy.clone())
                } else if self.freeze {
                    super::values::freeze(v, self.syntax)?
                } else {
                    super::values::thaw(v, self.syntax)
                }
            }
            V::List(v) => V::List(v.iter().map(|v| self.value(v)).collect::<Option<_>>()?),
            V::Map(_) if super::values::immutable_data(v) => v.clone(),
            V::Map(v) => V::Map(self.args(v)?.into()),
            V::Dict(v) => V::Dict(
                v.iter()
                    .map(|(k, v)| Some((k.clone(), self.value(v)?)))
                    .collect::<Option<_>>()?,
            ),
            V::Anchor {
                owner,
                x,
                y,
                reference,
            } => V::Anchor {
                owner: self.id(*owner)?,
                x: *x,
                y: *y,
                reference: reference
                    .as_ref()
                    .map(|r| {
                        Some(crate::model::AnchorReference {
                            instance: self.id(r.instance)?,
                            name: r.name.clone(),
                        })
                    })
                    .transpose_option()?,
            },
            V::Geometry(q) => V::Geometry(self.query(q)?),
            V::Measured(q, v) => V::Measured(self.query(q)?, Box::new(self.value(v)?)),
            V::Function { .. } => return None,
            _ => v.clone(),
        })
    }
    fn shareable(&self, v: &V) -> bool {
        match v {
            V::Object(o) => {
                let key = Rc::as_ptr(o) as usize;
                if let Some(current) = self.current.borrow().get(&key) {
                    return *current;
                }
                let current = super::values::immutable_kind(&o.borrow().kind)
                    && super::values::current(v, self.syntax);
                self.current.borrow_mut().insert(key, current);
                current
            }
            V::List(a) => a.iter().all(|v| self.shareable(v)),
            V::Map(a) => a.values().all(|v| self.shareable(v)),
            V::Dict(a) => a.values().all(|v| self.shareable(v)),
            V::Function { .. } => false,
            V::Measured(q, v) => self.shareable(v) && self.shareable(&V::Geometry(q.clone())),
            V::Geometry(q) => q.steps.iter().all(|s| match s {
                Step::Member(_) => true,
                Step::Index(v) => self.shareable(v),
                Step::Call(_, p, a) => {
                    p.iter().all(|v| self.shareable(v)) && a.values().all(|v| self.shareable(v))
                }
            }),
            _ => true,
        }
    }
    fn fill(&self, objects: &[Rc<RefCell<Object>>]) -> Option<Vec<Rc<RefCell<Object>>>> {
        let mut result = vec![];
        for original in objects {
            let o = original.borrow();
            let copy = self.map.get(&(Rc::as_ptr(original) as usize))?.clone();
            let mut next = header(&o);
            next.id = self.id(o.id)?;
            next.parent = self.id(o.parent)?;
            // Unchanged allocation ranges keep their internal anchor IDs.
            // Immutable recipes may then share their payload until a write.
            let can_share = !self.freeze && self.start == self.target;
            let shareable = |a: &Args| a.values().all(|v| self.shareable(v));
            next.args = if can_share && shareable(&o.args) {
                o.args.clone()
            } else {
                self.args(&o.args)?.into()
            };
            next.layers = if can_share && o.layers.iter().all(|(_, a)| shareable(a)) {
                o.layers.clone()
            } else {
                o.layers
                    .iter()
                    .map(|(k, a)| Some((k.clone(), self.args(a)?)))
                    .collect::<Option<_>>()?
            };
            fn node(
                n: &Arc<serde_json::Value>,
                start: usize,
                target: usize,
                names: &BTreeMap<String, String>,
            ) -> Arc<serde_json::Value> {
                if start == target && names.is_empty() {
                    return n.clone();
                }
                fn name(
                    value: &serde_json::Value,
                    start: usize,
                    target: usize,
                    names: &BTreeMap<String, String>,
                ) -> Option<String> {
                    let name = value["id"].as_str()?;
                    names.get(name).cloned().or_else(|| {
                        name.strip_prefix("instance-")
                            .and_then(|s| s.parse::<usize>().ok())
                            .filter(|i| *i > start && start != target)
                            .map(|id| format!("instance-{}", target + id - start))
                    })
                }
                fn changed(
                    n: &serde_json::Value,
                    start: usize,
                    target: usize,
                    names: &BTreeMap<String, String>,
                ) -> bool {
                    name(n, start, target, names).is_some()
                        || n["children"]
                            .as_array()
                            .is_some_and(|a| a.iter().any(|n| changed(n, start, target, names)))
                }
                fn rebase(
                    n: &mut serde_json::Value,
                    start: usize,
                    target: usize,
                    names: &BTreeMap<String, String>,
                ) {
                    if let Some(value) = name(n, start, target, names) {
                        n["id"] = serde_json::json!(value);
                    }
                    if let Some(a) = n
                        .get_mut("children")
                        .and_then(serde_json::Value::as_array_mut)
                    {
                        for n in a {
                            rebase(n, start, target, names);
                        }
                    }
                }
                if !changed(n, start, target, names) {
                    return n.clone();
                }
                let mut n = n.as_ref().clone();
                rebase(&mut n, start, target, names);
                Arc::new(n)
            }
            next.nodes = o
                .nodes
                .iter()
                .map(|n| node(n, self.start, self.target, &self.names))
                .collect();
            next.node = o
                .node
                .as_ref()
                .map(|n| node(n, self.start, self.target, &self.names));
            *copy.borrow_mut() = next;
            result.push(copy);
        }
        Some(result)
    }
}
fn skeleton(objects: &[Rc<RefCell<Object>>]) -> HashMap<usize, Rc<RefCell<Object>>> {
    objects
        .iter()
        .map(|o| {
            let copy = header(&o.borrow());
            (Rc::as_ptr(o) as usize, Rc::new(RefCell::new(copy)))
        })
        .collect()
}
fn header(o: &Object) -> Object {
    Object {
        syntax_site: o.syntax_site,
        cache_identity: o.cache_identity,
        placed_key: None,
        origin_revision: o.origin_revision,
        origin_dependencies: o.origin_dependencies.clone(),
        kind: o.kind.clone(),
        args: Default::default(),
        file: o.file.clone(),
        loc: o.loc,
        id: o.id,
        nodes: vec![],
        layers: Default::default(),
        sealed: o.sealed,
        width: o.width,
        height: o.height,
        node: None,
        parent: o.parent,
    }
}
impl Contributions {
    pub fn capture(
        engine: &Engine,
        start: usize,
        context: Context,
        value: &V,
        syntax: &SyntaxCache,
        next: &mut u64,
    ) -> Option<(Self, V)> {
        let originals: Vec<_> = engine
            .objects
            .range(start + 1..)
            .filter(|(_, o)| !super::values::immutable_kind(&o.borrow().kind))
            .map(|(_, o)| o.clone())
            .collect();
        if originals
            .iter()
            .any(|o| !matches!(o.borrow().kind.as_str(), "group" | "instance"))
        {
            return None;
        }
        for o in &originals {
            let mut o = o.borrow_mut();
            if o.kind == "group" && o.cache_identity == 0 {
                *next += 1;
                o.cache_identity = *next;
            }
        }
        let copy = Copy {
            map: skeleton(&originals),
            start,
            target: start,
            syntax,
            freeze: true,
            names: BTreeMap::new(),
            current: Default::default(),
        };
        let objects = copy.fill(&originals)?;
        let value = copy.value(value)?;
        let mut origins = vec![];
        for o in &objects {
            let o = o.borrow();
            if o.syntax_site != 0 {
                origins.push((o.syntax_site, syntax.origin(o.syntax_site)?.revision));
            }
        }
        let bytes = std::mem::size_of::<Self>()
            + objects.capacity() * std::mem::size_of::<Rc<RefCell<Object>>>()
            + origins.capacity() * 16
            + engine
                .instance_names
                .iter()
                .map(|(name, _)| name.capacity() + 96)
                .sum::<usize>()
            + context.settings.capacity()
            + context
                .names
                .iter()
                .map(|(name, _)| name.capacity() + 96)
                .sum::<usize>();
        Some((
            Self {
                objects,
                origins,
                start,
                unnamed: engine.unnamed_instances,
                names: engine.instance_names.clone(),
                context,
                bytes,
            },
            value,
        ))
    }
    pub fn memory(&self, footprint: &mut super::memory::Footprint) {
        for object in &self.objects {
            footprint.object(object);
        }
    }
    pub fn available(&self, engine: &Engine, syntax: &SyntaxCache) -> bool {
        self.context.matches(engine)
            && self
                .origins
                .iter()
                .all(|(site, rev)| syntax.origin(*site).is_some_and(|o| o.revision == *rev))
    }
    pub fn replay(&self, engine: &mut Engine, value: &V, syntax: &SyntaxCache) -> Option<V> {
        let _profile = super::profile("contribution_replay");
        let mut names = BTreeMap::new();
        let unnamed = engine.unnamed_instances;
        if unnamed != self.context.unnamed {
            for i in self.context.unnamed + 1..=self.unnamed {
                names.insert(
                    format!("@{i}"),
                    format!("@{}", unnamed + i - self.context.unnamed),
                );
            }
        }
        let named = |name: &str, n: usize| {
            if n == 1 {
                name.to_owned()
            } else {
                format!("{name}#{n}")
            }
        };
        for (name, end) in &self.names {
            let start = self.context.names.get(name).copied().unwrap_or(0);
            let target = engine.instance_names.get(name).copied().unwrap_or(0);
            if start != target {
                for i in start + 1..=*end {
                    names.insert(named(name, i), named(name, target + i - start));
                }
            }
        }
        let copy = Copy {
            map: skeleton(&self.objects),
            start: self.start,
            target: engine.serial,
            syntax,
            freeze: false,
            names,
            current: Default::default(),
        };
        let objects = copy.fill(&self.objects)?;
        let value = copy.value(value)?;
        for o in objects {
            let id = o.borrow().id;
            engine.objects.insert(id, o);
        }
        engine.unnamed_instances += self.unnamed - self.context.unnamed;
        for (name, end) in &self.names {
            let start = self.context.names.get(name).copied().unwrap_or(0);
            if *end > start {
                *engine.instance_names.entry(name.clone()).or_default() += end - start;
            }
        }
        Some(value)
    }
}
trait Transpose<T> {
    fn transpose_option(self) -> Option<Option<T>>;
}
impl<T> Transpose<T> for Option<Option<T>> {
    fn transpose_option(self) -> Option<Option<T>> {
        match self {
            None => Some(None),
            Some(v) => v.map(Some),
        }
    }
}
