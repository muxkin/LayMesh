//! Page transitions reuse the pinned GL Transitions catalog. CSS is configuration;
//! the presentation renderer executes the upstream SkSL effect.
use crate::{Diagnostic, Loc, Result, animation::values, model::*};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::LazyLock};

pub static CATALOG: LazyLock<Vec<Value>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../assets/transitions.json")).unwrap()
});

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageTransition {
    pub name: String,
    pub duration_ms: f64,
    pub easing: values::Easing,
    pub params: BTreeMap<String, Value>,
    pub textures: BTreeMap<String, String>,
    // Compile-time assets become ordinary display-list resources. They are
    // removed from inspection and the playback metadata after packaging.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub assets: BTreeMap<String, Value>,
}

pub fn supported(key: &str) -> bool {
    let key = key.replace('-', "_");
    matches!(key.as_str(), "_laymesh_transition" | "_laymesh_transition_duration" | "_laymesh_transition_timing_function")
        || key.starts_with("_laymesh_transition_param_")
        || key.starts_with("_laymesh_transition_texture_")
}

pub fn compile(args: &Args, file: &str, loc: Loc) -> Result<Option<PageTransition>> {
    let fail = |message: String| Diagnostic::new("E_TRANSITION", message, file, loc);
    let name = string(args, "_laymesh_transition", "none");
    if name == "none" { return Ok(None); }
    let entry = CATALOG.iter().find(|v| v["name"] == name)
        .ok_or_else(|| fail(format!("Unknown page transition: {name}")))?;
    let duration = values::time(&string(args, "_laymesh_transition_duration", "700ms"))
        .map_err(fail)? * 1000.;
    if !duration.is_finite() || duration < 0. { return Err(fail("Transition duration must be finite and nonnegative".into())); }
    let easing = values::Easing::parse(&string(args, "_laymesh_transition_timing_function", "ease-in-out")).map_err(fail)?;
    let mut params: BTreeMap<String, Value> = serde_json::from_value(entry["defaultParams"].clone()).unwrap();
    let mut textures = BTreeMap::new();
    let mut assets = BTreeMap::new();
    for (key, value) in args {
        if let Some(suffix) = key.strip_prefix("_laymesh_transition_param_") {
            let css = suffix.replace('_', "-");
            let name = entry["cssParams"][&css].as_str().ok_or_else(|| fail(format!("Unknown parameter {css} for {}", entry["name"])))?;
            let kind = entry["paramsTypes"][name].as_str().unwrap();
            let raw = value.as_str();
            let parsed = if kind == "bool" {
                match raw.as_str() { "true" => json!(true), "false" => json!(false), _ => return Err(fail(format!("{css} requires true or false"))) }
            } else {
                let n = if kind.contains("vec") { kind.chars().last().unwrap().to_digit(10).unwrap() as usize } else { 1 };
                let numbers = raw.replace(',', " ").split_whitespace().map(|s| s.parse::<f64>().map_err(|_| format!("Invalid transition parameter number: {s}"))).collect::<std::result::Result<Vec<_>, _>>().map_err(fail)?;
                if numbers.len() != n || numbers.iter().any(|v| !v.is_finite() || (kind == "int" || kind.starts_with("ivec")) && (v.fract() != 0. || *v < i32::MIN as f64 || *v > i32::MAX as f64)) {
                    return Err(fail(format!("{css} requires {n} finite {kind} component(s)")));
                }
                if n == 1 { json!(numbers[0]) } else { json!(numbers) }
            };
            params.insert(name.into(), parsed);
        }
        if let Some(suffix) = key.strip_prefix("_laymesh_transition_texture_") {
            let css = suffix.replace('_', "-");
            let name = entry["cssTextures"][&css].as_str().ok_or_else(|| fail(format!("Unknown texture {css} for {}", entry["name"])))?;
            let asset = value.json();
            if !asset["data"].is_string() && !asset["rasterKey"].is_string() { return Err(fail(format!("{css} requires a local image url()"))); }
            if asset["mime"] == "image/svg+xml" { return Err(fail(format!("{css} requires a raster image (PNG, JPEG, WebP, TIFF); SVG textures are not supported"))); }
            textures.insert(name.into(), name.into());
            assets.insert(name.into(), asset);
        }
    }
    for required in entry["textures"].as_array().into_iter().flatten().filter_map(Value::as_str) {
        if !textures.contains_key(required) { return Err(fail(format!("{name} requires texture {required}; configure -laymesh-transition-texture-* with a local image url()"))); }
    }
    Ok(Some(PageTransition {name, duration_ms: duration, easing, params, textures, assets}))
}
