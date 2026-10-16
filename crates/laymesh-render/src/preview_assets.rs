//! Full-resolution display derivatives with immutable image/font resources.
use super::*;
use crate::{config::PreviewSettings, raster_policy};
use image::DynamicImage;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
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
fn process_images(tasks: &[ImageTask], settings: &PreviewSettings) -> Result<()> {
    let next = AtomicUsize::new(0);
    let failure = Mutex::new(None);
    // Only owned pixels and encoding settings reach these workers, never compiler Rc/RefCell state.
    std::thread::scope(|scope| {
        for _ in 0..settings.threads().min(tasks.len()) {
            let tasks = &tasks;
            let next = &next;
            let failure = &failure;
            scope.spawn(move || {
                loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some(task) = tasks.get(index) else {
                        break;
                    };
                    let result = raster_policy::preview_bytes(
                        &task.pixels,
                        settings.jpeg_quality,
                        settings.webp_quality,
                        settings.webp_method,
                    )
                    .and_then(|(_, bytes)| write_resource(&task.path, &bytes));
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
    Ok(())
}
fn remap_node_resources(
    nodes: &mut [crate::display_list::DrawNode],
    ids: &HashMap<String, String>,
) {
    use crate::display_list::{DrawKind, DrawNode, Paint};
    fn paint(p: &mut Paint, ids: &HashMap<String, String>) {
        if let Paint::Pattern { nodes, .. } = p {
            visit(nodes, ids);
        }
    }
    fn visit(nodes: &mut [DrawNode], ids: &HashMap<String, String>) {
        for n in nodes {
            match &mut n.kind {
                DrawKind::Image { resource, .. } => {
                    if let Some(id) = ids.get(resource) {
                        *resource = id.clone();
                    }
                }
                DrawKind::Group { children, mask, .. } => {
                    visit(children, ids);
                    if let Some(mask) = mask {
                        visit(std::slice::from_mut(&mut mask.node), ids);
                    }
                }
                DrawKind::Path { fill, .. } => {
                    paint(fill, ids);
                }
                DrawKind::Glyphs { .. } => {}
            }
        }
    }
    visit(nodes, ids);
}
fn externalize_resources(
    values: &mut std::collections::BTreeMap<String, crate::display_list::Resource>,
    directory: &Path,
    settings: &PreviewSettings,
) -> Result<(Vec<Json>, HashMap<String, String>)> {
    let mut resources = Vec::new();
    let mut ids = HashMap::new();
    let mut tasks = Vec::new();
    let mut registered = std::collections::BTreeMap::new();
    for (original, mut resource) in std::mem::take(values) {
        let id = if resource.data.is_empty()
            && (!original.starts_with("pixels-") || original.contains("-preview-q"))
        {
            original
        } else if resource.mime.starts_with("image/") {
            let id = format!(
                "{original}-preview-q{}-wq{}-m{}",
                settings.jpeg_quality, settings.webp_quality, settings.webp_method
            );
            let path = directory.join(format!("{id}.resource"));
            // Immutable derivatives include their encoding settings in the
            // key. Recover MIME from the file header on warm refreshes without
            // decoding the original full-resolution image again.
            if path.is_file() {
                use std::io::Read;
                let mut header = [0; 12];
                let mut file = std::fs::File::open(&path).map_err(|e| error(e.to_string()))?;
                file.read_exact(&mut header)
                    .map_err(|e| error(e.to_string()))?;
                resource.mime = if &header[..4] == b"RIFF" && &header[8..] == b"WEBP" {
                    "image/webp"
                } else if header[..2] == [0xff, 0xd8] {
                    "image/jpeg"
                } else {
                    return Err(error("无效预览图片缓存"));
                }
                .into();
            } else {
                let pixels = if let Some(key) = original.strip_prefix("pixels-") {
                    laymesh_core::asset_cache::pixels(key)
                        .ok_or_else(|| error("预览图片缓存已过期"))?
                } else {
                    Arc::new(
                        image::load_from_memory(&resource.data)
                            .map_err(|e| error(e.to_string()))?,
                    )
                };
                resource.mime = if raster_policy::has_transparency(&pixels) {
                    "image/webp"
                } else {
                    "image/jpeg"
                }
                .into();
                tasks.push(ImageTask { pixels, path });
            }
            ids.insert(original, id.clone());
            id
        } else {
            write_resource(
                &directory.join(format!("{original}.resource")),
                &resource.data,
            )?;
            original
        };
        let path = directory.join(format!("{id}.resource"));
        resources.push(json!({"id":id,"path":path,"mime":resource.mime,"fontIndex":resource.font_index,"family":resource.family}));
        resource.data = Default::default();
        registered.insert(id, resource);
    }
    process_images(&tasks, settings)?;
    *values = registered;
    Ok((resources, ids))
}
/// Lower changed scene contributions and externalize only the resulting patch.
/// Unchanged command trees remain borrowed from the acknowledged baseline.
pub fn display_update_frame(
    scene: &Scene,
    previous_scene: &Scene,
    previous: &DisplayList,
    directory: &Path,
    settings: &PreviewSettings,
) -> Result<(crate::display_update::DisplayUpdate, Vec<Json>)> {
    use crate::display_update::ChildSource;
    let mut update =
        crate::display_list::preview_display_update_reusing(scene, previous_scene, previous)?;
    let (resources, ids) = if let Some(values) = &mut update.resources {
        externalize_resources(values, directory, settings)?
    } else {
        (vec![], HashMap::new())
    };
    if !ids.is_empty() {
        if let Some(roots) = &mut update.roots {
            remap_node_resources(roots, &ids);
        }
        for change in &mut update.nodes {
            remap_node_resources(std::slice::from_mut(&mut change.node), &ids);
        }
        for change in &mut update.structure {
            for child in &mut change.insert {
                if let ChildSource::New(node) = child {
                    remap_node_resources(std::slice::from_mut(node), &ids);
                }
            }
        }
        fn remap(value: &mut Json, ids: &HashMap<String, String>) {
            match value {
                Json::String(s) => {
                    if let Some(id) = ids.get(s) {
                        *s = id.clone();
                    }
                }
                Json::Array(a) => {
                    for v in a {
                        remap(v, ids)
                    }
                }
                Json::Object(o) => {
                    for v in o.values_mut() {
                        remap(v, ids)
                    }
                }
                _ => {}
            }
        }
        for property in &mut update.properties {
            for (key, value) in &mut property.values {
                if key == "kind.resource" || key == "kind.fill" {
                    remap(value, &ids);
                }
            }
        }
    }
    Ok((update, resources))
}

/// Protocol 3 carries drawing commands, with immutable encoded resources beside them.
pub fn display_frame(
    scene: &Scene,
    directory: &Path,
    settings: &PreviewSettings,
) -> Result<(DisplayList, Vec<Json>)> {
    let mut list = crate::display_list::preview_display_list(scene)?;
    let (resources, ids) = externalize_resources(&mut list.resources, directory, settings)?;
    remap_node_resources(&mut list.nodes, &ids);
    Ok((list, resources))
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
        visit(
            std::sync::Arc::make_mut(node),
            directory,
            settings,
            &mut resources,
            &mut tasks,
        )?;
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
    process_images(&tasks, settings)?;
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
