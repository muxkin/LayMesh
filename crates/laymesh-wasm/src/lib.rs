//! Browser bindings share native layout and CPU drawing; SVG is an export format.
use laymesh_core::{
    engine::compile_preview_source,
    model::{Host, Scene},
};
use laymesh_language::{LanguageService, byte_offset, utf16_offset};
use serde_json::{Value as J, json};
use std::cell::RefCell;
use std::collections::BTreeMap;
use wasm_bindgen::prelude::*;
thread_local! {
    static PREVIEW_FONTS: RefCell<BTreeMap<String, Vec<u8>>> = RefCell::new(BTreeMap::new());
    static DRAW_RESOURCES: RefCell<BTreeMap<String, Vec<u8>>> = RefCell::new(BTreeMap::new());
}

#[wasm_bindgen]
pub fn register_render_resource(id: &str, bytes: &[u8]) {
    DRAW_RESOURCES.with(|resources| resources.borrow_mut().insert(id.into(), bytes.to_vec()));
}
#[wasm_bindgen]
pub fn set_render_cache_budget(mib: usize) {
    laymesh_render::raster::set_image_cache_budget(mib);
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
    raster: laymesh_render::raster::PreparedRaster,
}
#[wasm_bindgen]
pub struct RgbaFrame {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}
#[wasm_bindgen]
impl RgbaFrame {
    #[wasm_bindgen(getter)]
    pub fn width(&self) -> u32 {
        self.width
    }
    #[wasm_bindgen(getter)]
    pub fn height(&self) -> u32 {
        self.height
    }
    #[wasm_bindgen(getter)]
    pub fn rgba(&self) -> Vec<u8> {
        self.rgba.clone()
    }
    /// Consume this frame for Canvas/worker transfer without cloning its
    /// complete pixel buffer. The reusable getter remains available.
    pub fn into_rgba(self) -> Vec<u8> {
        self.rgba
    }
}
#[wasm_bindgen]
impl DrawJob {
    pub fn display_list(&self) -> Result<String, JsValue> {
        let mut list = self.raster.display_list().clone();
        for resource in list.resources.values_mut() {
            resource.data.clear();
        }
        serde_json::to_string(&list).map_err(js_error)
    }
    /// Millimetre viewport; output dimensions include browser device pixel ratio.
    pub fn raster(
        &mut self,
        viewport: &str,
        width: u32,
        height: u32,
    ) -> Result<RgbaFrame, JsValue> {
        let r: [f64; 4] = serde_json::from_str(viewport).map_err(js_error)?;
        let result = self
            .raster
            .render_region(kurbo::Rect::new(r[0], r[1], r[2], r[3]), width, height)
            .map_err(js_error)?;
        Ok(RgbaFrame {
            width: result.width,
            height: result.height,
            rgba: result.rgba,
        })
    }
    pub fn set_cache_budget(&mut self, mib: usize) {
        self.raster.set_cache_budget(mib);
    }
}
#[wasm_bindgen]
pub fn prepare_display_list(json: &str) -> Result<DrawJob, JsValue> {
    let mut list: laymesh_render::DisplayList = serde_json::from_str(json).map_err(js_error)?;
    if list.schema_version != laymesh_render::display_list::DISPLAY_LIST_VERSION {
        return Err(js_error("Unsupported display list version"));
    }
    DRAW_RESOURCES.with(|resources| {
        for (id, resource) in &mut list.resources {
            if resource.data.is_empty() {
                resource.data = resources
                    .borrow()
                    .get(id)
                    .cloned()
                    .ok_or_else(|| js_error(format!("Missing drawing resource {id}")))?;
            }
        }
        Ok::<(), JsValue>(())
    })?;
    Ok(DrawJob {
        raster: laymesh_render::raster::PreparedRaster::new(list).map_err(js_error)?,
    })
}

/// Register Page-only body fonts as bytes, avoiding a large JSON byte array on every render.
#[wasm_bindgen]
pub fn register_preview_font(filename: &str, bytes: &[u8]) {
    PREVIEW_FONTS.with(|fonts| {
        fonts
            .borrow_mut()
            .insert(filename.to_owned(), bytes.to_vec());
    });
}
/// A scene remains in WASM memory between layout and SVG generation, avoiding
/// serialization of embedded fonts just to measure the two stages separately.
#[wasm_bindgen]
pub struct RenderJob {
    scene: Scene,
}
#[wasm_bindgen]
impl RenderJob {
    pub fn draw_job(&self) -> Result<DrawJob, JsValue> {
        Ok(DrawJob {
            raster: laymesh_render::raster::PreparedRaster::new(
                laymesh_render::to_display_list(&self.scene).map_err(js_error)?,
            )
            .map_err(js_error)?,
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
        json!({"inspection":laymesh_language::inspect::inspect_scene(&self.scene),"warnings":self.scene.warnings}).to_string()
    }
}
#[wasm_bindgen]
pub fn prepare_render(
    source: &str,
    filename: &str,
    files_json: &str,
) -> Result<RenderJob, JsValue> {
    let files: J =
        serde_json::from_str(files_json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let mut host = Host {
        native: false,
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
    PREVIEW_FONTS.with(|fonts| {
        host.files.extend(
            fonts
                .borrow()
                .iter()
                .map(|(name, data)| (name.clone(), data.clone())),
        );
    });
    let scene = compile_preview_source(source, filename, host).map_err(|e| {
        let mut diagnostic = serde_json::to_value(&e).unwrap();
        // Native errors remain unchanged. This extra field is a browser resource
        // request; the worker only honors paths in its local resource manifest.
        if e.code == "E_ASSET" {
            if let Some(path) = e.message.strip_prefix("无法读取资源：") {
                diagnostic["resource"] = json!(path);
            }
        }
        JsValue::from_str(&diagnostic.to_string())
    })?;
    Ok(RenderJob { scene })
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
    let mut service = LanguageService::new(locale);
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
                    .and_then(|u| files.get(u))
                    .map(String::as_str)
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
}
