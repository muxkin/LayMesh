//! Browser bindings compile retained scenes and sample animation for CanvasKit.
use laymesh_core::{
    engine::compile_preview_source,
    model::{Host, Scene},
};
use laymesh_language::{LanguageService, byte_offset, utf16_offset};
use serde_json::{Value as J, json};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use std::sync::Arc;
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn rendering_memory_stats() -> Vec<f64> {
    laymesh_core::wasm_allocator::allocation_stats()
        .map(|v| v as f64)
        .to_vec()
}
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace=performance,js_name=now)]
    fn performance_now() -> f64;
}
thread_local! {
    static LANGUAGE_SERVICE: RefCell<LanguageService> = RefCell::new(LanguageService::new("en"));
    static PREVIEW_FONTS: RefCell<BTreeMap<String, Arc<Vec<u8>>>> = RefCell::new(BTreeMap::new());
    static PREVIEW_ASSETS: RefCell<BTreeMap<String, Arc<Vec<u8>>>> = RefCell::new(BTreeMap::new());
    static DRAW_RESOURCES: RefCell<BTreeMap<String, std::sync::Arc<Vec<u8>>>> = RefCell::new(BTreeMap::new());
    static PREVIEW_DEPENDENCIES: Rc<RefCell<BTreeSet<String>>> = Rc::new(RefCell::new(BTreeSet::new()));
}

#[wasm_bindgen]
pub fn clear_preview_dependencies() {
    PREVIEW_DEPENDENCIES.with(|paths| paths.borrow_mut().clear());
}
#[wasm_bindgen]
pub fn preview_dependencies() -> String {
    PREVIEW_DEPENDENCIES.with(|paths| serde_json::to_string(&*paths.borrow()).unwrap())
}

#[wasm_bindgen]
pub fn register_render_resource(id: &str, bytes: &[u8]) {
    DRAW_RESOURCES.with(|resources| {
        resources
            .borrow_mut()
            .insert(id.into(), std::sync::Arc::new(bytes.to_vec()))
    });
}
#[wasm_bindgen]
pub fn retain_render_resources(ids: &str) -> Result<(), JsValue> {
    let ids: Vec<String> = serde_json::from_str(ids).map_err(js_error)?;
    DRAW_RESOURCES.with(|r| r.borrow_mut().retain(|id, _| ids.contains(id)));
    Ok(())
}
fn js_error(e: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&e.to_string())
}

#[wasm_bindgen]
pub struct DrawJob {
    scene: laymesh_render::retained::RetainedScene,
    source: Option<(Arc<Scene>, bool)>,
}
#[wasm_bindgen]
impl DrawJob {
    pub fn presentation_state(&self, time: f64, reduced: bool) -> Result<String, JsValue> {
        serde_json::to_string(
            &self
                .scene
                .sample_presentation(time, reduced)
                .map_err(js_error)?,
        )
        .map_err(js_error)
    }
    pub fn frame_state(&self, time: f64, reduced: bool) -> Result<String, JsValue> {
        serde_json::to_string(&self.scene.sample(time, reduced, false).map_err(js_error)?)
            .map_err(js_error)
    }
    pub fn transform_state(&self, time: f64, reduced: bool) -> Result<String, JsValue> {
        serde_json::to_string(
            &self
                .scene
                .sample_transforms(time, reduced, false)
                .map_err(js_error)?,
        )
        .map_err(js_error)
    }
    pub fn resource_bytes(&self, id: &str) -> Result<Vec<u8>, JsValue> {
        self.scene
            .base()
            .resources
            .get(id)
            .map(|r| r.data.as_ref().clone())
            .ok_or_else(|| js_error("Missing drawing resource"))
    }
    /// Workers already own the drawing; the UI only needs its resource headers.
    /// Keep display_list() available for consumers that need complete commands.
    pub fn animation_sample(&self, time: f64, reduced: bool) -> Result<String, JsValue> {
        if !time.is_finite() || time < 0. {
            return Err(js_error("Invalid animation time"));
        }
        serde_json::to_string(
            &self
                .scene
                .base()
                .animation
                .as_ref()
                .map(|a| a.timeline(reduced).sample(time)),
        )
        .map_err(js_error)
    }
    pub fn drawing_info(&self) -> Result<String, JsValue> {
        let list = self.scene.base();
        let resources: BTreeMap<_, _> = list
            .resources
            .iter()
            .map(|(id, r)| {
                (
                    id,
                    json!({"mime":r.mime,"fontIndex":r.font_index,"family":r.family}),
                )
            })
            .collect();
        serde_json::to_string(&json!({"schemaVersion":list.schema_version,"width":list.width,"height":list.height,"resources":resources,"media":list.media,"animation":list.animation,"pageTransition":list.page_transition})).map_err(js_error)
    }
    /// Updates are accepted only after the client validates their baseline ID.
    pub fn update_display_list(&mut self, json: &str) -> Result<(), JsValue> {
        let mut update: laymesh_render::display_update::DisplayUpdate =
            serde_json::from_str(json).map_err(js_error)?;
        if let Some(resources) = &mut update.resources {
            resolve_resources(resources)?;
        }
        self.scene.update(update).map_err(js_error)?;
        self.source = None;
        Ok(())
    }
    pub fn display_list(&self) -> Result<String, JsValue> {
        let mut list = self.scene.base().clone();
        for resource in list.resources.values_mut() {
            resource.data = Default::default();
        }
        serde_json::to_string(&list).map_err(js_error)
    }
}
#[wasm_bindgen]
pub fn prepare_display_list(json: &str) -> Result<DrawJob, JsValue> {
    let mut list: laymesh_render::DisplayList = serde_json::from_str(json).map_err(js_error)?;
    if !laymesh_render::display_list::supported_version(list.schema_version) {
        return Err(js_error("Unsupported display list version"));
    }
    resolve_resources(&mut list.resources)?;
    Ok(DrawJob {
        scene: laymesh_render::retained::RetainedScene::new(list),
        source: None,
    })
}
fn resolve_resources(
    list: &mut BTreeMap<String, laymesh_render::display_list::Resource>,
) -> Result<(), JsValue> {
    DRAW_RESOURCES.with(|resources| {
        let resources = resources.borrow();
        for (id, resource) in list {
            if resource.data.is_empty() {
                resource.data = resources
                    .get(id)
                    .cloned()
                    .ok_or_else(|| js_error(format!("Missing drawing resource {id}")))?;
            }
        }
        Ok::<(), JsValue>(())
    })
}

#[wasm_bindgen]
pub fn register_preview_asset(path: &str, bytes: &[u8]) {
    PREVIEW_ASSETS.with(|files| {
        let mut files = files.borrow_mut();
        if files.get(path).is_none_or(|old| old.as_slice() != bytes) {
            files.insert(path.into(), Arc::new(bytes.to_vec()));
        }
    });
}
#[wasm_bindgen]
pub fn clear_preview_assets() {
    PREVIEW_ASSETS.with(|files| files.borrow_mut().clear());
}

#[wasm_bindgen]
pub fn describe_font(bytes: &[u8]) -> String {
    serde_json::to_string(&laymesh_core::text::font_metadata(bytes)).unwrap()
}
#[wasm_bindgen]
pub fn decode_image(bytes: &[u8], path: &str) -> Result<String, JsValue> {
    laymesh_core::assets::load(bytes, path, path, laymesh_core::Loc::default())
        .and_then(|v| {
            serde_json::to_string(&v).map_err(|e| {
                laymesh_core::Diagnostic::new(
                    "E_IMAGE",
                    e.to_string(),
                    path,
                    laymesh_core::Loc::default(),
                )
            })
        })
        .map_err(js_error)
}
/// Register Page-only body fonts as bytes, avoiding a large JSON byte array on every render.
#[wasm_bindgen]
pub fn register_preview_font(filename: &str, bytes: &[u8]) {
    PREVIEW_FONTS.with(|fonts| {
        fonts
            .borrow_mut()
            .insert(filename.to_owned(), Arc::new(bytes.to_vec()));
    });
}
/// A scene remains in WASM memory between layout and SVG generation, avoiding
/// serialization of embedded fonts just to measure the two stages separately.
#[wasm_bindgen]
pub struct RenderJob {
    scene: Arc<Scene>,
    document: Option<J>,
}
/// A document plan is evaluated once per presentation snapshot. Page compilation
/// remains explicit so callers can report progress and cancel between pages.
#[wasm_bindgen]
pub struct PresentationPlan {
    plan: laymesh_core::document::DocumentPlan,
    host: Host,
}
#[wasm_bindgen]
impl PresentationPlan {
    pub fn metadata(&self) -> String {
        json!({"name":self.plan.name,"pages":self.plan.pages}).to_string()
    }
    pub fn page(&self, index: usize, files_json: &str) -> Result<RenderJob, JsValue> {
        let mut host = self.host.clone();
        let extra = render_host(files_json)?;
        host.files.extend(extra.files);
        host.shared_files.extend(extra.shared_files);
        let page = self.plan.page(index + 1).map_err(diagnostic_js_error)?;
        let scene = laymesh_core::document::compile_preview_page(page, host)
            .map_err(diagnostic_js_error)?;
        Ok(RenderJob {
            scene: Arc::new(scene),
            document: Some(
                json!({"name":self.plan.name,"pages":self.plan.pages,"pageId":page.id,"pageNumber":index+1}),
            ),
        })
    }
}
#[wasm_bindgen]
pub fn prepare_presentation(
    source: &str,
    filename: &str,
    files_json: &str,
) -> Result<PresentationPlan, JsValue> {
    let host = render_host(files_json)?;
    let plan = laymesh_core::document::plan_source(source, filename, host.clone())
        .map_err(diagnostic_js_error)?;
    Ok(PresentationPlan { plan, host })
}
#[wasm_bindgen]
impl RenderJob {
    pub fn draw_job_outlined(&self) -> Result<DrawJob, JsValue> {
        let mut list = laymesh_render::to_display_list(&self.scene).map_err(js_error)?;
        laymesh_render::display_list::outline_nodes(&mut list.nodes);
        Ok(DrawJob {
            scene: laymesh_render::retained::RetainedScene::new(list),
            source: Some((self.scene.clone(), true)),
        })
    }
    pub fn update_draw_job_incremental(
        &self,
        draw: &mut DrawJob,
        outlined: bool,
    ) -> Result<String, JsValue> {
        let mut update = if let Some((previous, _)) =
            draw.source.as_ref().filter(|(_, mode)| *mode == outlined)
        {
            laymesh_render::display_list::display_update_reusing(
                &self.scene,
                previous,
                draw.scene.base(),
                outlined,
            )
            .map_err(js_error)?
        } else {
            let mut next = laymesh_render::to_display_list(&self.scene).map_err(js_error)?;
            if outlined {
                laymesh_render::display_list::outline_nodes(&mut next.nodes);
            }
            laymesh_render::display_update::DisplayUpdate::between_incremental(
                draw.scene.base(),
                &next,
            )
        };
        let resources = update.resources.take();
        let mut value = serde_json::to_value(&update).map_err(js_error)?;
        if let Some(resources) = &resources {
            value["resources"] = resources
                .iter()
                .map(|(id, r)| {
                    (
                        id.clone(),
                        json!({"mime":r.mime,"data":[],"fontIndex":r.font_index,"family":r.family}),
                    )
                })
                .collect::<serde_json::Map<_, _>>()
                .into();
        }
        update.resources = resources;
        draw.scene.update(update).map_err(js_error)?;
        draw.source = Some((self.scene.clone(), outlined));
        Ok(value.to_string())
    }

    pub fn update_draw_job(&self, draw: &mut DrawJob) -> Result<(), JsValue> {
        let next = laymesh_render::to_display_list(&self.scene).map_err(js_error)?;
        let update =
            laymesh_render::display_update::DisplayUpdate::between(draw.scene.base(), &next);
        draw.scene.update(update).map_err(js_error)?;
        draw.source = Some((self.scene.clone(), false));
        Ok(())
    }
    pub fn draw_job(&self) -> Result<DrawJob, JsValue> {
        Ok(DrawJob {
            source: Some((self.scene.clone(), false)),
            scene: laymesh_render::retained::RetainedScene::new(
                laymesh_render::to_display_list(&self.scene).map_err(js_error)?,
            ),
        })
    }
    pub fn display_list(&self) -> Result<String, JsValue> {
        serde_json::to_string(&laymesh_render::to_display_list(&self.scene).map_err(js_error)?)
            .map_err(js_error)
    }
    pub fn svg(&self) -> Result<String, JsValue> {
        laymesh_render::render_svg(&self.scene).map_err(|e| JsValue::from_str(&e.to_string()))
    }
    pub fn inspection(&self) -> String {
        let mut value = json!({"inspection":laymesh_language::inspect::inspect_scene(&self.scene),"warnings":self.scene.warnings,"requestedFonts":self.scene.font_requests});
        if let Some(document) = &self.document {
            value["document"] = document.clone();
        }
        value.to_string()
    }
}
#[wasm_bindgen]
pub fn prepare_render(
    source: &str,
    filename: &str,
    files_json: &str,
) -> Result<RenderJob, JsValue> {
    let host = render_host(files_json)?;
    let scene = compile_preview_source(source, filename, host).map_err(diagnostic_js_error)?;
    Ok(RenderJob {
        scene: Arc::new(scene),
        document: None,
    })
}
#[wasm_bindgen]
pub fn prepare_document_render(
    source: &str,
    filename: &str,
    files_json: &str,
    page_id: &str,
) -> Result<RenderJob, JsValue> {
    let host = render_host(files_json)?;
    let plan = laymesh_core::document::plan_source(source, filename, host.clone())
        .map_err(diagnostic_js_error)?;
    let mut document = json!({"name":plan.name,"pages":plan.pages});
    let number = if page_id.is_empty() {
        1
    } else {
        plan.pages
            .iter()
            .position(|p| p.id == page_id)
            .ok_or_else(|| {
                JsValue::from_str(
                    &json!({"code":"E_PAGE","file":filename,
                    "message":"Selected page ID no longer exists","document":document})
                    .to_string(),
                )
            })?
            + 1
    };
    let page = plan.page(number).map_err(diagnostic_js_error)?;
    document["pageId"] = json!(page.id);
    document["pageNumber"] = json!(number);
    let scene = laymesh_core::document::compile_preview_page(page, host).map_err(|e| {
        let mut error = diagnostic_value(e);
        error["document"] = document.clone();
        JsValue::from_str(&error.to_string())
    })?;
    Ok(RenderJob {
        scene: Arc::new(scene),
        document: Some(document),
    })
}
fn render_host(files_json: &str) -> Result<Host, JsValue> {
    let files: J =
        serde_json::from_str(files_json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let mut host = Host {
        native: false,
        dependencies: Some(PREVIEW_DEPENDENCIES.with(Rc::clone)),
        ..Host::default()
    };
    if let Some(files) = files.as_object() {
        for (name, value) in files {
            let data = if let Some(s) = value.as_str() {
                s.as_bytes().to_vec()
            } else if let Some(a) = value.as_array() {
                a.iter().map(|v| v.as_u64().unwrap_or(0) as u8).collect()
            } else {
                continue;
            };
            host.files.insert(name.clone(), data);
        }
    }
    PREVIEW_ASSETS.with(|files| {
        host.shared_files
            .extend(files.borrow().iter().map(|(k, v)| (k.clone(), v.clone())))
    });
    PREVIEW_FONTS.with(|fonts| {
        host.shared_files.extend(
            fonts
                .borrow()
                .iter()
                .map(|(name, data)| (name.clone(), data.clone())),
        );
    });
    Ok(host)
}
fn diagnostic_js_error(e: laymesh_core::Diagnostic) -> JsValue {
    JsValue::from_str(&diagnostic_value(e).to_string())
}
fn diagnostic_value(e: laymesh_core::Diagnostic) -> J {
    let mut diagnostic = serde_json::to_value(&e).unwrap();
    // Native errors remain unchanged. This extra field is a browser resource
    // request; the worker only honors paths in its local resource manifest.
    if e.code == "E_ASSET" {
        if let Some(path) = e.message.strip_prefix("无法读取资源：") {
            diagnostic["resource"] = json!(path);
        }
    }
    diagnostic
}
#[cfg(test)]
mod document_tests {
    use super::*;
    #[test]
    fn wasm_document_selection_matches_native_core_and_retained_commands() {
        let source = "d=document()\nfor p in [page(src=\"a.lay\",id=\"a\"),page(src=\"b.lay\",id=\"b\")] { d.add(p) }";
        let files=json!({"/a.lay":"p=canvas(size=(10,20))\np.add(rect(size=(4,4),fill=\"#f00\"))","/b.lay":"p=canvas(size=(2,1),unit=\"cm\")\np.add(rect(size=(0.4,0.4),fill=\"#00f\"))"}).to_string();
        let job = prepare_document_render(source, "/report.laydoc", &files, "b").unwrap();
        let info: J = serde_json::from_str(&job.inspection()).unwrap();
        assert_eq!(info["document"]["pageNumber"], 2);
        assert_eq!(info["inspection"]["page"]["unit"], "cm");
        let host = render_host(&files).unwrap();
        let plan =
            laymesh_core::document::plan_source(source, "/report.laydoc", host.clone()).unwrap();
        let scene = laymesh_core::document::compile_page(plan.page(2).unwrap(), host).unwrap();
        let list = laymesh_render::to_display_list(&scene).unwrap();
        let job = prepare_display_list(&serde_json::to_string(&list).unwrap()).unwrap();
        let commands: laymesh_render::DisplayList =
            serde_json::from_str(&job.display_list().unwrap()).unwrap();
        assert_eq!(commands.nodes, list.nodes);
        assert!(commands.resources.values().all(|r| r.data.is_empty()));
        for (id, r) in &list.resources {
            assert_eq!(
                job.resource_bytes(id).unwrap().as_slice(),
                r.data.as_slice()
            );
        }
    }
}
#[wasm_bindgen]
pub fn render(source: &str, filename: &str, files_json: &str) -> Result<String, JsValue> {
    let job = prepare_render(source, filename, files_json)?;
    let mut result: J = serde_json::from_str(&job.inspection()).unwrap();
    result["svg"] = json!(job.svg()?);
    Ok(result.to_string())
}
#[wasm_bindgen]
pub fn color_convert(input: &str) -> Result<String, JsValue> {
    let v: J = serde_json::from_str(input).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let c = laymesh_core::color::Color::new(
        v["space"].as_str().unwrap_or("rgb"),
        [
            v["channels"][0].as_f64().unwrap_or(f64::NAN),
            v["channels"][1].as_f64().unwrap_or(f64::NAN),
            v["channels"][2].as_f64().unwrap_or(f64::NAN),
        ],
        v["alpha"].as_f64().unwrap_or(1.),
    )
    .map_err(|e| JsValue::from_str(&e))?;
    Ok(json!({"rgba":c.rgba,"mapped":c.mapped,"hex":c.hex(),"rgb":c.channels_in("rgb"),"hsv":c.channels_in("hsv"),"oklch":c.channels_in("oklch")}).to_string())
}
#[wasm_bindgen]
pub fn color_presentations(
    files: &str,
    filename: &str,
    from: usize,
    to: usize,
    rgba: &str,
) -> Result<String, JsValue> {
    let files: BTreeMap<String, String> =
        serde_json::from_str(files).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let values: [f64; 4] =
        serde_json::from_str(rgba).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let mut service = LanguageService::new("en");
    service.documents = files;
    let source = service
        .documents
        .get(filename)
        .map(String::as_str)
        .unwrap_or("");
    let mut result = service.color_presentations(
        filename,
        byte_offset(source, from),
        byte_offset(source, to),
        values,
    );
    for v in result.as_array_mut().unwrap() {
        v["from"] = json!(from);
        v["to"] = json!(to);
    }
    Ok(result.to_string())
}
#[wasm_bindgen]
pub fn language_query(
    files_json: &str,
    filename: &str,
    method: &str,
    utf16: usize,
    locale: &str,
) -> Result<String, JsValue> {
    let files: BTreeMap<String, String> =
        serde_json::from_str(files_json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    LANGUAGE_SERVICE.with(|cached| {
        let mut service = cached.borrow_mut();
        service.locale = locale.into();
        service.documents = files;
        let source = service
            .documents
            .get(filename)
            .map(String::as_str)
            .unwrap_or("");
        let mut result = service.query(method, filename, byte_offset(source, utf16));
        fn convert(v: &mut J, source: &str, files: &BTreeMap<String, String>) {
            match v {
                J::Array(a) => {
                    for x in a {
                        convert(x, source, files)
                    }
                }
                J::Object(m) => {
                    let own = m
                        .get("uri")
                        .and_then(J::as_str)
                        .and_then(|u| {
                            files
                                .get(u)
                                .map(String::as_str)
                                .or_else(|| laymesh_language::stylesheets::content(u))
                        })
                        .unwrap_or(source);
                    for key in ["from", "to"] {
                        if let Some(n) = m.get(key).and_then(J::as_u64) {
                            m.insert(key.into(), json!(utf16_offset(own, n as usize)));
                        }
                    }
                    for (key, value) in m.iter_mut() {
                        if key != "from" && key != "to" {
                            convert(value, own, files);
                        }
                    }
                }
                _ => {}
            }
        }
        convert(&mut result, source, &service.documents);
        Ok(result.to_string())
    })
}

/// Shared semantic compilation kernel, retained for the worker lifetime.
#[wasm_bindgen]
pub struct CompileSession {
    inner: laymesh_core::session::CompileSession,
    files: Host,
    plan: Option<laymesh_core::document::DocumentPlan>,
    entry: String,
}
#[wasm_bindgen]
impl CompileSession {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        let mut inner = laymesh_core::session::CompileSession::new(false);
        inner.set_dependencies(Some(PREVIEW_DEPENDENCIES.with(Rc::clone)));
        Self {
            inner,
            files: Host::default(),
            plan: None,
            entry: String::new(),
        }
    }
    pub fn apply_changes(&mut self, revision: f64, changes: &str) -> Result<(), JsValue> {
        if !revision.is_finite()
            || revision.fract() != 0.
            || !(0. ..=9_007_199_254_740_991.).contains(&revision)
        {
            return Err(js_error("Invalid source version"));
        }
        let changes: BTreeMap<String, Option<String>> =
            serde_json::from_str(changes).map_err(js_error)?;
        let mut files = self.files.clone();
        for (path, value) in changes {
            match value {
                Some(value) => {
                    files
                        .shared_files
                        .insert(path, Arc::new(value.into_bytes()));
                }
                None => {
                    files.shared_files.remove(&path);
                }
            }
        }
        let resources = render_host("{}")?;
        let mut host = files.clone();
        host.shared_files.extend(resources.shared_files);
        host.dependencies = resources.dependencies;
        self.inner
            .sync_host(&host, revision as u64)
            .map_err(diagnostic_js_error)?;
        self.files = files;
        self.plan = None;
        Ok(())
    }
    pub fn set_time_slice(&mut self, milliseconds: f64) {
        self.inner.set_profile_clock(performance_now);
        self.inner
            .set_deadline((milliseconds > 0.).then(|| performance_now() + milliseconds));
    }
    pub fn set_budget(&mut self, bytes: usize) {
        self.inner.set_budget(bytes);
    }
    pub fn set_profiling(&mut self, enabled: bool) {
        self.inner.set_profile_clock(performance_now);
        self.inner.set_profiling(enabled);
    }
    pub fn metadata(&mut self, entry: &str) -> Result<String, JsValue> {
        self.entry = entry.into();
        self.plan = if entry.ends_with(".laydoc") {
            let source = self.inner.source(entry).map_err(diagnostic_js_error)?;
            Some(
                self.inner
                    .plan(&source, entry)
                    .map_err(diagnostic_js_error)?,
            )
        } else {
            None
        };
        Ok(if let Some(plan) = &self.plan {
            json!({"name":plan.name,"pages":plan.pages})
        } else {
            json!({"pages":[{"id":"page","name":entry.rsplit('/').next().unwrap_or(entry)}]})
        }
        .to_string())
    }
    pub fn set_active_page(&mut self, page_id: &str) {
        let file = self
            .plan
            .as_ref()
            .and_then(|p| p.pages.iter().find(|p| p.id == page_id))
            .map_or(self.entry.as_str(), |p| p.src.as_str());
        self.inner.set_focus_file(file);
    }
    pub fn evaluate(&mut self, page_id: &str) -> Result<RenderJob, JsValue> {
        let (file, transition, document) = if let Some(plan) = &self.plan {
            let index = plan
                .pages
                .iter()
                .position(|p| p.id == page_id)
                .ok_or_else(|| js_error("Selected page no longer exists"))?;
            let page = &plan.pages[index];
            (
                page.src.clone(),
                page.transition.clone(),
                Some(
                    json!({"name":plan.name,"pages":plan.pages,"pageId":page.id,"pageNumber":index+1}),
                ),
            )
        } else {
            (self.entry.clone(), None, None)
        };
        let scene = if let Some(plan) = &self.plan {
            self.inner
                .evaluate_page(plan.pages.iter().find(|p| p.id == page_id).unwrap())
        } else {
            self.inner.evaluate_file(&file, transition)
        }
        .map_err(diagnostic_js_error)?;
        Ok(RenderJob { scene, document })
    }
    pub fn metrics(&self) -> String {
        serde_json::to_string(&self.inner.metrics()).unwrap()
    }
    pub fn snapshot(&self, file: &str) -> Option<RenderJob> {
        self.inner.snapshot(file).map(|(_, scene)| RenderJob {
            scene,
            document: None,
        })
    }
    pub fn dispose(&mut self) {
        self.inner.dispose();
        self.files.shared_files.clear();
        self.plan = None;
    }
}
#[wasm_bindgen]
pub fn remove_preview_asset(path: &str) {
    PREVIEW_ASSETS.with(|files| files.borrow_mut().remove(path));
}

/// Shared object-local 3D rasterizer; the host supplies its 2D subject texture.
#[wasm_bindgen]
pub fn raster_surface3d(
    mesh: &str,
    texture: &[u8],
    tw: u32,
    th: u32,
    region: &[f64],
    width: u32,
    height: u32,
) -> Result<Vec<u8>, JsValue> {
    let mesh: laymesh_render::surface3d::Mesh = serde_json::from_str(mesh).map_err(js_error)?;
    let region: [f64; 4] = region
        .try_into()
        .map_err(|_| js_error("Invalid surface region"))?;
    laymesh_render::surface3d::raster(&mesh, texture, tw, th, region, width, height)
        .map_err(js_error)
}

#[wasm_bindgen]
pub fn surface_texture_region(
    mesh: &str,
    region: &[f64],
    density: f64,
) -> Result<Vec<f64>, JsValue> {
    let mesh: laymesh_render::surface3d::Mesh = serde_json::from_str(mesh).map_err(js_error)?;
    let region: [f64; 4] = region
        .try_into()
        .map_err(|_| js_error("Invalid surface region"))?;
    Ok(
        laymesh_render::surface3d::texture_region(&mesh, region, density)
            .map_or_else(Vec::new, |r| r.to_vec()),
    )
}

/// Rebuild curved object geometry for the actual projected pixel density.
#[wasm_bindgen]
pub fn refine_surface3d(mesh: &str, density: f64) -> Result<String, JsValue> {
    let mesh: laymesh_render::surface3d::Mesh = serde_json::from_str(mesh).map_err(js_error)?;
    let mesh = laymesh_render::surface3d::refine(&mesh, density).map_err(js_error)?;
    serde_json::to_string(&mesh).map_err(js_error)
}
#[wasm_bindgen]
pub fn surface_material_texture(
    texture: &[u8],
    coverage: &[u8],
    width: u32,
    height: u32,
    material_alpha: Option<f64>,
) -> Result<Vec<u8>, JsValue> {
    laymesh_render::surface3d::material_texture(texture, coverage, width, height, material_alpha)
        .map_err(js_error)
}
