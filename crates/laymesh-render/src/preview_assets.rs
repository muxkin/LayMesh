//! Full-resolution display derivatives with immutable image/font resources.
use super::*;
use crate::{config::PreviewSettings, raster_policy};
use image::DynamicImage;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};
#[derive(Clone)]
struct ImageTask {
    pixels: Arc<DynamicImage>,
    path: PathBuf,
}
pub fn write_resource(path: &Path, bytes: &[u8]) -> Result<()> {
    if path.is_file() {
        return Ok(());
    }
    let temporary = path.with_extension(format!(
        "{}.tmp",
        std::thread::current().name().unwrap_or("foreground")
    ));
    std::fs::write(&temporary, bytes).map_err(|e| error(e.to_string()))?;
    if path.is_file() {
        let _ = std::fs::remove_file(temporary);
        return Ok(());
    }
    std::fs::rename(&temporary, path).map_err(|e| error(e.to_string()))
}
pub fn frame(
    scene: &Scene,
    directory: &Path,
    settings: &PreviewSettings,
) -> Result<(String, Vec<Json>)> {
    fn image(
        n: &mut Json,
        dir: &Path,
        s: &PreviewSettings,
        resources: &mut HashMap<String, Json>,
        tasks: &mut HashMap<String, ImageTask>,
    ) -> Result<()> {
        if n["mime"] == "image/svg+xml" {
            return Ok(());
        }
        let pixels = raster_policy::pixels(n)?;
        let key = n["rasterKey"]
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| laymesh_core::asset_cache::insert(pixels.as_ref().clone()));
        let alpha = raster_policy::has_transparency(&pixels);
        let id = format!(
            "{key}-full-q{}-wq{}-m{}",
            s.jpeg_quality, s.webp_quality, s.webp_method
        );
        let path = dir.join(format!("{id}.{}", if alpha { "webp" } else { "jpg" }));
        resources.insert(id.clone(),json!({"id":id,"path":path,"mime":if alpha {"image/webp"}else{"image/jpeg"},"rasterKey":key,"width":pixels.width(),"height":pixels.height()}));
        if !path.is_file() {
            tasks
                .entry(id.clone())
                .or_insert(ImageTask { pixels, path });
        }
        n["resourceUri"] = json!(format!("laymesh-resource:{id}"));
        n["previewAsset"] = json!(key);
        n["data"] = json!("");
        Ok(())
    }
    fn visit(
        n: &mut Json,
        dir: &Path,
        s: &PreviewSettings,
        r: &mut HashMap<String, Json>,
        t: &mut HashMap<String, ImageTask>,
    ) -> Result<()> {
        if n["kind"] == "image" {
            image(n, dir, s, r, t)?;
        }
        for key in ["fill", "stroke"] {
            if n[key]["mime"].is_string() {
                image(&mut n[key], dir, s, r, t)?;
            }
        }
        if let Some(children) = n["children"].as_array_mut() {
            for child in children {
                visit(child, dir, s, r, t)?;
            }
        }
        Ok(())
    }
    let mut resources = HashMap::new();
    let mut tasks = HashMap::new();
    let mut scene = scene.clone();
    for node in &mut scene.nodes {
        visit(node, directory, settings, &mut resources, &mut tasks)?;
    }
    if scene.background["mime"].is_string() {
        image(
            &mut scene.background,
            directory,
            settings,
            &mut resources,
            &mut tasks,
        )?;
    }
    let tasks: Vec<_> = tasks.into_values().collect();
    let next = AtomicUsize::new(0);
    let memory = (Mutex::new(0usize), Condvar::new());
    let failure = Mutex::new(None);
    let budget = settings.processing_memory_mb * 1024 * 1024;
    // Only owned pixels and encoding settings reach these workers, never compiler Rc/RefCell state.
    std::thread::scope(|scope| {
        for _ in 0..settings.threads().min(tasks.len()) {
            let tasks = &tasks;
            let next = &next;
            let memory = &memory;
            let failure = &failure;
            scope.spawn(move || {
                loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some(task) = tasks.get(index) else {
                        break;
                    };
                    let estimate = task.pixels.as_bytes().len().saturating_mul(3);
                    let mut used = memory.0.lock().unwrap();
                    while *used > 0 && used.saturating_add(estimate) > budget {
                        used = memory.1.wait(used).unwrap();
                    }
                    *used = used.saturating_add(estimate);
                    drop(used);
                    let result = raster_policy::preview_bytes(
                        &task.pixels,
                        settings.jpeg_quality,
                        settings.webp_quality,
                        settings.webp_method,
                    )
                    .and_then(|(_, bytes)| write_resource(&task.path, &bytes));
                    let mut used = memory.0.lock().unwrap();
                    *used = used.saturating_sub(estimate);
                    memory.1.notify_all();
                    drop(used);
                    if let Err(e) = result {
                        *failure.lock().unwrap() = Some(e);
                    }
                }
            });
        }
    });
    if let Some(e) = failure.into_inner().unwrap() {
        return Err(e);
    }
    let mut css = String::new();
    for (key, (weight, italic)) in used_fonts(&scene) {
        if let Some(font) = scene.fonts.get(&key) {
            let id = format!("font-{}", laymesh_core::asset_cache::digest(&font.data));
            let path = directory.join(format!("{id}.font"));
            write_resource(&path, &font.data)?;
            resources.insert(id.clone(), json!({"id":id,"path":path,"mime":"font/ttf"}));
            css.push_str(&format!("@font-face{{font-family:'{}';font-weight:{weight};font-style:{};src:url(laymesh-resource:{id})}}",escape(&key),if italic {"italic"}else{"normal"}));
        }
    }
    let svg = svg(&scene, false)?.replacen("</defs>", &format!("<style>{css}</style></defs>"), 1);
    Ok((svg, resources.into_values().collect()))
}
