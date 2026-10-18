//! Playback-only bindings. No source compiler, language service or font shaping exports.
use serde_json::json;
use std::{cell::RefCell, collections::BTreeMap};
use wasm_bindgen::prelude::*;
thread_local! {
    static DRAW_RESOURCES: RefCell<BTreeMap<String, std::sync::Arc<Vec<u8>>>> = RefCell::new(BTreeMap::new());
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
        self.scene.update(update).map_err(js_error)
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
    if list.schema_version != laymesh_render::display_list::DISPLAY_LIST_VERSION {
        return Err(js_error("Unsupported display list version"));
    }
    resolve_resources(&mut list.resources)?;
    Ok(DrawJob {
        scene: laymesh_render::retained::RetainedScene::new(list),
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
