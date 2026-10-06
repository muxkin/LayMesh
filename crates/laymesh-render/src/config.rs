//! Read-only defaults resolution shared by every native entrypoint.
use super::*;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PreviewSettings {
    pub jpeg_quality: u8,
    pub webp_quality: u8,
    pub webp_method: u8,
    pub image_threads: usize,
    pub cache_mb: usize,
    pub processing_memory_mb: usize,
}
impl Default for PreviewSettings {
    fn default() -> Self {
        Self {
            jpeg_quality: 90,
            webp_quality: 90,
            webp_method: 0,
            image_threads: 0,
            cache_mb: 256,
            processing_memory_mb: 128,
        }
    }
}
impl PreviewSettings {
    pub fn validate(&self) -> Result<()> {
        if !(1..=100).contains(&self.jpeg_quality)
            || self.webp_quality > 100
            || self.webp_method > 6
            || self.image_threads > 32
            || !(1..=65536).contains(&self.cache_mb)
            || !(1..=65536).contains(&self.processing_memory_mb)
        {
            return Err(error("Invalid preview settings"));
        }
        Ok(())
    }
    pub fn threads(&self) -> usize {
        if self.image_threads > 0 {
            self.image_threads
        } else {
            std::thread::available_parallelism()
                .map_or(1, |v| v.get().saturating_sub(1).clamp(1, 2))
        }
    }
}
pub fn global_path() -> Option<PathBuf> {
    if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA").map(|v| PathBuf::from(v).join("laymesh/config.json"))
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME")
            .map(|v| PathBuf::from(v).join("Library/Application Support/laymesh/config.json"))
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|v| PathBuf::from(v).join(".config")))
            .map(|v| v.join("laymesh/config.json"))
    }
}
pub fn merge(base: &mut Json, next: &Json) {
    if let (Some(b), Some(n)) = (base.as_object_mut(), next.as_object()) {
        for (key, value) in n {
            merge(b.entry(key).or_insert(Json::Null), value);
        }
    } else {
        *base = next.clone();
    }
}
fn read(path: &Path) -> Result<Json> {
    let raw = std::fs::read(path).map_err(|e| error(format!("{}: {e}", path.display())))?;
    let value: Json =
        serde_json::from_slice(&raw).map_err(|e| error(format!("{}: {e}", path.display())))?;
    if !value.is_object()
        || value
            .as_object()
            .unwrap()
            .keys()
            .any(|k| !matches!(k.as_str(), "export" | "preview"))
    {
        return Err(error(format!(
            "{}: expected export/preview configuration",
            path.display()
        )));
    }
    Ok(value)
}
pub fn resolve(
    file: &Path,
    explicit: Option<&Path>,
    editor_user: &Json,
    editor_workspace: &Json,
) -> Result<(Json, Vec<String>)> {
    let mut value = json!({});
    let mut dependencies = vec![];
    if let Some(path) = global_path() {
        dependencies.push(path.to_string_lossy().into_owned());
        if path.is_file() {
            merge(&mut value, &read(&path)?);
        }
    }
    if editor_user.is_object() {
        merge(&mut value, editor_user);
    }
    if let Some(path) = explicit {
        dependencies.push(path.to_string_lossy().into_owned());
        merge(&mut value, &read(path)?);
    } else {
        for directory in file.parent().unwrap_or(Path::new(".")).ancestors() {
            let path = directory.join(".laymesh.json");
            dependencies.push(path.to_string_lossy().into_owned());
            if path.is_file() {
                merge(&mut value, &read(&path)?);
                break;
            }
        }
    }
    if editor_workspace.is_object() {
        merge(&mut value, editor_workspace);
    }
    Ok((value, dependencies))
}
pub fn export_options(
    config: &Json,
    extension: &str,
    explicit: &ExportOptions,
) -> Result<ExportOptions> {
    explicit.validate(extension)?;
    let format = export_format(extension)?;
    let mut value = json!({});
    let common = &config["export"];
    if !common.is_null() && !common.is_object() {
        return Err(error("export must be an object"));
    }
    if common.as_object().is_some_and(|v| {
        v.keys().any(|k| {
            !matches!(
                k.as_str(),
                "dpi"
                    | "svg"
                    | "pdf"
                    | "pptx"
                    | "png"
                    | "jpeg"
                    | "tiff"
                    | "webp"
                    | "bmp"
                    | "gif"
                    | "ico"
                    | "pam"
                    | "ppm"
                    | "pgm"
                    | "pbm"
                    | "tga"
            )
        })
    }) {
        return Err(error("Unknown export configuration key"));
    }
    if format != "svg" {
        if let Some(dpi) = common.get("dpi") {
            value["dpi"] = dpi.clone();
        }
    }
    if let Some(per_format) = common.get(format) {
        if !per_format.is_object() {
            return Err(error("Format defaults must be an object"));
        }
        merge(&mut value, per_format);
    }
    let mut explicit = serde_json::to_value(explicit).unwrap();
    explicit
        .as_object_mut()
        .unwrap()
        .retain(|_, v| !v.is_null());
    merge(&mut value, &explicit);
    let options: ExportOptions = serde_json::from_value(value).map_err(|e| error(e.to_string()))?;
    options.validate(extension)?;
    Ok(options)
}
pub fn preview_settings(config: &Json, explicit: &Json) -> Result<PreviewSettings> {
    let mut value = serde_json::to_value(PreviewSettings::default()).unwrap();
    if !config["preview"].is_null() {
        merge(&mut value, &config["preview"]);
    }
    if explicit.is_object() {
        merge(&mut value, explicit);
    }
    let settings: PreviewSettings =
        serde_json::from_value(value).map_err(|e| error(e.to_string()))?;
    settings.validate()?;
    Ok(settings)
}

pub fn fingerprint(config: &Json) -> String {
    let builtins = json!({"dpi":1200,"pdf_image_compression":"auto","pdf_jpeg_quality":90,"pdf_downsample":true,"pdf_recompress_jpeg":false,"pdf_preserve_16bit":true,"pdf_preserve_alpha":true,"pdf_alpha_background":"#ffffff","pdf_auto_palette_limit":32,"pdf_auto_flatness_threshold":0.9,"preview":PreviewSettings::default(),"schema":2});
    laymesh_core::asset_cache::digest(
        json!({"builtins":builtins,"config":config})
            .to_string()
            .as_bytes(),
    )
}
