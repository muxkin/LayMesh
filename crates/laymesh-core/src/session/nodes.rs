//! Retain completed vector-node derivations independently of their placement.
use super::*;
use crate::model::{Args, Object, V};
use serde_json::{Value as J, json};
type StyleReadsMap = BTreeMap<String, Rc<crate::style::StyleRead>>;
#[derive(Clone)]
pub(crate) struct CachedNode {
    value: Arc<J>,
    bytes: usize,
    used: u64,
    steps: usize,
    allocations: usize,
    styles: StyleReadsMap,
}
#[derive(Default)]
pub(crate) struct NodeCache {
    values: HashMap<String, CachedNode>,
    pub frames: Vec<StyleReadsMap>,
    bytes: usize,
    tick: u64,
    bounds: HashMap<usize, (std::sync::Weak<J>, Option<kurbo::Rect>)>,
}
impl NodeCache {
    pub fn bytes(&self) -> usize {
        self.bytes + self.bounds.len() * 128
    }
    pub fn prune_bounds(&mut self) {
        self.bounds.retain(|_, (node, _)| node.strong_count() > 0);
    }
    pub fn retain(&mut self, live: &std::collections::HashSet<super::lifetime::Key>) {
        self.values
            .retain(|key, _| live.contains(&super::lifetime::Key::Node(key.clone())));
        self.bounds.retain(|_, (node, _)| node.strong_count() > 0);
        self.bytes = self.values.values().map(|v| v.bytes).sum();
    }
}
pub(crate) fn key(engine: &Engine, o: &Object, args: &Args) -> Option<String> {
    if o.kind == "group" && o.cache_identity != 0 {
        let mut inputs = args.clone();
        if let Some(V::Map(parent)) = inputs.get_mut("__style_parent") {
            *parent = engine.style_parent_inputs(parent).into();
        }
        // Public names of unbound instances still follow legacy allocation
        // order. Replaying a graph may rebase those names without changing its
        // semantic identity or any geometry.
        fn names(n: &J, output: &mut Vec<String>) {
            if let Some(name) = n["id"].as_str() {
                output.push(name.into());
            }
            if let Some(a) = n["children"].as_array() {
                for n in a {
                    names(n, output);
                }
            }
        }
        let mut rebound = vec![];
        for n in &o.nodes {
            names(n, &mut rebound);
        }
        return Some(
            json!([
                "group",
                o.cache_identity,
                rebound,
                engine.unit,
                engine.dpi,
                super::geometry::semantic_value(&V::Map(inputs.into()), 0)?
            ])
            .to_string(),
        );
    }
    if !matches!(
        o.kind.as_str(),
        "path"
            | "polyline"
            | "polygon"
            | "rect"
            | "ellipse"
            | "line"
            | "star"
            | "ring"
            | "arc"
            | "sector"
    ) {
        return None;
    }
    // Endpoint materials can invoke a second style cascade. They retain their
    // own geometry cache until their nested style dependencies are recorded.
    if args.keys().any(|key| {
        matches!(
            key.as_str(),
            "start_head" | "end_head" | "head" | "effects" | "clip_path"
        )
    }) {
        return None;
    }
    let input: Args = args
        .iter()
        .filter(|(key, _)| {
            !(paint_independent(o, args) && crate::style::appearance_property(key))
                && !matches!(
                    key.as_str(),
                    "__style_parent"
                        | "__style_variables"
                        | "__style_type"
                        | "__style_file"
                        | "__css_root_font"
                        | "__call_origin"
                        | "__placement_origin"
                        | "__arg_locations"
                )
        })
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    Some(
        json!([
            o.kind,
            engine.unit,
            engine.dpi,
            super::geometry::semantic_value(&V::Map(input.into()), 0)?
        ])
        .to_string(),
    )
}
fn paint_independent(o: &Object, args: &Args) -> bool {
    matches!(
        o.kind.as_str(),
        "path"
            | "polyline"
            | "polygon"
            | "rect"
            | "ellipse"
            | "line"
            | "star"
            | "ring"
            | "arc"
            | "sector"
    ) && !args
        .values()
        .any(|v| matches!(v,V::Map(a) if a.get("kind").is_some_and(|k|k.as_str()=="imagePaint")))
        && !args.keys().any(|key| {
            matches!(
                key.as_str(),
                "effects"
                    | "filter"
                    | "__css_box_shadows"
                    | "head"
                    | "start_head"
                    | "end_head"
                    | "clip_path"
            )
        })
}
fn repaint(node: &mut J, engine: &Engine, o: &Object, args: &Args) -> Option<usize> {
    let mut updates = 0;
    if o.kind == "group" {
        // Wrappers with their own geometry are separate derivations. A plain
        // group keeps its child contributions and their placement transforms.
        if args.keys().any(|key| {
            matches!(
                key.as_str(),
                "background"
                    | "border_color"
                    | "border_width"
                    | "border_radius"
                    | "padding"
                    | "effects"
                    | "filter"
                    | "__css_box_shadows"
            )
        }) {
            return None;
        }
        let children = node.get_mut("children")?.as_array_mut()?;
        if children.len() != o.layers.len() {
            return None;
        }
        let mut paints: HashMap<String, (Args, J, J, J)> = HashMap::new();
        for (child, (operation, placement)) in children.iter_mut().zip(o.layers.iter()) {
            if operation == "__fuse" {
                return None;
            }
            let material = placement.get("__material")?.object()?;
            let material = material.borrow();
            let leaf = paint_independent(&material, &material.args)
                && paint_independent(&material, placement);
            let mut explicit: Args = material
                .args
                .iter()
                .filter(|(key, _)| {
                    !leaf
                        || !matches!(
                            key.as_str(),
                            "commands" | "__arg_locations" | "__call_origin" | "__placement_origin"
                        )
                })
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            explicit.insert("__style_file".into(), V::text(&material.file));
            if let Some(class) = placement.get("class") {
                explicit.insert("class".into(), class.clone());
            }
            let paint_key = leaf.then(|| {
                json!([
                    material.kind,
                    crate::model::args_json(&explicit),
                    crate::model::args_json(
                        &placement
                            .iter()
                            .filter(|(key, _)| crate::style::appearance_property(key)
                                || key.starts_with("border_")
                                || key.starts_with("line_"))
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect()
                    )
                ])
                .to_string()
            });
            if let Some((computed, fill, stroke, opacity)) =
                paint_key.as_ref().and_then(|key| paints.get(key))
            {
                updates += usize::from(
                    child["fill"] != *fill
                        || child["strokeStyle"] != *stroke
                        || child["opacity"] != *opacity,
                );
                child["fill"] = fill.clone();
                child["strokeStyle"] = stroke.clone();
                child["opacity"] = json!(
                    crate::model::num(placement, "opacity", 1.)
                        * crate::model::num(computed, "opacity", 1.)
                );
                continue;
            }
            let mut computed = super::queries::without_recording(|| {
                engine.styled(&material.kind, &explicit, args)
            })
            .ok()?;
            for (key, value) in placement {
                if !matches!(
                    key.as_str(),
                    "anchor"
                        | "target"
                        | "offset"
                        | "rotation"
                        | "opacity"
                        | "start"
                        | "end"
                        | "start_offset"
                        | "end_offset"
                        | "start_offset_space"
                        | "end_offset_space"
                        | "__arg_locations"
                        | "__call_origin"
                        | "__material"
                        | "__result_id"
                ) {
                    computed.insert(key.clone(), value.clone());
                }
            }
            updates += repaint(child, engine, &material, &computed)?;
            if let Some(key) = paint_key {
                paints.insert(
                    key,
                    (
                        computed.clone(),
                        child["fill"].clone(),
                        child["strokeStyle"].clone(),
                        child["opacity"].clone(),
                    ),
                );
            }
            child["opacity"] = json!(
                crate::model::num(placement, "opacity", 1.)
                    * crate::model::num(&computed, "opacity", 1.)
            );
        }
        node["opacity"] = json!(crate::model::num(args, "opacity", 1.));
        return Some(updates);
    }
    if !paint_independent(o, args) {
        return None;
    }
    let line = matches!(o.kind.as_str(), "line" | "polyline" | "arc");
    let color = crate::model::string(
        args,
        if line { "line_color" } else { "border_color" },
        if line { "#000000" } else { "none" },
    );
    let width = crate::model::length(
        args,
        if line { "line_width" } else { "border_width" },
        if color == "none" { 0. } else { 0.3 },
        "pt",
        engine.dpi,
    );
    let fill = args
        .get("fill")
        .or_else(|| args.get("background"))
        .map(V::json)
        .unwrap_or(json!("none"));
    let stroke = engine.stroke(args, line, width);
    let opacity = json!(crate::model::num(args, "opacity", 1.));
    if node["fill"] != fill || node["strokeStyle"] != stroke || node["opacity"] != opacity {
        node["fill"] = fill;
        node["strokeStyle"] = stroke;
        node["opacity"] = opacity;
        updates += 1;
    }
    if let Some(recipe) = node.get_mut("geometryRecipe") {
        super::geometry::rebase_recipe(recipe, args);
    }
    Some(updates)
}
pub(crate) fn materialized(key: &str, engine: &mut Engine, o: &Object, args: &Args) -> Option<J> {
    if let Some(mut node) = get(key, engine) {
        if o.kind != "group" {
            if let Some(recipe) = node.get_mut("geometryRecipe") {
                super::geometry::rebase_recipe(recipe, args);
            }
        }
        if paint_independent(o, args) {
            let updates = repaint(&mut node, engine, o, args)?;
            if let Some(state) = active() {
                state.borrow_mut().metrics.updated_appearance_nodes += updates;
            }
        }
        return Some(node);
    }
    if o.kind != "group" {
        return None;
    }
    let state = active()?;
    let cached = state.borrow().nodes.values.get(key)?.clone();
    if engine.steps + cached.steps > 1_000_000 {
        return None;
    }
    if cached
        .styles
        .values()
        .any(|r| !r.matches(engine) && !r.paint_only_change(engine))
    {
        return None;
    }
    let reads = style_reads(Some(key));
    let mut node = cached.value.as_ref().clone();
    let updates = repaint(&mut node, engine, o, args)?;
    for read in cached.styles.values() {
        super::queries::style(read.current(engine));
    }
    let reads = reads.finish();
    engine.steps += cached.steps;
    engine.serial += cached.allocations;
    {
        let mut s = state.borrow_mut();
        s.metrics.updated_appearance_nodes += updates;
        s.metrics.reused_nodes += 1;
    }
    put(
        key.into(),
        &node,
        cached.steps,
        cached.allocations,
        reads,
        engine,
    );
    Some(node)
}
pub(crate) fn get(key: &str, engine: &mut Engine) -> Option<J> {
    let _profile = super::profile("retained_node_copy");
    get_shared(key, engine).map(|v| v.as_ref().clone())
}
pub(crate) fn get_shared(key: &str, engine: &mut Engine) -> Option<Arc<J>> {
    let state = active()?;
    let value = state.borrow().nodes.values.get(key)?.clone();
    if engine.steps + value.steps > 1_000_000 {
        return None;
    }
    for read in value.styles.values() {
        if !read.matches(engine) {
            return None;
        }
    }
    for read in value.styles.values() {
        super::queries::style(read.clone());
    }
    engine.steps += value.steps;
    engine.serial += value.allocations;
    let mut state = state.borrow_mut();
    state.lifetime.touch(super::lifetime::Key::Node(key.into()));
    state.nodes.tick += 1;
    let tick = state.nodes.tick;
    if let Some(entry) = state.nodes.values.get_mut(key) {
        entry.used = tick;
    }
    state.metrics.reused_nodes += 1;
    Some(value.value)
}
pub(crate) struct StyleReads(Option<Rc<RefCell<State>>>);
impl StyleReads {
    pub fn finish(mut self) -> StyleReadsMap {
        self.0
            .take()
            .map(|s| {
                let mut s = s.borrow_mut();
                s.lifetime.end();
                s.nodes.frames.pop().unwrap()
            })
            .unwrap_or_default()
    }
}
impl Drop for StyleReads {
    fn drop(&mut self) {
        if let Some(state) = self.0.take() {
            let mut state = state.borrow_mut();
            state.lifetime.end();
            state.nodes.frames.pop();
        }
    }
}
pub(crate) fn style_reads(key: Option<&str>) -> StyleReads {
    let state = active().filter(|_| key.is_some());
    if let Some(state) = &state {
        let mut state = state.borrow_mut();
        state
            .lifetime
            .begin(super::lifetime::Key::Node(key.unwrap().into()), true);
        state.nodes.frames.push(BTreeMap::new());
    }
    StyleReads(state)
}
pub(crate) fn put(
    key: String,
    value: &J,
    steps: usize,
    allocations: usize,
    styles: StyleReadsMap,
    engine: &Engine,
) {
    put_shared(
        key,
        Arc::new(value.clone()),
        steps,
        allocations,
        styles,
        engine,
    );
}
/// Charge retained JSON directly. Serializing a group just to estimate its
/// cache size would re-encode every unchanged path on a new placement.
fn retained_bytes(value: &J) -> usize {
    std::mem::size_of::<J>()
        + match value {
            J::String(text) => text.capacity(),
            J::Array(values) => {
                values.iter().map(retained_bytes).sum::<usize>()
                    + (values.capacity() - values.len()) * std::mem::size_of::<J>()
            }
            J::Object(values) => values
                .iter()
                .map(|(key,value)|
            // Include the owned key and a conservative map-node overhead.
            key.capacity()+std::mem::size_of::<String>()+64+retained_bytes(value))
                .sum(),
            _ => 0,
        }
}
pub(crate) fn put_shared(
    key: String,
    value: Arc<J>,
    steps: usize,
    allocations: usize,
    styles: StyleReadsMap,
    _engine: &Engine,
) {
    let Some(state) = active() else { return };
    let mut state = state.borrow_mut();
    if state.interrupted.is_some() {
        return;
    }
    let bytes = key.len()
        + retained_bytes(&value)
        + styles.values().map(|r| r.bytes()).sum::<usize>()
        + 256;
    let entry = CachedNode {
        value,
        bytes,
        used: state.nodes.tick,
        steps,
        allocations,
        styles,
    };
    let key_copy = key.clone();
    if let Some(old) = state.nodes.values.insert(key, entry) {
        state.nodes.bytes -= old.bytes;
    }
    state.nodes.bytes += bytes;
    state.lifetime.touch(super::lifetime::Key::Node(key_copy));
}
pub(crate) fn detach_for_rename(key: &str, value: &Arc<J>) -> Option<CachedNode> {
    let state = active()?;
    let mut state = state.borrow_mut();
    if !state
        .nodes
        .values
        .get(key)
        .is_some_and(|cached| Arc::ptr_eq(&cached.value, value))
    {
        return None;
    }
    let mut entry = state.nodes.values.remove(key)?;
    state.nodes.bytes -= entry.bytes;
    // Release the cache's Arc so the caller can rename the unique scene value.
    entry.value = Arc::new(J::Null);
    Some(entry)
}
pub(crate) fn renamed(key: &str, value: &Arc<J>, detached: Option<CachedNode>) {
    if let Some(state) = active() {
        let mut state = state.borrow_mut();
        if let Some(mut entry) = detached {
            entry.value = value.clone();
            state.nodes.bytes += entry.bytes;
            state.nodes.values.insert(key.into(), entry);
        } else if let Some(entry) = state.nodes.values.get_mut(key) {
            entry.value = value.clone();
        }
    }
}
pub(crate) fn placement_key(
    engine: &Engine,
    o: &Object,
    args: &Args,
    parent: &Args,
) -> Option<String> {
    if o.kind != "group" || o.cache_identity == 0 {
        return None;
    }
    let parent = engine.style_parent_inputs(parent);
    fn input(v: &V) -> Option<J> {
        Some(match v {
            V::Anchor {
                owner,
                x,
                y,
                reference,
            } => json!([
                "anchor",
                owner,
                x,
                y,
                reference.as_ref().map(|r| (r.instance, &r.name))
            ]),
            V::Map(a) => json!(
                a.iter()
                    .map(|(k, v)| Some((k, input(v)?)))
                    .collect::<Option<BTreeMap<_, _>>>()?
            ),
            V::List(a) => json!(a.iter().map(input).collect::<Option<Vec<_>>>()?),
            _ => super::geometry::value(v, 0)?,
        })
    }
    let inputs = input(&V::Map(args.clone().into()))?;
    let parent = super::geometry::semantic_value(&V::Map(parent.into()), 0)?;
    // Keep public names and ancestor inputs in the key; sibling insertion may
    // rebase public names while preserving the group's computations.
    let material = key(engine, o, &o.args)?;
    Some(
        json!([
            "placed",
            engine.instance_identity()?,
            material,
            inputs,
            parent,
            engine.serial
        ])
        .to_string(),
    )
}

pub(crate) fn decorated_bounds(node: &Arc<J>) -> Option<kurbo::Rect> {
    let Some(state) = active() else {
        return crate::art::decorated_bounds(node);
    };
    let key = Arc::as_ptr(node) as usize;
    if let Some((old, bounds)) = state.borrow().nodes.bounds.get(&key) {
        if old.upgrade().is_some_and(|old| Arc::ptr_eq(&old, node)) {
            return *bounds;
        }
    }
    let bounds = crate::art::decorated_bounds(node);
    let mut state = state.borrow_mut();
    state
        .nodes
        .bounds
        .insert(key, (Arc::downgrade(node), bounds));

    bounds
}
