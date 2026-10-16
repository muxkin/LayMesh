//! Recorded reads and ordered writes for value computations. Mutable drawing
//! operations remain explicit effects; their derivations are cached separately.
use super::*;
use crate::model::{Scope, V};
use serde_json::{Value as J, json};
use std::collections::BTreeSet;

#[derive(Clone)]
struct Read {
    address: Address,
    name: String,
    value: Option<J>,
}
#[derive(Clone)]
struct Write {
    address: Address,
    name: String,
    value: V,
}
#[derive(Clone)]
pub(crate) struct Record {
    resources: BTreeMap<String, Option<Arc<Vec<u8>>>>,
    blocks: Rc<super::memory::Blocks>,
    styles: BTreeMap<String, Rc<crate::style::StyleRead>>,
    reads: Vec<Read>,
    writes: Vec<Write>,
    value: V,
    steps: usize,
    depth: usize,
    bytes: usize,
    used: u64,
    entry: String,
    allocations: usize,
    contributions: Option<super::contributions::Contributions>,
    observes_style_geometry: bool,
}
struct Frame {
    resources: BTreeMap<String, Option<Arc<Vec<u8>>>>,
    styles: BTreeMap<String, Rc<crate::style::StyleRead>>,
    addresses: HashMap<u64, Address>,
    birth: u64,
    reads: Vec<Read>,
    writes: Vec<Write>,
    written: BTreeSet<(Address, String)>,
    valid: bool,
    max_depth: usize,
    serial: usize,
    construction: Option<super::contributions::Context>,
    observes_style_geometry: bool,
}
#[derive(Default)]
pub(crate) struct Queries {
    records: HashMap<(u64, u64), Record>,
    frames: Vec<Frame>,
    shared_refs: HashMap<(u8, usize), (usize, usize)>,
    clock: u64,
    bytes: usize,
    suspended: usize,
}

/// Cascade validation must not turn its own implementation into dependencies
/// of the caller. Cancellation and syntax/resource caches remain active.
pub(crate) fn without_recording<T>(run: impl FnOnce() -> T) -> T {
    struct Restore(Option<Rc<RefCell<State>>>);
    impl Drop for Restore {
        fn drop(&mut self) {
            if let Some(state) = &self.0 {
                state.borrow_mut().queries.suspended -= 1;
            }
        }
    }
    let state = active();
    if let Some(state) = &state {
        state.borrow_mut().queries.suspended += 1;
    }
    let _restore = Restore(state);
    run()
}

pub(crate) fn observe_style_geometry() {
    if let Some(state) = active() {
        let mut state = state.borrow_mut();
        if state.queries.suspended > 0 {
            return;
        }
        for frame in &mut state.queries.frames {
            frame.observes_style_geometry = true;
        }
    }
}
thread_local! { static BIRTH: std::cell::Cell<u64> = const { std::cell::Cell::new(0) }; }
pub(crate) fn birth() -> u64 {
    BIRTH.with(|n| {
        let next = n.get() + 1;
        n.set(next);
        next
    })
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Address {
    depth: usize,
    closures: Vec<(String, usize)>,
}
fn ancestors(scope: &Scope) -> Vec<Scope> {
    let mut next = Some(scope.clone());
    let mut scopes = vec![];
    while let Some(scope) = next {
        next = scope.borrow().parent.clone();
        scopes.push(scope);
    }
    scopes
}
fn resolve(scope: &Scope, address: &Address) -> Option<Scope> {
    let mut scope = ancestors(scope).get(address.depth)?.clone();
    for (name, depth) in &address.closures {
        let function = scope.borrow().values.get(name)?.clone();
        let V::Function { scope: closure, .. } = function else {
            return None;
        };
        scope = ancestors(&closure).get(*depth)?.clone();
    }
    Some(scope)
}
pub(crate) fn signature(v: &V) -> Option<J> {
    Some(match v {
        V::Null => json!(["null"]),
        V::Bool(b) => json!(["bool", b]),
        V::Number(n, u) => json!(["number", n, u]),
        V::Text(t, r) => json!(["text", t, r]),
        V::Color(c) => json!(["color", format!("{c:?}")]),
        V::List(a) => json!(["list", a.iter().map(signature).collect::<Option<Vec<_>>>()?]),
        V::Dict(m) => json!([
            "dict",
            m.iter()
                .map(|(k, v)| Some((k, signature(v)?)))
                .collect::<Option<Vec<_>>>()?
        ]),
        V::Map(m) => json!([
            "map",
            m.iter()
                .map(|(k, v)| Some((k, signature(v)?)))
                .collect::<Option<Vec<_>>>()?
        ]),
        V::Function { revision, file, .. } if *revision != 0 => json!(["function", revision, file]),
        V::Object(o) => super::values::signature(&o.borrow())?,
        _ => return None,
    })
}
fn retainable(v: &V) -> bool {
    match v {
        V::Null | V::Bool(_) | V::Number(..) | V::Text(..) | V::Color(_) => true,
        V::List(v) => v.iter().all(retainable),
        V::Dict(v) => v.values().all(retainable),
        V::Map(v) => v.values().all(retainable),
        _ => false,
    }
}
pub(crate) fn read(request: &Scope, owner: Option<u64>, name: &str, value: Option<&V>) {
    let Some(state) = active() else { return };
    let mut state = state.borrow_mut();
    if state.queries.suspended > 0 || state.queries.frames.is_empty() {
        return;
    }
    let mut signature_cache = None;
    let request_birth = request.borrow().birth;
    for frame in &mut state.queries.frames {
        if !frame.valid {
            continue;
        }
        let owner = owner.unwrap_or(request_birth);
        let address = if let Some(address) = frame.addresses.get(&owner) {
            address.clone()
        } else if owner > frame.birth {
            continue;
        } else {
            frame.valid = false;
            continue;
        };
        if let Some(V::Function { scope: closure, .. }) = value {
            for (depth, scope) in ancestors(closure).iter().enumerate() {
                let mut path = address.clone();
                path.closures.push((name.into(), depth));
                frame.addresses.entry(scope.borrow().birth).or_insert(path);
            }
        }
        let key = (address.clone(), name.to_owned());
        if frame.written.contains(&key)
            || frame
                .reads
                .iter()
                .any(|r| r.address == address && r.name == name)
        {
            continue;
        }
        let sig = signature_cache.get_or_insert_with(|| value.map(signature));
        if matches!(sig, Some(None)) {
            frame.valid = false;
            continue;
        }
        frame.reads.push(Read {
            address: address.clone(),
            name: name.into(),
            value: sig.clone().flatten(),
        });
    }
}
pub(crate) fn write(scope: &Scope, name: &str, before: Option<&V>, value: &V) {
    let Some(state) = active() else { return };
    let mut state = state.borrow_mut();
    if state.queries.suspended > 0 {
        return;
    }
    let owner = scope.borrow().birth;
    for frame in &mut state.queries.frames {
        if !frame.valid {
            continue;
        }
        let Some(address) = frame.addresses.get(&owner).cloned() else {
            if owner <= frame.birth {
                frame.valid = false;
            }
            continue;
        };
        if !retainable(value) {
            frame.valid = false;
            continue;
        }
        let key = (address.clone(), name.to_owned());
        if !frame.written.contains(&key)
            && !frame
                .reads
                .iter()
                .any(|r| r.address == address && r.name == name)
        {
            let sig = before.map(signature);
            if matches!(sig, Some(None)) {
                frame.valid = false;
                continue;
            }
            frame.reads.push(Read {
                address: address.clone(),
                name: name.into(),
                value: sig.flatten(),
            });
        }
        frame.written.insert(key);
        frame.writes.push(Write {
            address,
            name: name.into(),
            value: value.clone(),
        });
    }
}
pub(crate) fn tracking_styles() -> bool {
    active().is_some_and(|s| {
        let s = s.borrow();
        s.queries.suspended == 0
            && (!s.nodes.frames.is_empty() || s.queries.frames.iter().any(|f| f.valid))
    })
}
pub(crate) fn style(read: Rc<crate::style::StyleRead>) {
    if let Some(state) = active() {
        let mut state = state.borrow_mut();
        if state.queries.suspended > 0 {
            return;
        }
        for frame in &mut state.queries.frames {
            if frame.valid {
                frame
                    .styles
                    .entry(read.key.clone())
                    .or_insert_with(|| read.clone());
            }
        }
        for frame in &mut state.nodes.frames {
            frame
                .entry(read.key.clone())
                .or_insert_with(|| read.clone());
        }
    }
}
pub(crate) fn resource(host: &Host, path: &str, value: Option<&[u8]>) {
    let Some(state) = active() else { return };
    let mut state = state.borrow_mut();
    if state.queries.suspended > 0 {
        return;
    }
    if !state.queries.frames.iter().any(|f| f.valid) {
        return;
    }
    let data = value.map(|data| {
        host.shared_files
            .get(path)
            .cloned()
            .unwrap_or_else(|| Arc::new(data.to_vec()))
    });
    for frame in &mut state.queries.frames {
        if frame.valid {
            frame
                .resources
                .entry(path.into())
                .or_insert_with(|| data.clone());
        }
    }
}
pub(crate) fn construction(engine: &Engine) {
    if let Some(state) = active() {
        if state.borrow().queries.suspended > 0 {
            return;
        }
        for frame in &mut state.borrow_mut().queries.frames {
            if frame.valid && frame.construction.is_none() {
                frame.construction = Some(super::contributions::Context::capture(engine));
            }
        }
    }
}
pub(crate) fn mutate(id: usize) {
    if let Some(state) = active() {
        if state.borrow().queries.suspended > 0 {
            return;
        }
        for frame in &mut state.borrow_mut().queries.frames {
            if id <= frame.serial {
                frame.valid = false;
            }
        }
    }
}
pub(crate) fn effect() {
    if let Some(state) = active() {
        if state.borrow().queries.suspended > 0 {
            return;
        }
        for frame in &mut state.borrow_mut().queries.frames {
            frame.valid = false;
        }
    }
}
pub(crate) fn call_depth(depth: usize) {
    if let Some(state) = active() {
        if state.borrow().queries.suspended > 0 {
            return;
        }
        for frame in &mut state.borrow_mut().queries.frames {
            frame.max_depth = frame.max_depth.max(depth);
        }
    }
}
impl Queries {
    pub fn statistics(&self) -> J {
        let mut files = BTreeMap::<String, (usize, usize, usize)>::new();
        for r in self.records.values() {
            let f = files.entry(r.entry.clone()).or_default();
            f.0 += 1;
            f.1 += r.bytes;
            f.2 += usize::from(r.contributions.is_some());
        }
        let mut largest: Vec<_> = self.records.values().collect();
        largest.sort_unstable_by_key(|r| std::cmp::Reverse(r.bytes));
        json!({"files":files,"largest":largest.into_iter().take(10).map(|r|json!({"file":r.entry,"bytes":r.bytes,"steps":r.steps,"construction":r.contributions.is_some(),"reads":r.reads.len(),"styles":r.styles.len()})).collect::<Vec<_>>()})
    }
    pub fn resource_bytes(&self) -> usize {
        self.shared_refs
            .iter()
            .filter(|((kind, _), _)| *kind == 8)
            .map(|(_, (_, bytes))| *bytes)
            .sum()
    }
    pub fn bytes(&self) -> usize {
        self.bytes
    }
    pub fn retain(&mut self, live: &std::collections::HashSet<super::lifetime::Key>) {
        let removed: Vec<_> = self
            .records
            .keys()
            .filter(|(id, path)| !live.contains(&super::lifetime::Key::Query(*id, *path)))
            .copied()
            .collect();
        for key in removed {
            if let Some(record) = self.records.remove(&key) {
                self.release(record);
            }
        }
    }
    fn retain_resources(&mut self, record: &Record) {
        for (key, size) in record.blocks.iter() {
            let (count, _) = self.shared_refs.entry(*key).or_insert_with(|| {
                self.bytes += size;
                (0, *size)
            });
            *count += 1;
        }
    }
    fn release(&mut self, record: Record) {
        self.bytes -= record.bytes;
        for key in record.blocks.keys() {
            let (count, size) = self
                .shared_refs
                .get_mut(key)
                .expect("cached allocation ownership");
            *count -= 1;
            if *count == 0 {
                self.bytes -= *size;
                self.shared_refs.remove(key);
            }
        }
    }
    fn insert(&mut self, key: (u64, u64), record: Record) {
        if let Some(old) = self.records.remove(&key) {
            self.release(old);
        }
        self.retain_resources(&record);
        self.bytes += record.bytes;
        self.records.insert(key, record);
    }
}
pub(crate) fn begin(engine: &mut Engine, expr: &Expr, scope: &Scope, key: u64) -> Option<V> {
    let _profile = super::profile("query_lookup");
    let state = active()?;
    if state.borrow().queries.suspended > 0 {
        return None;
    }
    let record = state
        .borrow()
        .queries
        .records
        .get(&(expr.identity, key))
        .cloned();
    if let Some(record) = record {
        let mut valid =
            engine.steps + record.steps <= 1_000_000 && engine.calls + record.depth <= 64;
        for dep in &record.reads {
            let Some(scope) = resolve(scope, &dep.address) else {
                valid = false;
                break;
            };
            let value = scope.borrow().values.get(&dep.name).cloned();
            // Reads of an absent binding must also observe a newly introduced
            // shadow. The exact owning scope is part of the query input.
            read(
                &scope,
                Some(scope.borrow().birth),
                &dep.name,
                value.as_ref(),
            );
            if value.as_ref().map(signature) != dep.value.clone().map(Some) {
                valid = false;
                break;
            }
        }
        for read in record.styles.values() {
            if !read.matches(engine) {
                // Object recipes are independent of their eventual paint.
                // Keep the recipe and let materialization update the affected
                // properties; explicit geometry observations still invalidate.
                if record.contributions.is_none()
                    || record.observes_style_geometry
                    || !read.paint_only_change(engine)
                {
                    valid = false;
                    break;
                }
            }
            style(read.current(engine));
        }
        for (path, before) in &record.resources {
            if engine
                .host
                .read(path, &engine.file, expr.loc)
                .ok()
                .as_deref()
                != before.as_ref().map(|b| b.as_slice())
            {
                valid = false;
                break;
            }
        }
        if record.writes.iter().any(|w| {
            resolve(scope, &w.address).is_none_or(|s| s.borrow().readonly.contains(&w.name))
        }) {
            valid = false;
        }
        if let Some(graph) = &record.contributions {
            if !graph.available(engine, &state.borrow().syntax) {
                valid = false;
            }
        } else if !super::values::available(&record.value, &state.borrow().syntax) {
            valid = false;
        }
        if valid {
            state
                .borrow_mut()
                .lifetime
                .touch(super::lifetime::Key::Query(expr.identity, key));
            if record.observes_style_geometry {
                observe_style_geometry();
            }
            let value = if let Some(graph) = &record.contributions {
                construction(engine);
                graph
                    .replay(engine, &record.value, &state.borrow().syntax)
                    .expect("validated contribution graph")
            } else {
                super::values::thaw(&record.value, &state.borrow().syntax)
            };
            for write in &record.writes {
                let target = resolve(scope, &write.address).unwrap();
                let before = target.borrow().values.get(&write.name).cloned();
                self::write(&target, &write.name, before.as_ref(), &write.value);
                target
                    .borrow_mut()
                    .values
                    .insert(write.name.clone(), write.value.clone());
            }
            engine.steps += record.steps;
            engine.serial += record.allocations;
            call_depth(engine.calls + record.depth);

            let mut state = state.borrow_mut();
            state.metrics.reused_expressions += 1;
            if record.contributions.is_some() {
                state.metrics.reused_constructions += 1;
            }
            state.queries.clock += 1;
            let clock = state.queries.clock;
            if let Some(record) = state.queries.records.get_mut(&(expr.identity, key)) {
                record.used = clock;
            }
            return Some(value);
        }
    }
    let birth = BIRTH.with(|n| n.get());
    let mut state = state.borrow_mut();
    state.metrics.evaluated_expressions += 1;
    state
        .lifetime
        .begin(super::lifetime::Key::Query(expr.identity, key), false);
    state.queries.frames.push(Frame {
        resources: BTreeMap::new(),
        styles: BTreeMap::new(),
        addresses: ancestors(scope)
            .iter()
            .enumerate()
            .map(|(depth, s)| {
                (
                    s.borrow().birth,
                    Address {
                        depth,
                        closures: vec![],
                    },
                )
            })
            .collect(),
        birth,
        reads: vec![],
        writes: vec![],
        written: BTreeSet::new(),
        valid: true,
        max_depth: engine.calls,
        serial: engine.serial,
        construction: None,
        observes_style_geometry: false,
    });
    None
}
pub(crate) fn end(
    engine: &Engine,
    expr: &Expr,
    key: u64,
    start_steps: usize,
    start_calls: usize,
    value: &mut Result<V>,
) {
    let Some(state) = active() else { return };
    let mut state = state.borrow_mut();
    if state.queries.suspended > 0 {
        return;
    }
    state.lifetime.end();
    let Some(frame) = state.queries.frames.pop() else {
        return;
    };
    let Ok(value) = value else { return };
    if !frame.valid || state.interrupted.is_some() {
        return;
    }
    let (contributions, frozen) = if let Some(context) = frame.construction {
        let State {
            syntax,
            next_contribution,
            ..
        } = &mut *state;
        let Some((graph, value)) = super::contributions::Contributions::capture(
            engine,
            frame.serial,
            context,
            value,
            syntax,
            next_contribution,
        ) else {
            return;
        };
        (Some(graph), value)
    } else {
        let Some(value) = super::values::freeze(value, &state.syntax) else {
            return;
        };
        (None, value)
    };
    // Pure results are immutable. Return the same frozen definitions to the
    // enclosing query so it shares them instead of freezing another copy.
    if contributions.is_none() {
        *value = frozen.clone();
    }
    let mut memory = super::memory::Footprint::default();
    memory.value(&frozen);
    if let Some(graph) = &contributions {
        graph.memory(&mut memory);
    }
    for resource in frame.resources.values().flatten() {
        memory.resource(resource);
    }
    for read in frame.styles.values() {
        memory.style(read);
    }
    for write in &frame.writes {
        memory.value(&write.value);
    }
    let bytes = frame
        .resources
        .iter()
        .map(|(p, _)| p.len() + 128)
        .sum::<usize>()
        + frame
            .styles
            .keys()
            .map(|key| key.capacity() + 96)
            .sum::<usize>()
        + contributions.as_ref().map_or(0, |c| c.bytes)
        + memory.owned
        + memory.blocks.len() * 64
        + frame
            .reads
            .iter()
            .map(|r| {
                r.name.capacity()
                    + std::mem::size_of::<Read>()
                    + r.value.as_ref().map_or(0, super::memory::json_bytes)
            })
            .sum::<usize>()
        + frame
            .writes
            .iter()
            .map(|w| w.name.capacity() + std::mem::size_of::<Write>())
            .sum::<usize>()
        + 256;
    state.queries.clock += 1;
    let record = Record {
        resources: frame.resources,
        blocks: Rc::new(memory.blocks),
        styles: frame.styles,
        reads: frame.reads,
        writes: frame.writes,
        value: frozen.clone(),
        steps: engine.steps - start_steps,
        depth: frame.max_depth - start_calls,
        bytes,
        used: state.queries.clock,
        entry: engine
            .css_entry_file
            .clone()
            .unwrap_or_else(|| engine.file.clone()),
        allocations: engine.serial - frame.serial,
        contributions,
        observes_style_geometry: frame.observes_style_geometry,
    };
    state.queries.insert((expr.identity, key), record);
    state.metrics.cache_bytes = state.queries.bytes;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_resources_are_charged_once_and_released_with_the_last_record() {
        fn record(bytes: Arc<Vec<u8>>, used: u64) -> Record {
            let mut memory = super::memory::Footprint::default();
            memory.resource(&bytes);
            Record {
                blocks: Rc::new(memory.blocks),
                resources: BTreeMap::from([("font.ttf".into(), Some(bytes))]),
                styles: BTreeMap::new(),
                reads: vec![],
                writes: vec![],
                value: V::Null,
                steps: 0,
                depth: 0,
                bytes: 256,
                used,
                entry: "page".into(),
                allocations: 0,
                contributions: None,
                observes_style_geometry: false,
            }
        }
        let data = Arc::new(vec![0; 1024 * 1024]);
        let mut queries = Queries::default();
        for i in 0..100 {
            queries.insert((i, 0), record(data.clone(), i));
        }
        assert_eq!(queries.bytes, 1024 * 1024 + 128 + 100 * 256);
        queries.retain(
            &(0..100)
                .map(|i| super::lifetime::Key::Query(i, 0))
                .collect(),
        );
        assert_eq!(queries.records.len(), 100);
        let replacement = Arc::new(vec![1; 1024 * 1024]);
        queries.insert((0, 0), record(replacement.clone(), 100));
        assert_eq!(queries.shared_refs.len(), 2);
        queries.retain(&[super::lifetime::Key::Query(0, 0)].into_iter().collect());
        assert_eq!(queries.records.len(), 1);
        assert_eq!(queries.shared_refs.len(), 1);
        queries.retain(&Default::default());
        assert_eq!(queries.bytes, 0);
        assert!(queries.shared_refs.is_empty());
        assert_eq!(Arc::strong_count(&data), 1);
        assert_eq!(Arc::strong_count(&replacement), 1);
    }
}
