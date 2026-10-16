//! Versioned interactive compilation. A session owns immutable input snapshots,
//! syntax identities and reusable derivations; only successful evaluations commit.
use crate::{
    Diagnostic, Loc, Result,
    engine::Engine,
    model::{Host, Scene},
    parser::{self, Expr, ExprKind, Stmt, StmtKind},
};
use serde::Serialize;
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap},
    rc::Rc,
    sync::Arc,
};

mod contributions;
pub(crate) mod geometry;
pub(crate) mod lifetime;
mod memory;
pub(crate) mod nodes;
pub(crate) mod queries;
mod syntax;
pub(crate) mod values;
pub(crate) use syntax::SyntaxCache;

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompileMetrics {
    pub parsed_files: usize,
    pub reused_files: usize,
    pub parsed_bytes: usize,
    pub reused_statements: usize,
    pub evaluated_expressions: usize,
    pub reused_expressions: usize,
    pub reused_constructions: usize,
    pub materialized_nodes: usize,
    pub updated_appearance_nodes: usize,
    pub reused_text_layouts: usize,
    pub rebuilt_text_layouts: usize,
    pub reused_nodes: usize,
    pub rebuilt_geometry: usize,
    pub reused_geometry: usize,
    pub font_catalog_builds: usize,
    pub cache_bytes: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_cache_statistics: Option<serde_json::Value>,
    pub query_cache_bytes: usize,
    pub query_resource_bytes: usize,
    pub node_cache_bytes: usize,
    pub geometry_cache_bytes: usize,
    pub text_layout_cache_bytes: usize,
    pub evaluated_builtin_calls: BTreeMap<String, u64>,
    pub phase_micros: BTreeMap<String, u64>,
}

pub(crate) struct State {
    pub syntax: SyntaxCache,
    pub metrics: CompileMetrics,
    pub fonts: Option<crate::text::FontSystem>,
    pub fonts_borrowed: bool,
    pub font_native: Option<bool>,
    pub font_inputs: Vec<(String, Arc<Vec<u8>>)>,
    pub queries: queries::Queries,
    pub geometry: geometry::GeometryCache,
    pub nodes: nodes::NodeCache,
    pub lifetime: lifetime::Lifetime,
    pub next_contribution: u64,
    pub profiling: bool,
    pub clock: Option<fn() -> f64>,
    deadline: Option<f64>,
    checkpoints: u32,
    pub interrupted: Option<Diagnostic>,
    profile_samples: Rc<RefCell<BTreeMap<String, u64>>>,
    pub cancellation: Option<(Arc<std::sync::atomic::AtomicU64>, u64)>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            syntax: SyntaxCache::default(),
            metrics: CompileMetrics::default(),
            fonts: None,
            fonts_borrowed: false,
            font_native: None,
            font_inputs: vec![],
            queries: queries::Queries::default(),
            geometry: geometry::GeometryCache::default(),
            nodes: nodes::NodeCache::default(),
            lifetime: lifetime::Lifetime::default(),
            next_contribution: 0,
            profiling: false,
            clock: None,
            deadline: None,
            checkpoints: 0,
            interrupted: None,
            profile_samples: Default::default(),
            cancellation: None,
        }
    }
}
impl State {
    fn retain_caches(&mut self, live: &std::collections::HashSet<lifetime::Key>) {
        self.queries.retain(live);
        self.nodes.retain(live);
        self.geometry.retain(live);
        if let Some(fonts) = &mut self.fonts {
            fonts.retain_layouts(live);
        }
    }
}
thread_local! { static ACTIVE: RefCell<Option<Rc<RefCell<State>>>> = const { RefCell::new(None) }; }
pub(crate) fn active() -> Option<Rc<RefCell<State>>> {
    ACTIVE.with(|s| s.borrow().clone())
}
struct Enter(Option<Rc<RefCell<State>>>);
impl Drop for Enter {
    fn drop(&mut self) {
        ACTIVE.with(|s| *s.borrow_mut() = self.0.take());
    }
}
pub(crate) fn parse(source: &str, file: &str) -> Result<Rc<Vec<Stmt>>> {
    let _profile = profile("parse");
    if let Some(state) = active() {
        let mut state = state.borrow_mut();
        let reused = state.syntax.is_current(file, source);
        let result = state.syntax.parse(source, file);
        if reused {
            state.metrics.reused_files += 1;
        } else {
            state.metrics.parsed_files += 1;
            state.metrics.parsed_bytes += state.syntax.parsed_bytes;
            state.metrics.reused_statements += state.syntax.reused_statements;
        }
        result
    } else {
        parser::parse(source, file).map(Rc::new)
    }
}

pub(crate) fn take_fonts(host: &Host) -> (crate::text::FontSystem, Option<Rc<RefCell<State>>>) {
    let inputs: Vec<_> = host
        .files
        .iter()
        .map(|(p, b)| (p.clone(), Arc::new(b.clone())))
        .chain(
            host.shared_files
                .iter()
                .map(|(p, b)| (p.clone(), b.clone())),
        )
        .filter(|(p, _)| {
            std::path::Path::new(p)
                .extension()
                .and_then(|s| s.to_str())
                .is_some_and(|s| {
                    matches!(
                        s.to_ascii_lowercase().as_str(),
                        "ttf" | "otf" | "ttc" | "otc"
                    )
                })
        })
        .collect();
    // Literal-style helpers create nested engines. They must not take or
    // replace the outer compilation's catalog or its ownership metadata.
    let state = active().filter(|s| !s.borrow().fonts_borrowed);
    if let Some(state) = &state {
        state.borrow_mut().fonts_borrowed = true;
    }
    let cached = state.as_ref().and_then(|s| {
        let mut s = s.borrow_mut();
        if s.font_native == Some(host.native)
            && s.font_inputs == inputs
            && s.fonts
                .as_ref()
                .is_none_or(|fonts| !fonts.native_inputs_changed())
        {
            s.fonts.take()
        } else {
            s.fonts = None;
            None
        }
    });
    let mut fonts = cached.unwrap_or_else(|| {
        if let Some(state) = &state {
            let mut state = state.borrow_mut();
            state.font_native = Some(host.native);
            state.font_inputs = inputs.clone();
            state.metrics.font_catalog_builds += 1;
        }
        let mut fonts = crate::text::FontSystem::new(host.native);
        for (name, data) in inputs {
            fonts.register_shared_font(&name, data);
        }
        fonts
    });
    fonts.begin_evaluation(host);
    (fonts, state)
}

/// Changes are applied atomically before evaluation. Deletions are represented
/// by `None`, including failed/missing resource dependencies.
pub struct CompileSession {
    state: Rc<RefCell<State>>,
    host: Host,
    revision: u64,
    committed: BTreeMap<String, (u64, Arc<Scene>)>,
    disposed: bool,
    initialized: bool,
    cache_enabled: bool,
}
impl CompileSession {
    pub fn new(native: bool) -> Self {
        Self {
            state: Rc::new(RefCell::new(State::default())),
            host: Host {
                native,
                ..Host::default()
            },
            revision: 0,
            committed: BTreeMap::new(),
            disposed: false,
            initialized: false,
            cache_enabled: true,
        }
    }
    /// The one-shot compatibility path uses the same transactional evaluator,
    /// with no syntax/query state installed and no reusable derivations.
    pub fn uncached(host: Host) -> Self {
        let mut session = Self::new(host.native);
        session.host = host;
        session.cache_enabled = false;
        session
    }
    pub fn set_profiling(&mut self, enabled: bool) {
        self.state.borrow_mut().profiling = enabled;
    }
    /// Optional monotonic milliseconds for hosts without std::time::Instant.
    pub fn set_profile_clock(&mut self, clock: fn() -> f64) {
        self.state.borrow_mut().clock = Some(clock);
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    /// Bound synchronous WASM work. A yielded transaction is retried after the
    /// worker processes input, retaining completed semantic query results.
    pub fn set_deadline(&mut self, deadline: Option<f64>) {
        let mut state = self.state.borrow_mut();
        state.deadline = deadline;
        state.checkpoints = 0;
        state.interrupted = None;
    }
    /// Compatibility with older hosts; cache ownership now follows live pages.
    pub fn set_budget(&mut self, _bytes: usize) {}
    pub fn set_focus_file(&mut self, _file: &str) {}
    pub fn retain_files(&mut self, files: &std::collections::HashSet<String>) {
        self.committed.retain(|file, _| files.contains(file));
        let mut state = self.state.borrow_mut();
        let live = state.lifetime.retain_files(files);
        state.retain_caches(&live);
    }
    pub fn metrics(&self) -> CompileMetrics {
        let state = self.state.borrow();
        let mut metrics = state.metrics.clone();
        metrics.phase_micros = state.profile_samples.borrow().clone();
        if state.profiling {
            metrics.query_cache_statistics = Some(state.queries.statistics());
        }
        metrics.query_cache_bytes = state.queries.bytes();
        metrics.query_resource_bytes = state.queries.resource_bytes();
        metrics.node_cache_bytes = state.nodes.bytes();
        metrics.geometry_cache_bytes = state.geometry.bytes();
        metrics.text_layout_cache_bytes = state.fonts.as_ref().map_or(0, |f| f.layout_bytes);
        metrics.cache_bytes = metrics.query_cache_bytes
            + metrics.node_cache_bytes
            + metrics.geometry_cache_bytes
            + metrics.text_layout_cache_bytes;
        metrics
    }
    pub fn apply_changes(
        &mut self,
        revision: u64,
        changes: impl IntoIterator<Item = (String, Option<Vec<u8>>)>,
    ) -> Result<()> {
        self.apply_shared_changes(
            revision,
            changes.into_iter().map(|(p, v)| (p, v.map(Arc::new))),
        )
    }
    pub fn apply_shared_changes(
        &mut self,
        revision: u64,
        changes: impl IntoIterator<Item = (String, Option<Arc<Vec<u8>>>)>,
    ) -> Result<()> {
        let changes: BTreeMap<_, _> = changes.into_iter().collect();
        let changed = changes
            .iter()
            .any(|(p, v)| match (self.host.shared_files.get(p), v) {
                (None, None) => false,
                (Some(a), Some(b)) => !Arc::ptr_eq(a, b) && a != b,
                _ => true,
            });
        if self.disposed
            || revision < self.revision
            || (self.initialized && revision == self.revision && changed)
        {
            return Err(Diagnostic::new(
                "E_SESSION",
                "Compilation session revision is stale, reused or closed",
                "",
                Loc::default(),
            ));
        }
        for (path, value) in changes {
            match value {
                Some(value) => {
                    self.host.shared_files.insert(path, value);
                }
                None => {
                    self.host.shared_files.remove(&path);
                    let mut state = self.state.borrow_mut();
                    state.syntax.remove(&path);
                    let live = state.lifetime.remove_file(&path);
                    state.retain_caches(&live);
                    self.committed.remove(&path);
                }
            }
        }
        self.revision = revision;
        self.initialized = true;
        Ok(())
    }
    pub fn set_cancellation(&mut self, latest: Arc<std::sync::atomic::AtomicU64>, expected: u64) {
        let mut state = self.state.borrow_mut();
        state.interrupted = None;
        state.cancellation = Some((latest, expected));
    }
    pub fn set_dependencies(
        &mut self,
        dependencies: Option<Rc<RefCell<std::collections::BTreeSet<String>>>>,
    ) {
        self.host.dependencies = dependencies;
    }
    pub fn source(&self, file: &str) -> Result<String> {
        let bytes = self.host.read(file, file, Loc::default())?;
        String::from_utf8(bytes)
            .map_err(|_| Diagnostic::new("E_ENCODING", "Source is not UTF-8", file, Loc::default()))
    }
    pub fn evaluate_file(
        &mut self,
        file: &str,
        transition: Option<crate::document::TransitionSpec>,
    ) -> Result<Arc<Scene>> {
        let source = self.source(file)?;
        self.evaluate(&source, file, transition)
    }
    /// Synchronize a legacy request without copying unchanged resource buffers.
    pub fn sync_host(&mut self, host: &Host, revision: u64) -> Result<()> {
        let mut changes = Vec::new();
        for (path, data) in &host.files {
            if self.host.bytes(path) != Some(data.as_slice()) {
                changes.push((path.clone(), Some(Arc::new(data.clone()))));
            }
        }
        for (path, data) in &host.shared_files {
            if self
                .host
                .shared_files
                .get(path)
                .is_none_or(|old| !Arc::ptr_eq(old, data) && old != data)
            {
                changes.push((path.clone(), Some(data.clone())));
            }
        }
        for path in self.host.shared_files.keys() {
            if host.bytes(path).is_none() {
                changes.push((path.clone(), None));
            }
        }
        self.apply_shared_changes(revision, changes)?;
        self.host.native = host.native;
        self.host.dependencies = host.dependencies.clone();
        Ok(())
    }
    pub fn evaluate(
        &mut self,
        source: &str,
        file: &str,
        transition: Option<crate::document::TransitionSpec>,
    ) -> Result<Arc<Scene>> {
        if self.disposed {
            return Err(Diagnostic::new(
                "E_SESSION",
                "Compilation session is closed",
                file,
                Loc::default(),
            ));
        }
        self.state.borrow_mut().metrics = CompileMetrics::default();
        self.state.borrow_mut().lifetime.start();
        self.state.borrow().profile_samples.borrow_mut().clear();
        let old = ACTIVE.with(|s| s.replace(self.cache_enabled.then(|| self.state.clone())));
        let _entered = Enter(old);
        let compile = || {
            let mut engine = Engine::new(self.host.clone());
            engine.page_transition_override = transition;
            engine.compile(source, file)
        };
        let result = if self.cache_enabled {
            crate::geometry_cache::interactive(compile)
        } else {
            compile()
        };
        {
            let mut state = self.state.borrow_mut();
            state.metrics.cache_bytes =
                state.queries.bytes() + state.geometry.bytes() + state.nodes.bytes();
        }
        let result = checkpoint(file, Loc::default()).and(result);
        {
            let mut state = self.state.borrow_mut();
            let live = state.lifetime.finish(file, result.is_ok());
            state.retain_caches(&live);
        }
        let scene = match result {
            Ok(scene) => Arc::new(scene),
            Err(error) => {
                crate::asset_cache::finish_compilation();
                return Err(error);
            }
        };
        self.committed
            .insert(file.to_owned(), (self.revision, scene.clone()));
        self.state.borrow_mut().nodes.prune_bounds();
        crate::asset_cache::finish_compilation();
        Ok(scene)
    }
    pub fn plan(&mut self, source: &str, file: &str) -> Result<crate::document::DocumentPlan> {
        if self.disposed {
            return Err(Diagnostic::new(
                "E_SESSION",
                "Compilation session is closed",
                file,
                Loc::default(),
            ));
        }
        let old = ACTIVE.with(|s| s.replace(self.cache_enabled.then(|| self.state.clone())));
        let _entered = Enter(old);
        self.state.borrow_mut().lifetime.start();
        let result = crate::document::plan_source(source, file, self.host.clone());
        let result = checkpoint(file, Loc::default()).and(result);
        if let Ok(plan) = &result {
            let files = plan
                .pages
                .iter()
                .map(|p| p.src.clone())
                .chain(std::iter::once(file.to_owned()))
                .collect();
            self.retain_files(&files);
        }
        result
    }
    pub fn snapshot(&self, file: &str) -> Option<(u64, Arc<Scene>)> {
        self.committed.get(file).cloned()
    }
    pub fn dispose(&mut self) {
        self.disposed = true;
        self.committed.clear();
        self.host.shared_files.clear();
        *self.state.borrow_mut() = State::default();
        crate::asset_cache::prune();
    }
}

pub(crate) struct ProfileScope {
    data: Option<(
        Rc<RefCell<BTreeMap<String, u64>>>,
        &'static str,
        ProfileStart,
    )>,
}
enum ProfileStart {
    Clock(fn() -> f64, f64),
    #[cfg(not(target_arch = "wasm32"))]
    Native(std::time::Instant),
}
pub(crate) fn profile(name: &'static str) -> ProfileScope {
    let data = active().filter(|s| s.borrow().profiling).and_then(|s| {
        let start = if let Some(clock) = s.borrow().clock {
            ProfileStart::Clock(clock, clock())
        } else {
            #[cfg(not(target_arch = "wasm32"))]
            {
                ProfileStart::Native(std::time::Instant::now())
            }
            #[cfg(target_arch = "wasm32")]
            {
                return None;
            }
        };
        let samples = s.borrow().profile_samples.clone();
        Some((samples, name, start))
    });
    ProfileScope { data }
}
impl Drop for ProfileScope {
    fn drop(&mut self) {
        if let Some((samples, name, start)) = &self.data {
            let micros = match start {
                ProfileStart::Clock(clock, start) => ((clock() - start).max(0.) * 1000.) as u64,
                #[cfg(not(target_arch = "wasm32"))]
                ProfileStart::Native(start) => start.elapsed().as_micros() as u64,
            };
            *samples.borrow_mut().entry((*name).into()).or_default() += micros;
        }
    }
}

pub(crate) fn checkpoint(file: &str, loc: Loc) -> Result<()> {
    if let Some(state) = active() {
        let mut state = state.borrow_mut();
        if let Some(error) = &state.interrupted {
            return Err(error.clone());
        }
        if state
            .cancellation
            .as_ref()
            .is_some_and(|(latest, expected)| {
                latest.load(std::sync::atomic::Ordering::Relaxed) != *expected
            })
        {
            let error = Diagnostic::new("E_CANCELLED", "Compilation was superseded", file, loc);
            state.interrupted = Some(error.clone());
            return Err(error);
        }
        if state.deadline.is_some() {
            state.checkpoints = state.checkpoints.wrapping_add(1);
            if state.checkpoints % 128 == 0
                && state
                    .deadline
                    .zip(state.clock)
                    .is_some_and(|(deadline, clock)| clock() >= deadline)
            {
                let error =
                    Diagnostic::new("E_YIELD", "Compilation time slice completed", file, loc);
                state.interrupted = Some(error.clone());
                return Err(error);
            }
        }
    }
    Ok(())
}
